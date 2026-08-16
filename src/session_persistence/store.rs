use std::cell::Cell;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use rusqlite::{Connection, OpenFlags, TransactionBehavior, params};
use uuid::Uuid;

use crate::application_service::{
    ApplicationMaterialUseDeclaration, DeclaredSessionAuthority, PreparedHumanDecision,
    begin_application_review,
};
use crate::candidate::SessionTermEntry;
use crate::session_persistence::canonical::{
    PRODUCT_SESSION_FORMAT_VERSION, PRODUCT_SESSION_FORMAT_VERSION_V2, SessionCanonicalCapture,
    capture_from_session, persist_ledger_event, restore_transcript,
};
use crate::session_persistence::error::{
    AuthorityScope, SessionPersistenceError, StaleAuthorityPrecondition,
};
use crate::transcript::Transcript;

thread_local! {
    static FAIL_BEFORE_COMMIT: Cell<bool> = const { Cell::new(false) };
    static FAIL_AFTER_TARGET_BEFORE_LEDGER: Cell<bool> = const { Cell::new(false) };
}

#[doc(hidden)]
pub fn arm_fail_before_commit_for_test() {
    FAIL_BEFORE_COMMIT.with(|flag| flag.set(true));
}

#[doc(hidden)]
pub fn disarm_fail_before_commit_for_test() {
    FAIL_BEFORE_COMMIT.with(|flag| flag.set(false));
}

#[doc(hidden)]
pub fn arm_fail_after_target_before_ledger_for_test() {
    FAIL_AFTER_TARGET_BEFORE_LEDGER.with(|flag| flag.set(true));
}

