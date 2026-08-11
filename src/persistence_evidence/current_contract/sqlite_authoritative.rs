//! Bounded, spike-only SQLite authority for current-contract v3 fixtures.
//!
//! This is a distinct candidate for later 01C comparison.  Its canonical
//! authority is held in the relational tables in this module; the only
//! serialized payload is the explicitly non-authoritative derived cache.
//! It neither imports nor adapts the historical `NormalizedSemanticState`
//! SQLite candidate.

use std::cell::Cell;
use std::collections::BTreeMap;
use std::fs::{self, File, OpenOptions};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use rusqlite::{
    Connection, OpenFlags, OptionalExtension, Transaction, TransactionBehavior, params,
};
use serde::Serialize;
use sha2::{Digest, Sha256};

use super::derivation::{derive_contract_projection, finalize_derived_fields};
use super::model::{
    CurrentContractState, EvidenceAnalysisSnapshot, EvidenceDetectorIdentity,
    EvidenceExactReusableCorrection, EvidenceGovernanceActor, EvidenceMaterialUseDeclaration,
    EvidenceProjectScope, EvidenceReuseCandidateKey, EvidenceReuseEnabledAnalysisBinding,
    EvidenceReuseGovernanceEvent, EvidenceReviewCase, EvidenceReviewLedgerEvent,
    EvidenceSessionAuthority, EvidenceSourceDecisionLocator, EvidenceSourceRevision,
};
use super::oracle::{CurrentContractOracle, canonical_fingerprint};

pub const SQLITE_AUTHORITATIVE_CANDIDATE_ID: &str =
    "current-contract-sqlite-authoritative-candidate";
pub const SQLITE_AUTHORITATIVE_CANDIDATE_VERSION: &str = "01C-SQLITE-2";
pub const SQLITE_AUTHORITATIVE_FORMAT_VERSION: u32 = 1;

const MAX_TEXT_BYTES: usize = 2 * 1024 * 1024;
const MAX_ROWS_PER_TABLE: usize = 4_096;
const MAX_PAGE_COUNT: i64 = 4_096;
const MAX_DATABASE_BYTES: u64 = 32 * 1024 * 1024;
const DEFAULT_LEASE_DURATION_MS: i64 = 30_000;
const DERIVED_CACHE_SCHEMA_VERSION: i64 = 1;
const CREATION_MARKER_NAME: &str = ".creation-incomplete";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SqliteAuthoritySession {
    session_id: String,
    root: PathBuf,
}

impl SqliteAuthoritySession {
    pub fn session_id(&self) -> &str {
        &self.session_id
    }

