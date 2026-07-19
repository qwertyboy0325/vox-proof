use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::PathBuf;

use rusqlite::{Connection, params};
use serde_json;

use super::super::adapter::{
    AdapterError, AuthoritativeCommand, CandidateCapabilities, DuplicatedSession,
    EvidenceSessionHandle, EvidenceSessionRef, MaintenanceOperation, OptionalCapability,
    OptionalOperationOutcome, PersistenceCandidateAdapter, SemanticOpenMode,
};
use super::super::fixture::EvidenceFixture;
use super::super::model::NormalizedSemanticState;
use super::super::scenario::ScenarioIdentity;
use super::fault::{FaultPoint, FaultRegistry};
use super::semantic_ops::apply_command;

pub const CURRENT_FORMAT_VERSION: u32 = 1;
const DERIVED_INDEX_KEY: &str = "queue-index-v1";

pub struct EmbeddedRelationalAdapter {
    storage_root: PathBuf,
    faults: FaultRegistry,
    handles: RefCell<BTreeMap<String, (String, SemanticOpenMode)>>,
    next_handle: RefCell<u64>,
}

impl EmbeddedRelationalAdapter {
    pub fn new(storage_root: impl Into<PathBuf>) -> Self {
        let storage_root = storage_root.into();
        fs::create_dir_all(&storage_root).expect("storage root must be creatable");
        Self {
            storage_root,
            faults: FaultRegistry::default(),
            handles: RefCell::new(BTreeMap::new()),
            next_handle: RefCell::new(0),
        }
    }

    fn session_dir(&self, session_id: &str) -> PathBuf {
        self.storage_root.join(session_id)
    }

    fn db_path(locator: &str) -> PathBuf {
        PathBuf::from(locator).join("session.db")
    }

    fn open_connection(locator: &str) -> Result<Connection, AdapterError> {
        Connection::open(Self::db_path(locator))
            .map_err(|error| AdapterError::new("sqlite-open-failed", error.to_string()))
    }

    fn read_meta(connection: &Connection, key: &str) -> Result<Option<String>, AdapterError> {
        connection
            .query_row(
                "SELECT value FROM session_meta WHERE key = ?1",
                [key],
                |row| row.get(0),
            )
            .optional()
            .map_err(|error| AdapterError::new("sqlite-read-meta", error.to_string()))
    }

    fn load_state(connection: &Connection) -> Result<NormalizedSemanticState, AdapterError> {
        let json: String = connection
            .query_row(
                "SELECT state_json FROM canonical_state WHERE id = 1",
                [],
                |row| row.get(0),
            )
            .map_err(|error| AdapterError::new("sqlite-load-state", error.to_string()))?;
        let state: NormalizedSemanticState = serde_json::from_str(&json)
            .map_err(|error| AdapterError::new("state-deserialize-failed", error.to_string()))?;
        Ok(state.normalize())
    }

    fn save_state(
        connection: &Connection,
        state: &NormalizedSemanticState,
    ) -> Result<(), AdapterError> {
        let json = serde_json::to_string(&state.clone().normalize())
            .map_err(|error| AdapterError::new("state-serialize-failed", error.to_string()))?;
        connection
            .execute(
                "UPDATE canonical_state SET state_json = ?1 WHERE id = 1",
                [json],
            )
            .map_err(|error| AdapterError::new("sqlite-save-state", error.to_string()))?;
        Ok(())
    }