#[doc(hidden)]
pub fn disarm_fail_after_target_before_ledger_for_test() {
    FAIL_AFTER_TARGET_BEFORE_LEDGER.with(|flag| flag.set(false));
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OpenMode {
    Writable,
    ReadOnly,
}

pub struct ProductSessionStore {
    root: PathBuf,
}

pub struct OpenedStoreSession {
    pub session_id: String,
    pub db_path: PathBuf,
    pub connection: Connection,
    pub mode: OpenMode,
    pub writer_token: Option<String>,
    pub format_version: u32,
}

pub struct DurableAck {
    pub committed_generation: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionListSummary {
    pub session_id: String,
    pub created_at_unix_ms: i64,
    pub authority_display_label: String,
    pub review_case_count: usize,
    pub review_ledger_head: usize,
    pub project_display_name: Option<String>,
}

impl ProductSessionStore {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn list_session_ids(&self) -> Result<Vec<String>, SessionPersistenceError> {
        if !self.root.exists() {
            return Ok(Vec::new());
        }
        let mut session_ids = Vec::new();
        for entry in fs::read_dir(&self.root)
            .map_err(|error| SessionPersistenceError::Io(error.to_string()))?
        {
            let entry = entry.map_err(|error| SessionPersistenceError::Io(error.to_string()))?;
            if !entry
                .file_type()
                .map_err(|error| SessionPersistenceError::Io(error.to_string()))?
                .is_dir()
            {
                continue;
            }
            let session_id = entry.file_name().to_string_lossy().into_owned();
            if validate_session_id(&session_id).is_ok() && self.database_path(&session_id).exists()
            {
                session_ids.push(session_id);
            }
        }
        session_ids.sort();
        Ok(session_ids)
    }

    pub fn list_session_summaries(
        &self,
    ) -> Result<Vec<SessionListSummary>, SessionPersistenceError> {
        let session_ids = self.list_session_ids()?;
        let mut summaries = Vec::with_capacity(session_ids.len());
        for session_id in session_ids {
            let db_path = self.database_path(&session_id);
            let connection =
                Connection::open_with_flags(&db_path, OpenFlags::SQLITE_OPEN_READ_ONLY)
                    .map_err(|error| SessionPersistenceError::Sqlite(error.to_string()))?;
            configure_connection(&connection)?;
            let format_version = load_format_version(&connection, &session_id)?;
            if !supported_session_format(format_version) {
                continue;
            }
            let created_at_unix_ms: i64 = connection
                .query_row(
                    "SELECT created_at_unix_ms FROM session_metadata WHERE session_id = ?1",
                    [&session_id],
                    |row| row.get(0),
                )
                .map_err(|error| SessionPersistenceError::Sqlite(error.to_string()))?;
            let authority_display_label: String = connection
                .query_row(
                    "SELECT authority_display_label FROM declarations WHERE session_id = ?1",
                    [&session_id],
                    |row| row.get(0),
                )
                .map_err(|error| SessionPersistenceError::Sqlite(error.to_string()))?;
            let review_case_count: i64 = connection
                .query_row(
                    "SELECT COUNT(*) FROM review_cases WHERE session_id = ?1",
                    [&session_id],
                    |row| row.get(0),
                )
                .map_err(|error| SessionPersistenceError::Sqlite(error.to_string()))?;
            let review_ledger_head: i64 = connection
                .query_row(
                    "SELECT review_ledger_head FROM command_tokens WHERE session_id = ?1",
                    [&session_id],
                    |row| row.get(0),
                )
                .map_err(|error| SessionPersistenceError::Sqlite(error.to_string()))?;
            let project_display_name = load_list_project_display_name(&connection, &session_id)?;
            summaries.push(SessionListSummary {
                session_id,
                created_at_unix_ms,
                authority_display_label,
                review_case_count: review_case_count as usize,
                review_ledger_head: review_ledger_head as usize,
                project_display_name,
            });
        }
        summaries.sort_by(|left, right| {
            right
                .created_at_unix_ms
                .cmp(&left.created_at_unix_ms)
                .then_with(|| left.session_id.cmp(&right.session_id))
        });
        Ok(summaries)
    }

    pub fn create_session(
        &self,
        transcript: Transcript,
        session_terms: Vec<SessionTermEntry>,
        material_use: ApplicationMaterialUseDeclaration,
        session_authority: DeclaredSessionAuthority,
    ) -> Result<(String, OpenedStoreSession), SessionPersistenceError> {
        let session =
            begin_application_review(transcript, session_terms, material_use, session_authority)
                .map_err(SessionPersistenceError::Replay)?;
        let capture = capture_from_session(&session)?;
        let session_id = Uuid::new_v4().to_string();
        let db_path = self.database_path(&session_id);
        if db_path.exists() {
            return Err(SessionPersistenceError::SessionAlreadyExists);
        }
        fs::create_dir_all(db_path.parent().expect("database parent"))
            .map_err(|error| SessionPersistenceError::Io(error.to_string()))?;
        let mut connection = Connection::open(&db_path)
            .map_err(|error| SessionPersistenceError::Sqlite(error.to_string()))?;
        configure_connection(&connection)?;
        initialize_schema(&connection)?;
        let writer_token = Uuid::new_v4().to_string();
        let tx = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|error| SessionPersistenceError::Sqlite(error.to_string()))?;
        insert_initial_canonical(&tx, &session_id, &capture)?;
        tx.execute(
            "INSERT INTO authority_transitions (session_id, generation, acknowledgement_status) VALUES (?1, 1, 'committed')",
            [&session_id],
        )
        .map_err(|error| SessionPersistenceError::Sqlite(error.to_string()))?;
        let pid = std::process::id() as i64;
        tx.execute(
            "INSERT INTO writer_ownership (session_id, writer_token, holder_pid, lease_expires_at_unix_ms) VALUES (?1, ?2, ?3, 0)",
            params![session_id, writer_token.clone(), pid],
        )
        .map_err(|error| SessionPersistenceError::Sqlite(error.to_string()))?;
        tx.commit()
            .map_err(|error| SessionPersistenceError::Sqlite(error.to_string()))?;
        Ok((
            session_id.clone(),
            OpenedStoreSession {
                session_id,
                db_path,
                connection,
                mode: OpenMode::Writable,
                writer_token: Some(writer_token),
                format_version: PRODUCT_SESSION_FORMAT_VERSION,
            },
        ))
    }

    pub fn create_session_bound(
        &self,
        transcript: Transcript,
        session_terms: Vec<SessionTermEntry>,
        material_use: ApplicationMaterialUseDeclaration,
        session_authority: DeclaredSessionAuthority,
        project_id: &str,
        project_display_name: &str,
    ) -> Result<(String, OpenedStoreSession), SessionPersistenceError> {
        validate_session_id(project_id).map_err(|_| {
            SessionPersistenceError::CanonicalMismatch("invalid project id".to_owned())
        })?;
        let session =
            begin_application_review(transcript, session_terms, material_use, session_authority)
                .map_err(SessionPersistenceError::Replay)?;
        let capture = capture_from_session(&session)?;
        let session_id = Uuid::new_v4().to_string();
        let db_path = self.database_path(&session_id);
        if db_path.exists() {
            return Err(SessionPersistenceError::SessionAlreadyExists);
        }
        fs::create_dir_all(db_path.parent().expect("database parent"))
            .map_err(|error| SessionPersistenceError::Io(error.to_string()))?;
        let mut connection = Connection::open(&db_path)
            .map_err(|error| SessionPersistenceError::Sqlite(error.to_string()))?;
        configure_connection(&connection)?;
        initialize_schema(&connection)?;
        initialize_schema_v2_extras(&connection)?;
        let writer_token = Uuid::new_v4().to_string();
        let tx = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|error| SessionPersistenceError::Sqlite(error.to_string()))?;
        insert_initial_canonical(&tx, &session_id, &capture)?;
        tx.execute(
            "UPDATE session_metadata SET format_version = ?1 WHERE session_id = ?2",
            params![PRODUCT_SESSION_FORMAT_VERSION_V2, session_id],
        )
        .map_err(|error| SessionPersistenceError::Sqlite(error.to_string()))?;
        let updated = tx
            .execute(
                "UPDATE project_scope SET stable_id = ?1, display_name = ?2 WHERE session_id = ?3",
                params![project_id, project_display_name, session_id],
            )
            .map_err(|error| SessionPersistenceError::Sqlite(error.to_string()))?;
        if updated != 1 {
            return Err(SessionPersistenceError::CanonicalMismatch(
                "v2 project scope bind".to_owned(),
            ));
        }
        tx.execute(
            "INSERT INTO project_binding (session_id, project_id, project_memory_format_version) VALUES (?1, ?2, ?3)",
            params![
                session_id,
                project_id,
                crate::project_memory::PROJECT_MEMORY_FORMAT_VERSION
            ],
        )
        .map_err(|error| SessionPersistenceError::Sqlite(error.to_string()))?;
        tx.execute(
            "INSERT INTO authority_transitions (session_id, generation, acknowledgement_status) VALUES (?1, 1, 'committed')",
            [&session_id],
        )
        .map_err(|error| SessionPersistenceError::Sqlite(error.to_string()))?;
        let pid = std::process::id() as i64;
        tx.execute(
            "INSERT INTO writer_ownership (session_id, writer_token, holder_pid, lease_expires_at_unix_ms) VALUES (?1, ?2, ?3, 0)",
            params![session_id, writer_token.clone(), pid],
        )
        .map_err(|error| SessionPersistenceError::Sqlite(error.to_string()))?;
        tx.commit()
            .map_err(|error| SessionPersistenceError::Sqlite(error.to_string()))?;
        Ok((
            session_id.clone(),
            OpenedStoreSession {
                session_id,
                db_path,
                connection,
                mode: OpenMode::Writable,
                writer_token: Some(writer_token),
                format_version: PRODUCT_SESSION_FORMAT_VERSION_V2,
            },
        ))
    }

    pub fn open_session(
        &self,
        session_id: &str,
        mode: OpenMode,
    ) -> Result<OpenedStoreSession, SessionPersistenceError> {
        validate_session_id(session_id)?;
        let db_path = self.database_path(session_id);
        if !db_path.exists() {
            return Err(SessionPersistenceError::SessionNotFound);
        }
        let flags = match mode {
            OpenMode::Writable => OpenFlags::SQLITE_OPEN_READ_WRITE,
            OpenMode::ReadOnly => OpenFlags::SQLITE_OPEN_READ_ONLY,
        };
        let mut connection = Connection::open_with_flags(&db_path, flags)
            .map_err(|error| SessionPersistenceError::Sqlite(error.to_string()))?;
        configure_connection(&connection)?;
        let format_version = load_format_version(&connection, session_id)?;
        if !supported_session_format(format_version) {
            return Err(SessionPersistenceError::UnsupportedFormatVersion {
                found: format_version,
                supported: PRODUCT_SESSION_FORMAT_VERSION_V2,
            });
        }
        if format_version == PRODUCT_SESSION_FORMAT_VERSION_V2 {
            ensure_v2_p3_schema(&connection)?;
        }
        let writer_token = if mode == OpenMode::Writable {
            let token = Uuid::new_v4().to_string();
            let pid = std::process::id() as i64;
            let tx = connection
                .transaction_with_behavior(TransactionBehavior::Immediate)
                .map_err(|error| SessionPersistenceError::Sqlite(error.to_string()))?;
            let updated = tx
                .execute(
                    "UPDATE writer_ownership SET writer_token = ?1, holder_pid = ?2, lease_expires_at_unix_ms = 0 WHERE session_id = ?3 AND writer_token IS NULL",
                    params![token, pid, session_id],
                )
                .map_err(|error| SessionPersistenceError::Sqlite(error.to_string()))?;
            if updated != 1 {
                return Err(SessionPersistenceError::WriterOwnershipHeld);
            }
            tx.commit()
                .map_err(|error| SessionPersistenceError::Sqlite(error.to_string()))?;
            Some(token)
        } else {
            None
        };
        Ok(OpenedStoreSession {
            session_id: session_id.to_owned(),
            db_path,
            connection,
            mode,
            writer_token,
            format_version,
        })
    }

    pub fn append_review_ledger_event(
        opened: &mut OpenedStoreSession,
        prepared: &PreparedHumanDecision,
    ) -> Result<DurableAck, SessionPersistenceError> {
        if opened.mode != OpenMode::Writable {
            return Err(SessionPersistenceError::SessionNotWritable);
        }
        if opened.writer_token.is_none() {
            return Err(SessionPersistenceError::WriterOwnershipHeld);
        }
        let writer_token = opened
            .writer_token
            .as_ref()
            .expect("writer token checked above")
            .clone();
        let tx = opened
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|error| SessionPersistenceError::Sqlite(error.to_string()))?;
        verify_writer_token_in_transaction(&tx, &opened.session_id, &writer_token)?;
        let current_head: usize = tx
            .query_row(
                "SELECT review_ledger_head FROM command_tokens WHERE session_id = ?1",
                [&opened.session_id],
                |row| row.get(0),
            )
            .map_err(|error| SessionPersistenceError::Sqlite(error.to_string()))?;
        if current_head != prepared.expected_review_ledger_head {
            return Err(SessionPersistenceError::StaleAuthorityPrecondition(
                StaleAuthorityPrecondition {
                    scope: AuthorityScope::ReviewLedger,
                },
            ));
        }
        let revision_tag: String = tx
            .query_row(
                "SELECT revision_tag FROM source_payload WHERE session_id = ?1",
                [&opened.session_id],
                |row| row.get(0),
            )
            .map_err(|error| SessionPersistenceError::Sqlite(error.to_string()))?;
        let observed_revision = parse_revision_tag(&revision_tag)?;
        if FAIL_BEFORE_COMMIT.with(|flag| flag.get()) {
            return Err(SessionPersistenceError::Sqlite(
                "test fail before commit".to_owned(),
            ));
        }
        if let Some(target) = &prepared.reuse_proposal_target {
            let persisted_target =
                crate::session_persistence::canonical::persist_reuse_proposal_target(target);
            let target_json = serde_json::to_string(&persisted_target)
                .map_err(|error| SessionPersistenceError::CanonicalMismatch(error.to_string()))?;
            tx.execute(
                "INSERT OR IGNORE INTO reuse_proposal_targets (session_id, target_identity, target_json) VALUES (?1, ?2, ?3)",
                params![
                    opened.session_id,
                    target.identity().to_tagged_string(),
                    target_json
                ],
            )
            .map_err(|error| SessionPersistenceError::Sqlite(error.to_string()))?;
            if FAIL_AFTER_TARGET_BEFORE_LEDGER.with(|flag| flag.get()) {
                return Err(SessionPersistenceError::Sqlite(
                    "test fail after target before ledger".to_owned(),
                ));
            }
        }
        let persisted = persist_ledger_event(&match &prepared.reuse_proposal_target {
            Some(target) => crate::review::ReviewLedgerEvent::ReuseProposalDecisionRecorded {
                target_identity: target.identity(),
                observed_revision,
                decision: prepared.decision.clone(),
            },
            None => crate::review::ReviewLedgerEvent::DecisionRecorded {
                case_id: prepared.target.case_id().ok_or_else(|| {
                    SessionPersistenceError::CanonicalMismatch(
                        "canonical decision missing case id".to_owned(),
                    )
                })?,
                observed_revision,
                decision: prepared.decision.clone(),
            },
        });
        let event_json = serde_json::to_string(&persisted)
            .map_err(|error| SessionPersistenceError::CanonicalMismatch(error.to_string()))?;
        tx.execute(
            "INSERT INTO review_ledger_events (session_id, event_index, event_json) VALUES (?1, ?2, ?3)",
            params![opened.session_id, current_head as i64, event_json],
        )
        .map_err(|error| SessionPersistenceError::Sqlite(error.to_string()))?;
        let new_head = current_head + 1;
        tx.execute(
            "UPDATE command_tokens SET review_ledger_head = ?1 WHERE session_id = ?2",
            params![new_head as i64, opened.session_id],
        )
        .map_err(|error| SessionPersistenceError::Sqlite(error.to_string()))?;
        let generation: i64 = tx
            .query_row(
                "SELECT COALESCE(MAX(generation), 0) + 1 FROM authority_transitions WHERE session_id = ?1",
                [&opened.session_id],
                |row| row.get(0),
            )
            .map_err(|error| SessionPersistenceError::Sqlite(error.to_string()))?;
        tx.execute(
            "INSERT INTO authority_transitions (session_id, generation, acknowledgement_status) VALUES (?1, ?2, 'committed')",
            params![opened.session_id, generation],
        )
        .map_err(|error| SessionPersistenceError::Sqlite(error.to_string()))?;
        tx.commit()
            .map_err(|error| SessionPersistenceError::Sqlite(error.to_string()))?;
        Ok(DurableAck {
            committed_generation: generation as u64,
        })
    }

    pub fn release_writer(opened: &mut OpenedStoreSession) -> Result<(), SessionPersistenceError> {
        if opened.mode != OpenMode::Writable {
            return Ok(());
        }
        let writer_token = opened
            .writer_token
            .as_ref()
            .ok_or_else(|| SessionPersistenceError::WriterOwnershipHeld)?;
        let updated = opened
            .connection
            .execute(
                "UPDATE writer_ownership SET writer_token = NULL, holder_pid = NULL, lease_expires_at_unix_ms = 0 WHERE session_id = ?1 AND writer_token = ?2",
                params![opened.session_id, writer_token],
            )
            .map_err(|error| SessionPersistenceError::Sqlite(error.to_string()))?;
        if updated != 1 {
            return Err(SessionPersistenceError::WriterOwnershipHeld);
        }
        opened.writer_token = None;
        Ok(())
    }

    pub fn commit_v2_frozen_project_reuse(
        opened: &mut OpenedStoreSession,
        prepared: &crate::application_reuse::PreparedActiveAnalysis,
        precondition_selection_token: &str,
    ) -> Result<(), SessionPersistenceError> {
        if opened.format_version != PRODUCT_SESSION_FORMAT_VERSION_V2 {
            return Err(SessionPersistenceError::CanonicalMismatch(
                "v2 freeze requires session format 2".to_owned(),
            ));
        }
        if opened.mode != OpenMode::Writable {
            return Err(SessionPersistenceError::SessionNotWritable);
        }
        if opened.writer_token.is_none() {
            return Err(SessionPersistenceError::WriterOwnershipHeld);
        }
        let writer_token = opened
            .writer_token
            .as_ref()
            .expect("writer token checked above")
            .clone();
        let project_snapshot = prepared.project_memory_snapshot_identity.ok_or_else(|| {
            SessionPersistenceError::CanonicalMismatch(
                "v2 freeze missing project memory snapshot".to_owned(),
            )
        })?;
        let frozen = crate::reuse_proposal_target::FrozenProjectReuseAnalysis::new(
            project_snapshot,
            prepared.governance_event_boundary,
            prepared.reuse_analysis_snapshot,
        );
        if frozen.freeze_identity.to_tagged_string() != prepared.expected_selection_token {
            return Err(SessionPersistenceError::StaleAuthorityPrecondition(
                StaleAuthorityPrecondition {
                    scope: AuthorityScope::ActiveAnalysis,
                },
            ));
        }
        let tx = opened
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|error| SessionPersistenceError::Sqlite(error.to_string()))?;
        verify_writer_token_in_transaction(&tx, &opened.session_id, &writer_token)?;
        let current_selection: String = tx
            .query_row(
                "SELECT active_analysis_snapshot_identity FROM command_tokens WHERE session_id = ?1",
                [&opened.session_id],
                |row| row.get(0),
            )
            .map_err(|error| SessionPersistenceError::Sqlite(error.to_string()))?;
        if current_selection != precondition_selection_token {
            return Err(SessionPersistenceError::StaleAuthorityPrecondition(
                StaleAuthorityPrecondition {
                    scope: AuthorityScope::ActiveAnalysis,
                },
            ));
        }
        let persisted =
            crate::session_persistence::canonical::persist_frozen_project_reuse(&frozen);
        let snapshot_json = serde_json::to_string(&persisted.reuse_analysis_snapshot)
            .map_err(|error| SessionPersistenceError::CanonicalMismatch(error.to_string()))?;
        tx.execute(
            "INSERT OR REPLACE INTO analysis_snapshots (session_id, snapshot_identity, snapshot_json) VALUES (?1, ?2, ?3)",
            params![
                opened.session_id,
                prepared.expected_selection_token,
                snapshot_json
            ],
        )
        .map_err(|error| SessionPersistenceError::Sqlite(error.to_string()))?;
        tx.execute(
            "INSERT OR REPLACE INTO frozen_project_reuse_analysis (
                session_id, freeze_identity, project_memory_snapshot_identity,
                governance_event_boundary, reuse_analysis_snapshot_json
             ) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                opened.session_id,
                persisted.freeze_identity,
                persisted.project_memory_snapshot_identity,
                persisted.governance_event_boundary as i64,
                snapshot_json
            ],
        )
        .map_err(|error| SessionPersistenceError::Sqlite(error.to_string()))?;
        tx.execute(
            "UPDATE command_tokens SET active_analysis_snapshot_identity = ?1 WHERE session_id = ?2",
            params![prepared.expected_selection_token, opened.session_id],
        )
        .map_err(|error| SessionPersistenceError::Sqlite(error.to_string()))?;
        let generation: i64 = tx
            .query_row(
                "SELECT COALESCE(MAX(generation), 0) + 1 FROM authority_transitions WHERE session_id = ?1",
                [&opened.session_id],
                |row| row.get(0),
            )
            .map_err(|error| SessionPersistenceError::Sqlite(error.to_string()))?;
        tx.execute(
            "INSERT INTO authority_transitions (session_id, generation, acknowledgement_status) VALUES (?1, ?2, 'committed')",
            params![opened.session_id, generation],
        )
        .map_err(|error| SessionPersistenceError::Sqlite(error.to_string()))?;
        tx.commit()
            .map_err(|error| SessionPersistenceError::Sqlite(error.to_string()))?;
        Ok(())
    }

    pub(crate) fn load_canonical_capture(
        opened: &OpenedStoreSession,
    ) -> Result<SessionCanonicalCapture, SessionPersistenceError> {
        load_canonical_capture_from_connection(&opened.connection, &opened.session_id)
    }

    fn database_path(&self, session_id: &str) -> PathBuf {
        self.root.join(session_id).join("session.db")
    }
}