    /// Test-support access to the physical candidate directory. It is not a
    /// semantic session identifier or a persistence-format field.
    #[doc(hidden)]
    pub fn storage_path_for_test(&self) -> &Path {
        &self.root
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SqliteOpenMode {
    Writable,
    ReadOnly,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CurrentContractPreconditions {
    pub expected_generation: u64,
    pub review_ledger_head: usize,
    pub reuse_governance_head: usize,
    pub active_analysis_snapshot_identity: String,
}

#[derive(Debug)]
pub struct OpenedSqliteAuthoritySession {
    pub session: SqliteAuthoritySession,
    pub committed_generation: u64,
    open_mode: SqliteOpenMode,
    adapter_identity: String,
    writer_token: Option<String>,
    writer_epoch: Option<i64>,
    normalized_state: CurrentContractState,
}

impl OpenedSqliteAuthoritySession {
    pub fn normalized_state(&self) -> &CurrentContractState {
        &self.normalized_state
    }

    pub fn authoritative_preconditions(&self) -> CurrentContractPreconditions {
        CurrentContractPreconditions {
            expected_generation: self.committed_generation,
            review_ledger_head: self
                .normalized_state
                .durable_command_tokens
                .review_ledger_head,
            reuse_governance_head: self
                .normalized_state
                .durable_command_tokens
                .reuse_governance_head,
            active_analysis_snapshot_identity: self
                .normalized_state
                .durable_command_tokens
                .active_analysis_snapshot_identity
                .clone(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DurableSqliteAck {
    pub committed_generation: u64,
    pub canonical_fingerprint: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SqliteAuthorityError {
    pub code: &'static str,
    pub message: String,
}

impl SqliteAuthorityError {
    fn new(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }
}

impl std::fmt::Display for SqliteAuthorityError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}: {}", self.code, self.message)
    }
}

impl std::error::Error for SqliteAuthorityError {}

/// Candidate-only relational storage surface for the frozen current-contract
/// v3 evidence contract.
pub struct SqliteAuthoritativeCandidateAdapter {
    storage_root: PathBuf,
    adapter_identity: String,
    fail_before_commit_for_test: Cell<bool>,
    fail_after_commit_before_ack_for_test: Cell<bool>,
}

impl SqliteAuthoritativeCandidateAdapter {
    pub fn new(storage_root: impl Into<PathBuf>) -> Result<Self, SqliteAuthorityError> {
        let storage_root = storage_root.into();
        match fs::symlink_metadata(&storage_root) {
            Ok(metadata) if is_static_filesystem_alias(&metadata) => {
                return Err(SqliteAuthorityError::new(
                    "aliased-storage-root",
                    "candidate storage root must not be a static filesystem alias",
                ));
            }
            Ok(metadata) if !metadata.is_dir() => {
                return Err(SqliteAuthorityError::new(
                    "storage-root-not-directory",
                    "candidate storage root must be a directory",
                ));
            }
            Ok(_) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                fs::create_dir_all(&storage_root)
                    .map_err(|error| io_error("create-storage-root", error))?;
            }
            Err(error) => return Err(io_error("stat-storage-root", error)),
        }
        let storage_root = fs::canonicalize(storage_root)
            .map_err(|error| io_error("canonicalize-storage-root", error))?;
        let metadata = fs::symlink_metadata(&storage_root)
            .map_err(|error| io_error("stat-storage-root", error))?;
        if is_static_filesystem_alias(&metadata) || !metadata.is_dir() {
            return Err(SqliteAuthorityError::new(
                "aliased-storage-root",
                "candidate storage root must resolve to an owned directory",
            ));
        }
        Ok(Self {
            storage_root,
            adapter_identity: uuid::Uuid::new_v4().to_string(),
            fail_before_commit_for_test: Cell::new(false),
            fail_after_commit_before_ack_for_test: Cell::new(false),
        })
    }

    pub fn candidate_id(&self) -> &'static str {
        SQLITE_AUTHORITATIVE_CANDIDATE_ID
    }

    pub fn candidate_version(&self) -> &'static str {
        SQLITE_AUTHORITATIVE_CANDIDATE_VERSION
    }

    pub fn format_version(&self) -> u32 {
        SQLITE_AUTHORITATIVE_FORMAT_VERSION
    }

    pub fn create(
        &self,
        state: &CurrentContractState,
    ) -> Result<SqliteAuthoritySession, SqliteAuthorityError> {
        let state = canonicalized_state(state.clone());
        validate_state(&state)?;
        validate_session_id(&state.session_id)?;
        validate_state_bounds(&state)?;

        let session = SqliteAuthoritySession {
            session_id: state.session_id.clone(),
            root: self.session_root(&state.session_id),
        };
        fs::create_dir(&session.root).map_err(|error| {
            if error.kind() == std::io::ErrorKind::AlreadyExists {
                SqliteAuthorityError::new(
                    "session-already-exists",
                    "candidate session already exists",
                )
            } else {
                io_error("create-session-root", error)
            }
        })?;
        OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(creation_marker_path(&session))
            .map_err(|error| io_error("create-session-marker", error))?;

        let result = (|| {
            let mut connection = self.open_new_database(&session)?;
            configure_writable_persistence(&connection)?;
            initialize_schema(&connection)?;
            let fingerprint = canonical_fingerprint(&state.canonical_projection());
            let tx = connection
                .transaction_with_behavior(TransactionBehavior::Immediate)
                .map_err(sql_error("sqlite-create-transaction"))?;
            replace_canonical_tables(&tx, &state)?;
            insert_session_metadata(&tx, &state, 1, &fingerprint)?;
            tx.execute(
                "INSERT INTO authority_transitions (generation, canonical_fingerprint, acknowledgement_status) VALUES (?1, ?2, 'committed')",
                params![1_i64, fingerprint],
            )
            .map_err(sql_error("sqlite-create-transition"))?;
            tx.execute(
                "INSERT INTO writer_ownership (id, token, owner_epoch, lease_duration_ms, lease_expires_at_unix_ms, process_instance_id, holder_pid) VALUES (1, NULL, 0, ?1, 0, '', NULL)",
                [DEFAULT_LEASE_DURATION_MS],
            )
            .map_err(sql_error("sqlite-create-writer"))?;
            rebuild_derived_cache_in_tx(&tx, &state, &fingerprint)?;
            tx.commit().map_err(sql_error("sqlite-create-commit"))?;

            let (metadata, reconstructed) = self.verify_fresh(&session, false)?;
            if metadata.committed_generation != 1
                || !CurrentContractOracle::compare(&state, &reconstructed).passed
            {
                return Err(SqliteAuthorityError::new(
                    "post-commit-verification-failed",
                    "initial SQLite authority did not reconstruct through the v3 oracle",
                ));
            }
            mark_transition_acknowledged(&connection, 1)?;
            Ok(())
        })();
        result?;
        fs::remove_file(creation_marker_path(&session))
            .map_err(|error| io_error("publish-session", error))?;
        Ok(session)
    }

    /// Reconstructs a session reference from only this adapter's bounded root
    /// and a semantic identity. The ID is never used as a physical pathname.
    pub fn open_existing(
        &self,
        session_id: impl Into<String>,
        mode: SqliteOpenMode,
    ) -> Result<OpenedSqliteAuthoritySession, SqliteAuthorityError> {
        let session_id = session_id.into();
        validate_session_id(&session_id)?;
        let session = SqliteAuthoritySession {
            root: self.session_root(&session_id),
            session_id,
        };
        self.validate_session_layout(&session, true)?;
        let mut connection = self.open_existing_database(&session, mode)?;
        // Validate the format and complete relational authority before any
        // writable open mutates a lease, journal mode, or derived cache.
        let (mut metadata, mut state) =
            load_authority_for_session(&connection, &session.session_id, false)?;
        let writer = if mode == SqliteOpenMode::Writable {
            configure_writable_persistence(&connection)?;
            let writer = acquire_writer_ownership(&mut connection, &self.adapter_identity)?;
            let reloaded = load_authority_for_session(&connection, &session.session_id, false);
            let (reloaded_metadata, reloaded_state) = match reloaded {
                Ok(value) => value,
                Err(error) => {
                    let _ = release_writer_ownership(&connection, &writer.0, writer.1);
                    return Err(error);
                }
            };
            if let Err(error) = (|| {
                rebuild_derived_cache(
                    &connection,
                    &reloaded_state,
                    &reloaded_metadata.canonical_fingerprint,
                )?;
                mark_transition_acknowledged(&connection, reloaded_metadata.committed_generation)
            })() {
                let _ = release_writer_ownership(&connection, &writer.0, writer.1);
                return Err(error);
            }
            metadata = reloaded_metadata;
            state = reloaded_state;
            Some(writer)
        } else {
            None
        };
        Ok(OpenedSqliteAuthoritySession {
            session,
            committed_generation: metadata.committed_generation,
            open_mode: mode,
            adapter_identity: self.adapter_identity.clone(),
            writer_token: writer.as_ref().map(|(token, _)| token.clone()),
            writer_epoch: writer.map(|(_, epoch)| epoch),
            normalized_state: state,
        })
    }

    pub fn apply_authoritative_transition(
        &self,
        opened: &mut OpenedSqliteAuthoritySession,
        preconditions: &CurrentContractPreconditions,
        next_state: &CurrentContractState,
    ) -> Result<DurableSqliteAck, SqliteAuthorityError> {
        self.validate_writable_handle(opened)?;
        self.validate_session_layout(&opened.session, true)?;
        let canonical_next_state = canonicalized_state(next_state.clone());
        validate_state(&canonical_next_state)?;
        validate_state_bounds(&canonical_next_state)?;
        if canonical_next_state.session_id != opened.session.session_id {
            return Err(SqliteAuthorityError::new(
                "session-identity-transition",
                "authoritative transition cannot change the semantic session identity",
            ));
        }

        let mut connection =
            self.open_existing_database(&opened.session, SqliteOpenMode::Writable)?;
        let token = opened.writer_token.as_deref().ok_or_else(|| {
            SqliteAuthorityError::new("not-authoritative-writer", "missing writer token")
        })?;
        let epoch = opened.writer_epoch.ok_or_else(|| {
            SqliteAuthorityError::new("not-authoritative-writer", "missing writer epoch")
        })?;
        validate_writer_ownership(&connection, token, epoch)?;
        let (metadata, _) =
            load_authority_for_session(&connection, &opened.session.session_id, false)?;
        validate_preconditions(&metadata, preconditions)?;
        configure_writable_persistence(&connection)?;

        let fingerprint = canonical_fingerprint(&canonical_next_state.canonical_projection());
        let next_generation = metadata
            .committed_generation
            .checked_add(1)
            .ok_or_else(|| {
                SqliteAuthorityError::new("generation-overflow", "generation overflow")
            })?;
        let tx = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(sql_error("sqlite-transition-begin"))?;
        let metadata_in_tx = load_session_metadata_tx(&tx)?;
        validate_preconditions(&metadata_in_tx, preconditions)?;
        validate_writer_ownership_tx(&tx, token, epoch)?;
        let duplicate: Option<i64> = tx
            .query_row(
                "SELECT 1 FROM authority_transitions WHERE canonical_fingerprint = ?1 LIMIT 1",
                [&fingerprint],
                |row| row.get(0),
            )
            .optional()
            .map_err(sql_error("sqlite-transition-duplicate-check"))?;
        if duplicate.is_some() {
            return Err(SqliteAuthorityError::new(
                "semantic-duplicate-transition",
                "authoritative transition repeats a committed canonical identity",
            ));
        }
        replace_canonical_tables(&tx, &canonical_next_state)?;
        update_session_metadata(&tx, &canonical_next_state, next_generation, &fingerprint)?;
        tx.execute(
            "INSERT INTO authority_transitions (generation, canonical_fingerprint, acknowledgement_status) VALUES (?1, ?2, 'committed')",
            params![u64_to_i64(next_generation)?, fingerprint],
        )
        .map_err(sql_error("sqlite-transition-record"))?;
        renew_writer_lease_tx(&tx, token, epoch)?;
        rebuild_derived_cache_in_tx(&tx, &canonical_next_state, &fingerprint)?;

        if self.fail_before_commit_for_test.replace(false) {
            return Err(SqliteAuthorityError::new(
                "injected-interrupted-transition",
                "test fault returned before SQLite commit",
            ));
        }
        tx.commit().map_err(sql_error("sqlite-transition-commit"))?;

        let (fresh_metadata, reconstructed) = self.verify_fresh(&opened.session, true)?;
        let comparison = CurrentContractOracle::compare(&canonical_next_state, &reconstructed);
        if fresh_metadata.committed_generation != next_generation || !comparison.passed {
            return Err(SqliteAuthorityError::new(
                "post-commit-verification-failed",
                "committed SQLite rows did not independently reconstruct through oracle v3",
            ));
        }
        if self.fail_after_commit_before_ack_for_test.replace(false) {
            return Err(SqliteAuthorityError::new(
                "injected-after-commit-before-ack",
                "test fault returned after commit and before acknowledgement",
            ));
        }
        validate_writer_ownership(&connection, token, epoch)?;
        mark_transition_acknowledged(&connection, next_generation)?;
        opened.committed_generation = next_generation;
        opened.normalized_state = canonical_next_state;
        Ok(DurableSqliteAck {
            committed_generation: next_generation,
            canonical_fingerprint: fingerprint,
        })
    }

    pub fn close(&self, opened: OpenedSqliteAuthoritySession) -> Result<(), SqliteAuthorityError> {
        if opened.open_mode == SqliteOpenMode::Writable {
            let token = opened.writer_token.as_deref().ok_or_else(|| {
                SqliteAuthorityError::new("not-authoritative-writer", "missing writer token")
            })?;
            let epoch = opened.writer_epoch.ok_or_else(|| {
                SqliteAuthorityError::new("not-authoritative-writer", "missing writer epoch")
            })?;
            let connection =
                self.open_existing_database(&opened.session, SqliteOpenMode::Writable)?;
            release_writer_ownership(&connection, token, epoch)?;
        }
        Ok(())
    }

    pub fn duplicate(
        &self,
        source: &mut OpenedSqliteAuthoritySession,
        new_session_id: impl Into<String>,
    ) -> Result<SqliteAuthoritySession, SqliteAuthorityError> {
        self.validate_writable_handle(source)?;
        let new_session_id = new_session_id.into();
        validate_session_id(&new_session_id)?;
        let connection = self.open_existing_database(&source.session, SqliteOpenMode::Writable)?;
        let token = source.writer_token.as_deref().ok_or_else(|| {
            SqliteAuthorityError::new("not-authoritative-writer", "missing writer token")
        })?;
        let epoch = source.writer_epoch.ok_or_else(|| {
            SqliteAuthorityError::new("not-authoritative-writer", "missing writer epoch")
        })?;
        validate_writer_ownership(&connection, token, epoch)?;
        let (_, mut copied) =
            load_authority_for_session(&connection, &source.session.session_id, false)?;
        configure_writable_persistence(&connection)?;
        copied.duplicated_from_session_id = Some(copied.session_id.clone());
        copied.session_id = new_session_id;
        copied.durable_command_tokens.evidence_writer_token =
            derive_duplicate_evidence_writer_token(&copied.session_id);
        finalize_derived_fields(&mut copied);
        copied = copied.normalize();
        self.create(&copied)
    }

    /// Test-only fault: abandon an in-transaction transition before commit.
    #[doc(hidden)]
    pub fn arm_fail_before_commit_for_test(&self) {
        self.fail_before_commit_for_test.set(true);
    }

    /// Test-only fault: commit authority but withhold acknowledgement.
    #[doc(hidden)]
    pub fn arm_fail_after_commit_before_ack_for_test(&self) {
        self.fail_after_commit_before_ack_for_test.set(true);
    }

    #[doc(hidden)]
    pub fn set_format_version_for_test(
        &self,
        opened: &OpenedSqliteAuthoritySession,
        format_version: u32,
    ) -> Result<(), SqliteAuthorityError> {
        self.validate_writable_handle(opened)?;
        let connection = self.open_existing_database(&opened.session, SqliteOpenMode::Writable)?;
        let token = opened.writer_token.as_deref().ok_or_else(|| {
            SqliteAuthorityError::new("not-authoritative-writer", "missing writer token")
        })?;
        validate_writer_ownership(&connection, token, opened.writer_epoch.unwrap_or_default())?;
        connection
            .execute(
                "UPDATE session_meta SET format_version = ?1 WHERE id = 1",
                [i64::from(format_version)],
            )
            .map_err(sql_error("sqlite-test-format"))?;
        Ok(())
    }

    #[doc(hidden)]
    pub fn tamper_canonical_provenance_for_test(
        &self,
        opened: &OpenedSqliteAuthoritySession,
    ) -> Result<(), SqliteAuthorityError> {
        self.validate_writable_handle(opened)?;
        let connection = self.open_existing_database(&opened.session, SqliteOpenMode::Writable)?;
        let token = opened.writer_token.as_deref().ok_or_else(|| {
            SqliteAuthorityError::new("not-authoritative-writer", "missing writer token")
        })?;
        validate_writer_ownership(&connection, token, opened.writer_epoch.unwrap_or_default())?;
        connection
            .execute(
                "UPDATE review_ledger_events SET provenance = 'automatic' WHERE event_index = 0",
                [],
            )
            .map_err(sql_error("sqlite-test-canonical-tamper"))?;
        Ok(())
    }

    #[doc(hidden)]
    pub fn tamper_derived_cache_for_test(
        &self,
        opened: &OpenedSqliteAuthoritySession,
        payload: &str,
    ) -> Result<(), SqliteAuthorityError> {
        self.validate_writable_handle(opened)?;
        let connection = self.open_existing_database(&opened.session, SqliteOpenMode::Writable)?;
        let token = opened.writer_token.as_deref().ok_or_else(|| {
            SqliteAuthorityError::new("not-authoritative-writer", "missing writer token")
        })?;
        validate_writer_ownership(&connection, token, opened.writer_epoch.unwrap_or_default())?;
        connection
            .execute(
                "UPDATE derived_contract_cache SET payload_json = ?1 WHERE id = 1",
                [payload],
            )
            .map_err(sql_error("sqlite-test-derived-tamper"))?;
        Ok(())
    }

    #[doc(hidden)]
    pub fn inject_oversized_derived_cache_for_test(
        &self,
        opened: &OpenedSqliteAuthoritySession,
    ) -> Result<(), SqliteAuthorityError> {
        self.validate_writable_handle(opened)?;
        let connection = self.open_existing_database(&opened.session, SqliteOpenMode::Writable)?;
        let token = opened.writer_token.as_deref().ok_or_else(|| {
            SqliteAuthorityError::new("not-authoritative-writer", "missing writer token")
        })?;
        validate_writer_ownership(&connection, token, opened.writer_epoch.unwrap_or_default())?;
        connection
            .execute_batch("PRAGMA ignore_check_constraints = ON;")
            .map_err(sql_error("sqlite-test-ignore-check"))?;
        let oversized = "x".repeat(MAX_TEXT_BYTES + 1);
        let outcome = connection.execute(
            "UPDATE derived_contract_cache SET payload_json = ?1 WHERE id = 1",
            [oversized],
        );
        let _ = connection.execute_batch("PRAGMA ignore_check_constraints = OFF;");
        outcome.map_err(sql_error("sqlite-test-oversized-cache"))?;
        Ok(())
    }

    #[doc(hidden)]
    pub fn set_lease_duration_for_test(
        &self,
        session: &SqliteAuthoritySession,
        lease_duration_ms: i64,
    ) -> Result<(), SqliteAuthorityError> {
        if lease_duration_ms <= 0 {
            return Err(SqliteAuthorityError::new(
                "invalid-test-lease-duration",
                "lease duration must be positive",
            ));
        }
        let connection = self.open_existing_database(session, SqliteOpenMode::Writable)?;
        connection
            .execute(
                "UPDATE writer_ownership SET lease_duration_ms = ?1 WHERE id = 1 AND token IS NULL",
                [lease_duration_ms],
            )
            .map_err(sql_error("sqlite-test-lease-duration"))?;
        Ok(())
    }

    #[doc(hidden)]
    pub fn expire_writer_lease_for_test(
        &self,
        opened: &OpenedSqliteAuthoritySession,
    ) -> Result<(), SqliteAuthorityError> {
        self.validate_writable_handle(opened)?;
        let connection = self.open_existing_database(&opened.session, SqliteOpenMode::Writable)?;
        let token = opened.writer_token.as_deref().ok_or_else(|| {
            SqliteAuthorityError::new("not-authoritative-writer", "missing writer token")
        })?;
        let epoch = opened.writer_epoch.ok_or_else(|| {
            SqliteAuthorityError::new("not-authoritative-writer", "missing writer epoch")
        })?;
        let updated = connection
            .execute(
                "UPDATE writer_ownership SET lease_expires_at_unix_ms = 0 WHERE id = 1 AND token = ?1 AND owner_epoch = ?2",
                params![token, epoch],
            )
            .map_err(sql_error("sqlite-test-expire-lease"))?;
        if updated != 1 {
            return Err(SqliteAuthorityError::new(
                "writer-epoch-mismatch",
                "test writer no longer owns the lease being expired",
            ));
        }
        Ok(())
    }

    fn validate_writable_handle(
        &self,
        opened: &OpenedSqliteAuthoritySession,
    ) -> Result<(), SqliteAuthorityError> {
        if opened.adapter_identity != self.adapter_identity {
            return Err(SqliteAuthorityError::new(
                "foreign-writer-handle",
                "writer handle belongs to another candidate instance",
            ));
        }
        if opened.open_mode != SqliteOpenMode::Writable
            || opened.writer_token.is_none()
            || opened.writer_epoch.is_none()
        {
            return Err(SqliteAuthorityError::new(
                "not-authoritative-writer",
                "operation requires a live writable authority handle",
            ));
        }
        Ok(())
    }

    fn session_root(&self, session_id: &str) -> PathBuf {
        self.storage_root.join(physical_storage_key(session_id))
    }

    fn validate_session_layout(
        &self,
        session: &SqliteAuthoritySession,
        require_published: bool,
    ) -> Result<(), SqliteAuthorityError> {
        validate_session_id(&session.session_id)?;
        let expected = self.session_root(&session.session_id);
        if session.root != expected {
            return Err(SqliteAuthorityError::new(
                "foreign-session-handle",
                "session handle does not belong to this bounded storage root",
            ));
        }
        let metadata = fs::symlink_metadata(&session.root)
            .map_err(|error| io_error("stat-session-root", error))?;
        if is_static_filesystem_alias(&metadata) || !metadata.is_dir() {
            return Err(SqliteAuthorityError::new(
                "aliased-session-path",
                "session directory must be an owned non-alias directory",
            ));
        }
        let canonical = fs::canonicalize(&session.root)
            .map_err(|error| io_error("canonicalize-session-root", error))?;
        if canonical != session.root || !canonical.starts_with(&self.storage_root) {
            return Err(SqliteAuthorityError::new(
                "aliased-session-path",
                "session directory escapes the canonical candidate root",
            ));
        }
        if require_published && creation_marker_path(session).exists() {
            return Err(SqliteAuthorityError::new(
                "incomplete-session-creation",
                "candidate session was not independently verified and published",
            ));
        }
        validate_existing_authority_leaf(&database_path(session), "aliased-sqlite-database")?;
        validate_optional_authority_leaf(&wal_path(session), "aliased-sqlite-wal")?;
        validate_optional_authority_leaf(&shm_path(session), "aliased-sqlite-shm")?;
        Ok(())
    }

    fn open_new_database(
        &self,
        session: &SqliteAuthoritySession,
    ) -> Result<Connection, SqliteAuthorityError> {
        let metadata = fs::symlink_metadata(&session.root)
            .map_err(|error| io_error("stat-session-root", error))?;
        if is_static_filesystem_alias(&metadata) || !metadata.is_dir() {
            return Err(SqliteAuthorityError::new(
                "aliased-session-path",
                "new session directory must be an owned non-alias directory",
            ));
        }
        let path = database_path(session);
        if path.exists() {
            return Err(SqliteAuthorityError::new(
                "session-already-exists",
                "new candidate session already has a database",
            ));
        }
        let connection = Connection::open(&path).map_err(sql_error("sqlite-create-open"))?;
        configure_connection(&connection, true)?;
        validate_existing_authority_leaf(&path, "aliased-sqlite-database")?;
        validate_optional_authority_leaf(&wal_path(session), "aliased-sqlite-wal")?;
        validate_optional_authority_leaf(&shm_path(session), "aliased-sqlite-shm")?;
        Ok(connection)
    }

    fn open_existing_database(
        &self,
        session: &SqliteAuthoritySession,
        mode: SqliteOpenMode,
    ) -> Result<Connection, SqliteAuthorityError> {
        self.validate_session_layout(session, true)?;
        let flags = match mode {
            SqliteOpenMode::Writable => OpenFlags::SQLITE_OPEN_READ_WRITE,
            SqliteOpenMode::ReadOnly => OpenFlags::SQLITE_OPEN_READ_ONLY,
        };
        let connection = Connection::open_with_flags(database_path(session), flags)
            .map_err(sql_error("sqlite-open"))?;
        configure_connection(&connection, mode == SqliteOpenMode::Writable)?;
        validate_existing_authority_leaf(&database_path(session), "aliased-sqlite-database")?;
        validate_optional_authority_leaf(&wal_path(session), "aliased-sqlite-wal")?;
        validate_optional_authority_leaf(&shm_path(session), "aliased-sqlite-shm")?;
        Ok(connection)
    }

    fn verify_fresh(
        &self,
        session: &SqliteAuthoritySession,
        require_published: bool,
    ) -> Result<(SessionMetadata, CurrentContractState), SqliteAuthorityError> {
        if require_published {
            self.validate_session_layout(session, true)?;
        } else {
            self.validate_session_layout_for_verification(session)?;
        }
        let connection =
            Connection::open_with_flags(database_path(session), OpenFlags::SQLITE_OPEN_READ_ONLY)
                .map_err(sql_error("sqlite-independent-reopen"))?;
        configure_connection(&connection, false)?;
        load_authority_for_session(&connection, &session.session_id, false)
    }

    fn validate_session_layout_for_verification(
        &self,
        session: &SqliteAuthoritySession,
    ) -> Result<(), SqliteAuthorityError> {
        validate_session_id(&session.session_id)?;
        if session.root != self.session_root(&session.session_id) {
            return Err(SqliteAuthorityError::new(
                "foreign-session-handle",
                "session handle does not belong to this bounded storage root",
            ));
        }
        let metadata = fs::symlink_metadata(&session.root)
            .map_err(|error| io_error("stat-session-root", error))?;
        if is_static_filesystem_alias(&metadata) || !metadata.is_dir() {
            return Err(SqliteAuthorityError::new(
                "aliased-session-path",
                "session directory must be an owned non-alias directory",
            ));
        }
        validate_existing_authority_leaf(&database_path(session), "aliased-sqlite-database")?;
        validate_optional_authority_leaf(&wal_path(session), "aliased-sqlite-wal")?;
        validate_optional_authority_leaf(&shm_path(session), "aliased-sqlite-shm")?;
        Ok(())
    }
}

#[derive(Debug)]
struct SessionMetadata {
    session_id: String,
    duplicated_from_session_id: Option<String>,
    format_version: u32,
    committed_generation: u64,
    canonical_fingerprint: String,
    session_authority: EvidenceSessionAuthority,
    material_use_declaration: EvidenceMaterialUseDeclaration,
    session_terms_identity: String,
    project_scope_stable_id: String,
    durable_review_ledger_head: usize,
    durable_reuse_governance_head: usize,
    active_analysis_snapshot_identity: String,
    evidence_writer_token: String,
}

#[derive(Debug)]
struct RawSessionMetadata {
    session_id: String,
    duplicated_from_session_id: Option<String>,
    format_version: i64,
    committed_generation: i64,
    canonical_fingerprint: String,
    session_authority_role_label: String,
    session_authority_display_label: String,
    material_use_basis: String,
    session_terms_identity: String,
    project_scope_stable_id: String,
    durable_review_ledger_head: i64,
    durable_reuse_governance_head: i64,
    active_analysis_snapshot_identity: String,
    evidence_writer_token: String,
}

#[derive(Debug)]
struct RawGovernanceRow {
    event_index: i64,
    event_kind: String,
    candidate_project_scope_stable_id: Option<String>,
    candidate_locator_source_revision_id: Option<String>,
    candidate_locator_analysis_snapshot_identity: Option<String>,
    candidate_locator_review_case_id: Option<String>,
    candidate_locator_ledger_position: Option<i64>,
    candidate_locator_decision_digest: Option<String>,
    candidate_locator_effective_at_ledger_length: Option<i64>,
    candidate_observed_text: Option<String>,
    candidate_confirmed_replacement: Option<String>,
    promotion_payload_observed_text: Option<String>,
    promotion_payload_confirmed_replacement: Option<String>,
    promotion_locator_source_revision_id: Option<String>,
    promotion_locator_analysis_snapshot_identity: Option<String>,
    promotion_locator_review_case_id: Option<String>,
    promotion_locator_ledger_position: Option<i64>,
    promotion_locator_decision_digest: Option<String>,
    promotion_locator_effective_at_ledger_length: Option<i64>,
    actor_role_label: Option<String>,
    actor_display_label: Option<String>,
    promotion_project_scope_stable_id: Option<String>,
    record_id: Option<i64>,
    predecessor_record_id: Option<i64>,
    successor_record_id: Option<i64>,
}

#[derive(Debug)]
struct GovernanceStorageRow {
    event_index: i64,
    event_kind: &'static str,
    candidate_project_scope_stable_id: Option<String>,
    candidate_locator_source_revision_id: Option<String>,
    candidate_locator_analysis_snapshot_identity: Option<String>,
    candidate_locator_review_case_id: Option<String>,
    candidate_locator_ledger_position: Option<i64>,
    candidate_locator_decision_digest: Option<String>,
    candidate_locator_effective_at_ledger_length: Option<i64>,
    candidate_observed_text: Option<String>,
    candidate_confirmed_replacement: Option<String>,
    promotion_payload_observed_text: Option<String>,
    promotion_payload_confirmed_replacement: Option<String>,
    promotion_locator_source_revision_id: Option<String>,
    promotion_locator_analysis_snapshot_identity: Option<String>,
    promotion_locator_review_case_id: Option<String>,
    promotion_locator_ledger_position: Option<i64>,
    promotion_locator_decision_digest: Option<String>,
    promotion_locator_effective_at_ledger_length: Option<i64>,
    actor_role_label: Option<String>,
    actor_display_label: Option<String>,
    promotion_project_scope_stable_id: Option<String>,
    record_id: Option<i64>,
    predecessor_record_id: Option<i64>,
    successor_record_id: Option<i64>,
}

fn initialize_schema(connection: &Connection) -> Result<(), SqliteAuthorityError> {
    connection
        .execute_batch(
            "
            CREATE TABLE session_meta (
                id INTEGER PRIMARY KEY CHECK (id = 1),
                semantic_session_id TEXT NOT NULL UNIQUE,
                duplicated_from_session_id TEXT,
                format_version INTEGER NOT NULL,
                committed_generation INTEGER NOT NULL CHECK (committed_generation >= 1),
                canonical_fingerprint TEXT NOT NULL,
                session_authority_role_label TEXT NOT NULL,
                session_authority_display_label TEXT NOT NULL,
                material_use_basis TEXT NOT NULL,
                session_terms_identity TEXT NOT NULL,
                project_scope_stable_id TEXT NOT NULL,
                durable_review_ledger_head INTEGER NOT NULL CHECK (durable_review_ledger_head >= 0),
                durable_reuse_governance_head INTEGER NOT NULL CHECK (durable_reuse_governance_head >= 0),
                active_analysis_snapshot_identity TEXT NOT NULL,
                evidence_writer_token TEXT NOT NULL
            );
            CREATE TABLE source_revisions (
                revision_id TEXT PRIMARY KEY,
                predecessor_revision_id TEXT,
                transcript_bytes TEXT NOT NULL
            );
            CREATE TABLE analysis_snapshots (
                identity TEXT PRIMARY KEY,
                source_revision_id TEXT NOT NULL,
                session_terms_identity TEXT NOT NULL,
                detector_config_id TEXT NOT NULL,
                detector_config_version TEXT NOT NULL,
                algorithm_id TEXT NOT NULL,
                algorithm_version TEXT NOT NULL
            );
            CREATE TABLE analysis_snapshot_detectors (
                analysis_snapshot_identity TEXT NOT NULL,
                detector_ordinal INTEGER NOT NULL CHECK (detector_ordinal >= 0),
                detector_id TEXT NOT NULL,
                detector_version TEXT NOT NULL,
                PRIMARY KEY (analysis_snapshot_identity, detector_ordinal),
                FOREIGN KEY (analysis_snapshot_identity) REFERENCES analysis_snapshots(identity)
            );
            CREATE TABLE review_cases (
                case_id TEXT PRIMARY KEY,
                origin TEXT NOT NULL,
                observed_revision_id TEXT NOT NULL,
                anchor_revision_id TEXT NOT NULL,
                anchor_segment_position INTEGER NOT NULL CHECK (anchor_segment_position >= 0),
                anchor_start_byte INTEGER NOT NULL CHECK (anchor_start_byte >= 0),
                anchor_end_byte INTEGER NOT NULL CHECK (anchor_end_byte >= 0),
                observed_source_bytes TEXT NOT NULL,
                alternative_count INTEGER NOT NULL CHECK (alternative_count >= 0)
            );
            CREATE TABLE review_ledger_events (
                event_index INTEGER PRIMARY KEY CHECK (event_index >= 0),
                case_id TEXT NOT NULL,
                observed_revision_id TEXT NOT NULL,
                action_kind TEXT NOT NULL,
                manual_replacement_bytes TEXT,
                alternative_index INTEGER,
                target_event_index INTEGER,
                provenance TEXT NOT NULL
            );
            CREATE TABLE reuse_governance_events (
                event_index INTEGER PRIMARY KEY CHECK (event_index >= 0),
                event_kind TEXT NOT NULL CHECK (event_kind IN ('promotion_candidate_rejected', 'promotion_accepted', 'reusable_influence_revoked', 'reusable_influence_superseded')),
                candidate_project_scope_stable_id TEXT,
                candidate_locator_source_revision_id TEXT,
                candidate_locator_analysis_snapshot_identity TEXT,
                candidate_locator_review_case_id TEXT,
                candidate_locator_ledger_position INTEGER,
                candidate_locator_decision_digest TEXT,
                candidate_locator_effective_at_ledger_length INTEGER,
                candidate_observed_text TEXT,
                candidate_confirmed_replacement TEXT,
                promotion_payload_observed_text TEXT,
                promotion_payload_confirmed_replacement TEXT,
                promotion_locator_source_revision_id TEXT,
                promotion_locator_analysis_snapshot_identity TEXT,
                promotion_locator_review_case_id TEXT,
                promotion_locator_ledger_position INTEGER,
                promotion_locator_decision_digest TEXT,
                promotion_locator_effective_at_ledger_length INTEGER,
                actor_role_label TEXT,
                actor_display_label TEXT,
                promotion_project_scope_stable_id TEXT,
                record_id INTEGER,
                predecessor_record_id INTEGER,
                successor_record_id INTEGER,
                CHECK (
                    (event_kind = 'promotion_candidate_rejected'
                        AND candidate_project_scope_stable_id IS NOT NULL
                        AND candidate_locator_source_revision_id IS NOT NULL
                        AND candidate_locator_analysis_snapshot_identity IS NOT NULL
                        AND candidate_locator_review_case_id IS NOT NULL
                        AND candidate_locator_ledger_position IS NOT NULL
                        AND candidate_locator_decision_digest IS NOT NULL
                        AND candidate_locator_effective_at_ledger_length IS NOT NULL
                        AND candidate_observed_text IS NOT NULL
                        AND candidate_confirmed_replacement IS NOT NULL
                        AND actor_role_label IS NOT NULL
                        AND actor_display_label IS NOT NULL
                        AND promotion_payload_observed_text IS NULL
                        AND promotion_payload_confirmed_replacement IS NULL
                        AND promotion_locator_source_revision_id IS NULL
                        AND promotion_project_scope_stable_id IS NULL
                        AND record_id IS NULL AND predecessor_record_id IS NULL AND successor_record_id IS NULL)
                    OR
                    (event_kind = 'promotion_accepted'
                        AND candidate_project_scope_stable_id IS NOT NULL
                        AND candidate_locator_source_revision_id IS NOT NULL
                        AND candidate_locator_analysis_snapshot_identity IS NOT NULL
                        AND candidate_locator_review_case_id IS NOT NULL
                        AND candidate_locator_ledger_position IS NOT NULL
                        AND candidate_locator_decision_digest IS NOT NULL
                        AND candidate_locator_effective_at_ledger_length IS NOT NULL
                        AND candidate_observed_text IS NOT NULL
                        AND candidate_confirmed_replacement IS NOT NULL
                        AND promotion_payload_observed_text IS NOT NULL
                        AND promotion_payload_confirmed_replacement IS NOT NULL
                        AND promotion_locator_source_revision_id IS NOT NULL
                        AND promotion_locator_analysis_snapshot_identity IS NOT NULL
                        AND promotion_locator_review_case_id IS NOT NULL
                        AND promotion_locator_ledger_position IS NOT NULL
                        AND promotion_locator_decision_digest IS NOT NULL
                        AND promotion_locator_effective_at_ledger_length IS NOT NULL
                        AND actor_role_label IS NOT NULL
                        AND actor_display_label IS NOT NULL
                        AND promotion_project_scope_stable_id IS NOT NULL
                        AND record_id IS NULL AND predecessor_record_id IS NULL AND successor_record_id IS NULL)
                    OR
                    (event_kind = 'reusable_influence_revoked'
                        AND actor_role_label IS NOT NULL AND actor_display_label IS NOT NULL
                        AND record_id IS NOT NULL
                        AND candidate_project_scope_stable_id IS NULL
                        AND predecessor_record_id IS NULL AND successor_record_id IS NULL)
                    OR
                    (event_kind = 'reusable_influence_superseded'
                        AND actor_role_label IS NOT NULL AND actor_display_label IS NOT NULL
                        AND predecessor_record_id IS NOT NULL AND successor_record_id IS NOT NULL
                        AND candidate_project_scope_stable_id IS NULL
                        AND record_id IS NULL)
                )
            );
            CREATE TABLE reuse_enabled_analysis_binding (
                id INTEGER PRIMARY KEY CHECK (id = 1),
                analysis_snapshot_identity TEXT NOT NULL,
                reusable_snapshot_identity TEXT NOT NULL,
                governance_event_boundary INTEGER NOT NULL CHECK (governance_event_boundary >= 0),
                projection_version TEXT NOT NULL,
                FOREIGN KEY (analysis_snapshot_identity) REFERENCES analysis_snapshots(identity)
            );
            CREATE TABLE authority_transitions (
                generation INTEGER PRIMARY KEY CHECK (generation >= 1),
                canonical_fingerprint TEXT NOT NULL UNIQUE,
                acknowledgement_status TEXT NOT NULL CHECK (acknowledgement_status IN ('committed', 'acknowledged'))
            );
            CREATE TABLE writer_ownership (
                id INTEGER PRIMARY KEY CHECK (id = 1),
                token TEXT,
                owner_epoch INTEGER NOT NULL CHECK (owner_epoch >= 0),
                lease_duration_ms INTEGER NOT NULL CHECK (lease_duration_ms > 0),
                lease_expires_at_unix_ms INTEGER NOT NULL,
                process_instance_id TEXT NOT NULL,
                holder_pid INTEGER
            );
            CREATE TABLE derived_contract_cache (
                id INTEGER PRIMARY KEY CHECK (id = 1),
                canonical_fingerprint TEXT NOT NULL,
                derived_fingerprint TEXT NOT NULL,
                payload_json TEXT NOT NULL,
                cache_schema_version INTEGER NOT NULL
            );
            ",
        )
        .map_err(sql_error("sqlite-init-schema"))?;
    Ok(())
}

fn configure_connection(
    connection: &Connection,
    writable: bool,
) -> Result<(), SqliteAuthorityError> {
    connection
        .busy_timeout(Duration::from_millis(500))
        .map_err(sql_error("sqlite-busy-timeout"))?;
    if writable {
        connection
            .execute_batch("PRAGMA foreign_keys = ON; PRAGMA trusted_schema = OFF;")
            .map_err(sql_error("sqlite-configure-writable"))?;
    } else {
        connection
            .execute_batch(
                "PRAGMA foreign_keys = ON; PRAGMA trusted_schema = OFF; PRAGMA query_only = ON;",
            )
            .map_err(sql_error("sqlite-configure-read-only"))?;
    }
    Ok(())
}

fn configure_writable_persistence(connection: &Connection) -> Result<(), SqliteAuthorityError> {
    connection
        .execute_batch(&format!(
            "PRAGMA journal_mode = WAL; PRAGMA synchronous = FULL; PRAGMA max_page_count = {MAX_PAGE_COUNT};"
        ))
        .map_err(sql_error("sqlite-configure-writable-persistence"))?;
    Ok(())
}

fn insert_session_metadata(
    tx: &Transaction<'_>,
    state: &CurrentContractState,
    generation: u64,
    fingerprint: &str,
) -> Result<(), SqliteAuthorityError> {
    tx.execute(
        "INSERT INTO session_meta (id, semantic_session_id, duplicated_from_session_id, format_version, committed_generation, canonical_fingerprint, session_authority_role_label, session_authority_display_label, material_use_basis, session_terms_identity, project_scope_stable_id, durable_review_ledger_head, durable_reuse_governance_head, active_analysis_snapshot_identity, evidence_writer_token) VALUES (1, ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)",
        params![
            state.session_id,
            state.duplicated_from_session_id,
            i64::from(SQLITE_AUTHORITATIVE_FORMAT_VERSION),
            u64_to_i64(generation)?,
            fingerprint,
            state.session_authority.role_label,
            state.session_authority.display_label,
            state.material_use_declaration.basis,
            state.session_terms_identity,
            state.project_scope.stable_id,
            usize_to_i64(state.durable_command_tokens.review_ledger_head)?,
            usize_to_i64(state.durable_command_tokens.reuse_governance_head)?,
            state.durable_command_tokens.active_analysis_snapshot_identity,
            state.durable_command_tokens.evidence_writer_token,
        ],
    )
    .map_err(sql_error("sqlite-insert-session-meta"))?;
    Ok(())
}

fn update_session_metadata(
    tx: &Transaction<'_>,
    state: &CurrentContractState,
    generation: u64,
    fingerprint: &str,
) -> Result<(), SqliteAuthorityError> {
    let updated = tx
        .execute(
            "UPDATE session_meta SET duplicated_from_session_id = ?1, committed_generation = ?2, canonical_fingerprint = ?3, session_authority_role_label = ?4, session_authority_display_label = ?5, material_use_basis = ?6, session_terms_identity = ?7, project_scope_stable_id = ?8, durable_review_ledger_head = ?9, durable_reuse_governance_head = ?10, active_analysis_snapshot_identity = ?11, evidence_writer_token = ?12 WHERE id = 1 AND semantic_session_id = ?13 AND format_version = ?14",
            params![
                state.duplicated_from_session_id,
                u64_to_i64(generation)?,
                fingerprint,
                state.session_authority.role_label,
                state.session_authority.display_label,
                state.material_use_declaration.basis,
                state.session_terms_identity,
                state.project_scope.stable_id,
                usize_to_i64(state.durable_command_tokens.review_ledger_head)?,
                usize_to_i64(state.durable_command_tokens.reuse_governance_head)?,
                state.durable_command_tokens.active_analysis_snapshot_identity,
                state.durable_command_tokens.evidence_writer_token,
                state.session_id,
                i64::from(SQLITE_AUTHORITATIVE_FORMAT_VERSION),
            ],
        )
        .map_err(sql_error("sqlite-update-session-meta"))?;
    if updated != 1 {
        return Err(SqliteAuthorityError::new(
            "canonical-corruption",
            "session metadata did not match the expected candidate format and identity",
        ));
    }
    Ok(())
}

fn replace_canonical_tables(
    tx: &Transaction<'_>,
    state: &CurrentContractState,
) -> Result<(), SqliteAuthorityError> {
    for statement in [
        "DELETE FROM reuse_enabled_analysis_binding",
        "DELETE FROM reuse_governance_events",
        "DELETE FROM review_ledger_events",
        "DELETE FROM review_cases",
        "DELETE FROM analysis_snapshot_detectors",
        "DELETE FROM analysis_snapshots",
        "DELETE FROM source_revisions",
    ] {
        tx.execute(statement, [])
            .map_err(sql_error("sqlite-replace-canonical"))?;
    }
    for revision in &state.source_revisions {
        tx.execute(
            "INSERT INTO source_revisions (revision_id, predecessor_revision_id, transcript_bytes) VALUES (?1, ?2, ?3)",
            params![revision.revision_id, revision.predecessor_revision_id, revision.transcript_bytes],
        )
        .map_err(sql_error("sqlite-save-source-revision"))?;
    }
    for snapshot in &state.analysis_snapshots {
        tx.execute(
            "INSERT INTO analysis_snapshots (identity, source_revision_id, session_terms_identity, detector_config_id, detector_config_version, algorithm_id, algorithm_version) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![snapshot.identity, snapshot.source_revision_id, snapshot.session_terms_identity, snapshot.detector_config_id, snapshot.detector_config_version, snapshot.algorithm_id, snapshot.algorithm_version],
        )
        .map_err(sql_error("sqlite-save-analysis-snapshot"))?;
        for (ordinal, detector) in snapshot.detectors.iter().enumerate() {
            tx.execute(
                "INSERT INTO analysis_snapshot_detectors (analysis_snapshot_identity, detector_ordinal, detector_id, detector_version) VALUES (?1, ?2, ?3, ?4)",
                params![snapshot.identity, usize_to_i64(ordinal)?, detector.id, detector.version],
            )
            .map_err(sql_error("sqlite-save-analysis-detector"))?;
        }
    }
    for review_case in &state.review_cases {
        tx.execute(
            "INSERT INTO review_cases (case_id, origin, observed_revision_id, anchor_revision_id, anchor_segment_position, anchor_start_byte, anchor_end_byte, observed_source_bytes, alternative_count) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![review_case.case_id, review_case.origin, review_case.observed_revision_id, review_case.anchor_revision_id, usize_to_i64(review_case.anchor_segment_position)?, usize_to_i64(review_case.anchor_start_byte)?, usize_to_i64(review_case.anchor_end_byte)?, review_case.observed_source_bytes, usize_to_i64(review_case.alternative_count)?],
        )
        .map_err(sql_error("sqlite-save-review-case"))?;
    }
    for event in &state.review_ledger_events {
        tx.execute(
            "INSERT INTO review_ledger_events (event_index, case_id, observed_revision_id, action_kind, manual_replacement_bytes, alternative_index, target_event_index, provenance) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![usize_to_i64(event.event_index)?, event.case_id, event.observed_revision_id, event.action_kind, event.manual_replacement_bytes, option_usize_to_i64(event.alternative_index)?, option_usize_to_i64(event.target_event_index)?, event.provenance],
        )
        .map_err(sql_error("sqlite-save-review-ledger"))?;
    }
    for event in &state.reuse_governance_events {
        let row = governance_storage_row(event)?;
        tx.execute(
            "INSERT INTO reuse_governance_events (event_index, event_kind, candidate_project_scope_stable_id, candidate_locator_source_revision_id, candidate_locator_analysis_snapshot_identity, candidate_locator_review_case_id, candidate_locator_ledger_position, candidate_locator_decision_digest, candidate_locator_effective_at_ledger_length, candidate_observed_text, candidate_confirmed_replacement, promotion_payload_observed_text, promotion_payload_confirmed_replacement, promotion_locator_source_revision_id, promotion_locator_analysis_snapshot_identity, promotion_locator_review_case_id, promotion_locator_ledger_position, promotion_locator_decision_digest, promotion_locator_effective_at_ledger_length, actor_role_label, actor_display_label, promotion_project_scope_stable_id, record_id, predecessor_record_id, successor_record_id) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21, ?22, ?23, ?24, ?25)",
            params![
                row.event_index, row.event_kind, row.candidate_project_scope_stable_id,
                row.candidate_locator_source_revision_id, row.candidate_locator_analysis_snapshot_identity,
                row.candidate_locator_review_case_id, row.candidate_locator_ledger_position,
                row.candidate_locator_decision_digest, row.candidate_locator_effective_at_ledger_length,
                row.candidate_observed_text, row.candidate_confirmed_replacement,
                row.promotion_payload_observed_text, row.promotion_payload_confirmed_replacement,
                row.promotion_locator_source_revision_id,
                row.promotion_locator_analysis_snapshot_identity, row.promotion_locator_review_case_id,
                row.promotion_locator_ledger_position, row.promotion_locator_decision_digest,
                row.promotion_locator_effective_at_ledger_length, row.actor_role_label,
                row.actor_display_label, row.promotion_project_scope_stable_id, row.record_id,
                row.predecessor_record_id, row.successor_record_id,
            ],
        )
        .map_err(sql_error("sqlite-save-reuse-governance"))?;
    }
    if let Some(binding) = &state.reuse_enabled_analysis_binding {
        tx.execute(
            "INSERT INTO reuse_enabled_analysis_binding (id, analysis_snapshot_identity, reusable_snapshot_identity, governance_event_boundary, projection_version) VALUES (1, ?1, ?2, ?3, ?4)",
            params![binding.analysis_snapshot_identity, binding.reusable_snapshot_identity, usize_to_i64(binding.governance_event_boundary)?, binding.projection_version],
        )
        .map_err(sql_error("sqlite-save-reuse-binding"))?;
    }
    Ok(())
}

fn governance_storage_row(
    event: &EvidenceReuseGovernanceEvent,
) -> Result<GovernanceStorageRow, SqliteAuthorityError> {
    match event {
        EvidenceReuseGovernanceEvent::PromotionCandidateRejected {
            event_index,
            candidate_key,
            actor,
        } => Ok(GovernanceStorageRow {
            event_index: usize_to_i64(*event_index)?,
            event_kind: "promotion_candidate_rejected",
            candidate_project_scope_stable_id: Some(candidate_key.project_scope_stable_id.clone()),
            candidate_locator_source_revision_id: Some(
                candidate_key.source_locator.source_revision_id.clone(),
            ),
            candidate_locator_analysis_snapshot_identity: Some(
                candidate_key
                    .source_locator
                    .source_analysis_snapshot_identity
                    .clone(),
            ),
            candidate_locator_review_case_id: Some(
                candidate_key.source_locator.source_review_case_id.clone(),
            ),
            candidate_locator_ledger_position: Some(usize_to_i64(
                candidate_key.source_locator.review_ledger_position,
            )?),
            candidate_locator_decision_digest: Some(
                candidate_key.source_locator.decision_digest.clone(),
            ),
            candidate_locator_effective_at_ledger_length: Some(usize_to_i64(
                candidate_key.source_locator.effective_at_ledger_length,
            )?),
            candidate_observed_text: Some(candidate_key.exact_payload.observed_text.clone()),
            candidate_confirmed_replacement: Some(
                candidate_key.exact_payload.confirmed_replacement.clone(),
            ),
            promotion_payload_observed_text: None,
            promotion_payload_confirmed_replacement: None,
            promotion_locator_source_revision_id: None,
            promotion_locator_analysis_snapshot_identity: None,
            promotion_locator_review_case_id: None,
            promotion_locator_ledger_position: None,
            promotion_locator_decision_digest: None,
            promotion_locator_effective_at_ledger_length: None,
            actor_role_label: Some(actor.role_label.clone()),
            actor_display_label: Some(actor.display_label.clone()),
            promotion_project_scope_stable_id: None,
            record_id: None,
            predecessor_record_id: None,
            successor_record_id: None,
        }),
        EvidenceReuseGovernanceEvent::PromotionAccepted {
            event_index,
            candidate_key,
            payload,
            source_locator,
            actor,
            project_scope_stable_id,
        } => Ok(GovernanceStorageRow {
            event_index: usize_to_i64(*event_index)?,
            event_kind: "promotion_accepted",
            candidate_project_scope_stable_id: Some(candidate_key.project_scope_stable_id.clone()),
            candidate_locator_source_revision_id: Some(
                candidate_key.source_locator.source_revision_id.clone(),
            ),
            candidate_locator_analysis_snapshot_identity: Some(
                candidate_key
                    .source_locator
                    .source_analysis_snapshot_identity
                    .clone(),
            ),
            candidate_locator_review_case_id: Some(
                candidate_key.source_locator.source_review_case_id.clone(),
            ),
            candidate_locator_ledger_position: Some(usize_to_i64(
                candidate_key.source_locator.review_ledger_position,
            )?),
            candidate_locator_decision_digest: Some(
                candidate_key.source_locator.decision_digest.clone(),
            ),
            candidate_locator_effective_at_ledger_length: Some(usize_to_i64(
                candidate_key.source_locator.effective_at_ledger_length,
            )?),
            candidate_observed_text: Some(candidate_key.exact_payload.observed_text.clone()),
            candidate_confirmed_replacement: Some(
                candidate_key.exact_payload.confirmed_replacement.clone(),
            ),
            promotion_payload_observed_text: Some(payload.observed_text.clone()),
            promotion_payload_confirmed_replacement: Some(payload.confirmed_replacement.clone()),
            promotion_locator_source_revision_id: Some(source_locator.source_revision_id.clone()),
            promotion_locator_analysis_snapshot_identity: Some(
                source_locator.source_analysis_snapshot_identity.clone(),
            ),
            promotion_locator_review_case_id: Some(source_locator.source_review_case_id.clone()),
            promotion_locator_ledger_position: Some(usize_to_i64(
                source_locator.review_ledger_position,
            )?),
            promotion_locator_decision_digest: Some(source_locator.decision_digest.clone()),
            promotion_locator_effective_at_ledger_length: Some(usize_to_i64(
                source_locator.effective_at_ledger_length,
            )?),
            actor_role_label: Some(actor.role_label.clone()),
            actor_display_label: Some(actor.display_label.clone()),
            promotion_project_scope_stable_id: Some(project_scope_stable_id.clone()),
            record_id: None,
            predecessor_record_id: None,
            successor_record_id: None,
        }),
        EvidenceReuseGovernanceEvent::ReusableInfluenceRevoked {
            event_index,
            record_id,
            actor,
        } => Ok(GovernanceStorageRow {
            event_index: usize_to_i64(*event_index)?,
            event_kind: "reusable_influence_revoked",
            candidate_project_scope_stable_id: None,
            candidate_locator_source_revision_id: None,
            candidate_locator_analysis_snapshot_identity: None,
            candidate_locator_review_case_id: None,
            candidate_locator_ledger_position: None,
            candidate_locator_decision_digest: None,
            candidate_locator_effective_at_ledger_length: None,
            candidate_observed_text: None,
            candidate_confirmed_replacement: None,
            promotion_payload_observed_text: None,
            promotion_payload_confirmed_replacement: None,
            promotion_locator_source_revision_id: None,
            promotion_locator_analysis_snapshot_identity: None,
            promotion_locator_review_case_id: None,
            promotion_locator_ledger_position: None,
            promotion_locator_decision_digest: None,
            promotion_locator_effective_at_ledger_length: None,
            actor_role_label: Some(actor.role_label.clone()),
            actor_display_label: Some(actor.display_label.clone()),
            promotion_project_scope_stable_id: None,
            record_id: Some(usize_to_i64(*record_id)?),
            predecessor_record_id: None,
            successor_record_id: None,
        }),
        EvidenceReuseGovernanceEvent::ReusableInfluenceSuperseded {
            event_index,
            predecessor_record_id,
            successor_record_id,
            actor,
        } => Ok(GovernanceStorageRow {
            event_index: usize_to_i64(*event_index)?,
            event_kind: "reusable_influence_superseded",
            candidate_project_scope_stable_id: None,
            candidate_locator_source_revision_id: None,
            candidate_locator_analysis_snapshot_identity: None,
            candidate_locator_review_case_id: None,
            candidate_locator_ledger_position: None,
            candidate_locator_decision_digest: None,
            candidate_locator_effective_at_ledger_length: None,
            candidate_observed_text: None,
            candidate_confirmed_replacement: None,
            promotion_payload_observed_text: None,
            promotion_payload_confirmed_replacement: None,
            promotion_locator_source_revision_id: None,
            promotion_locator_analysis_snapshot_identity: None,
            promotion_locator_review_case_id: None,
            promotion_locator_ledger_position: None,
            promotion_locator_decision_digest: None,
            promotion_locator_effective_at_ledger_length: None,
            actor_role_label: Some(actor.role_label.clone()),
            actor_display_label: Some(actor.display_label.clone()),
            promotion_project_scope_stable_id: None,
            record_id: None,
            predecessor_record_id: Some(usize_to_i64(*predecessor_record_id)?),
            successor_record_id: Some(usize_to_i64(*successor_record_id)?),
        }),
    }
}

fn load_authority_for_session(
    connection: &Connection,
    expected_session_id: &str,
    should_rebuild_cache: bool,
) -> Result<(SessionMetadata, CurrentContractState), SqliteAuthorityError> {
    integrity_check(connection)?;
    let metadata = load_session_metadata(connection)?;
    if metadata.session_id != expected_session_id {
        return Err(SqliteAuthorityError::new(
            "session-identity-mismatch",
            "persisted semantic session identity does not match the requested session",
        ));
    }
    if metadata.format_version != SQLITE_AUTHORITATIVE_FORMAT_VERSION {
        return Err(
            if metadata.format_version > SQLITE_AUTHORITATIVE_FORMAT_VERSION {
                SqliteAuthorityError::new(
                    "unsupported-newer-format",
                    "candidate refuses to interpret a newer SQLite-v3 format",
                )
            } else {
                SqliteAuthorityError::new(
                    "unsupported-format",
                    "candidate refuses to interpret an unsupported SQLite-v3 format",
                )
            },
        );
    }
    validate_canonical_storage_bounds(connection)?;
    validate_authority_transitions(connection, &metadata)?;
    let source_revisions = load_source_revisions(connection)?;
    let analysis_snapshots = load_analysis_snapshots(connection)?;
    let review_cases = load_review_cases(connection)?;
    let review_ledger_events = load_review_ledger_events(connection)?;
    let reuse_governance_events = load_reuse_governance_events(connection)?;
    let reuse_enabled_analysis_binding = load_reuse_binding(connection, &analysis_snapshots)?;
    let mut state = CurrentContractState {
        session_id: metadata.session_id.clone(),
        duplicated_from_session_id: metadata.duplicated_from_session_id.clone(),
        session_authority: metadata.session_authority.clone(),
        material_use_declaration: metadata.material_use_declaration.clone(),
        source_revisions,
        session_terms_identity: metadata.session_terms_identity.clone(),
        analysis_snapshots,
        review_cases,
        review_ledger_events,
        effective_review_status: Vec::new(),
        project_scope: EvidenceProjectScope {
            stable_id: metadata.project_scope_stable_id.clone(),
            // This label is classified as non-semantic / rebuildable. It is
            // deliberately not canonical SQLite authority.
            display_name: String::new(),
        },
        reuse_governance_events,
        effective_reusable_records: Vec::new(),
        historical_reusable_records: Vec::new(),
        reusable_snapshot_identity: String::new(),
        reuse_enabled_analysis_binding,
        derived_queue_projection: String::new(),
        durable_command_tokens: super::model::EvidenceDurableCommandTokens {
            review_ledger_head: metadata.durable_review_ledger_head,
            reuse_governance_head: metadata.durable_reuse_governance_head,
            active_analysis_snapshot_identity: metadata.active_analysis_snapshot_identity.clone(),
            evidence_writer_token: metadata.evidence_writer_token.clone(),
        },
    };
    finalize_derived_fields(&mut state);
    state = state.normalize();
    let result = CurrentContractOracle::validate(&state);
    if !result.passed {
        return Err(SqliteAuthorityError::new(
            "canonical-corruption",
            format!(
                "relational canonical rows failed current-contract oracle with {} violation(s)",
                result.violations.len()
            ),
        ));
    }
    if result.canonical_fingerprint != metadata.canonical_fingerprint {
        return Err(SqliteAuthorityError::new(
            "canonical-fingerprint-mismatch",
            "metadata fingerprint does not bind reconstructed canonical authority",
        ));
    }
    if should_rebuild_cache {
        rebuild_derived_cache(connection, &state, &result.canonical_fingerprint)?;
    }
    Ok((metadata, state))
}

fn integrity_check(connection: &Connection) -> Result<(), SqliteAuthorityError> {
    let check: String = connection
        .query_row("PRAGMA integrity_check", [], |row| row.get(0))
        .map_err(sql_error("canonical-corruption"))?;
    if check != "ok" {
        return Err(SqliteAuthorityError::new(
            "canonical-corruption",
            format!("SQLite integrity_check returned {check}"),
        ));
    }
    Ok(())
}

fn load_session_metadata(connection: &Connection) -> Result<SessionMetadata, SqliteAuthorityError> {
    let rows = table_count(connection, "session_meta")?;
    if rows != 1 {
        return Err(SqliteAuthorityError::new(
            "canonical-corruption",
            "session metadata must contain exactly one authority row",
        ));
    }
    let raw = connection
        .query_row(
            "SELECT semantic_session_id, duplicated_from_session_id, format_version, committed_generation, canonical_fingerprint, session_authority_role_label, session_authority_display_label, material_use_basis, session_terms_identity, project_scope_stable_id, durable_review_ledger_head, durable_reuse_governance_head, active_analysis_snapshot_identity, evidence_writer_token FROM session_meta WHERE id = 1",
            [],
            raw_session_metadata_from_row,
        )
        .map_err(sql_error("canonical-corruption"))?;
    session_metadata_from_raw(raw)
}

fn load_session_metadata_tx(tx: &Transaction<'_>) -> Result<SessionMetadata, SqliteAuthorityError> {
    let count: i64 = tx
        .query_row("SELECT COUNT(*) FROM session_meta", [], |row| row.get(0))
        .map_err(sql_error("canonical-corruption"))?;
    if count != 1 {
        return Err(SqliteAuthorityError::new(
            "canonical-corruption",
            "session metadata must contain exactly one authority row",
        ));
    }
    let raw = tx.query_row(
        "SELECT semantic_session_id, duplicated_from_session_id, format_version, committed_generation, canonical_fingerprint, session_authority_role_label, session_authority_display_label, material_use_basis, session_terms_identity, project_scope_stable_id, durable_review_ledger_head, durable_reuse_governance_head, active_analysis_snapshot_identity, evidence_writer_token FROM session_meta WHERE id = 1",
        [],
        raw_session_metadata_from_row,
    )
    .map_err(sql_error("canonical-corruption"))?;
    session_metadata_from_raw(raw)
}

fn raw_session_metadata_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<RawSessionMetadata> {
    Ok(RawSessionMetadata {
        session_id: row.get(0)?,
        duplicated_from_session_id: row.get(1)?,
        format_version: row.get(2)?,
        committed_generation: row.get(3)?,
        canonical_fingerprint: row.get(4)?,
        session_authority_role_label: row.get(5)?,
        session_authority_display_label: row.get(6)?,
        material_use_basis: row.get(7)?,
        session_terms_identity: row.get(8)?,
        project_scope_stable_id: row.get(9)?,
        durable_review_ledger_head: row.get(10)?,
        durable_reuse_governance_head: row.get(11)?,
        active_analysis_snapshot_identity: row.get(12)?,
        evidence_writer_token: row.get(13)?,
    })
}

fn session_metadata_from_raw(
    raw: RawSessionMetadata,
) -> Result<SessionMetadata, SqliteAuthorityError> {
    Ok(SessionMetadata {
        session_id: raw.session_id,
        duplicated_from_session_id: raw.duplicated_from_session_id,
        format_version: i64_to_u32(raw.format_version)?,
        committed_generation: i64_to_u64(raw.committed_generation)?,
        canonical_fingerprint: raw.canonical_fingerprint,
        session_authority: EvidenceSessionAuthority {
            role_label: raw.session_authority_role_label,
            display_label: raw.session_authority_display_label,
        },
        material_use_declaration: EvidenceMaterialUseDeclaration {
            basis: raw.material_use_basis,
        },
        session_terms_identity: raw.session_terms_identity,
        project_scope_stable_id: raw.project_scope_stable_id,
        durable_review_ledger_head: i64_to_usize(raw.durable_review_ledger_head)?,
        durable_reuse_governance_head: i64_to_usize(raw.durable_reuse_governance_head)?,
        active_analysis_snapshot_identity: raw.active_analysis_snapshot_identity,
        evidence_writer_token: raw.evidence_writer_token,
    })
}

fn validate_authority_transitions(
    connection: &Connection,
    metadata: &SessionMetadata,
) -> Result<(), SqliteAuthorityError> {
    let mut statement = connection
        .prepare("SELECT generation, canonical_fingerprint FROM authority_transitions ORDER BY generation")
        .map_err(sql_error("canonical-corruption"))?;
    let rows = statement
        .query_map([], |row| {
            Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?))
        })
        .map_err(sql_error("canonical-corruption"))?;
    let mut expected = 1_u64;
    let mut latest = None;
    let mut count = 0_usize;
    for row in rows {
        let (generation, fingerprint) = row.map_err(sql_error("canonical-corruption"))?;
        let generation = i64_to_u64(generation)?;
        if generation != expected {
            return Err(SqliteAuthorityError::new(
                "canonical-corruption",
                "authority transition generations are not contiguous",
            ));
        }
        latest = Some(fingerprint);
        expected = expected.checked_add(1).ok_or_else(|| {
            SqliteAuthorityError::new("generation-overflow", "generation overflow")
        })?;
        count += 1;
        if count > MAX_ROWS_PER_TABLE {
            return Err(SqliteAuthorityError::new(
                "row-count-exceeded",
                "authority transition count exceeds the bounded candidate limit",
            ));
        }
    }
    if count != u64_to_usize(metadata.committed_generation)?
        || latest.as_deref() != Some(metadata.canonical_fingerprint.as_str())
    {
        return Err(SqliteAuthorityError::new(
            "canonical-corruption",
            "transition log does not bind the current relational authority",
        ));
    }
    Ok(())
}