    fn initialize_session(
        connection: &Connection,
        state: &NormalizedSemanticState,
    ) -> Result<(), AdapterError> {
        connection
            .execute_batch(
                "CREATE TABLE session_meta (key TEXT PRIMARY KEY, value TEXT NOT NULL);
                 CREATE TABLE canonical_state (id INTEGER PRIMARY KEY CHECK (id = 1), state_json TEXT NOT NULL);
                 CREATE TABLE derived_cache (key TEXT PRIMARY KEY, value TEXT NOT NULL);
                 CREATE TABLE writer_lock (id INTEGER PRIMARY KEY CHECK (id = 1), token TEXT);",
            )
            .map_err(|error| AdapterError::new("sqlite-init-schema", error.to_string()))?;
        let meta = [
            ("session_id", state.session.session_id.as_str()),
            (
                "duplicated_from",
                state
                    .session
                    .duplicated_from_session_id
                    .as_deref()
                    .unwrap_or(""),
            ),
            ("format_version", &CURRENT_FORMAT_VERSION.to_string()),
            (
                "session_format_version",
                state.session_format_version.as_str(),
            ),
            (
                "interpretation_version",
                state.interpretation_version.as_str(),
            ),
        ];
        for (key, value) in meta {
            connection
                .execute(
                    "INSERT INTO session_meta (key, value) VALUES (?1, ?2)",
                    params![key, value],
                )
                .map_err(|error| AdapterError::new("sqlite-init-meta", error.to_string()))?;
        }
        let json = serde_json::to_string(&state.clone().normalize())
            .map_err(|error| AdapterError::new("state-serialize-failed", error.to_string()))?;
        connection
            .execute(
                "INSERT INTO canonical_state (id, state_json) VALUES (1, ?1)",
                [json],
            )
            .map_err(|error| AdapterError::new("sqlite-init-state", error.to_string()))?;
        connection
            .execute(
                "INSERT INTO derived_cache (key, value) VALUES (?1, ?2)",
                params![DERIVED_INDEX_KEY, "derived-index-v1"],
            )
            .map_err(|error| AdapterError::new("sqlite-init-derived", error.to_string()))?;
        connection
            .execute("INSERT INTO writer_lock (id, token) VALUES (1, NULL)", [])
            .map_err(|error| AdapterError::new("sqlite-init-lock", error.to_string()))?;
        Ok(())
    }

    fn format_version(connection: &Connection) -> Result<u32, AdapterError> {
        let value = Self::read_meta(connection, "format_version")?
            .ok_or_else(|| AdapterError::new("missing-format-version", "format version missing"))?;
        value
            .parse::<u32>()
            .map_err(|error| AdapterError::new("invalid-format-version", error.to_string()))
    }

    fn validate_format_for_mode(
        connection: &Connection,
        mode: SemanticOpenMode,
    ) -> Result<(), AdapterError> {
        let version = Self::format_version(connection)?;
        if version > CURRENT_FORMAT_VERSION && mode == SemanticOpenMode::Writable {
            return Err(AdapterError::new(
                "unsupported-newer-format",
                "unknown newer session format cannot open writable",
            ));
        }
        if version < CURRENT_FORMAT_VERSION && mode == SemanticOpenMode::Writable {
            return Err(AdapterError::new(
                "unsupported-older-format",
                "writable open requires migration for older format",
            ));
        }
        Ok(())
    }

    fn acquire_writer(connection: &Connection, token: &str) -> Result<(), AdapterError> {
        let updated = connection
            .execute(
                "UPDATE writer_lock SET token = ?1 WHERE id = 1 AND token IS NULL",
                [token],
            )
            .map_err(|error| AdapterError::new("sqlite-writer-lock", error.to_string()))?;
        if updated == 0 {
            return Err(AdapterError::new(
                "writer-already-open",
                "embedded relational store permits one authoritative writer",
            ));
        }
        Ok(())
    }

    fn release_writer(connection: &Connection, token: &str) -> Result<(), AdapterError> {
        connection
            .execute(
                "UPDATE writer_lock SET token = NULL WHERE id = 1 AND token = ?1",
                [token],
            )
            .map_err(|error| AdapterError::new("sqlite-writer-unlock", error.to_string()))?;
        Ok(())
    }
}

impl PersistenceCandidateAdapter for EmbeddedRelationalAdapter {
    fn candidate_id(&self) -> &str {
        "embedded-relational-sqlite-spike"
    }

    fn candidate_version(&self) -> &str {
        "1"
    }

    fn capabilities(&self) -> CandidateCapabilities {
        CandidateCapabilities {
            optional: BTreeSet::from([OptionalCapability::Compaction]),
            limitations: vec![
                "spike-only SQLite layout; not production schema".to_string(),
                "destructive historical GC not implemented".to_string(),
            ],
        }
    }

    fn create(&mut self, fixture: &EvidenceFixture) -> Result<EvidenceSessionRef, AdapterError> {
        let state = fixture.normalized_state();
        let session_id = state.session.session_id.clone();
        let locator = self.session_dir(&session_id);
        if locator.exists() {
            return Err(AdapterError::new(
                "already-created",
                "session directory already exists",
            ));
        }
        fs::create_dir_all(&locator)
            .map_err(|error| AdapterError::new("filesystem-create-session", error.to_string()))?;
        let connection = Self::open_connection(locator.to_str().expect("utf8 path"))?;
        connection
            .execute("PRAGMA synchronous=FULL", [])
            .map_err(|error| AdapterError::new("sqlite-pragma", error.to_string()))?;
        connection
            .query_row("PRAGMA journal_mode=WAL", [], |row| row.get::<_, String>(0))
            .map_err(|error| AdapterError::new("sqlite-pragma-wal", error.to_string()))?;
        let tx = connection
            .unchecked_transaction()
            .map_err(|error| AdapterError::new("sqlite-tx-begin", error.to_string()))?;
        Self::initialize_session(&tx, &state)?;
        tx.commit()
            .map_err(|error| AdapterError::new("sqlite-create-commit", error.to_string()))?;
        Ok(EvidenceSessionRef::new(
            session_id,
            locator.to_string_lossy().to_string(),
        ))
    }