pub(crate) fn load_canonical_capture_from_connection(
    connection: &Connection,
    session_id: &str,
) -> Result<SessionCanonicalCapture, SessionPersistenceError> {
    let format_version = load_format_version(connection, session_id)?;
    if !supported_session_format(format_version) {
        return Err(SessionPersistenceError::UnsupportedFormatVersion {
            found: format_version,
            supported: PRODUCT_SESSION_FORMAT_VERSION_V2,
        });
    }
    let transcript_json: String = connection
        .query_row(
            "SELECT payload_json FROM source_payload WHERE session_id = ?1",
            [session_id],
            |row| row.get(0),
        )
        .map_err(|error| SessionPersistenceError::Sqlite(error.to_string()))?;
    let transcript = restore_transcript(
        &serde_json::from_str(&transcript_json)
            .map_err(|error| SessionPersistenceError::CanonicalMismatch(error.to_string()))?,
    );
    let terms_json: String = connection
        .query_row(
            "SELECT terms_json FROM session_terms WHERE session_id = ?1",
            [session_id],
            |row| row.get(0),
        )
        .map_err(|error| SessionPersistenceError::Sqlite(error.to_string()))?;
    let session_terms: Vec<crate::session_persistence::canonical::PersistedSessionTermV1> =
        serde_json::from_str(&terms_json)
            .map_err(|error| SessionPersistenceError::CanonicalMismatch(error.to_string()))?;
    let session_terms =
        crate::session_persistence::canonical::restore_session_terms(&session_terms);
    let analysis_json: String = connection
        .query_row(
            "SELECT snapshot_json FROM analysis_snapshots WHERE session_id = ?1 ORDER BY rowid LIMIT 1",
            [session_id],
            |row| row.get(0),
        )
        .map_err(|error| SessionPersistenceError::Sqlite(error.to_string()))?;
    let analysis_snapshot: crate::session_persistence::canonical::PersistedAnalysisSnapshotV1 =
        serde_json::from_str(&analysis_json)
            .map_err(|error| SessionPersistenceError::CanonicalMismatch(error.to_string()))?;
    let active_analysis_selection_identity: String = connection
        .query_row(
            "SELECT active_analysis_snapshot_identity FROM command_tokens WHERE session_id = ?1",
            [session_id],
            |row| row.get(0),
        )
        .map_err(|error| SessionPersistenceError::Sqlite(error.to_string()))?;
    let mut review_cases = Vec::new();
    let mut stmt = connection
        .prepare(
            "SELECT case_json FROM review_cases WHERE session_id = ?1 ORDER BY local_index ASC",
        )
        .map_err(|error| SessionPersistenceError::Sqlite(error.to_string()))?;
    let rows = stmt
        .query_map([session_id], |row| row.get::<_, String>(0))
        .map_err(|error| SessionPersistenceError::Sqlite(error.to_string()))?;
    for row in rows {
        let case_json = row.map_err(|error| SessionPersistenceError::Sqlite(error.to_string()))?;
        review_cases.push(
            serde_json::from_str(&case_json)
                .map_err(|error| SessionPersistenceError::CanonicalMismatch(error.to_string()))?,
        );
    }
    let (material_use_basis, authority_role, authority_display_label): (String, String, String) =
        connection
            .query_row(
                "SELECT material_use_basis, authority_role, authority_display_label FROM declarations WHERE session_id = ?1",
                [session_id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .map_err(|error| SessionPersistenceError::Sqlite(error.to_string()))?;
    let review_ledger_head: usize = connection
        .query_row(
            "SELECT review_ledger_head FROM command_tokens WHERE session_id = ?1",
            [session_id],
            |row| row.get::<_, i64>(0),
        )
        .map_err(|error| SessionPersistenceError::Sqlite(error.to_string()))?
        as usize;
    let mut ledger_events = Vec::new();
    let mut stmt = connection
        .prepare(
            "SELECT event_json FROM review_ledger_events WHERE session_id = ?1 ORDER BY event_index ASC",
        )
        .map_err(|error| SessionPersistenceError::Sqlite(error.to_string()))?;
    let rows = stmt
        .query_map([session_id], |row| row.get::<_, String>(0))
        .map_err(|error| SessionPersistenceError::Sqlite(error.to_string()))?;
    for row in rows {
        let event_json = row.map_err(|error| SessionPersistenceError::Sqlite(error.to_string()))?;
        ledger_events.push(
            serde_json::from_str(&event_json)
                .map_err(|error| SessionPersistenceError::CanonicalMismatch(error.to_string()))?,
        );
    }
    let project_scope: crate::session_persistence::reuse_canonical::PersistedProjectScopeV1 =
        connection
            .query_row(
                "SELECT stable_id, display_name FROM project_scope WHERE session_id = ?1",
                [session_id],
                |row| {
                    Ok(
                        crate::session_persistence::reuse_canonical::PersistedProjectScopeV1 {
                            stable_id: row.get(0)?,
                            display_name: row.get(1)?,
                        },
                    )
                },
            )
            .map_err(|error| SessionPersistenceError::Sqlite(error.to_string()))?;
    let reuse_governance_head: usize = connection
        .query_row(
            "SELECT reuse_governance_head FROM command_tokens WHERE session_id = ?1",
            [session_id],
            |row| row.get::<_, i64>(0),
        )
        .map_err(|error| SessionPersistenceError::Sqlite(error.to_string()))?
        as usize;
    let mut reuse_governance_events = Vec::new();
    let mut governance_stmt = connection
        .prepare(
            "SELECT event_json FROM reuse_governance_events WHERE session_id = ?1 ORDER BY event_index ASC",
        )
        .map_err(|error| SessionPersistenceError::Sqlite(error.to_string()))?;
    let governance_rows = governance_stmt
        .query_map([session_id], |row| row.get::<_, String>(0))
        .map_err(|error| SessionPersistenceError::Sqlite(error.to_string()))?;
    for row in governance_rows {
        let event_json = row.map_err(|error| SessionPersistenceError::Sqlite(error.to_string()))?;
        reuse_governance_events.push(
            serde_json::from_str(&event_json)
                .map_err(|error| SessionPersistenceError::CanonicalMismatch(error.to_string()))?,
        );
    }
    let mut reuse_enabled_bindings = Vec::new();
    let mut binding_stmt = connection
        .prepare(
            "SELECT binding_id, binding_json FROM reuse_enabled_bindings WHERE session_id = ?1 ORDER BY binding_id ASC",
        )
        .map_err(|error| SessionPersistenceError::Sqlite(error.to_string()))?;
    let binding_rows = binding_stmt
        .query_map([session_id], |row| {
            Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?))
        })
        .map_err(|error| SessionPersistenceError::Sqlite(error.to_string()))?;
    for row in binding_rows {
        let (binding_id, binding_json) =
            row.map_err(|error| SessionPersistenceError::Sqlite(error.to_string()))?;
        let binding: crate::session_persistence::reuse_canonical::PersistedReuseEnabledBindingV1 =
            serde_json::from_str(&binding_json)
                .map_err(|error| SessionPersistenceError::CanonicalMismatch(error.to_string()))?;
        if binding.binding_id != binding_id as usize {
            return Err(SessionPersistenceError::CanonicalMismatch(
                "reuse binding id mismatch".to_owned(),
            ));
        }
        reuse_enabled_bindings.push(binding);
    }
    Ok(SessionCanonicalCapture {
        transcript: crate::session_persistence::canonical::persist_transcript(&transcript),
        session_terms: crate::session_persistence::canonical::persist_session_terms(&session_terms),
        analysis_snapshot,
        active_analysis_selection_identity,
        review_cases,
        material_use_basis,
        authority_role,
        authority_display_label,
        review_ledger_head,
        ledger_events,
        project_scope,
        reuse_governance_events,
        reuse_governance_head,
        reuse_enabled_bindings,
        format_version,
        bound_project_id: if format_version == PRODUCT_SESSION_FORMAT_VERSION_V2 {
            Some(load_bound_project_id(connection, session_id)?)
        } else {
            None
        },
        frozen_project_reuse: if format_version == PRODUCT_SESSION_FORMAT_VERSION_V2 {
            load_frozen_project_reuse(connection, session_id)?
        } else {
            None
        },
        reuse_proposal_targets: if format_version == PRODUCT_SESSION_FORMAT_VERSION_V2 {
            load_reuse_proposal_targets(connection, session_id)?
        } else {
            Vec::new()
        },
    })
}