fn load_source_revisions(
    connection: &Connection,
) -> Result<Vec<EvidenceSourceRevision>, SqliteAuthorityError> {
    let mut statement = connection
        .prepare(
            "SELECT revision_id, predecessor_revision_id, transcript_bytes FROM source_revisions ORDER BY revision_id",
        )
        .map_err(sql_error("canonical-corruption"))?;
    let rows = statement
        .query_map([], |row| {
            Ok(EvidenceSourceRevision {
                revision_id: row.get(0)?,
                predecessor_revision_id: row.get(1)?,
                transcript_bytes: row.get(2)?,
            })
        })
        .map_err(sql_error("canonical-corruption"))?;
    let mut revisions = Vec::new();
    for row in rows {
        revisions.push(row.map_err(sql_error("canonical-corruption"))?);
        if revisions.len() > MAX_ROWS_PER_TABLE {
            return Err(SqliteAuthorityError::new(
                "row-count-exceeded",
                "source revisions exceed the bounded candidate row limit",
            ));
        }
    }
    Ok(revisions)
}

fn load_analysis_snapshots(
    connection: &Connection,
) -> Result<Vec<EvidenceAnalysisSnapshot>, SqliteAuthorityError> {
    let mut detector_statement = connection
        .prepare(
            "SELECT analysis_snapshot_identity, detector_ordinal, detector_id, detector_version FROM analysis_snapshot_detectors ORDER BY analysis_snapshot_identity, detector_ordinal",
        )
        .map_err(sql_error("canonical-corruption"))?;
    let detector_rows = detector_statement
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, i64>(1)?,
                EvidenceDetectorIdentity {
                    id: row.get(2)?,
                    version: row.get(3)?,
                },
            ))
        })
        .map_err(sql_error("canonical-corruption"))?;
    let mut detectors = BTreeMap::<String, Vec<(usize, EvidenceDetectorIdentity)>>::new();
    for row in detector_rows {
        let (identity, ordinal, detector) = row.map_err(sql_error("canonical-corruption"))?;
        let ordinal = i64_to_usize(ordinal)?;
        detectors
            .entry(identity)
            .or_default()
            .push((ordinal, detector));
    }
    let mut statement = connection
        .prepare(
            "SELECT identity, source_revision_id, session_terms_identity, detector_config_id, detector_config_version, algorithm_id, algorithm_version FROM analysis_snapshots ORDER BY identity",
        )
        .map_err(sql_error("canonical-corruption"))?;
    let rows = statement
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, String>(4)?,
                row.get::<_, String>(5)?,
                row.get::<_, String>(6)?,
            ))
        })
        .map_err(sql_error("canonical-corruption"))?;
    let mut snapshots = Vec::new();
    for row in rows {
        let (
            identity,
            source_revision_id,
            session_terms_identity,
            detector_config_id,
            detector_config_version,
            algorithm_id,
            algorithm_version,
        ) = row.map_err(sql_error("canonical-corruption"))?;
        let mut snapshot_detectors = detectors.remove(&identity).unwrap_or_default();
        for (expected, (ordinal, _)) in snapshot_detectors.iter().enumerate() {
            if *ordinal != expected {
                return Err(SqliteAuthorityError::new(
                    "canonical-corruption",
                    "analysis detector ordering is not contiguous",
                ));
            }
        }
        snapshots.push(EvidenceAnalysisSnapshot {
            identity,
            source_revision_id,
            session_terms_identity,
            detectors: snapshot_detectors
                .drain(..)
                .map(|(_, detector)| detector)
                .collect(),
            detector_config_id,
            detector_config_version,
            algorithm_id,
            algorithm_version,
        });
    }
    if !detectors.is_empty() {
        return Err(SqliteAuthorityError::new(
            "canonical-corruption",
            "analysis detector references an absent canonical snapshot",
        ));
    }
    Ok(snapshots)
}

