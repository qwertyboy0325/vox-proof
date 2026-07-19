//! SQLite embedded-relational persistence candidate (Package 2B).
//!
//! Fingerprint v1 hashes command_kind, command_operation_id, and canonical
//! command_payload only. Post-commit verification uses an independent state
//! oracle; the fingerprint is stable across later unrelated authoritative commands.

use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use rusqlite::{Connection, TransactionBehavior, params};
use serde_json::{self, Value};
use sha2::{Digest, Sha256};
use uuid::Uuid;

use super::super::adapter::{
    AdapterError, AuthoritativeCommand, CandidateCapabilities, DuplicatedSession,
    EvidenceSessionHandle, EvidenceSessionRef, MaintenanceOperation, OptionalCapability,
    OptionalOperationOutcome, PersistenceCandidateAdapter, SemanticOpenMode, SemanticPrecondition,
};
use super::super::fixture::EvidenceFixture;
use super::super::model::NormalizedSemanticState;
use super::super::oracle::SemanticOracle;
use super::super::scenario::ScenarioIdentity;
use super::fault::{FaultExecutionMode, FaultPoint, FaultRegistry, handle_fault};
use super::semantic_ops::apply_command;

pub const CURRENT_FORMAT_VERSION: u32 = 1;
const DERIVED_INDEX_KEY: &str = "queue-index-v1";
const DERIVED_SCHEMA_VERSION: i64 = 1;
const DEFAULT_LEASE_DURATION_MS: i64 = 30_000;
const OUTCOME_FINGERPRINT_VERSION: u32 = 1;

#[derive(Debug)]
struct WritableHandleState {
    connection: Connection,
    token: String,
    owner_epoch: i64,
}

#[derive(Debug)]
enum HandleState {
    ReadOnly { locator: String },
    Writable(WritableHandleState),
}