fn insert_initial_canonical(
    tx: &rusqlite::Transaction<'_>,
    session_id: &str,
    capture: &SessionCanonicalCapture,
) -> Result<(), SessionPersistenceError> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| SessionPersistenceError::Io(error.to_string()))?
        .as_millis() as i64;
    tx.execute(
        "INSERT INTO session_metadata (session_id, format_version, duplicated_from_session_id, created_at_unix_ms) VALUES (?1, ?2, NULL, ?3)",
        params![session_id, PRODUCT_SESSION_FORMAT_VERSION, now],
    )
    .map_err(|error| SessionPersistenceError::Sqlite(error.to_string()))?;
    let transcript_json = serde_json::to_string(&capture.transcript)
        .map_err(|error| SessionPersistenceError::CanonicalMismatch(error.to_string()))?;
    tx.execute(
        "INSERT INTO source_payload (session_id, revision_tag, payload_json) VALUES (?1, ?2, ?3)",
        params![session_id, capture.transcript.revision_tag, transcript_json],
    )
    .map_err(|error| SessionPersistenceError::Sqlite(error.to_string()))?;
    let terms_json = serde_json::to_string(&capture.session_terms)
        .map_err(|error| SessionPersistenceError::CanonicalMismatch(error.to_string()))?;
    tx.execute(
        "INSERT INTO session_terms (session_id, terms_identity, terms_json) VALUES (?1, ?2, ?3)",
        params![
            session_id,
            capture.analysis_snapshot.session_terms_identity,
            terms_json
        ],
    )
    .map_err(|error| SessionPersistenceError::Sqlite(error.to_string()))?;
    tx.execute(
        "INSERT INTO declarations (session_id, material_use_basis, authority_role, authority_display_label) VALUES (?1, ?2, ?3, ?4)",
        params![
            session_id,
            capture.material_use_basis,
            capture.authority_role,
            capture.authority_display_label
        ],
    )
    .map_err(|error| SessionPersistenceError::Sqlite(error.to_string()))?;
    let snapshot_json = serde_json::to_string(&capture.analysis_snapshot)
        .map_err(|error| SessionPersistenceError::CanonicalMismatch(error.to_string()))?;
    tx.execute(
        "INSERT INTO analysis_snapshots (session_id, snapshot_identity, snapshot_json) VALUES (?1, ?2, ?3)",
        params![
            session_id,
            capture.active_analysis_selection_identity,
            snapshot_json
        ],
    )
    .map_err(|error| SessionPersistenceError::Sqlite(error.to_string()))?;
    for review_case in &capture.review_cases {
        let case_json = serde_json::to_string(review_case)
            .map_err(|error| SessionPersistenceError::CanonicalMismatch(error.to_string()))?;
        tx.execute(
            "INSERT INTO review_cases (session_id, local_index, case_json) VALUES (?1, ?2, ?3)",
            params![session_id, review_case.local_index as i64, case_json],
        )
        .map_err(|error| SessionPersistenceError::Sqlite(error.to_string()))?;
    }
    tx.execute(
        "INSERT INTO project_scope (session_id, stable_id, display_name) VALUES (?1, ?2, ?3)",
        params![
            session_id,
            capture.project_scope.stable_id,
            capture.project_scope.display_name
        ],
    )
    .map_err(|error| SessionPersistenceError::Sqlite(error.to_string()))?;
    for (event_index, event) in capture.reuse_governance_events.iter().enumerate() {
        let event_json = serde_json::to_string(event)
            .map_err(|error| SessionPersistenceError::CanonicalMismatch(error.to_string()))?;
        tx.execute(
            "INSERT INTO reuse_governance_events (session_id, event_index, event_json) VALUES (?1, ?2, ?3)",
            params![session_id, event_index as i64, event_json],
        )
        .map_err(|error| SessionPersistenceError::Sqlite(error.to_string()))?;
    }
    for binding in &capture.reuse_enabled_bindings {
        let binding_json = serde_json::to_string(binding)
            .map_err(|error| SessionPersistenceError::CanonicalMismatch(error.to_string()))?;
        tx.execute(
            "INSERT INTO reuse_enabled_bindings (session_id, binding_id, binding_json) VALUES (?1, ?2, ?3)",
            params![session_id, binding.binding_id as i64, binding_json],
        )
        .map_err(|error| SessionPersistenceError::Sqlite(error.to_string()))?;
    }
    tx.execute(
        "INSERT INTO command_tokens (session_id, review_ledger_head, reuse_governance_head, active_analysis_snapshot_identity) VALUES (?1, 0, ?2, ?3)",
        params![
            session_id,
            capture.reuse_governance_head as i64,
            capture.active_analysis_selection_identity
        ],
    )
    .map_err(|error| SessionPersistenceError::Sqlite(error.to_string()))?;
    Ok(())
}