fn load_review_cases(
    connection: &Connection,
) -> Result<Vec<EvidenceReviewCase>, SqliteAuthorityError> {
    let mut statement = connection
        .prepare(
            "SELECT case_id, origin, observed_revision_id, anchor_revision_id, anchor_segment_position, anchor_start_byte, anchor_end_byte, observed_source_bytes, alternative_count FROM review_cases ORDER BY case_id",
        )
        .map_err(sql_error("canonical-corruption"))?;
    let rows = statement
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, i64>(4)?,
                row.get::<_, i64>(5)?,
                row.get::<_, i64>(6)?,
                row.get::<_, String>(7)?,
                row.get::<_, i64>(8)?,
            ))
        })
        .map_err(sql_error("canonical-corruption"))?;
    let mut review_cases = Vec::new();
    for row in rows {
        let (
            case_id,
            origin,
            observed_revision_id,
            anchor_revision_id,
            anchor_segment_position,
            anchor_start_byte,
            anchor_end_byte,
            observed_source_bytes,
            alternative_count,
        ) = row.map_err(sql_error("canonical-corruption"))?;
        review_cases.push(EvidenceReviewCase {
            case_id,
            origin,
            observed_revision_id,
            anchor_revision_id,
            anchor_segment_position: i64_to_usize(anchor_segment_position)?,
            anchor_start_byte: i64_to_usize(anchor_start_byte)?,
            anchor_end_byte: i64_to_usize(anchor_end_byte)?,
            observed_source_bytes,
            alternative_count: i64_to_usize(alternative_count)?,
        });
        if review_cases.len() > MAX_ROWS_PER_TABLE {
            return Err(SqliteAuthorityError::new(
                "row-count-exceeded",
                "review cases exceed the bounded candidate row limit",
            ));
        }
    }
    Ok(review_cases)
}