    fn open(
        &mut self,
        session: &EvidenceSessionRef,
        mode: SemanticOpenMode,
    ) -> Result<EvidenceSessionHandle, AdapterError> {
        let connection = Self::open_connection(session.adapter_locator())?;
        Self::validate_format_for_mode(&connection, mode)?;
        *self.next_handle.borrow_mut() += 1;
        let handle_id = format!("embedded-handle:{}", self.next_handle.borrow());
        if mode == SemanticOpenMode::Writable {
            Self::acquire_writer(&connection, &handle_id)?;
        }
        // Validate canonical state loads before acknowledging open.
        Self::load_state(&connection)?;
        self.handles.borrow_mut().insert(
            handle_id.clone(),
            (session.adapter_locator().to_string(), mode),
        );
        Ok(EvidenceSessionHandle::new(session.clone(), mode, handle_id))
    }

    fn close(&mut self, handle: &EvidenceSessionHandle) -> Result<(), AdapterError> {
        let Some((locator, mode)) = self.handles.borrow_mut().remove(handle.adapter_handle())
        else {
            return Err(AdapterError::new("not-open", "handle is not open"));
        };
        if mode == SemanticOpenMode::Writable {
            let connection = Self::open_connection(&locator)?;
            Self::release_writer(&connection, handle.adapter_handle())?;
        }
        Ok(())
    }

    fn apply_authoritative_command(
        &mut self,
        handle: &EvidenceSessionHandle,
        command: &AuthoritativeCommand,
    ) -> Result<(), AdapterError> {
        if handle.mode != SemanticOpenMode::Writable {
            return Err(AdapterError::new(
                "not-authoritative-writer",
                "command requires writable handle",
            ));
        }
        let locator = handle.session.adapter_locator();
        let connection = Self::open_connection(locator)?;
        if self
            .faults
            .take_if_armed(FaultPoint::FailBeforeDurabilityCommit)
            .is_some()
        {
            return Err(AdapterError::new(
                "simulated-durability-failure",
                "logical fault injected before durability commit",
            ));
        }
        let tx = connection
            .unchecked_transaction()
            .map_err(|error| AdapterError::new("sqlite-tx-begin", error.to_string()))?;
        let mut state = Self::load_state(&tx)?;
        apply_command(&mut state, command)?;
        Self::save_state(&tx, &state)?;
        tx.commit()
            .map_err(|error| AdapterError::new("sqlite-commit-failed", error.to_string()))?;
        Ok(())
    }

    fn read_normalized_state(
        &self,
        handle: &EvidenceSessionHandle,
    ) -> Result<NormalizedSemanticState, AdapterError> {
        if !self.handles.borrow().contains_key(handle.adapter_handle()) {
            return Err(AdapterError::new("not-open", "handle is not open"));
        }
        let connection = Self::open_connection(handle.session.adapter_locator())?;
        Self::load_state(&connection)
    }

    fn attempt_read_only_open(
        &mut self,
        session: &EvidenceSessionRef,
    ) -> Result<EvidenceSessionHandle, AdapterError> {
        self.open(session, SemanticOpenMode::ReadOnly)
    }