fn initialize_schema(connection: &Connection) -> Result<(), SessionPersistenceError> {
    connection
        .execute_batch(
            "
            CREATE TABLE session_metadata (
              session_id TEXT PRIMARY KEY,
              format_version INTEGER NOT NULL,
              duplicated_from_session_id TEXT,
              created_at_unix_ms INTEGER NOT NULL
            );
            CREATE TABLE source_payload (
              session_id TEXT PRIMARY KEY,
              revision_tag TEXT NOT NULL,
              payload_json TEXT NOT NULL
            );
            CREATE TABLE session_terms (
              session_id TEXT PRIMARY KEY,
              terms_identity TEXT NOT NULL,
              terms_json TEXT NOT NULL
            );
            CREATE TABLE declarations (
              session_id TEXT PRIMARY KEY,
              material_use_basis TEXT NOT NULL,
              authority_role TEXT NOT NULL,
              authority_display_label TEXT NOT NULL
            );
            CREATE TABLE analysis_snapshots (
              session_id TEXT NOT NULL,
              snapshot_identity TEXT NOT NULL,
              snapshot_json TEXT NOT NULL
            );
            CREATE TABLE review_cases (
              session_id TEXT NOT NULL,
              local_index INTEGER NOT NULL,
              case_json TEXT NOT NULL,
              PRIMARY KEY (session_id, local_index)
            );
            CREATE TABLE review_ledger_events (
              session_id TEXT NOT NULL,
              event_index INTEGER NOT NULL,
              event_json TEXT NOT NULL,
              PRIMARY KEY (session_id, event_index)
            );
            CREATE TABLE project_scope (
              session_id TEXT PRIMARY KEY,
              stable_id TEXT NOT NULL,
              display_name TEXT NOT NULL
            );
            CREATE TABLE reuse_governance_events (
              session_id TEXT NOT NULL,
              event_index INTEGER NOT NULL,
              event_json TEXT NOT NULL,
              PRIMARY KEY (session_id, event_index)
            );
            CREATE TABLE reuse_enabled_bindings (
              session_id TEXT NOT NULL,
              binding_id INTEGER NOT NULL,
              binding_json TEXT NOT NULL,
              PRIMARY KEY (session_id, binding_id)
            );
            CREATE TABLE command_tokens (
              session_id TEXT PRIMARY KEY,
              review_ledger_head INTEGER NOT NULL,
              reuse_governance_head INTEGER NOT NULL,
              active_analysis_snapshot_identity TEXT NOT NULL
            );
            CREATE TABLE authority_transitions (
              session_id TEXT NOT NULL,
              generation INTEGER NOT NULL,
              acknowledgement_status TEXT NOT NULL,
              PRIMARY KEY (session_id, generation)
            );
            CREATE TABLE writer_ownership (
              session_id TEXT PRIMARY KEY,
              writer_token TEXT,
              holder_pid INTEGER,
              lease_expires_at_unix_ms INTEGER NOT NULL
            );
            ",
        )
        .map_err(|error| SessionPersistenceError::Sqlite(error.to_string()))?;
    Ok(())
}