fn load_review_ledger_events(
    connection: &Connection,
) -> Result<Vec<EvidenceReviewLedgerEvent>, SqliteAuthorityError> {
    let mut statement = connection
        .prepare(
            "SELECT event_index, case_id, observed_revision_id, action_kind, manual_replacement_bytes, alternative_index, target_event_index, provenance FROM review_ledger_events ORDER BY event_index",
        )
        .map_err(sql_error("canonical-corruption"))?;
    let rows = statement
        .query_map([], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, Option<String>>(4)?,
                row.get::<_, Option<i64>>(5)?,
                row.get::<_, Option<i64>>(6)?,
                row.get::<_, String>(7)?,
            ))
        })
        .map_err(sql_error("canonical-corruption"))?;
    let mut events = Vec::new();
    for row in rows {
        let (
            event_index,
            case_id,
            observed_revision_id,
            action_kind,
            manual_replacement_bytes,
            alternative_index,
            target_event_index,
            provenance,
        ) = row.map_err(sql_error("canonical-corruption"))?;
        events.push(EvidenceReviewLedgerEvent {
            event_index: i64_to_usize(event_index)?,
            case_id,
            observed_revision_id,
            action_kind,
            manual_replacement_bytes,
            alternative_index: option_i64_to_usize(alternative_index)?,
            target_event_index: option_i64_to_usize(target_event_index)?,
            provenance,
        });
        if events.len() > MAX_ROWS_PER_TABLE {
            return Err(SqliteAuthorityError::new(
                "row-count-exceeded",
                "review ledger exceeds the bounded candidate row limit",
            ));
        }
    }
    Ok(events)
}