    fn duplicate_session(
        &mut self,
        source: &EvidenceSessionHandle,
        new_session_id: &str,
    ) -> Result<DuplicatedSession, AdapterError> {
        let state = self.read_normalized_state(source)?;
        let mut duplicate_state = state.clone();
        duplicate_state.session.duplicated_from_session_id =
            Some(duplicate_state.session.session_id.clone());
        duplicate_state.session.session_id = new_session_id.to_string();
        let locator = self.session_dir(new_session_id);
        if locator.exists() {
            return Err(AdapterError::new(
                "duplicate-session-exists",
                "duplicate session identity already exists",
            ));
        }
        fs::create_dir_all(&locator)
            .map_err(|error| AdapterError::new("filesystem-create-session", error.to_string()))?;
        fs::copy(
            Self::db_path(source.session.adapter_locator()),
            Self::db_path(locator.to_str().expect("utf8 path")),
        )
        .map_err(|error| AdapterError::new("filesystem-copy-session", error.to_string()))?;
        let connection = Self::open_connection(locator.to_str().expect("utf8 path"))?;
        connection
            .execute(
                "UPDATE session_meta SET value = ?1 WHERE key = 'session_id'",
                [new_session_id],
            )
            .map_err(|error| AdapterError::new("sqlite-update-session-id", error.to_string()))?;
        connection
            .execute(
                "UPDATE session_meta SET value = ?1 WHERE key = 'duplicated_from'",
                [state.session.session_id.as_str()],
            )
            .map_err(|error| AdapterError::new("sqlite-update-lineage", error.to_string()))?;
        Self::save_state(&connection, &duplicate_state)?;
        connection
            .execute("UPDATE writer_lock SET token = NULL WHERE id = 1", [])
            .map_err(|error| AdapterError::new("sqlite-reset-lock", error.to_string()))?;
        Ok(DuplicatedSession {
            session: EvidenceSessionRef::new(new_session_id, locator.to_string_lossy().to_string()),
            normalized_state: duplicate_state.normalize(),
        })
    }

    fn corrupt_or_fault_inject(
        &mut self,
        session: &EvidenceSessionRef,
        scenario: &ScenarioIdentity,
    ) -> Result<(), AdapterError> {
        match scenario.scenario_id.as_str() {
            "derived-state-corruption" => {
                self.faults.arm_for_scenario(
                    scenario,
                    FaultPoint::CorruptDerivedArtifact,
                    super::fault::FaultLayer::Logical,
                );
                let connection = Self::open_connection(session.adapter_locator())?;
                connection
                    .execute(
                        "UPDATE derived_cache SET value = 'corrupted' WHERE key = ?1",
                        [DERIVED_INDEX_KEY],
                    )
                    .map_err(|error| {
                        AdapterError::new("sqlite-corrupt-derived", error.to_string())
                    })?;
            }
            "canonical-reference-corruption" => {
                let connection = Self::open_connection(session.adapter_locator())?;
                connection
                    .execute(
                        "UPDATE canonical_state SET state_json = ?1 WHERE id = 1",
                        ["{invalid-canonical-json"],
                    )
                    .map_err(|error| {
                        AdapterError::new("sqlite-corrupt-canonical", error.to_string())
                    })?;
            }
            "unknown-newer-format" => {
                let connection = Self::open_connection(session.adapter_locator())?;
                connection
                    .execute(
                        "UPDATE session_meta SET value = ?1 WHERE key = 'format_version'",
                        [(CURRENT_FORMAT_VERSION + 1).to_string()],
                    )
                    .map_err(|error| {
                        AdapterError::new("sqlite-set-newer-format", error.to_string())
                    })?;
            }
            "interrupted-authoritative-transition" => {
                self.faults.arm_for_scenario(
                    scenario,
                    FaultPoint::FailBeforeDurabilityCommit,
                    super::fault::FaultLayer::Logical,
                );
            }
            "interrupted-compaction" => {
                self.faults.arm_for_scenario(
                    scenario,
                    FaultPoint::InterruptCompaction,
                    super::fault::FaultLayer::Logical,
                );
            }
            _ => {}
        }
        Ok(())
    }

    fn cleanup_or_compact_if_supported(
        &mut self,
        handle: &EvidenceSessionHandle,
        operation: MaintenanceOperation,
    ) -> Result<OptionalOperationOutcome, AdapterError> {
        let capability = operation.required_capability();
        if !self.capabilities().supports(capability) {
            return Ok(OptionalOperationOutcome::Unsupported {
                capability,
                limitation: "operation not supported by embedded relational spike".to_string(),
            });
        }
        if self
            .faults
            .take_if_armed(FaultPoint::InterruptCompaction)
            .is_some()
        {
            return Err(AdapterError::new(
                "simulated-compaction-interrupt",
                "compaction interrupted before completion",
            ));
        }
        let connection = Self::open_connection(handle.session.adapter_locator())?;
        connection
            .execute("VACUUM", [])
            .map_err(|error| AdapterError::new("sqlite-vacuum", error.to_string()))?;
        Ok(OptionalOperationOutcome::Completed)
    }
}

trait OptionalRow<T> {
    fn optional(self) -> Result<Option<T>, rusqlite::Error>;
}

impl<T> OptionalRow<T> for Result<T, rusqlite::Error> {
    fn optional(self) -> Result<Option<T>, rusqlite::Error> {
        match self {
            Ok(value) => Ok(Some(value)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(error) => Err(error),
        }
    }
}