fn initialize_schema_v2_extras(connection: &Connection) -> Result<(), SessionPersistenceError> {
    connection
        .execute_batch(
            "
            CREATE TABLE project_binding (
              session_id TEXT PRIMARY KEY,
              project_id TEXT NOT NULL,
              project_memory_format_version INTEGER NOT NULL
            );
            CREATE TABLE frozen_project_reuse_analysis (
              session_id TEXT PRIMARY KEY,
              freeze_identity TEXT NOT NULL,
              project_memory_snapshot_identity TEXT NOT NULL,
              governance_event_boundary INTEGER NOT NULL,
              reuse_analysis_snapshot_json TEXT NOT NULL
            );
            CREATE TABLE reuse_proposal_targets (
              session_id TEXT NOT NULL,
              target_identity TEXT NOT NULL,
              target_json TEXT NOT NULL,
              PRIMARY KEY (session_id, target_identity)
            );
            ",
        )
        .map_err(|error| SessionPersistenceError::Sqlite(error.to_string()))?;
    Ok(())
}

fn ensure_v2_p3_schema(connection: &Connection) -> Result<(), SessionPersistenceError> {
    connection
        .execute_batch(
            "
            CREATE TABLE IF NOT EXISTS frozen_project_reuse_analysis (
              session_id TEXT PRIMARY KEY,
              freeze_identity TEXT NOT NULL,
              project_memory_snapshot_identity TEXT NOT NULL,
              governance_event_boundary INTEGER NOT NULL,
              reuse_analysis_snapshot_json TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS reuse_proposal_targets (
              session_id TEXT NOT NULL,
              target_identity TEXT NOT NULL,
              target_json TEXT NOT NULL,
              PRIMARY KEY (session_id, target_identity)
            );
            ",
        )
        .map_err(|error| SessionPersistenceError::Sqlite(error.to_string()))?;
    Ok(())
}

fn supported_session_format(format_version: u32) -> bool {
    format_version == PRODUCT_SESSION_FORMAT_VERSION
        || format_version == PRODUCT_SESSION_FORMAT_VERSION_V2
}

pub(crate) fn load_list_project_display_name(
    connection: &Connection,
    session_id: &str,
) -> Result<Option<String>, SessionPersistenceError> {
    let (stable_id, display_name): (String, String) = connection
        .query_row(
            "SELECT stable_id, display_name FROM project_scope WHERE session_id = ?1",
            [session_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .map_err(|error| SessionPersistenceError::Sqlite(error.to_string()))?;
    if stable_id.is_empty() || display_name.trim().is_empty() {
        Ok(None)
    } else {
        Ok(Some(display_name))
    }
}

pub(crate) fn load_bound_project_id(
    connection: &Connection,
    session_id: &str,
) -> Result<String, SessionPersistenceError> {
    connection
        .query_row(
            "SELECT project_id FROM project_binding WHERE session_id = ?1",
            [session_id],
            |row| row.get(0),
        )
        .map_err(|error| SessionPersistenceError::Sqlite(error.to_string()))
}

fn load_frozen_project_reuse(
    connection: &Connection,
    session_id: &str,
) -> Result<
    Option<crate::session_persistence::canonical::PersistedFrozenProjectReuseAnalysisV1>,
    SessionPersistenceError,
> {
    let mut stmt = connection
        .prepare(
            "SELECT freeze_identity, project_memory_snapshot_identity, governance_event_boundary, reuse_analysis_snapshot_json
             FROM frozen_project_reuse_analysis WHERE session_id = ?1",
        )
        .map_err(|error| SessionPersistenceError::Sqlite(error.to_string()))?;
    let mut rows = stmt
        .query([session_id])
        .map_err(|error| SessionPersistenceError::Sqlite(error.to_string()))?;
    let Some(row) = rows
        .next()
        .map_err(|error| SessionPersistenceError::Sqlite(error.to_string()))?
    else {
        return Ok(None);
    };
    let freeze_identity: String = row
        .get(0)
        .map_err(|error| SessionPersistenceError::Sqlite(error.to_string()))?;
    let project_memory_snapshot_identity: String = row
        .get(1)
        .map_err(|error| SessionPersistenceError::Sqlite(error.to_string()))?;
    let governance_event_boundary: i64 = row
        .get(2)
        .map_err(|error| SessionPersistenceError::Sqlite(error.to_string()))?;
    let snapshot_json: String = row
        .get(3)
        .map_err(|error| SessionPersistenceError::Sqlite(error.to_string()))?;
    let reuse_analysis_snapshot = serde_json::from_str(&snapshot_json)
        .map_err(|error| SessionPersistenceError::CanonicalMismatch(error.to_string()))?;
    Ok(Some(
        crate::session_persistence::canonical::PersistedFrozenProjectReuseAnalysisV1 {
            freeze_identity,
            project_memory_snapshot_identity,
            governance_event_boundary: governance_event_boundary as usize,
            reuse_analysis_snapshot,
        },
    ))
}

fn load_reuse_proposal_targets(
    connection: &Connection,
    session_id: &str,
) -> Result<
    Vec<crate::session_persistence::canonical::PersistedReuseProposalTargetV1>,
    SessionPersistenceError,
> {
    let mut stmt = connection
        .prepare(
            "SELECT target_identity, target_json FROM reuse_proposal_targets WHERE session_id = ?1 ORDER BY target_identity ASC",
        )
        .map_err(|error| SessionPersistenceError::Sqlite(error.to_string()))?;
    let rows = stmt
        .query_map([session_id], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })
        .map_err(|error| SessionPersistenceError::Sqlite(error.to_string()))?;
    let mut targets = Vec::new();
    for row in rows {
        let (target_identity, target_json) =
            row.map_err(|error| SessionPersistenceError::Sqlite(error.to_string()))?;
        let target: crate::session_persistence::canonical::PersistedReuseProposalTargetV1 =
            serde_json::from_str(&target_json)
                .map_err(|error| SessionPersistenceError::CanonicalMismatch(error.to_string()))?;
        if target.target_identity != target_identity {
            return Err(SessionPersistenceError::CanonicalMismatch(
                "reuse proposal target identity mismatch".to_owned(),
            ));
        }
        targets.push(target);
    }
    Ok(targets)
}

fn configure_connection(connection: &Connection) -> Result<(), SessionPersistenceError> {
    connection
        .pragma_update(None, "foreign_keys", "ON")
        .map_err(|error| SessionPersistenceError::Sqlite(error.to_string()))?;
    connection
        .pragma_update(None, "journal_mode", "WAL")
        .map_err(|error| SessionPersistenceError::Sqlite(error.to_string()))?;
    connection
        .pragma_update(None, "synchronous", "FULL")
        .map_err(|error| SessionPersistenceError::Sqlite(error.to_string()))?;
    Ok(())
}

fn load_format_version(
    connection: &Connection,
    session_id: &str,
) -> Result<u32, SessionPersistenceError> {
    connection
        .query_row(
            "SELECT format_version FROM session_metadata WHERE session_id = ?1",
            [session_id],
            |row| row.get(0),
        )
        .map_err(|error| SessionPersistenceError::Sqlite(error.to_string()))
}

fn validate_session_id(session_id: &str) -> Result<(), SessionPersistenceError> {
    if session_id.is_empty() || session_id.contains('/') || session_id.contains('\\') {
        return Err(SessionPersistenceError::InvalidSessionId);
    }
    Ok(())
}

pub(crate) fn verify_writer_token_in_transaction(
    tx: &rusqlite::Transaction<'_>,
    session_id: &str,
    expected_token: &str,
) -> Result<(), SessionPersistenceError> {
    let db_token: Option<String> = tx
        .query_row(
            "SELECT writer_token FROM writer_ownership WHERE session_id = ?1",
            [session_id],
            |row| row.get(0),
        )
        .map_err(|error| SessionPersistenceError::Sqlite(error.to_string()))?;
    match db_token.as_deref() {
        Some(token) if token == expected_token => Ok(()),
        _ => Err(SessionPersistenceError::WriterOwnershipHeld),
    }
}

pub(crate) fn parse_revision_tag(
    tag: &str,
) -> Result<crate::anchor::TranscriptRevisionId, SessionPersistenceError> {
    let hex = tag
        .strip_prefix("rev:sha256-v1:")
        .ok_or_else(|| SessionPersistenceError::CanonicalMismatch("revision tag".to_owned()))?;
    let mut digest = [0_u8; 32];
    for (index, chunk) in hex.as_bytes().chunks(2).enumerate() {
        if index >= 32 || chunk.len() != 2 {
            return Err(SessionPersistenceError::CanonicalMismatch(
                "revision digest".to_owned(),
            ));
        }
        let hi = (chunk[0] as char).to_digit(16).ok_or_else(|| {
            SessionPersistenceError::CanonicalMismatch("revision digest".to_owned())
        })? as u8;
        let lo = (chunk[1] as char).to_digit(16).ok_or_else(|| {
            SessionPersistenceError::CanonicalMismatch("revision digest".to_owned())
        })? as u8;
        digest[index] = (hi << 4) | lo;
    }
    Ok(crate::anchor::TranscriptRevisionId::from_sha256_digest(
        digest,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn schema_initializes_all_families() {
        let connection = Connection::open_in_memory().expect("memory db");
        initialize_schema(&connection).expect("schema");
        let tables = [
            "session_metadata",
            "source_payload",
            "session_terms",
            "declarations",
            "analysis_snapshots",
            "review_cases",
            "review_ledger_events",
            "project_scope",
            "reuse_governance_events",
            "reuse_enabled_bindings",
            "command_tokens",
            "authority_transitions",
            "writer_ownership",
        ];
        for table in tables {
            let count: i64 = connection
                .query_row(
                    "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = ?1",
                    [table],
                    |row| row.get(0),
                )
                .expect("table exists");
            assert_eq!(count, 1, "missing table {table}");
        }
    }
}