fn load_reuse_governance_events(
    connection: &Connection,
) -> Result<Vec<EvidenceReuseGovernanceEvent>, SqliteAuthorityError> {
    let mut statement = connection
        .prepare(
            "SELECT event_index, event_kind, candidate_project_scope_stable_id, candidate_locator_source_revision_id, candidate_locator_analysis_snapshot_identity, candidate_locator_review_case_id, candidate_locator_ledger_position, candidate_locator_decision_digest, candidate_locator_effective_at_ledger_length, candidate_observed_text, candidate_confirmed_replacement, promotion_payload_observed_text, promotion_payload_confirmed_replacement, promotion_locator_source_revision_id, promotion_locator_analysis_snapshot_identity, promotion_locator_review_case_id, promotion_locator_ledger_position, promotion_locator_decision_digest, promotion_locator_effective_at_ledger_length, actor_role_label, actor_display_label, promotion_project_scope_stable_id, record_id, predecessor_record_id, successor_record_id FROM reuse_governance_events ORDER BY event_index",
        )
        .map_err(sql_error("canonical-corruption"))?;
    let rows = statement
        .query_map([], |row| {
            Ok(RawGovernanceRow {
                event_index: row.get(0)?,
                event_kind: row.get(1)?,
                candidate_project_scope_stable_id: row.get(2)?,
                candidate_locator_source_revision_id: row.get(3)?,
                candidate_locator_analysis_snapshot_identity: row.get(4)?,
                candidate_locator_review_case_id: row.get(5)?,
                candidate_locator_ledger_position: row.get(6)?,
                candidate_locator_decision_digest: row.get(7)?,
                candidate_locator_effective_at_ledger_length: row.get(8)?,
                candidate_observed_text: row.get(9)?,
                candidate_confirmed_replacement: row.get(10)?,
                promotion_payload_observed_text: row.get(11)?,
                promotion_payload_confirmed_replacement: row.get(12)?,
                promotion_locator_source_revision_id: row.get(13)?,
                promotion_locator_analysis_snapshot_identity: row.get(14)?,
                promotion_locator_review_case_id: row.get(15)?,
                promotion_locator_ledger_position: row.get(16)?,
                promotion_locator_decision_digest: row.get(17)?,
                promotion_locator_effective_at_ledger_length: row.get(18)?,
                actor_role_label: row.get(19)?,
                actor_display_label: row.get(20)?,
                promotion_project_scope_stable_id: row.get(21)?,
                record_id: row.get(22)?,
                predecessor_record_id: row.get(23)?,
                successor_record_id: row.get(24)?,
            })
        })
        .map_err(sql_error("canonical-corruption"))?;
    let mut events = Vec::new();
    for row in rows {
        events.push(governance_event_from_row(
            row.map_err(sql_error("canonical-corruption"))?,
        )?);
    }
    Ok(events)
}

fn governance_event_from_row(
    row: RawGovernanceRow,
) -> Result<EvidenceReuseGovernanceEvent, SqliteAuthorityError> {
    let event_index = i64_to_usize(row.event_index)?;
    let actor = || -> Result<EvidenceGovernanceActor, SqliteAuthorityError> {
        Ok(EvidenceGovernanceActor {
            role_label: required_text(row.actor_role_label.clone(), "actor_role_label")?,
            display_label: required_text(row.actor_display_label.clone(), "actor_display_label")?,
        })
    };
    match row.event_kind.as_str() {
        "promotion_candidate_rejected" => {
            Ok(EvidenceReuseGovernanceEvent::PromotionCandidateRejected {
                event_index,
                candidate_key: candidate_key_from_row(&row)?,
                actor: actor()?,
            })
        }
        "promotion_accepted" => Ok(EvidenceReuseGovernanceEvent::PromotionAccepted {
            event_index,
            candidate_key: candidate_key_from_row(&row)?,
            payload: EvidenceExactReusableCorrection {
                observed_text: required_text(
                    row.promotion_payload_observed_text.clone(),
                    "promotion_payload_observed_text",
                )?,
                confirmed_replacement: required_text(
                    row.promotion_payload_confirmed_replacement.clone(),
                    "promotion_payload_confirmed_replacement",
                )?,
            },
            source_locator: promotion_locator_from_row(&row)?,
            actor: actor()?,
            project_scope_stable_id: required_text(
                row.promotion_project_scope_stable_id.clone(),
                "promotion_project_scope_stable_id",
            )?,
        }),
        "reusable_influence_revoked" => {
            Ok(EvidenceReuseGovernanceEvent::ReusableInfluenceRevoked {
                event_index,
                record_id: i64_to_usize(required_i64(row.record_id, "record_id")?)?,
                actor: actor()?,
            })
        }
        "reusable_influence_superseded" => {
            Ok(EvidenceReuseGovernanceEvent::ReusableInfluenceSuperseded {
                event_index,
                predecessor_record_id: i64_to_usize(required_i64(
                    row.predecessor_record_id,
                    "predecessor_record_id",
                )?)?,
                successor_record_id: i64_to_usize(required_i64(
                    row.successor_record_id,
                    "successor_record_id",
                )?)?,
                actor: actor()?,
            })
        }
        _ => Err(SqliteAuthorityError::new(
            "canonical-corruption",
            "unknown reuse governance event kind",
        )),
    }
}

fn candidate_key_from_row(
    row: &RawGovernanceRow,
) -> Result<EvidenceReuseCandidateKey, SqliteAuthorityError> {
    Ok(EvidenceReuseCandidateKey {
        project_scope_stable_id: required_text(
            row.candidate_project_scope_stable_id.clone(),
            "candidate_project_scope_stable_id",
        )?,
        source_locator: locator_from_values(
            row.candidate_locator_source_revision_id.clone(),
            row.candidate_locator_analysis_snapshot_identity.clone(),
            row.candidate_locator_review_case_id.clone(),
            row.candidate_locator_ledger_position,
            row.candidate_locator_decision_digest.clone(),
            row.candidate_locator_effective_at_ledger_length,
        )?,
        exact_payload: EvidenceExactReusableCorrection {
            observed_text: required_text(
                row.candidate_observed_text.clone(),
                "candidate_observed_text",
            )?,
            confirmed_replacement: required_text(
                row.candidate_confirmed_replacement.clone(),
                "candidate_confirmed_replacement",
            )?,
        },
    })
}

fn promotion_locator_from_row(
    row: &RawGovernanceRow,
) -> Result<EvidenceSourceDecisionLocator, SqliteAuthorityError> {
    locator_from_values(
        row.promotion_locator_source_revision_id.clone(),
        row.promotion_locator_analysis_snapshot_identity.clone(),
        row.promotion_locator_review_case_id.clone(),
        row.promotion_locator_ledger_position,
        row.promotion_locator_decision_digest.clone(),
        row.promotion_locator_effective_at_ledger_length,
    )
}