pub struct EmbeddedRelationalAdapter {
    storage_root: PathBuf,
    faults: FaultRegistry,
    handles: RefCell<BTreeMap<String, HandleState>>,
    next_handle: RefCell<u64>,
    process_instance_id: String,
    test_clock_ms: RefCell<Option<i64>>,
    last_clock_ms: RefCell<Option<i64>>,
    force_post_commit_verify_failure: RefCell<bool>,
    force_sqlite_busy_on_tx_begin_once: RefCell<bool>,
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
            process_instance_id: new_process_instance_id(),
            test_clock_ms: RefCell::new(None),
            last_clock_ms: RefCell::new(None),
            force_post_commit_verify_failure: RefCell::new(false),
            force_sqlite_busy_on_tx_begin_once: RefCell::new(false),
        }
    }

    /// Test-only clock override for lease expiry scenarios.
    pub fn set_test_clock_ms(&self, millis: Option<i64>) {
        *self.test_clock_ms.borrow_mut() = millis;
    }

    /// Test-only hook to simulate post-commit verify failure (BT-016).
    pub fn set_force_post_commit_verify_failure(&self, enabled: bool) {
        *self.force_post_commit_verify_failure.borrow_mut() = enabled;
    }

    /// Test-only hook to simulate one SQLITE_BUSY before the command transaction begins (BT-015).
    pub fn set_force_sqlite_busy_on_tx_begin_once(&self, enabled: bool) {
        *self.force_sqlite_busy_on_tx_begin_once.borrow_mut() = enabled;
    }

    pub fn arm_test_fault(&self, point: super::fault::FaultPoint) {
        self.faults.arm_for_test(point);
    }

    pub fn arm_fault_abort(&self, point: FaultPoint) {
        self.faults
            .arm_point(point, FaultExecutionMode::ProcessAbort);
    }

    pub fn arm_fault_return_error(&self, point: FaultPoint) {
        self.faults
            .arm_point(point, FaultExecutionMode::ReturnError);
    }

    pub fn set_fault_execution_mode(&self, mode: FaultExecutionMode) {
        self.faults.set_execution_mode(mode);
    }

    pub fn storage_root(&self) -> &Path {
        &self.storage_root
    }

    /// Load canonical state from persisted session.db via a fresh connection (independent reopen path).
    pub fn load_persisted_state(locator: &str) -> Result<NormalizedSemanticState, AdapterError> {
        let connection = Self::open_connection(locator)?;
        Self::integrity_check(&connection)?;
        Self::validate_format_for_read_only(&connection)?;
        let state = Self::load_canonical_state(&connection)?;
        Self::validate_or_rebuild_derived_cache(&connection)?;
        Ok(state)
    }

    pub fn observe_writer_ownership(
        locator: &str,
    ) -> Result<(Option<String>, i64, i64), AdapterError> {
        let connection = Self::open_connection(locator)?;
        connection
            .query_row(
                "SELECT token, owner_epoch, lease_expires_at_unix_ms FROM writer_ownership WHERE id = 1",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .map_err(map_sqlite_error("writer-ownership-observe"))
    }

    pub fn observe_applied_command(
        locator: &str,
        command_operation_id: &str,
    ) -> Result<Option<(String, String)>, AdapterError> {
        let connection = Self::open_connection(locator)?;
        connection
            .query_row(
                "SELECT command_kind, outcome_status FROM applied_authoritative_commands WHERE command_operation_id = ?1",
                [command_operation_id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()
            .map_err(map_sqlite_error("command-observe"))
    }

    fn session_dir(&self, session_id: &str) -> PathBuf {
        self.storage_root.join(session_id)
    }

    fn db_path(locator: &str) -> PathBuf {
        PathBuf::from(locator).join("session.db")
    }

    fn now_unix_ms(&self) -> Result<i64, AdapterError> {
        if let Some(clock) = *self.test_clock_ms.borrow() {
            if let Some(last) = *self.last_clock_ms.borrow()
                && clock < last
            {
                return Err(AdapterError::new(
                    "lease-clock-ambiguous",
                    "backward wall-clock jump is ambiguous; fail closed",
                ));
            }
            *self.last_clock_ms.borrow_mut() = Some(clock);
            return Ok(clock);
        }
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_millis() as i64)
            .map_err(|error| AdapterError::new("clock-read-failed", error.to_string()))?;
        if let Some(last) = *self.last_clock_ms.borrow()
            && now < last
        {
            return Err(AdapterError::new(
                "lease-clock-ambiguous",
                "backward wall-clock jump is ambiguous; fail closed",
            ));
        }
        *self.last_clock_ms.borrow_mut() = Some(now);
        Ok(now)
    }

    fn open_connection(locator: &str) -> Result<Connection, AdapterError> {
        let connection = Connection::open(Self::db_path(locator))
            .map_err(|error| AdapterError::new("sqlite-open-failed", error.to_string()))?;
        Self::apply_pragmas(&connection)?;
        Ok(connection)
    }

    fn apply_pragmas(connection: &Connection) -> Result<(), AdapterError> {
        connection
            .execute("PRAGMA synchronous=FULL", [])
            .map_err(|error| AdapterError::new("sqlite-pragma", error.to_string()))?;
        connection
            .query_row("PRAGMA journal_mode=WAL", [], |row| row.get::<_, String>(0))
            .map_err(|error| AdapterError::new("sqlite-pragma-wal", error.to_string()))?;
        connection
            .execute("PRAGMA foreign_keys=ON", [])
            .map_err(|error| AdapterError::new("sqlite-pragma-fk", error.to_string()))?;
        connection
            .execute_batch("PRAGMA busy_timeout=5000;")
            .map_err(|error| AdapterError::new("sqlite-pragma-busy", error.to_string()))?;
        Ok(())
    }

    fn integrity_check(connection: &Connection) -> Result<(), AdapterError> {
        let result: String = connection
            .query_row("PRAGMA integrity_check", [], |row| row.get(0))
            .map_err(|error| AdapterError::new("integrity-check-failed", error.to_string()))?;
        if result != "ok" {
            return Err(AdapterError::new(
                "integrity-check-failed",
                format!("integrity_check returned {result}"),
            ));
        }
        Ok(())
    }

    fn has_legacy_blob_schema(connection: &Connection) -> bool {
        connection
            .query_row(
                "SELECT 1 FROM sqlite_master WHERE type='table' AND name='canonical_state'",
                [],
                |_| Ok(()),
            )
            .is_ok()
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

    fn format_version(connection: &Connection) -> Result<u32, AdapterError> {
        let value = Self::read_meta(connection, "format_version")?
            .ok_or_else(|| AdapterError::new("missing-format-version", "format version missing"))?;
        value
            .parse::<u32>()
            .map_err(|error| AdapterError::new("invalid-format-version", error.to_string()))
    }

    fn validate_format_for_writable(connection: &Connection) -> Result<(), AdapterError> {
        if Self::has_legacy_blob_schema(connection) {
            return Err(AdapterError::new(
                "unsupported-older-format",
                "legacy blob schema is unsupported for writable open",
            ));
        }
        let version = Self::format_version(connection)?;
        if version > CURRENT_FORMAT_VERSION {
            return Err(AdapterError::new(
                "unsupported-newer-format",
                "unknown newer session format cannot open writable",
            ));
        }
        if version < CURRENT_FORMAT_VERSION {
            return Err(AdapterError::new(
                "unsupported-older-format",
                "writable open requires exact format version match",
            ));
        }
        Ok(())
    }

    fn validate_format_for_read_only(connection: &Connection) -> Result<(), AdapterError> {
        if Self::has_legacy_blob_schema(connection) {
            return Err(AdapterError::new(
                "unsupported-older-format",
                "legacy blob schema is unsupported",
            ));
        }
        let version = Self::format_version(connection)?;
        if version > CURRENT_FORMAT_VERSION {
            return Err(AdapterError::new(
                "unsupported-newer-format",
                "unknown newer session format",
            ));
        }
        if version < CURRENT_FORMAT_VERSION {
            return Err(AdapterError::new(
                "unsupported-older-format",
                "read-only open requires exact format version match",
            ));
        }
        Ok(())
    }

    fn initialize_schema(connection: &Connection) -> Result<(), AdapterError> {
        connection
            .execute_batch(
                "CREATE TABLE session_meta (key TEXT PRIMARY KEY, value TEXT NOT NULL);
                 CREATE TABLE source_revisions (revision_id TEXT PRIMARY KEY, payload_json TEXT NOT NULL);
                 CREATE TABLE review_cases (case_id TEXT PRIMARY KEY, payload_json TEXT NOT NULL);
                 CREATE TABLE review_ledger_events (event_id TEXT PRIMARY KEY, sequence INTEGER NOT NULL UNIQUE, payload_json TEXT NOT NULL);
                 CREATE TABLE review_case_raised_events (event_id TEXT PRIMARY KEY, sequence INTEGER NOT NULL UNIQUE, payload_json TEXT NOT NULL);
                 CREATE TABLE analysis_results (analysis_result_id TEXT PRIMARY KEY, payload_json TEXT NOT NULL);
                 CREATE TABLE active_analysis_selection (id INTEGER PRIMARY KEY CHECK (id = 1), selection_json TEXT);
                 CREATE TABLE knowledge_snapshot_references (knowledge_snapshot_id TEXT PRIMARY KEY, payload_json TEXT NOT NULL);
                 CREATE TABLE lineage_conflicts (conflict_id TEXT PRIMARY KEY, payload_json TEXT NOT NULL);
                 CREATE TABLE artifacts (artifact_id TEXT PRIMARY KEY, retention_class TEXT NOT NULL, payload_json TEXT NOT NULL);
                 CREATE TABLE retention_references (reference_id TEXT PRIMARY KEY, payload_json TEXT NOT NULL);
                 CREATE TABLE derived_cache (key TEXT PRIMARY KEY, value TEXT NOT NULL, schema_version INTEGER NOT NULL, content_hash TEXT NOT NULL);
                 CREATE TABLE writer_ownership (id INTEGER PRIMARY KEY CHECK (id = 1), token TEXT, owner_epoch INTEGER NOT NULL DEFAULT 0, lease_duration_ms INTEGER NOT NULL DEFAULT 30000, lease_expires_at_unix_ms INTEGER NOT NULL DEFAULT 0, process_instance_id TEXT NOT NULL DEFAULT '', holder_pid INTEGER);
                 CREATE TABLE applied_authoritative_commands (command_operation_id TEXT PRIMARY KEY, command_kind TEXT NOT NULL, outcome_status TEXT NOT NULL CHECK (outcome_status IN ('committed', 'acknowledged')), outcome_fingerprint TEXT NOT NULL, applied_at_unix_ms INTEGER NOT NULL);",
            )
            .map_err(|error| AdapterError::new("sqlite-init-schema", error.to_string()))?;
        Ok(())
    }

    fn persist_canonical_tables(
        connection: &Connection,
        state: &NormalizedSemanticState,
    ) -> Result<(), AdapterError> {
        let normalized = state.clone().normalize();
        connection
            .execute("DELETE FROM source_revisions", [])
            .map_err(|e| AdapterError::new("sqlite-save-state", e.to_string()))?;
        connection
            .execute("DELETE FROM review_cases", [])
            .map_err(|e| AdapterError::new("sqlite-save-state", e.to_string()))?;
        connection
            .execute("DELETE FROM review_case_raised_events", [])
            .map_err(|e| AdapterError::new("sqlite-save-state", e.to_string()))?;
        connection
            .execute("DELETE FROM review_ledger_events", [])
            .map_err(|e| AdapterError::new("sqlite-save-state", e.to_string()))?;
        connection
            .execute("DELETE FROM analysis_results", [])
            .map_err(|e| AdapterError::new("sqlite-save-state", e.to_string()))?;
        connection
            .execute("DELETE FROM active_analysis_selection", [])
            .map_err(|e| AdapterError::new("sqlite-save-state", e.to_string()))?;
        connection
            .execute("DELETE FROM knowledge_snapshot_references", [])
            .map_err(|e| AdapterError::new("sqlite-save-state", e.to_string()))?;
        connection
            .execute("DELETE FROM lineage_conflicts", [])
            .map_err(|e| AdapterError::new("sqlite-save-state", e.to_string()))?;
        connection
            .execute("DELETE FROM artifacts", [])
            .map_err(|e| AdapterError::new("sqlite-save-state", e.to_string()))?;
        connection
            .execute("DELETE FROM retention_references", [])
            .map_err(|e| AdapterError::new("sqlite-save-state", e.to_string()))?;

        for revision in &normalized.source_revisions {
            let json = serde_json::to_string(revision)
                .map_err(|e| AdapterError::new("state-serialize-failed", e.to_string()))?;
            connection
                .execute(
                    "INSERT INTO source_revisions (revision_id, payload_json) VALUES (?1, ?2)",
                    params![revision.revision_id, json],
                )
                .map_err(|e| AdapterError::new("sqlite-save-state", e.to_string()))?;
        }
        for case in &normalized.review_cases {
            let json = serde_json::to_string(case)
                .map_err(|e| AdapterError::new("state-serialize-failed", e.to_string()))?;
            connection
                .execute(
                    "INSERT INTO review_cases (case_id, payload_json) VALUES (?1, ?2)",
                    params![case.case_id, json],
                )
                .map_err(|e| AdapterError::new("sqlite-save-state", e.to_string()))?;
        }
        for event in &normalized.review_case_raised_events {
            let json = serde_json::to_string(event)
                .map_err(|e| AdapterError::new("state-serialize-failed", e.to_string()))?;
            connection
                .execute(
                    "INSERT INTO review_case_raised_events (event_id, sequence, payload_json) VALUES (?1, ?2, ?3)",
                    params![event.event_id, event.sequence as i64, json],
                )
                .map_err(|e| AdapterError::new("sqlite-save-state", e.to_string()))?;
        }
        for event in &normalized.review_ledger_events {
            let json = serde_json::to_string(event)
                .map_err(|e| AdapterError::new("state-serialize-failed", e.to_string()))?;
            connection
                .execute(
                    "INSERT INTO review_ledger_events (event_id, sequence, payload_json) VALUES (?1, ?2, ?3)",
                    params![event.event_id, event.sequence as i64, json],
                )
                .map_err(|e| AdapterError::new("sqlite-save-state", e.to_string()))?;
        }
        for analysis in &normalized.analysis_results {
            let json = serde_json::to_string(analysis)
                .map_err(|e| AdapterError::new("state-serialize-failed", e.to_string()))?;
            connection
                .execute(
                    "INSERT INTO analysis_results (analysis_result_id, payload_json) VALUES (?1, ?2)",
                    params![analysis.analysis_result_id, json],
                )
                .map_err(|e| AdapterError::new("sqlite-save-state", e.to_string()))?;
        }
        if let Some(selection) = &normalized.active_analysis_selection {
            let json = serde_json::to_string(selection)
                .map_err(|e| AdapterError::new("state-serialize-failed", e.to_string()))?;
            connection
                .execute(
                    "INSERT INTO active_analysis_selection (id, selection_json) VALUES (1, ?1)",
                    [json],
                )
                .map_err(|e| AdapterError::new("sqlite-save-state", e.to_string()))?;
        }
        for reference in &normalized.knowledge_snapshot_references {
            let json = serde_json::to_string(reference)
                .map_err(|e| AdapterError::new("state-serialize-failed", e.to_string()))?;
            connection
                .execute(
                    "INSERT INTO knowledge_snapshot_references (knowledge_snapshot_id, payload_json) VALUES (?1, ?2)",
                    params![reference.knowledge_snapshot_id, json],
                )
                .map_err(|e| AdapterError::new("sqlite-save-state", e.to_string()))?;
        }
        for conflict in &normalized.lineage_conflicts {
            let json = serde_json::to_string(conflict)
                .map_err(|e| AdapterError::new("state-serialize-failed", e.to_string()))?;
            connection
                .execute(
                    "INSERT INTO lineage_conflicts (conflict_id, payload_json) VALUES (?1, ?2)",
                    params![conflict.conflict_id, json],
                )
                .map_err(|e| AdapterError::new("sqlite-save-state", e.to_string()))?;
        }
        for artifact in &normalized.artifacts {
            let json = serde_json::to_string(artifact)
                .map_err(|e| AdapterError::new("state-serialize-failed", e.to_string()))?;
            let retention_class = format!("{:?}", artifact.class);
            connection
                .execute(
                    "INSERT INTO artifacts (artifact_id, retention_class, payload_json) VALUES (?1, ?2, ?3)",
                    params![artifact.artifact_id, retention_class, json],
                )
                .map_err(|e| AdapterError::new("sqlite-save-state", e.to_string()))?;
        }
        for reference in &normalized.retention_references {
            let json = serde_json::to_string(reference)
                .map_err(|e| AdapterError::new("state-serialize-failed", e.to_string()))?;
            let reference_id = format!(
                "{}:{}:{:?}",
                reference.root_id, reference.artifact_id, reference.relation
            );
            connection
                .execute(
                    "INSERT INTO retention_references (reference_id, payload_json) VALUES (?1, ?2)",
                    params![reference_id, json],
                )
                .map_err(|e| AdapterError::new("sqlite-save-state", e.to_string()))?;
        }
        Ok(())
    }

    fn load_canonical_state(
        connection: &Connection,
    ) -> Result<NormalizedSemanticState, AdapterError> {
        super::super::canonical_sql_reader::load_from_connection(connection)
    }

    fn initialize_session(
        connection: &Connection,
        state: &NormalizedSemanticState,
    ) -> Result<(), AdapterError> {
        Self::initialize_schema(connection)?;
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
        Self::persist_canonical_tables(connection, state)?;
        connection
            .execute(
                "INSERT INTO writer_ownership (id, token, owner_epoch, lease_duration_ms, lease_expires_at_unix_ms, process_instance_id, holder_pid) VALUES (1, NULL, 0, ?1, 0, '', NULL)",
                [DEFAULT_LEASE_DURATION_MS],
            )
            .map_err(|error| AdapterError::new("sqlite-init-lock", error.to_string()))?;
        Ok(())
    }

    fn derive_queue_index_v1(connection: &Connection) -> Result<(String, String), AdapterError> {
        let state = Self::load_canonical_state(connection)?;
        let mut entries = Vec::new();
        for case in &state.review_cases {
            let head_event_id = state
                .review_ledger_events
                .iter()
                .filter(|event| event.case_id == case.case_id)
                .max_by_key(|event| event.sequence)
                .map(|event| event.event_id.clone());
            entries.push(serde_json::json!({
                "case_id": case.case_id,
                "head_event_id": head_event_id,
            }));
        }
        let value = serde_json::to_string(&entries)
            .map_err(|e| AdapterError::new("derived-rebuild-failed", e.to_string()))?;
        let mut hasher = Sha256::new();
        hasher.update(value.as_bytes());
        let hash = format!("{:x}", hasher.finalize());
        Ok((value, hash))
    }

    fn validate_or_rebuild_derived_cache(connection: &Connection) -> Result<(), AdapterError> {
        let (value, hash) = Self::derive_queue_index_v1(connection)?;
        let stored = connection
            .query_row(
                "SELECT value, schema_version, content_hash FROM derived_cache WHERE key = ?1",
                [DERIVED_INDEX_KEY],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, i64>(1)?,
                        row.get::<_, String>(2)?,
                    ))
                },
            )
            .optional()
            .map_err(|e| AdapterError::new("derived-rebuild-failed", e.to_string()))?;
        let needs_rebuild = match stored {
            None => true,
            Some((stored_value, schema_version, content_hash)) => {
                schema_version != DERIVED_SCHEMA_VERSION
                    || content_hash != hash
                    || stored_value != value
            }
        };
        if needs_rebuild {
            connection
                .execute(
                    "INSERT INTO derived_cache (key, value, schema_version, content_hash) VALUES (?1, ?2, ?3, ?4)
                     ON CONFLICT(key) DO UPDATE SET value = excluded.value, schema_version = excluded.schema_version, content_hash = excluded.content_hash",
                    params![DERIVED_INDEX_KEY, value, DERIVED_SCHEMA_VERSION, hash],
                )
                .map_err(|e| AdapterError::new("derived-rebuild-failed", e.to_string()))?;
        }
        Ok(())
    }

    fn seed_derived_cache(connection: &Connection) -> Result<(), AdapterError> {
        let (value, hash) = Self::derive_queue_index_v1(connection)?;
        connection
            .execute(
                "INSERT INTO derived_cache (key, value, schema_version, content_hash) VALUES (?1, ?2, ?3, ?4)",
                params![DERIVED_INDEX_KEY, value, DERIVED_SCHEMA_VERSION, hash],
            )
            .map_err(|e| AdapterError::new("sqlite-init-derived", e.to_string()))?;
        Ok(())
    }

    fn acquire_writer_ownership(
        connection: &mut Connection,
        process_instance_id: &str,
        now_ms: i64,
    ) -> Result<(String, i64), AdapterError> {
        let tx = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(map_sqlite_error("sqlite-writer-lock"))?;
        let (current_token, owner_epoch, lease_expires): (Option<String>, i64, i64) = tx
            .query_row(
                "SELECT token, owner_epoch, lease_expires_at_unix_ms FROM writer_ownership WHERE id = 1",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .map_err(map_sqlite_error("sqlite-writer-lock"))?;
        if current_token.is_some() && lease_expires > now_ms {
            return Err(AdapterError::new(
                "writer-already-open",
                "embedded relational store permits one authoritative writer",
            ));
        }
        let new_token = new_writer_token();
        let new_epoch = if current_token.is_some() {
            owner_epoch + 1
        } else {
            owner_epoch.max(0)
        };
        let lease_duration_ms = lease_duration_ms_from_env();
        let lease_expires_at = now_ms + lease_duration_ms;
        let updated = tx
            .execute(
                "UPDATE writer_ownership SET token = ?1, owner_epoch = ?2, lease_duration_ms = ?3, lease_expires_at_unix_ms = ?4, process_instance_id = ?5, holder_pid = ?6 WHERE id = 1 AND (token IS NULL OR lease_expires_at_unix_ms <= ?7)",
                params![
                    new_token.as_str(),
                    new_epoch,
                    lease_duration_ms,
                    lease_expires_at,
                    process_instance_id,
                    std::process::id() as i64,
                    now_ms,
                ],
            )
            .map_err(map_sqlite_error("sqlite-writer-lock"))?;
        if updated == 0 {
            return Err(AdapterError::new(
                "writer-already-open",
                "writer ownership acquisition lost race",
            ));
        }
        tx.commit()
            .map_err(map_sqlite_error("sqlite-writer-lock"))?;
        Ok((new_token, new_epoch))
    }

    fn revalidate_writer_on_connection(
        connection: &Connection,
        token: &str,
        owner_epoch: i64,
        now_ms: i64,
    ) -> Result<(), AdapterError> {
        let (stored_token, stored_epoch, lease_expires): (Option<String>, i64, i64) = connection
            .query_row(
                "SELECT token, owner_epoch, lease_expires_at_unix_ms FROM writer_ownership WHERE id = 1",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .map_err(map_sqlite_error("writer-epoch-mismatch"))?;
        if stored_token.as_deref() != Some(token) || stored_epoch != owner_epoch {
            return Err(AdapterError::new(
                "writer-epoch-mismatch",
                "writer token or epoch no longer valid",
            ));
        }
        if lease_expires <= now_ms {
            return Err(AdapterError::new(
                "writer-lease-expired",
                "writer lease expired before authoritative command",
            ));
        }
        Ok(())
    }

    fn verify_retry_command_identity(
        connection: &Connection,
        command: &AuthoritativeCommand,
        stored_kind: &str,
        stored_fingerprint: &str,
        token: &str,
        owner_epoch: i64,
        now_ms: i64,
    ) -> Result<(), AdapterError> {
        Self::revalidate_writer_on_connection(connection, token, owner_epoch, now_ms)?;
        let command_kind = Self::command_kind_label(command);
        if stored_kind != command_kind {
            return Err(AdapterError::new(
                "command-operation-id-mismatch",
                "command_operation_id reused with different command identity",
            ));
        }
        let retry_fingerprint = Self::compute_outcome_fingerprint_v1(command)?;
        if stored_fingerprint != retry_fingerprint {
            return Err(AdapterError::new(
                "command-operation-id-mismatch",
                "command_operation_id reused with different command identity",
            ));
        }
        Ok(())
    }

    fn revalidate_writer_in_tx(
        tx: &rusqlite::Transaction<'_>,
        token: &str,
        owner_epoch: i64,
        now_ms: i64,
    ) -> Result<(), AdapterError> {
        let (stored_token, stored_epoch, lease_expires): (Option<String>, i64, i64) = tx
            .query_row(
                "SELECT token, owner_epoch, lease_expires_at_unix_ms FROM writer_ownership WHERE id = 1",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .map_err(map_sqlite_error("writer-epoch-mismatch"))?;
        if stored_token.as_deref() != Some(token) || stored_epoch != owner_epoch {
            return Err(AdapterError::new(
                "writer-epoch-mismatch",
                "writer token or epoch no longer valid",
            ));
        }
        if lease_expires <= now_ms {
            return Err(AdapterError::new(
                "writer-lease-expired",
                "writer lease expired before authoritative command",
            ));
        }
        Ok(())
    }

    fn renew_writer_lease_in_tx(
        tx: &rusqlite::Transaction<'_>,
        token: &str,
        owner_epoch: i64,
        now_ms: i64,
    ) -> Result<(), AdapterError> {
        Self::revalidate_writer_in_tx(tx, token, owner_epoch, now_ms)?;
        tx.execute(
            "UPDATE writer_ownership SET lease_expires_at_unix_ms = ?1 WHERE id = 1 AND token = ?2 AND owner_epoch = ?3",
            params![
                now_ms + lease_duration_ms_from_env(),
                token,
                owner_epoch
            ],
        )
        .map_err(map_sqlite_error("sqlite-writer-lock"))?;
        Ok(())
    }

    fn release_writer_ownership(connection: &Connection, token: &str) -> Result<(), AdapterError> {
        connection
            .execute(
                "UPDATE writer_ownership SET token = NULL WHERE id = 1 AND token = ?1",
                [token],
            )
            .map_err(map_sqlite_error("sqlite-writer-unlock"))?;
        Ok(())
    }

    fn command_operation_id(command: &AuthoritativeCommand) -> &str {
        match command {
            AuthoritativeCommand::AppendCorrectionEvent {
                command_operation_id,
                ..
            }
            | AuthoritativeCommand::AttachAnalysisResult {
                command_operation_id,
                ..
            }
            | AuthoritativeCommand::SelectActiveAnalysis {
                command_operation_id,
                ..
            }
            | AuthoritativeCommand::ExecuteCleanupPlan {
                command_operation_id,
                ..
            } => command_operation_id,
        }
    }

    fn command_kind_label(command: &AuthoritativeCommand) -> &'static str {
        match command {
            AuthoritativeCommand::AppendCorrectionEvent { .. } => "append-correction",
            AuthoritativeCommand::AttachAnalysisResult { .. } => "attach-analysis",
            AuthoritativeCommand::SelectActiveAnalysis { .. } => "select-active-analysis",
            AuthoritativeCommand::ExecuteCleanupPlan { .. } => "execute-cleanup",
        }
    }

    fn command_payload_json(command: &AuthoritativeCommand) -> Result<Value, AdapterError> {
        let payload = match command {
            AuthoritativeCommand::AppendCorrectionEvent {
                event,
                preconditions,
                ..
            } => {
                serde_json::json!({
                    "event": event,
                    "preconditions": canonical_preconditions(preconditions),
                })
            }
            AuthoritativeCommand::AttachAnalysisResult {
                analysis_result,
                preconditions,
                ..
            } => serde_json::json!({
                "analysis_result": analysis_result,
                "preconditions": canonical_preconditions(preconditions),
            }),
            AuthoritativeCommand::SelectActiveAnalysis {
                selection,
                preconditions,
                ..
            } => serde_json::json!({
                "selection": selection,
                "preconditions": canonical_preconditions(preconditions),
            }),
            AuthoritativeCommand::ExecuteCleanupPlan {
                plan_id,
                preconditions,
                ..
            } => serde_json::json!({
                "plan_id": plan_id,
                "preconditions": canonical_preconditions(preconditions),
            }),
        };
        Ok(payload)
    }

    fn compute_outcome_fingerprint_v1(
        command: &AuthoritativeCommand,
    ) -> Result<String, AdapterError> {
        let fingerprint_body = serde_json::json!({
            "command_kind": Self::command_kind_label(command),
            "command_operation_id": Self::command_operation_id(command),
            "command_payload": Self::command_payload_json(command)?,
            "fingerprint_version": OUTCOME_FINGERPRINT_VERSION,
        });
        let canonical = serde_json::to_string(&fingerprint_body)
            .map_err(|e| AdapterError::new("fingerprint-serialize-failed", e.to_string()))?;
        let mut hasher = Sha256::new();
        hasher.update(canonical.as_bytes());
        Ok(format!("{:x}", hasher.finalize()))
    }

    fn lookup_applied_command(
        connection: &Connection,
        command_operation_id: &str,
    ) -> Result<Option<(String, String, String)>, AdapterError> {
        connection
            .query_row(
                "SELECT command_kind, outcome_status, outcome_fingerprint FROM applied_authoritative_commands WHERE command_operation_id = ?1",
                [command_operation_id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .optional()
            .map_err(map_sqlite_error("sqlite-command-lookup"))
    }

    fn verify_committed_command_reconciliation(
        connection: &Connection,
        command: &AuthoritativeCommand,
        stored_fingerprint: &str,
    ) -> Result<(), AdapterError> {
        let current = Self::load_canonical_state(connection)?;
        let expected_fingerprint = Self::compute_outcome_fingerprint_v1(command)?;
        if expected_fingerprint != stored_fingerprint {
            return Err(AdapterError::new(
                "post-commit-verify-failed",
                "stored fingerprint does not match recomputed reconciliation fingerprint",
            ));
        }
        if let AuthoritativeCommand::ExecuteCleanupPlan { plan_id, .. } = command {
            if current
                .artifacts
                .iter()
                .any(|artifact| artifact.artifact_id.contains(plan_id))
            {
                return Err(AdapterError::new(
                    "post-commit-verify-failed",
                    "cleanup plan outcome not reflected in authoritative state",
                ));
            }
            return Ok(());
        }
        let pre_state = derive_pre_state_for_verify(&current, command)?;
        let mut expected = pre_state;
        apply_command(&mut expected, command)?;
        let oracle = SemanticOracle::compare(
            &expected.canonical_projection(),
            &current.canonical_projection(),
        );
        if !oracle.passed {
            return Err(AdapterError::new(
                "post-commit-verify-failed",
                "authoritative state does not match committed command outcome",
            ));
        }
        Ok(())
    }

    fn acknowledge_applied_command(
        connection: &Connection,
        command_operation_id: &str,
        token: &str,
        owner_epoch: i64,
        now_ms: i64,
    ) -> Result<(), AdapterError> {
        let tx = connection
            .unchecked_transaction()
            .map_err(map_sqlite_error("sqlite-command-ack"))?;
        Self::revalidate_writer_in_tx(&tx, token, owner_epoch, now_ms)?;
        tx.execute(
            "UPDATE applied_authoritative_commands SET outcome_status = 'acknowledged' WHERE command_operation_id = ?1 AND outcome_status = 'committed'",
            [command_operation_id],
        )
        .map_err(map_sqlite_error("sqlite-command-ack"))?;
        tx.commit()
            .map_err(map_sqlite_error("sqlite-command-ack"))?;
        Ok(())
    }

    fn map_busy_error(error: AdapterError) -> AdapterError {
        if error.message.contains("database is locked")
            || error.message.contains("SQLITE_BUSY")
            || error.message.contains("snapshot")
        {
            AdapterError::new("sqlite-busy", error.message)
        } else {
            error
        }
    }

    fn take_sqlite_fault(faults: &FaultRegistry, point: FaultPoint) -> Result<(), AdapterError> {
        if let Some(fault) = faults.take_if_armed(point) {
            return Self::dispatch_fault(fault);
        }
        if point == FaultPoint::BeforeSqliteCommit
            && let Some(fault) = faults.take_if_armed(FaultPoint::FailBeforeDurabilityCommit)
        {
            return Self::dispatch_fault(fault);
        }
        Ok(())
    }

    fn take_duplicate_fault(faults: &FaultRegistry, point: FaultPoint) -> Result<(), AdapterError> {
        if let Some(fault) = faults.take_if_armed(point) {
            return Self::dispatch_fault(fault);
        }
        Ok(())
    }

    fn dispatch_fault(fault: super::fault::PendingFault) -> Result<(), AdapterError> {
        match fault.execution_mode {
            FaultExecutionMode::ProcessAbort => handle_fault(fault),
            FaultExecutionMode::ReturnError => Err(AdapterError::new(
                "simulated-durability-failure",
                format!("logical fault injected at {:?}", fault.point),
            )),
        }
    }

    fn wal_checkpoint_truncate(connection: &Connection) -> Result<(), AdapterError> {
        connection
            .query_row("PRAGMA wal_checkpoint(TRUNCATE)", [], |row| {
                let _busy: i32 = row.get(0)?;
                let _log: i32 = row.get(1)?;
                let _checkpointed: i32 = row.get(2)?;
                Ok(())
            })
            .map_err(map_sqlite_error("sqlite-checkpoint-failed"))?;
        Ok(())
    }

    fn assert_no_wal_companions(db_path: &Path) -> Result<(), AdapterError> {
        let path_str = db_path.to_string_lossy();
        for suffix in ["-wal", "-shm"] {
            let companion = PathBuf::from(format!("{path_str}{suffix}"));
            if companion.exists() {
                return Err(AdapterError::new(
                    "duplicate-publish-blocked",
                    format!("live WAL companion remains: {}", companion.display()),
                ));
            }
        }
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
        let tx = connection
            .unchecked_transaction()
            .map_err(map_sqlite_error("sqlite-tx-begin"))?;
        Self::initialize_session(&tx, &state)?;
        Self::seed_derived_cache(&tx)?;
        tx.commit()
            .map_err(map_sqlite_error("sqlite-create-commit"))?;
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
        let locator = session.adapter_locator().to_string();
        let mut connection = Self::open_connection(&locator)?;
        Self::integrity_check(&connection)?;

        if mode == SemanticOpenMode::Writable {
            Self::validate_format_for_writable(&connection)?;
            let now_ms = self.now_unix_ms()?;
            let (token, owner_epoch) =
                Self::acquire_writer_ownership(&mut connection, &self.process_instance_id, now_ms)?;
            if let Err(error) = Self::load_canonical_state(&connection) {
                let _ = Self::release_writer_ownership(&connection, &token);
                return Err(error);
            }
            if let Err(error) = Self::validate_or_rebuild_derived_cache(&connection) {
                let _ = Self::release_writer_ownership(&connection, &token);
                return Err(error);
            }
            *self.next_handle.borrow_mut() += 1;
            let handle_id = format!("embedded-handle:{}", self.next_handle.borrow());
            self.handles.borrow_mut().insert(
                handle_id.clone(),
                HandleState::Writable(WritableHandleState {
                    connection,
                    token,
                    owner_epoch,
                }),
            );
            Ok(EvidenceSessionHandle::new(session.clone(), mode, handle_id))
        } else {
            Self::validate_format_for_read_only(&connection)?;
            Self::load_canonical_state(&connection)?;
            Self::validate_or_rebuild_derived_cache(&connection)?;
            *self.next_handle.borrow_mut() += 1;
            let handle_id = format!("embedded-handle:{}", self.next_handle.borrow());
            self.handles
                .borrow_mut()
                .insert(handle_id.clone(), HandleState::ReadOnly { locator });
            Ok(EvidenceSessionHandle::new(session.clone(), mode, handle_id))
        }
    }

    fn close(&mut self, handle: &EvidenceSessionHandle) -> Result<(), AdapterError> {
        let state = self
            .handles
            .borrow_mut()
            .remove(handle.adapter_handle())
            .ok_or_else(|| AdapterError::new("not-open", "handle is not open"))?;
        if let HandleState::Writable(writable) = state {
            Self::release_writer_ownership(&writable.connection, &writable.token)?;
            if self
                .faults
                .take_if_armed(FaultPoint::DuringCheckpoint)
                .is_none()
            {
                let _ = Self::wal_checkpoint_truncate(&writable.connection);
            }
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
        let command_operation_id = Self::command_operation_id(command).to_string();
        if command_operation_id.is_empty() {
            return Err(AdapterError::new(
                "missing-command-operation-id",
                "command_operation_id is required",
            ));
        }
        if Uuid::parse_str(&command_operation_id).is_err() {
            return Err(AdapterError::new(
                "invalid-command-operation-id",
                "command_operation_id must be a UUID",
            ));
        }

        let mut handles = self.handles.borrow_mut();
        let writable = match handles.get_mut(handle.adapter_handle()) {
            Some(HandleState::Writable(writable)) => writable,
            _ => {
                return Err(AdapterError::new("not-open", "writable handle is not open"));
            }
        };

        let connection = &mut writable.connection;
        let token = writable.token.clone();
        let owner_epoch = writable.owner_epoch;
        let now_ms = self.now_unix_ms()?;
        let command_kind = Self::command_kind_label(command);

        if let Some((stored_kind, outcome_status, stored_fingerprint)) =
            Self::lookup_applied_command(connection, &command_operation_id)?
        {
            Self::verify_retry_command_identity(
                connection,
                command,
                &stored_kind,
                &stored_fingerprint,
                &token,
                owner_epoch,
                now_ms,
            )?;
            match outcome_status.as_str() {
                "acknowledged" => return Ok(()),
                "committed" => {
                    Self::verify_committed_command_reconciliation(
                        connection,
                        command,
                        &stored_fingerprint,
                    )?;
                    Self::acknowledge_applied_command(
                        connection,
                        &command_operation_id,
                        &token,
                        owner_epoch,
                        now_ms,
                    )?;
                    return Ok(());
                }
                other => {
                    return Err(AdapterError::new(
                        "sqlite-command-lookup",
                        format!("unknown outcome_status {other}"),
                    ));
                }
            }
        }

        let pre_state = Self::load_canonical_state(connection)?;
        let fingerprint = Self::compute_outcome_fingerprint_v1(command)?;

        Self::take_sqlite_fault(&self.faults, FaultPoint::FailBeforeDurabilityCommit)?;

        if *self.force_sqlite_busy_on_tx_begin_once.borrow() {
            *self.force_sqlite_busy_on_tx_begin_once.borrow_mut() = false;
            return Err(AdapterError::new("sqlite-busy", "database is locked"));
        }

        let tx = connection
            .transaction_with_behavior(TransactionBehavior::Deferred)
            .map_err(|e| Self::map_busy_error(map_sqlite_error("sqlite-tx-begin")(e)))?;
        Self::revalidate_writer_in_tx(&tx, &token, owner_epoch, now_ms)?;
        let mut state = pre_state.clone();
        apply_command(&mut state, command)?;
        Self::persist_canonical_tables(&tx, &state)?;
        tx.execute(
            "INSERT INTO applied_authoritative_commands (command_operation_id, command_kind, outcome_status, outcome_fingerprint, applied_at_unix_ms) VALUES (?1, ?2, 'committed', ?3, ?4)",
            params![command_operation_id, command_kind, fingerprint, now_ms],
        )
        .map_err(map_sqlite_error("sqlite-command-insert"))?;
        Self::renew_writer_lease_in_tx(&tx, &token, owner_epoch, now_ms)?;

        Self::take_sqlite_fault(&self.faults, FaultPoint::BeforeSqliteCommit)?;
        tx.commit()
            .map_err(|e| Self::map_busy_error(map_sqlite_error("sqlite-commit-failed")(e)))?;

        if let Some(fault) = self
            .faults
            .take_if_armed(FaultPoint::AfterSqliteCommitBeforeAck)
        {
            return Self::dispatch_fault(fault);
        }

        if *self.force_post_commit_verify_failure.borrow() {
            return Err(AdapterError::new(
                "post-commit-verify-failed",
                "post-commit authoritative verification failed",
            ));
        }

        Self::verify_committed_command_reconciliation(connection, command, &fingerprint)?;
        Self::acknowledge_applied_command(
            connection,
            &command_operation_id,
            &token,
            owner_epoch,
            now_ms,
        )?;
        Ok(())
    }

    fn read_normalized_state(
        &self,
        handle: &EvidenceSessionHandle,
    ) -> Result<NormalizedSemanticState, AdapterError> {
        let handles = self.handles.borrow();
        match handles.get(handle.adapter_handle()) {
            Some(HandleState::Writable(writable)) => {
                Self::load_canonical_state(&writable.connection)
            }
            Some(HandleState::ReadOnly { locator }) => {
                let connection = Self::open_connection(locator)?;
                Self::load_canonical_state(&connection)
            }
            None => Err(AdapterError::new("not-open", "handle is not open")),
        }
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
        let source_locator = source.session.adapter_locator().to_string();
        let source_state = {
            let handles = self.handles.borrow();
            match handles.get(source.adapter_handle()) {
                Some(HandleState::Writable(writable)) => {
                    Self::load_canonical_state(&writable.connection)?
                }
                Some(HandleState::ReadOnly { .. }) => {
                    return Err(AdapterError::new(
                        "duplicate-requires-writable-source",
                        "duplication requires source writable handle",
                    ));
                }
                None => {
                    return Err(AdapterError::new("not-open", "source handle is not open"));
                }
            }
        };

        let dest_dir = self.session_dir(new_session_id);
        if dest_dir.exists() {
            return Err(AdapterError::new(
                "duplicate-session-exists",
                "duplicate session identity already exists",
            ));
        }
        let mut dest_guard = DuplicateDestGuard::new(dest_dir.clone());
        fs::create_dir_all(&dest_dir)
            .map_err(|error| AdapterError::new("filesystem-create-session", error.to_string()))?;

        let temp_db = dest_dir.join("session.db.tmp");
        let published_db = Self::db_path(dest_dir.to_str().expect("utf8 path"));
        let now_ms = self.now_unix_ms()?;

        {
            let handles = self.handles.borrow();
            let Some(HandleState::Writable(writable)) = handles.get(source.adapter_handle()) else {
                unreachable!("checked above");
            };
            Self::revalidate_writer_on_connection(
                &writable.connection,
                &writable.token,
                writable.owner_epoch,
                now_ms,
            )?;
            (|| -> Result<(), AdapterError> {
                let mut dest_connection =
                    Connection::open(&temp_db).map_err(map_sqlite_error("sqlite-open-failed"))?;
                Self::apply_pragmas(&dest_connection)?;
                {
                    let backup =
                        rusqlite::backup::Backup::new(&writable.connection, &mut dest_connection)
                            .map_err(map_sqlite_error("sqlite-backup-failed"))?;
                    if let Some(fault) = self.faults.take_if_armed(FaultPoint::DuringBackupCopy) {
                        return Self::dispatch_fault(fault);
                    }
                    backup
                        .run_to_completion(100, std::time::Duration::from_millis(10), None)
                        .map_err(map_sqlite_error("sqlite-backup-failed"))?;
                }

                Self::take_duplicate_fault(
                    &self.faults,
                    FaultPoint::AfterBackupBeforeDestinationIdentityChange,
                )?;

                Self::integrity_check(&dest_connection)?;
                let identity_tx = dest_connection
                    .unchecked_transaction()
                    .map_err(map_sqlite_error("sqlite-tx-begin"))?;
                identity_tx
                    .execute(
                        "UPDATE session_meta SET value = ?1 WHERE key = 'session_id'",
                        [new_session_id],
                    )
                    .map_err(map_sqlite_error("sqlite-update-session-id"))?;
                identity_tx
                    .execute(
                        "UPDATE session_meta SET value = ?1 WHERE key = 'duplicated_from'",
                        [source_state.session.session_id.as_str()],
                    )
                    .map_err(map_sqlite_error("sqlite-update-lineage"))?;
                identity_tx
                .execute(
                    "UPDATE writer_ownership SET token = NULL, lease_expires_at_unix_ms = 0 WHERE id = 1",
                    [],
                )
                .map_err(map_sqlite_error("sqlite-reset-lock"))?;
                identity_tx
                    .execute("DELETE FROM applied_authoritative_commands", [])
                    .map_err(map_sqlite_error("sqlite-reset-applied-commands"))?;
                identity_tx
                    .commit()
                    .map_err(map_sqlite_error("sqlite-commit-failed"))?;

                Self::take_duplicate_fault(
                    &self.faults,
                    FaultPoint::AfterDestinationIdentityChangeBeforePublish,
                )?;

                let mut expected_duplicate = source_state.clone();
                expected_duplicate.session.duplicated_from_session_id =
                    Some(source_state.session.session_id.clone());
                expected_duplicate.session.session_id = new_session_id.to_string();
                let loaded_duplicate = Self::load_canonical_state(&dest_connection)?;
                let oracle = SemanticOracle::compare(
                    &expected_duplicate.canonical_projection(),
                    &loaded_duplicate.canonical_projection(),
                );
                if !oracle.passed {
                    return Err(AdapterError::new(
                        "duplicate-semantic-validation-failed",
                        "destination semantic oracle failed before publish",
                    ));
                }

                if let Some(fault) = self.faults.take_if_armed(FaultPoint::DuringCheckpoint) {
                    return Self::dispatch_fault(fault);
                }
                Self::wal_checkpoint_truncate(&dest_connection)?;
                drop(dest_connection);
                Ok(())
            })()?;
        }

        Self::assert_no_wal_companions(&temp_db)?;

        fs::rename(&temp_db, &published_db)
            .map_err(|error| AdapterError::new("duplicate-publish-failed", error.to_string()))?;
        Self::take_duplicate_fault(&self.faults, FaultPoint::AfterPublishBeforeReturn)?;
        Self::assert_no_wal_companions(&published_db)?;

        let verify_locator = dest_dir.to_string_lossy().to_string();
        let verify_connection = Self::open_connection(&verify_locator)?;
        Self::integrity_check(&verify_connection)?;
        let published_state = Self::load_canonical_state(&verify_connection)?;
        if published_state.session.session_id != new_session_id {
            return Err(AdapterError::new(
                "duplicate-identity-verify-failed",
                "published destination session_id mismatch",
            ));
        }

        let source_reopen = Self::open_connection(&source_locator)?;
        let source_reloaded = Self::load_canonical_state(&source_reopen)?;
        let source_oracle = SemanticOracle::compare(
            &source_state.canonical_projection(),
            &source_reloaded.canonical_projection(),
        );
        if !source_oracle.passed {
            return Err(AdapterError::new(
                "duplicate-source-mutated",
                "source canonical state changed during duplication",
            ));
        }

        dest_guard.disarm();
        Ok(DuplicatedSession {
            session: EvidenceSessionRef::new(new_session_id, verify_locator),
            normalized_state: published_state,
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
                        "UPDATE derived_cache SET value = 'corrupted', content_hash = 'bad' WHERE key = ?1",
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
                        "UPDATE review_cases SET payload_json = ?1 WHERE case_id = (SELECT case_id FROM review_cases LIMIT 1)",
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
        let handles = self.handles.borrow();
        let connection = match handles.get(handle.adapter_handle()) {
            Some(HandleState::Writable(writable)) => &writable.connection,
            Some(HandleState::ReadOnly { .. }) => {
                return Err(AdapterError::new(
                    "not-authoritative-writer",
                    "compaction requires writable handle",
                ));
            }
            None => return Err(AdapterError::new("not-open", "handle is not open")),
        };
        if self
            .faults
            .take_if_armed(FaultPoint::DuringCheckpoint)
            .is_some()
        {
            return Err(AdapterError::new(
                "simulated-checkpoint-interrupt",
                "checkpoint interrupted before completion",
            ));
        }
        connection
            .execute("VACUUM", [])
            .map_err(|error| AdapterError::new("sqlite-vacuum", error.to_string()))?;
        Ok(OptionalOperationOutcome::Completed)
    }
}

fn derive_pre_state_for_verify(
    current: &NormalizedSemanticState,
    command: &AuthoritativeCommand,
) -> Result<NormalizedSemanticState, AdapterError> {
    let mut pre_state = current.clone();
    match command {
        AuthoritativeCommand::AppendCorrectionEvent { event, .. } => {
            if let Some(pos) = pre_state
                .review_ledger_events
                .iter()
                .position(|existing| existing.event_id == event.event_id)
            {
                pre_state.review_ledger_events.remove(pos);
            } else {
                return Err(AdapterError::new(
                    "post-commit-verify-failed",
                    "committed append event not found in authoritative state",
                ));
            }
        }
        AuthoritativeCommand::AttachAnalysisResult {
            analysis_result, ..
        } => {
            pre_state.analysis_results.retain(|existing| {
                existing.analysis_result_id != analysis_result.analysis_result_id
            });
        }
        AuthoritativeCommand::SelectActiveAnalysis {
            selection,
            preconditions,
            ..
        } => {
            if current
                .active_analysis_selection
                .as_ref()
                .is_none_or(|active| active.selection_event_id != selection.selection_event_id)
            {
                return Err(AdapterError::new(
                    "post-commit-verify-failed",
                    "committed active selection not found in authoritative state",
                ));
            }
            for precondition in preconditions {
                if let SemanticPrecondition::ActiveAnalysisSelection {
                    expected_analysis_result_id,
                } = precondition
                {
                    pre_state.active_analysis_selection =
                        expected_analysis_result_id
                            .as_ref()
                            .map(|analysis_result_id| {
                                super::super::model::ActiveAnalysisSelection {
                                    analysis_result_id: analysis_result_id.clone(),
                                    selection_event_id: "derived-precondition-selection"
                                        .to_string(),
                                }
                            });
                }
            }
        }
        AuthoritativeCommand::ExecuteCleanupPlan { .. } => {
            return Err(AdapterError::new(
                "post-commit-verify-failed",
                "execute cleanup plan uses dedicated reconciliation path",
            ));
        }
    }
    Ok(pre_state.normalize())
}

struct DuplicateDestGuard {
    path: PathBuf,
    disarmed: bool,
}

impl DuplicateDestGuard {
    fn new(path: PathBuf) -> Self {
        Self {
            path,
            disarmed: false,
        }
    }

    fn disarm(&mut self) {
        self.disarmed = true;
    }
}

impl Drop for DuplicateDestGuard {
    fn drop(&mut self) {
        if !self.disarmed {
            let _ = fs::remove_dir_all(&self.path);
        }
    }
}

fn canonical_preconditions(preconditions: &[SemanticPrecondition]) -> Value {
    let entries: Vec<Value> = preconditions
        .iter()
        .map(|precondition| match precondition {
            SemanticPrecondition::SourceRevisionExists {
                expected_revision_id,
            } => serde_json::json!({
                "kind": "SourceRevisionExists",
                "expected_revision_id": expected_revision_id,
            }),
            SemanticPrecondition::ReviewLedgerHead { expected_event_id } => serde_json::json!({
                "kind": "ReviewLedgerHead",
                "expected_event_id": expected_event_id,
            }),
            SemanticPrecondition::ActiveAnalysisSelection {
                expected_analysis_result_id,
            } => serde_json::json!({
                "kind": "ActiveAnalysisSelection",
                "expected_analysis_result_id": expected_analysis_result_id,
            }),
            SemanticPrecondition::AnalysisAttachmentSet {
                expected_analysis_result_ids,
            } => {
                let mut ids = expected_analysis_result_ids.clone();
                ids.sort();
                serde_json::json!({
                    "kind": "AnalysisAttachmentSet",
                    "expected_analysis_result_ids": ids,
                })
            }
        })
        .collect();
    Value::Array(entries)
}

fn map_sqlite_error(code: &'static str) -> impl Fn(rusqlite::Error) -> AdapterError {
    move |error| {
        let message = error.to_string();
        if message.contains("database is locked") || message.contains("SQLITE_BUSY") {
            AdapterError::new("sqlite-busy", message)
        } else {
            AdapterError::new(code, message)
        }
    }
}

fn lease_duration_ms_from_env() -> i64 {
    std::env::var("VOXPROOF_LEASE_DURATION_MS")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(DEFAULT_LEASE_DURATION_MS)
}

fn new_process_instance_id() -> String {
    Uuid::new_v4().to_string()
}

fn new_writer_token() -> String {
    Uuid::new_v4().to_string()
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