fn locator_from_values(
    source_revision_id: Option<String>,
    source_analysis_snapshot_identity: Option<String>,
    source_review_case_id: Option<String>,
    review_ledger_position: Option<i64>,
    decision_digest: Option<String>,
    effective_at_ledger_length: Option<i64>,
) -> Result<EvidenceSourceDecisionLocator, SqliteAuthorityError> {
    Ok(EvidenceSourceDecisionLocator {
        source_revision_id: required_text(source_revision_id, "source_revision_id")?,
        source_analysis_snapshot_identity: required_text(
            source_analysis_snapshot_identity,
            "source_analysis_snapshot_identity",
        )?,
        source_review_case_id: required_text(source_review_case_id, "source_review_case_id")?,
        review_ledger_position: i64_to_usize(required_i64(
            review_ledger_position,
            "review_ledger_position",
        )?)?,
        decision_digest: required_text(decision_digest, "decision_digest")?,
        effective_at_ledger_length: i64_to_usize(required_i64(
            effective_at_ledger_length,
            "effective_at_ledger_length",
        )?)?,
    })
}

fn load_reuse_binding(
    connection: &Connection,
    snapshots: &[EvidenceAnalysisSnapshot],
) -> Result<Option<EvidenceReuseEnabledAnalysisBinding>, SqliteAuthorityError> {
    let count = table_count(connection, "reuse_enabled_analysis_binding")?;
    if count > 1 {
        return Err(SqliteAuthorityError::new(
            "canonical-corruption",
            "reuse-enabled binding has more than one authority row",
        ));
    }
    let row: Option<(String, String, i64, String)> = connection
        .query_row(
            "SELECT analysis_snapshot_identity, reusable_snapshot_identity, governance_event_boundary, projection_version FROM reuse_enabled_analysis_binding WHERE id = 1",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )
        .optional()
        .map_err(sql_error("canonical-corruption"))?;
    let Some((
        analysis_snapshot_identity,
        reusable_snapshot_identity,
        governance_event_boundary,
        projection_version,
    )) = row
    else {
        return Ok(None);
    };
    let analysis_snapshot = snapshots
        .iter()
        .find(|snapshot| snapshot.identity == analysis_snapshot_identity)
        .cloned()
        .ok_or_else(|| {
            SqliteAuthorityError::new(
                "canonical-corruption",
                "reuse-enabled binding references an absent analysis snapshot",
            )
        })?;
    Ok(Some(EvidenceReuseEnabledAnalysisBinding {
        analysis_snapshot,
        analysis_snapshot_identity,
        reusable_snapshot_identity,
        governance_event_boundary: i64_to_usize(governance_event_boundary)?,
        projection_version,
    }))
}

fn validate_canonical_storage_bounds(connection: &Connection) -> Result<(), SqliteAuthorityError> {
    for (table, columns) in [
        (
            "session_meta",
            &[
                "semantic_session_id",
                "duplicated_from_session_id",
                "canonical_fingerprint",
                "session_authority_role_label",
                "session_authority_display_label",
                "material_use_basis",
                "session_terms_identity",
                "project_scope_stable_id",
                "active_analysis_snapshot_identity",
                "evidence_writer_token",
            ][..],
        ),
        (
            "source_revisions",
            &["revision_id", "predecessor_revision_id", "transcript_bytes"][..],
        ),
        (
            "analysis_snapshots",
            &[
                "identity",
                "source_revision_id",
                "session_terms_identity",
                "detector_config_id",
                "detector_config_version",
                "algorithm_id",
                "algorithm_version",
            ][..],
        ),
        (
            "analysis_snapshot_detectors",
            &[
                "analysis_snapshot_identity",
                "detector_id",
                "detector_version",
            ][..],
        ),
        (
            "review_cases",
            &[
                "case_id",
                "origin",
                "observed_revision_id",
                "anchor_revision_id",
                "observed_source_bytes",
            ][..],
        ),
        (
            "review_ledger_events",
            &[
                "case_id",
                "observed_revision_id",
                "action_kind",
                "manual_replacement_bytes",
                "provenance",
            ][..],
        ),
        (
            "reuse_governance_events",
            &[
                "event_kind",
                "candidate_project_scope_stable_id",
                "candidate_locator_source_revision_id",
                "candidate_locator_analysis_snapshot_identity",
                "candidate_locator_review_case_id",
                "candidate_locator_decision_digest",
                "candidate_observed_text",
                "candidate_confirmed_replacement",
                "promotion_payload_observed_text",
                "promotion_payload_confirmed_replacement",
                "promotion_locator_source_revision_id",
                "promotion_locator_analysis_snapshot_identity",
                "promotion_locator_review_case_id",
                "promotion_locator_decision_digest",
                "actor_role_label",
                "actor_display_label",
                "promotion_project_scope_stable_id",
            ][..],
        ),
        (
            "reuse_enabled_analysis_binding",
            &[
                "analysis_snapshot_identity",
                "reusable_snapshot_identity",
                "projection_version",
            ][..],
        ),
        (
            "authority_transitions",
            &["canonical_fingerprint", "acknowledgement_status"][..],
        ),
        ("writer_ownership", &["token", "process_instance_id"][..]),
    ] {
        let count = table_count(connection, table)?;
        if count > MAX_ROWS_PER_TABLE {
            return Err(SqliteAuthorityError::new(
                "row-count-exceeded",
                format!("{table} exceeds the bounded candidate row limit"),
            ));
        }
        for column in columns {
            let sql =
                format!("SELECT 1 FROM {table} WHERE length(CAST({column} AS BLOB)) > ?1 LIMIT 1");
            let oversized: Option<i64> = connection
                .query_row(&sql, [usize_to_i64(MAX_TEXT_BYTES)?], |row| row.get(0))
                .optional()
                .map_err(sql_error("canonical-corruption"))?;
            if oversized.is_some() {
                return Err(SqliteAuthorityError::new(
                    "canonical-payload-too-large",
                    format!("{table}.{column} exceeds the bounded candidate payload limit"),
                ));
            }
        }
    }
    Ok(())
}

fn table_count(connection: &Connection, table: &str) -> Result<usize, SqliteAuthorityError> {
    let sql = format!("SELECT COUNT(*) FROM {table}");
    let count: i64 = connection
        .query_row(&sql, [], |row| row.get(0))
        .map_err(sql_error("canonical-corruption"))?;
    i64_to_usize(count)
}

fn rebuild_derived_cache(
    connection: &Connection,
    state: &CurrentContractState,
    fingerprint: &str,
) -> Result<(), SqliteAuthorityError> {
    let (derived, violations) = derive_contract_projection(state);
    if !violations.is_empty() {
        return Err(SqliteAuthorityError::new(
            "canonical-corruption",
            "canonical authority cannot rebuild its derived projection",
        ));
    }
    let payload = serialize_bounded(&derived, "derived-cache-too-large")?;
    let derived_fingerprint = super::oracle::derived_fingerprint(&derived);
    connection
        .execute(
            "INSERT INTO derived_contract_cache (id, canonical_fingerprint, derived_fingerprint, payload_json, cache_schema_version) VALUES (1, ?1, ?2, ?3, ?4) ON CONFLICT(id) DO UPDATE SET canonical_fingerprint = excluded.canonical_fingerprint, derived_fingerprint = excluded.derived_fingerprint, payload_json = excluded.payload_json, cache_schema_version = excluded.cache_schema_version",
            params![fingerprint, derived_fingerprint, payload, DERIVED_CACHE_SCHEMA_VERSION],
        )
        .map_err(sql_error("sqlite-rebuild-derived-cache"))?;
    Ok(())
}

fn rebuild_derived_cache_in_tx(
    tx: &Transaction<'_>,
    state: &CurrentContractState,
    fingerprint: &str,
) -> Result<(), SqliteAuthorityError> {
    let (derived, violations) = derive_contract_projection(state);
    if !violations.is_empty() {
        return Err(SqliteAuthorityError::new(
            "canonical-corruption",
            "canonical authority cannot rebuild its derived projection",
        ));
    }
    let payload = serialize_bounded(&derived, "derived-cache-too-large")?;
    let derived_fingerprint = super::oracle::derived_fingerprint(&derived);
    tx.execute(
        "INSERT INTO derived_contract_cache (id, canonical_fingerprint, derived_fingerprint, payload_json, cache_schema_version) VALUES (1, ?1, ?2, ?3, ?4) ON CONFLICT(id) DO UPDATE SET canonical_fingerprint = excluded.canonical_fingerprint, derived_fingerprint = excluded.derived_fingerprint, payload_json = excluded.payload_json, cache_schema_version = excluded.cache_schema_version",
        params![fingerprint, derived_fingerprint, payload, DERIVED_CACHE_SCHEMA_VERSION],
    )
    .map_err(sql_error("sqlite-rebuild-derived-cache"))?;
    Ok(())
}

fn acquire_writer_ownership(
    connection: &mut Connection,
    process_instance_id: &str,
) -> Result<(String, i64), SqliteAuthorityError> {
    let now = now_unix_ms()?;
    let tx = connection
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(sql_error("sqlite-writer-lock"))?;
    let (current_token, owner_epoch, lease_duration_ms, lease_expires_at): (Option<String>, i64, i64, i64) = tx
        .query_row(
            "SELECT token, owner_epoch, lease_duration_ms, lease_expires_at_unix_ms FROM writer_ownership WHERE id = 1",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )
        .map_err(sql_error("canonical-corruption"))?;
    if current_token.is_some() && lease_expires_at > now {
        return Err(SqliteAuthorityError::new(
            "writer-already-open",
            "SQLite-v3 candidate permits only one live authoritative writer",
        ));
    }
    let token = format!("sqlite-v3-writer:{}", uuid::Uuid::new_v4());
    let next_epoch = if current_token.is_some() {
        owner_epoch.checked_add(1).ok_or_else(|| {
            SqliteAuthorityError::new("writer-epoch-overflow", "writer epoch overflow")
        })?
    } else {
        owner_epoch
    };
    let updated = tx
        .execute(
            "UPDATE writer_ownership SET token = ?1, owner_epoch = ?2, lease_expires_at_unix_ms = ?3, process_instance_id = ?4, holder_pid = ?5 WHERE id = 1 AND (token IS NULL OR lease_expires_at_unix_ms <= ?6)",
            params![
                token,
                next_epoch,
                now.checked_add(lease_duration_ms).ok_or_else(|| SqliteAuthorityError::new("lease-overflow", "writer lease overflow"))?,
                process_instance_id,
                i64::from(std::process::id()),
                now,
            ],
        )
        .map_err(sql_error("sqlite-writer-lock"))?;
    if updated != 1 {
        return Err(SqliteAuthorityError::new(
            "writer-already-open",
            "writer ownership acquisition lost a concurrent race",
        ));
    }
    tx.commit().map_err(sql_error("sqlite-writer-lock"))?;
    Ok((token, next_epoch))
}

fn validate_writer_ownership(
    connection: &Connection,
    token: &str,
    epoch: i64,
) -> Result<(), SqliteAuthorityError> {
    let now = now_unix_ms()?;
    let (stored_token, stored_epoch, lease_expires_at): (Option<String>, i64, i64) = connection
        .query_row(
            "SELECT token, owner_epoch, lease_expires_at_unix_ms FROM writer_ownership WHERE id = 1",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .map_err(sql_error("canonical-corruption"))?;
    if stored_token.as_deref() != Some(token) || stored_epoch != epoch {
        return Err(SqliteAuthorityError::new(
            "writer-epoch-mismatch",
            "writer token or epoch no longer owns this authority",
        ));
    }
    if lease_expires_at <= now {
        return Err(SqliteAuthorityError::new(
            "writer-lease-expired",
            "writer lease expired before authoritative transition",
        ));
    }
    Ok(())
}

fn validate_writer_ownership_tx(
    tx: &Transaction<'_>,
    token: &str,
    epoch: i64,
) -> Result<(), SqliteAuthorityError> {
    let now = now_unix_ms()?;
    let (stored_token, stored_epoch, lease_expires_at): (Option<String>, i64, i64) = tx
        .query_row(
            "SELECT token, owner_epoch, lease_expires_at_unix_ms FROM writer_ownership WHERE id = 1",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .map_err(sql_error("canonical-corruption"))?;
    if stored_token.as_deref() != Some(token) || stored_epoch != epoch {
        return Err(SqliteAuthorityError::new(
            "writer-epoch-mismatch",
            "writer token or epoch no longer owns this authority",
        ));
    }
    if lease_expires_at <= now {
        return Err(SqliteAuthorityError::new(
            "writer-lease-expired",
            "writer lease expired before authoritative transition",
        ));
    }
    Ok(())
}

fn renew_writer_lease_tx(
    tx: &Transaction<'_>,
    token: &str,
    epoch: i64,
) -> Result<(), SqliteAuthorityError> {
    validate_writer_ownership_tx(tx, token, epoch)?;
    let now = now_unix_ms()?;
    let duration: i64 = tx
        .query_row(
            "SELECT lease_duration_ms FROM writer_ownership WHERE id = 1",
            [],
            |row| row.get(0),
        )
        .map_err(sql_error("canonical-corruption"))?;
    let updated = tx
        .execute(
            "UPDATE writer_ownership SET lease_expires_at_unix_ms = ?1 WHERE id = 1 AND token = ?2 AND owner_epoch = ?3",
            params![
                now.checked_add(duration).ok_or_else(|| SqliteAuthorityError::new("lease-overflow", "writer lease overflow"))?,
                token,
                epoch,
            ],
        )
        .map_err(sql_error("sqlite-renew-writer"))?;
    if updated != 1 {
        return Err(SqliteAuthorityError::new(
            "writer-epoch-mismatch",
            "writer lease could not be renewed",
        ));
    }
    Ok(())
}

fn release_writer_ownership(
    connection: &Connection,
    token: &str,
    epoch: i64,
) -> Result<(), SqliteAuthorityError> {
    connection
        .execute(
            "UPDATE writer_ownership SET token = NULL, lease_expires_at_unix_ms = 0, process_instance_id = '', holder_pid = NULL WHERE id = 1 AND token = ?1 AND owner_epoch = ?2",
            params![token, epoch],
        )
        .map_err(sql_error("sqlite-writer-unlock"))?;
    Ok(())
}

fn mark_transition_acknowledged(
    connection: &Connection,
    generation: u64,
) -> Result<(), SqliteAuthorityError> {
    let updated = connection
        .execute(
            "UPDATE authority_transitions SET acknowledgement_status = 'acknowledged' WHERE generation = ?1",
            [u64_to_i64(generation)?],
        )
        .map_err(sql_error("sqlite-acknowledge-transition"))?;
    if updated != 1 {
        return Err(SqliteAuthorityError::new(
            "canonical-corruption",
            "committed transition is absent while acknowledging authority",
        ));
    }
    Ok(())
}

fn validate_preconditions(
    metadata: &SessionMetadata,
    preconditions: &CurrentContractPreconditions,
) -> Result<(), SqliteAuthorityError> {
    if metadata.committed_generation != preconditions.expected_generation {
        return Err(SqliteAuthorityError::new(
            "stale-generation-precondition",
            "expected committed generation is stale",
        ));
    }
    if metadata.durable_review_ledger_head != preconditions.review_ledger_head {
        return Err(SqliteAuthorityError::new(
            "stale-review-ledger-precondition",
            "review-ledger head precondition is stale",
        ));
    }
    if metadata.durable_reuse_governance_head != preconditions.reuse_governance_head {
        return Err(SqliteAuthorityError::new(
            "stale-reuse-governance-precondition",
            "reuse-governance head precondition is stale",
        ));
    }
    if metadata.active_analysis_snapshot_identity != preconditions.active_analysis_snapshot_identity
    {
        return Err(SqliteAuthorityError::new(
            "stale-analysis-selection-precondition",
            "active analysis selection precondition is stale",
        ));
    }
    Ok(())
}

fn canonicalized_state(mut state: CurrentContractState) -> CurrentContractState {
    finalize_derived_fields(&mut state);
    state.normalize()
}

fn validate_state(state: &CurrentContractState) -> Result<(), SqliteAuthorityError> {
    let result = CurrentContractOracle::validate(state);
    if result.passed {
        Ok(())
    } else {
        Err(SqliteAuthorityError::new(
            "current-contract-oracle-rejected",
            format!(
                "{} current-contract oracle violation(s)",
                result.violations.len()
            ),
        ))
    }
}

fn validate_state_bounds(state: &CurrentContractState) -> Result<(), SqliteAuthorityError> {
    let bytes = serde_json::to_vec(&state.canonical_projection())
        .map_err(|error| SqliteAuthorityError::new("state-serialize-failed", error.to_string()))?;
    if bytes.len() > MAX_TEXT_BYTES {
        return Err(SqliteAuthorityError::new(
            "canonical-payload-too-large",
            "outbound canonical authority exceeds the bounded candidate payload limit",
        ));
    }
    let detector_count = state
        .analysis_snapshots
        .iter()
        .try_fold(0_usize, |count, snapshot| {
            count.checked_add(snapshot.detectors.len())
        })
        .ok_or_else(|| SqliteAuthorityError::new("row-count-overflow", "row count overflow"))?;
    let record_count = state
        .source_revisions
        .len()
        .checked_add(state.analysis_snapshots.len())
        .and_then(|count| count.checked_add(detector_count))
        .and_then(|count| count.checked_add(state.review_cases.len()))
        .and_then(|count| count.checked_add(state.review_ledger_events.len()))
        .and_then(|count| count.checked_add(state.reuse_governance_events.len()))
        .ok_or_else(|| SqliteAuthorityError::new("row-count-overflow", "row count overflow"))?;
    if record_count > MAX_ROWS_PER_TABLE {
        return Err(SqliteAuthorityError::new(
            "row-count-exceeded",
            "outbound canonical authority exceeds the bounded candidate row limit",
        ));
    }
    Ok(())
}

fn serialize_bounded(
    value: &impl Serialize,
    code: &'static str,
) -> Result<String, SqliteAuthorityError> {
    let bytes = serde_json::to_vec(value).map_err(|error| {
        SqliteAuthorityError::new("derived-cache-serialize-failed", error.to_string())
    })?;
    if bytes.len() > MAX_TEXT_BYTES {
        return Err(SqliteAuthorityError::new(
            code,
            "derived cache exceeds the bounded candidate payload limit",
        ));
    }
    String::from_utf8(bytes).map_err(|error| {
        SqliteAuthorityError::new("derived-cache-serialize-failed", error.to_string())
    })
}

fn validate_session_id(session_id: &str) -> Result<(), SqliteAuthorityError> {
    if session_id.is_empty()
        || session_id.len() > 128
        || session_id.contains('\0')
        || session_id.contains('/')
        || session_id.contains('\\')
        || session_id == "."
        || session_id == ".."
    {
        return Err(SqliteAuthorityError::new(
            "unsafe-session-id",
            "semantic session identity must be bounded, non-NUL, and not a path",
        ));
    }
    Ok(())
}

fn physical_storage_key(session_id: &str) -> String {
    let digest = Sha256::digest(session_id.as_bytes());
    let mut key = String::with_capacity("vp-session-v1-".len() + digest.len() * 2);
    key.push_str("vp-session-v1-");
    for byte in digest {
        use std::fmt::Write as _;
        write!(&mut key, "{byte:02x}").expect("writing to String cannot fail");
    }
    key
}

fn database_path(session: &SqliteAuthoritySession) -> PathBuf {
    session.root.join("current-contract-v3.sqlite")
}

fn wal_path(session: &SqliteAuthoritySession) -> PathBuf {
    session.root.join("current-contract-v3.sqlite-wal")
}

fn shm_path(session: &SqliteAuthoritySession) -> PathBuf {
    session.root.join("current-contract-v3.sqlite-shm")
}

fn creation_marker_path(session: &SqliteAuthoritySession) -> PathBuf {
    session.root.join(CREATION_MARKER_NAME)
}

fn validate_existing_authority_leaf(
    path: &Path,
    alias_code: &'static str,
) -> Result<(), SqliteAuthorityError> {
    let metadata =
        fs::symlink_metadata(path).map_err(|error| io_error("stat-authority-leaf", error))?;
    if is_static_filesystem_alias(&metadata) || !metadata.is_file() {
        return Err(SqliteAuthorityError::new(
            alias_code,
            "candidate authority leaf must be a regular non-alias file",
        ));
    }
    if metadata.len() > MAX_DATABASE_BYTES {
        return Err(SqliteAuthorityError::new(
            "canonical-storage-too-large",
            "candidate authority leaf exceeds the bounded storage limit",
        ));
    }
    let file = File::open(path).map_err(|error| io_error("open-authority-leaf", error))?;
    validate_opened_authority_leaf(&file, alias_code)
}

fn validate_optional_authority_leaf(
    path: &Path,
    alias_code: &'static str,
) -> Result<(), SqliteAuthorityError> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if is_static_filesystem_alias(&metadata) || !metadata.is_file() => {
            Err(SqliteAuthorityError::new(
                alias_code,
                "candidate authority leaf must be a regular non-alias file",
            ))
        }
        Ok(_) => validate_existing_authority_leaf(path, alias_code),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(io_error("stat-authority-leaf", error)),
    }
}

fn validate_opened_authority_leaf(
    file: &File,
    alias_code: &'static str,
) -> Result<(), SqliteAuthorityError> {
    let metadata = file
        .metadata()
        .map_err(|error| io_error("stat-opened-authority-leaf", error))?;
    if !metadata.is_file() || metadata.file_type().is_symlink() {
        return Err(SqliteAuthorityError::new(
            alias_code,
            "opened candidate authority leaf is not a regular file",
        ));
    }
    if authority_hard_link_count(file, &metadata)? != 1 {
        return Err(SqliteAuthorityError::new(
            "authority-leaf-hard-linked",
            "candidate authority leaf must have exactly one hard link",
        ));
    }
    Ok(())
}

fn authority_hard_link_count(
    file: &File,
    metadata: &fs::Metadata,
) -> Result<u64, SqliteAuthorityError> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        let _ = file;
        Ok(metadata.nlink())
    }
    #[cfg(windows)]
    {
        let _ = metadata;
        authority_hard_link_count_windows(file)
    }
    #[cfg(not(any(unix, windows)))]
    {
        let _ = (file, metadata);
        Err(SqliteAuthorityError::new(
            "authority-link-count-unavailable",
            "candidate cannot verify authority link count on this platform",
        ))
    }
}

#[cfg(windows)]
fn authority_hard_link_count_windows(file: &File) -> Result<u64, SqliteAuthorityError> {
    use std::os::windows::io::AsRawHandle;
    use windows_sys::Win32::Foundation::HANDLE;
    use windows_sys::Win32::Storage::FileSystem::{
        BY_HANDLE_FILE_INFORMATION, GetFileInformationByHandle,
    };

    let mut information = std::mem::MaybeUninit::<BY_HANDLE_FILE_INFORMATION>::zeroed();
    // SAFETY: `file` is a live handle for the authority leaf being checked and
    // `information` is writable ABI-compatible output storage.
    let succeeded = unsafe {
        GetFileInformationByHandle(file.as_raw_handle() as HANDLE, information.as_mut_ptr())
    };
    if succeeded == 0 {
        return Err(SqliteAuthorityError::new(
            "authority-link-count-unavailable",
            std::io::Error::last_os_error().to_string(),
        ));
    }
    // SAFETY: Windows initializes all fields on a successful call.
    Ok(unsafe { information.assume_init() }.nNumberOfLinks as u64)
}

fn is_static_filesystem_alias(metadata: &fs::Metadata) -> bool {
    if metadata.file_type().is_symlink() {
        return true;
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        use windows_sys::Win32::Storage::FileSystem::FILE_ATTRIBUTE_REPARSE_POINT;

        (metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT) != 0
    }
    #[cfg(not(windows))]
    {
        false
    }
}

fn derive_duplicate_evidence_writer_token(session_id: &str) -> String {
    if let Some(contract_local_id) = session_id.strip_prefix("session:current-contract:") {
        return format!("writer:{contract_local_id}");
    }
    let mut encoded = String::with_capacity(session_id.len() * 2);
    for byte in session_id.as_bytes() {
        use std::fmt::Write as _;
        write!(&mut encoded, "{byte:02x}").expect("writing to String cannot fail");
    }
    format!("writer-encoded-session-id-v1:{encoded}")
}

fn required_text(
    value: Option<String>,
    field: &'static str,
) -> Result<String, SqliteAuthorityError> {
    value.ok_or_else(|| {
        SqliteAuthorityError::new(
            "canonical-corruption",
            format!("missing required {field} in relational authority"),
        )
    })
}

fn required_i64(value: Option<i64>, field: &'static str) -> Result<i64, SqliteAuthorityError> {
    value.ok_or_else(|| {
        SqliteAuthorityError::new(
            "canonical-corruption",
            format!("missing required {field} in relational authority"),
        )
    })
}

fn usize_to_i64(value: usize) -> Result<i64, SqliteAuthorityError> {
    i64::try_from(value).map_err(|_| {
        SqliteAuthorityError::new("integer-range", "usize does not fit SQLite integer")
    })
}

fn option_usize_to_i64(value: Option<usize>) -> Result<Option<i64>, SqliteAuthorityError> {
    value.map(usize_to_i64).transpose()
}

fn i64_to_usize(value: i64) -> Result<usize, SqliteAuthorityError> {
    usize::try_from(value).map_err(|_| {
        SqliteAuthorityError::new(
            "canonical-corruption",
            "SQLite integer cannot be represented as a non-negative usize",
        )
    })
}

fn option_i64_to_usize(value: Option<i64>) -> Result<Option<usize>, SqliteAuthorityError> {
    value.map(i64_to_usize).transpose()
}

fn i64_to_u64(value: i64) -> Result<u64, SqliteAuthorityError> {
    u64::try_from(value).map_err(|_| {
        SqliteAuthorityError::new(
            "canonical-corruption",
            "SQLite generation cannot be represented as a non-negative u64",
        )
    })
}

fn u64_to_usize(value: u64) -> Result<usize, SqliteAuthorityError> {
    usize::try_from(value).map_err(|_| {
        SqliteAuthorityError::new(
            "integer-conversion-failed",
            "u64 value does not fit platform usize",
        )
    })
}

fn i64_to_u32(value: i64) -> Result<u32, SqliteAuthorityError> {
    u32::try_from(value).map_err(|_| {
        SqliteAuthorityError::new(
            "canonical-corruption",
            "SQLite format cannot be represented as a u32",
        )
    })
}

fn u64_to_i64(value: u64) -> Result<i64, SqliteAuthorityError> {
    i64::try_from(value)
        .map_err(|_| SqliteAuthorityError::new("integer-range", "u64 does not fit SQLite integer"))
}

fn now_unix_ms() -> Result<i64, SqliteAuthorityError> {
    let duration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| SqliteAuthorityError::new("clock-before-epoch", error.to_string()))?;
    i64::try_from(duration.as_millis())
        .map_err(|_| SqliteAuthorityError::new("clock-range", "clock exceeds SQLite integer range"))
}

fn sql_error(code: &'static str) -> impl Fn(rusqlite::Error) -> SqliteAuthorityError {
    move |error| SqliteAuthorityError::new(code, error.to_string())
}

fn io_error(code: &'static str, error: std::io::Error) -> SqliteAuthorityError {
    SqliteAuthorityError::new(code, error.to_string())
}
