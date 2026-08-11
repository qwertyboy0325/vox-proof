//! Bounded, spike-only append authority for current-contract v3 fixtures.
//!
//! This is a candidate test adapter, not a selected persistence mechanism or a
//! production session store.  Its append records contain complete semantic
//! transitions so the current-contract oracle remains the sole semantic oracle.

use std::collections::BTreeSet;
use std::fs::{self, File, OpenOptions};
use std::io::{BufRead, BufReader, Read, Write};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use super::derivation::finalize_derived_fields;
use super::model::CurrentContractState;
use super::oracle::{CurrentContractOracle, canonical_fingerprint};

pub const APPEND_AUTHORITATIVE_CANDIDATE_ID: &str =
    "current-contract-append-authoritative-candidate";
pub const APPEND_AUTHORITATIVE_CANDIDATE_VERSION: &str = "01B-2";
pub const APPEND_AUTHORITATIVE_FORMAT_VERSION: u32 = 1;
const MAX_RECORD_BYTES: usize = 2 * 1024 * 1024;
const MAX_RECORD_COUNT: usize = 4_096;
const MAX_MANIFEST_BYTES: usize = 64 * 1024;
const CREATION_MARKER_NAME: &str = ".creation-incomplete";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppendAuthoritySession {
    session_id: String,
    root: PathBuf,
}

impl AppendAuthoritySession {
    pub fn session_id(&self) -> &str {
        &self.session_id
    }

    /// Test-support access to the physical candidate directory; it is not a
    /// semantic session identifier or persistence-format field.
    #[doc(hidden)]
    pub fn storage_path_for_test(&self) -> &Path {
        &self.root
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppendTailStatus {
    Clean,
    IncompleteUncommitted { sequence: u64 },
}

#[derive(Debug)]
pub struct OpenedAppendAuthoritySession {
    pub session: AppendAuthoritySession,
    pub committed_sequence: u64,
    pub tail_status: AppendTailStatus,
    pub checkpoint_status: AppendCheckpointStatus,
    open_mode: AppendOpenMode,
    adapter_identity: String,
    writer_guard: Option<File>,
    writer_token: Option<String>,
    normalized_state: CurrentContractState,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppendCheckpointStatus {
    Current,
    Behind { last_error_code: &'static str },
}

impl OpenedAppendAuthoritySession {
    pub fn normalized_state(&self) -> &CurrentContractState {
        &self.normalized_state
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DurableAppendAck {
    pub committed_sequence: u64,
    pub canonical_fingerprint: String,
    pub checkpoint_status: AppendCheckpointStatus,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CleanupPlan {
    pub temporary_artifacts: Vec<String>,
    pub destructive_cleanup_permitted: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppendAuthorityError {
    pub code: &'static str,
    pub message: String,
}

impl AppendAuthorityError {
    fn new(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }
}

impl std::fmt::Display for AppendAuthorityError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}: {}", self.code, self.message)
    }
}

impl std::error::Error for AppendAuthorityError {}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct AppendManifest {
    format_version: u32,
    session_id: String,
    committed_sequence: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "record_kind", rename_all = "snake_case")]
enum AppendRecord {
    State {
        sequence: u64,
        state: Box<CurrentContractState>,
    },
    Commit {
        sequence: u64,
        canonical_fingerprint: String,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppendOpenMode {
    Writable,
    ReadOnly,
}

/// Candidate-only append storage surface for package 01B.
pub struct AppendAuthoritativeCandidateAdapter {
    storage_root: PathBuf,
    adapter_identity: String,
}

impl AppendAuthoritativeCandidateAdapter {
    pub fn new(storage_root: impl Into<PathBuf>) -> Result<Self, AppendAuthorityError> {
        let storage_root = storage_root.into();
        fs::create_dir_all(&storage_root)
            .map_err(|error| io_error("create-storage-root", error))?;
        let storage_root = fs::canonicalize(storage_root)
            .map_err(|error| io_error("canonicalize-storage-root", error))?;
        Ok(Self {
            storage_root,
            adapter_identity: uuid::Uuid::new_v4().to_string(),
        })
    }

    pub fn candidate_id(&self) -> &'static str {
        APPEND_AUTHORITATIVE_CANDIDATE_ID
    }

    pub fn candidate_version(&self) -> &'static str {
        APPEND_AUTHORITATIVE_CANDIDATE_VERSION
    }

    pub fn create(
        &self,
        state: &CurrentContractState,
    ) -> Result<AppendAuthoritySession, AppendAuthorityError> {
        validate_state(state)?;
        validate_session_id(&state.session_id)?;
        let root = self.session_root(&state.session_id);
        fs::create_dir(&root).map_err(|error| {
            if error.kind() == std::io::ErrorKind::AlreadyExists {
                AppendAuthorityError::new(
                    "session-already-exists",
                    "candidate session already exists",
                )
            } else {
                io_error("create-session-root", error)
            }
        })?;
        let session = AppendAuthoritySession {
            session_id: state.session_id.clone(),
            root,
        };
        OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(creation_marker_path(&session))
            .map_err(|error| io_error("create-session-marker", error))?;
        fs::create_dir(session.root.join("temporary"))
            .map_err(|error| io_error("create-session-root", error))?;
        let (writer_guard, _) = self.acquire_writer_lock(&session)?;
        let manifest = AppendManifest {
            format_version: APPEND_AUTHORITATIVE_FORMAT_VERSION,
            session_id: session.session_id.clone(),
            committed_sequence: 0,
        };
        write_manifest(&session, &manifest)?;
        let log_file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(log_path(&session))
            .map_err(|error| io_error("create-append-log", error))?;
        validate_opened_authority_leaf(&log_file, "canonical append log")?;
        drop(log_file);
        self.append_committed_state(&session, 0, 0, state, &manifest, true)?;
        fs::remove_file(creation_marker_path(&session))
            .map_err(|error| io_error("publish-session", error))?;
        drop(writer_guard);
        Ok(session)
    }

    /// Reconstructs a session reference from only this adapter's bounded root and a validated ID.
    pub fn open_existing(
        &self,
        session_id: impl Into<String>,
        mode: AppendOpenMode,
    ) -> Result<OpenedAppendAuthoritySession, AppendAuthorityError> {
        let session_id = session_id.into();
        validate_session_id(&session_id)?;
        self.open(
            &AppendAuthoritySession {
                root: self.session_root(&session_id),
                session_id,
            },
            mode,
        )
    }

    fn open(
        &self,
        session: &AppendAuthoritySession,
        mode: AppendOpenMode,
    ) -> Result<OpenedAppendAuthoritySession, AppendAuthorityError> {
        self.validate_session_layout(session)?;
        let (writer_guard, writer_token) = if mode == AppendOpenMode::Writable {
            let (guard, token) = self.acquire_writer_lock(session)?;
            (Some(guard), Some(token))
        } else {
            (None, None)
        };
        let manifest = read_manifest(session)?;
        validate_manifest(session, &manifest, mode)?;
        let mut replayed = replay(session)?;
        if manifest.committed_sequence > replayed.committed_sequence {
            return Err(AppendAuthorityError::new(
                "manifest-ahead-of-append-authority",
                "manifest claims an uncommitted authoritative boundary",
            ));
        }
        if mode == AppendOpenMode::Writable && replayed.tail_status != AppendTailStatus::Clean {
            recover_committed_prefix(session, &replayed)?;
            replayed = replay(session)?;
        }
        let checkpoint_status = if manifest.committed_sequence < replayed.committed_sequence {
            if mode == AppendOpenMode::Writable {
                repair_manifest_checkpoint(session, &manifest, replayed.committed_sequence)?;
                AppendCheckpointStatus::Current
            } else {
                AppendCheckpointStatus::Behind {
                    last_error_code: "manifest-behind-append-authority",
                }
            }
        } else {
            AppendCheckpointStatus::Current
        };
        Ok(OpenedAppendAuthoritySession {
            session: session.clone(),
            committed_sequence: replayed.committed_sequence,
            tail_status: replayed.tail_status,
            checkpoint_status,
            open_mode: mode,
            adapter_identity: self.adapter_identity.clone(),
            writer_guard,
            writer_token,
            normalized_state: replayed.state,
        })
    }

    pub fn append_authoritative_transition(
        &self,
        opened: &mut OpenedAppendAuthoritySession,
        expected_committed_sequence: u64,
        next_state: &CurrentContractState,
    ) -> Result<DurableAppendAck, AppendAuthorityError> {
        if opened.open_mode != AppendOpenMode::Writable {
            return Err(AppendAuthorityError::new(
                "not-authoritative-writer",
                "authoritative append requires a writable open",
            ));
        }
        self.validate_writer_lock(opened)?;
        let mut manifest = read_manifest(&opened.session)?;
        validate_manifest(&opened.session, &manifest, AppendOpenMode::Writable)?;
        let mut current = replay(&opened.session)?;
        if current.tail_status != AppendTailStatus::Clean {
            recover_committed_prefix(&opened.session, &current)?;
            current = replay(&opened.session)?;
        }
        if manifest.committed_sequence > current.committed_sequence {
            return Err(AppendAuthorityError::new(
                "manifest-ahead-of-append-authority",
                "manifest claims an uncommitted authoritative boundary",
            ));
        }
        if manifest.committed_sequence < current.committed_sequence {
            repair_manifest_checkpoint(&opened.session, &manifest, current.committed_sequence)?;
            manifest.committed_sequence = current.committed_sequence;
            opened.checkpoint_status = AppendCheckpointStatus::Current;
        }
        if current.committed_sequence != expected_committed_sequence {
            return Err(AppendAuthorityError::new(
                "stale-append-precondition",
                "expected committed sequence is stale",
            ));
        }
        let canonical_next_state = canonicalized_state(next_state.clone());
        validate_state(&canonical_next_state)?;
        if canonical_next_state.session_id != opened.session.session_id {
            return Err(AppendAuthorityError::new(
                "session-identity-transition",
                "authoritative append cannot change the session identity",
            ));
        }
        let next_fingerprint = canonical_fingerprint(&canonical_next_state.canonical_projection());
        if current.canonical_fingerprints.contains(&next_fingerprint) {
            return Err(AppendAuthorityError::new(
                "semantic-duplicate-transition",
                "an append transition must add distinct canonical authority",
            ));
        }
        let acknowledgement = self.append_committed_state(
            &opened.session,
            current.committed_sequence,
            current.record_count,
            &canonical_next_state,
            &manifest,
            false,
        )?;
        opened.committed_sequence = acknowledgement.committed_sequence;
        opened.tail_status = AppendTailStatus::Clean;
        opened.checkpoint_status = acknowledgement.checkpoint_status.clone();
        opened.normalized_state = canonical_next_state;
        Ok(acknowledgement)
    }

    pub fn close(&self, opened: OpenedAppendAuthoritySession) -> Result<(), AppendAuthorityError> {
        drop(opened);
        Ok(())
    }

    pub fn duplicate(
        &self,
        source: &mut OpenedAppendAuthoritySession,
        new_session_id: impl Into<String>,
    ) -> Result<AppendAuthoritySession, AppendAuthorityError> {
        let new_session_id = new_session_id.into();
        validate_session_id(&new_session_id)?;
        self.validate_writer_lock(source)?;
        let replayed = replay(&source.session)?;
        if replayed.tail_status != AppendTailStatus::Clean {
            recover_committed_prefix(&source.session, &replayed)?;
        }
        let mut copied = replay(&source.session)?.state;
        copied.duplicated_from_session_id = Some(copied.session_id.clone());
        copied.session_id = new_session_id;
        let duplicate_writer_token = derive_duplicate_evidence_writer_token(&copied.session_id);
        if duplicate_writer_token == copied.durable_command_tokens.evidence_writer_token {
            return Err(AppendAuthorityError::new(
                "duplicate-evidence-writer-identity-collision",
                "semantic duplication must create an independent evidence writer identity",
            ));
        }
        copied.durable_command_tokens.evidence_writer_token = duplicate_writer_token;
        self.create(&copied)
    }

    pub fn plan_cleanup(
        &self,
        session: &AppendAuthoritySession,
    ) -> Result<CleanupPlan, AppendAuthorityError> {
        self.validate_session_layout(session)?;
        let temporary = session.root.join("temporary");
        let mut temporary_artifacts = Vec::new();
        for entry in fs::read_dir(temporary).map_err(|error| io_error("read-temporary", error))? {
            let entry = entry.map_err(|error| io_error("read-temporary-entry", error))?;
            let name = entry.file_name().to_string_lossy().to_string();
            if !name.contains('/') && !name.contains('\\') {
                temporary_artifacts.push(name);
            }
        }
        temporary_artifacts.sort();
        Ok(CleanupPlan {
            temporary_artifacts,
            destructive_cleanup_permitted: false,
        })
    }

    /// Harness hook: exercise the exact production record-capacity preflight without I/O.
    pub fn validate_record_capacity_for_test(
        &self,
        current_record_count: usize,
    ) -> Result<(), AppendAuthorityError> {
        validate_record_capacity(current_record_count)
    }

    /// Harness hook: append a valid state record without its commit acknowledgement.
    pub fn append_incomplete_tail_for_test(
        &self,
        opened: &mut OpenedAppendAuthoritySession,
        state: &CurrentContractState,
    ) -> Result<(), AppendAuthorityError> {
        self.validate_writer_lock(opened)?;
        validate_state(state)?;
        let replayed = replay(&opened.session)?;
        if replayed.tail_status != AppendTailStatus::Clean {
            return Err(AppendAuthorityError::new(
                "fault-injection-tail-not-clean",
                "incomplete-tail injection requires a clean append boundary",
            ));
        }
        let sequence = replayed.committed_sequence.checked_add(1).ok_or_else(|| {
            AppendAuthorityError::new("append-sequence-overflow", "append sequence overflow")
        })?;
        append_record(
            &opened.session,
            &AppendRecord::State {
                sequence,
                state: Box::new(canonicalized_state(state.clone())),
            },
        )
    }

    /// Harness hook: append hostile bytes to test malformed-tail fail-closed behavior.
    pub fn append_raw_tail_for_test(
        &self,
        opened: &mut OpenedAppendAuthoritySession,
        bytes: &[u8],
    ) -> Result<(), AppendAuthorityError> {
        self.validate_writer_lock(opened)?;
        validate_regular_file(&log_path(&opened.session), "aliased-append-log-path")?;
        let mut file = OpenOptions::new()
            .append(true)
            .open(log_path(&opened.session))
            .map_err(|error| io_error("open-append-log", error))?;
        validate_opened_authority_leaf(&file, "canonical append log")?;
        file.write_all(bytes)
            .map_err(|error| io_error("write-hostile-tail", error))?;
        file.sync_all()
            .map_err(|error| io_error("sync-hostile-tail", error))
    }

    /// Harness hook: model an unsupported newer on-disk format without changing state.
    pub fn set_format_version_for_test(
        &self,
        opened: &mut OpenedAppendAuthoritySession,
        format_version: u32,
    ) -> Result<(), AppendAuthorityError> {
        self.validate_writer_lock(opened)?;
        let mut manifest = read_manifest(&opened.session)?;
        manifest.format_version = format_version;
        write_manifest(&opened.session, &manifest)
    }

    /// Harness hook: create a temporary, non-authoritative artifact for cleanup planning.
    pub fn write_temporary_artifact_for_test(
        &self,
        opened: &mut OpenedAppendAuthoritySession,
        name: &str,
    ) -> Result<(), AppendAuthorityError> {
        self.validate_writer_lock(opened)?;
        validate_temporary_name(name)?;
        fs::write(
            opened.session.root.join("temporary").join(name),
            b"temporary",
        )
        .map_err(|error| io_error("write-temporary", error))
    }

    /// Harness hook: prove canonical loss is not repaired from temporary or derived data.
    pub fn remove_canonical_log_for_test(
        &self,
        opened: &mut OpenedAppendAuthoritySession,
    ) -> Result<(), AppendAuthorityError> {
        self.validate_writer_lock(opened)?;
        fs::remove_file(log_path(&opened.session))
            .map_err(|error| io_error("remove-canonical-log", error))
    }

    /// Harness hook: modifies a committed state payload without changing its commit binding.
    pub fn tamper_committed_state_for_test(
        &self,
        opened: &mut OpenedAppendAuthoritySession,
        from: &str,
        to: &str,
    ) -> Result<(), AppendAuthorityError> {
        self.validate_writer_lock(opened)?;
        let path = log_path(&opened.session);
        let bytes = fs::read(&path).map_err(|error| io_error("read-append-log", error))?;
        let text = String::from_utf8(bytes)
            .map_err(|error| AppendAuthorityError::new("append-log-not-utf8", error.to_string()))?;
        let updated = text.replacen(from, to, 1);
        if updated == text {
            return Err(AppendAuthorityError::new(
                "tamper-source-not-found",
                "requested committed payload text was not found",
            ));
        }
        fs::write(path, updated).map_err(|error| io_error("tamper-append-log", error))
    }

    fn append_committed_state(
        &self,
        session: &AppendAuthoritySession,
        committed_sequence: u64,
        current_record_count: usize,
        state: &CurrentContractState,
        manifest: &AppendManifest,
        require_manifest_checkpoint: bool,
    ) -> Result<DurableAppendAck, AppendAuthorityError> {
        validate_record_capacity(current_record_count)?;
        let sequence = committed_sequence.checked_add(1).ok_or_else(|| {
            AppendAuthorityError::new("append-sequence-overflow", "append sequence overflow")
        })?;
        let canonical_state = canonicalized_state(state.clone());
        let fingerprint = canonical_fingerprint(&canonical_state.canonical_projection());
        let state_record = serialize_append_record(&AppendRecord::State {
            sequence,
            state: Box::new(canonical_state.clone()),
        })?;
        let commit_record = serialize_append_record(&AppendRecord::Commit {
            sequence,
            canonical_fingerprint: fingerprint.clone(),
        })?;
        append_serialized_record(session, &state_record)?;
        append_serialized_record(session, &commit_record)?;
        let mut next_manifest = manifest.clone();
        next_manifest.committed_sequence = sequence;
        let checkpoint_status = match write_manifest(session, &next_manifest) {
            Ok(()) => AppendCheckpointStatus::Current,
            Err(error) if !require_manifest_checkpoint => AppendCheckpointStatus::Behind {
                last_error_code: error.code,
            },
            Err(error) => return Err(error),
        };
        Ok(DurableAppendAck {
            committed_sequence: sequence,
            canonical_fingerprint: fingerprint,
            checkpoint_status,
        })
    }

    fn acquire_writer_lock(
        &self,
        session: &AppendAuthoritySession,
    ) -> Result<(File, String), AppendAuthorityError> {
        validate_optional_regular_file(&writer_lock_path(session), "aliased-writer-lock-path")?;
        let token = format!("append-authoritative-writer:{}", uuid::Uuid::new_v4());
        let lock = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(writer_lock_path(session))
            .map_err(|error| io_error("open-writer-lock", error))?;
        validate_opened_authority_leaf(&lock, "authoritative writer lock")?;
        lock.try_lock().map_err(|error| match error {
            std::fs::TryLockError::WouldBlock => AppendAuthorityError::new(
                "writer-already-open",
                "candidate session already has an authoritative writer",
            ),
            std::fs::TryLockError::Error(error) => io_error("acquire-writer-lock", error),
        })?;
        Ok((lock, token))
    }

    fn validate_writer_lock(
        &self,
        opened: &OpenedAppendAuthoritySession,
    ) -> Result<(), AppendAuthorityError> {
        if opened.adapter_identity != self.adapter_identity {
            return Err(AppendAuthorityError::new(
                "foreign-writer-handle",
                "writer handle belongs to a different adapter instance",
            ));
        }
        self.validate_session_layout(&opened.session)?;
        let Some(token) = opened.writer_token.as_deref() else {
            return Err(AppendAuthorityError::new(
                "not-authoritative-writer",
                "writable handle lacks a writer lock",
            ));
        };
        if opened.writer_guard.is_some() && !token.is_empty() {
            Ok(())
        } else {
            Err(AppendAuthorityError::new(
                "writer-lock-token-mismatch",
                "writable handle no longer owns the candidate writer lock",
            ))
        }
    }

    fn validate_session_layout(
        &self,
        session: &AppendAuthoritySession,
    ) -> Result<(), AppendAuthorityError> {
        validate_session_id(&session.session_id)?;
        let expected_root = self.session_root(&session.session_id);
        if session.root != expected_root {
            return Err(AppendAuthorityError::new(
                "foreign-session-handle",
                "session handle does not belong to this adapter root",
            ));
        }
        let metadata = fs::symlink_metadata(&expected_root)
            .map_err(|error| io_error("stat-session-root", error))?;
        if is_static_filesystem_alias(&metadata) {
            return Err(AppendAuthorityError::new(
                "aliased-session-path",
                "session root must not be a filesystem alias",
            ));
        }
        if !metadata.is_dir() {
            return Err(AppendAuthorityError::new(
                "session-path-not-directory",
                "session path is not a directory",
            ));
        }
        let canonical = fs::canonicalize(&expected_root)
            .map_err(|error| io_error("canonicalize-session-root", error))?;
        if canonical != expected_root || !canonical.starts_with(&self.storage_root) {
            return Err(AppendAuthorityError::new(
                "aliased-session-path",
                "session root escapes the canonical adapter root",
            ));
        }
        if creation_marker_path(session).exists() {
            return Err(AppendAuthorityError::new(
                "incomplete-session-creation",
                "candidate session creation was not published",
            ));
        }
        validate_regular_file(&manifest_path(session), "aliased-manifest-path")?;
        validate_regular_file(&log_path(session), "aliased-append-log-path")?;
        validate_optional_regular_file(&writer_lock_path(session), "aliased-writer-lock-path")?;
        let temporary = session.root.join("temporary");
        let temporary_metadata = fs::symlink_metadata(&temporary)
            .map_err(|error| io_error("stat-temporary-root", error))?;
        if is_static_filesystem_alias(&temporary_metadata) || !temporary_metadata.is_dir() {
            return Err(AppendAuthorityError::new(
                "aliased-temporary-path",
                "temporary path must be an owned directory",
            ));
        }
        Ok(())
    }

    fn session_root(&self, session_id: &str) -> PathBuf {
        self.storage_root.join(physical_storage_key(session_id))
    }
}

struct ReplayResult {
    committed_sequence: u64,
    tail_status: AppendTailStatus,
    state: CurrentContractState,
    canonical_fingerprints: BTreeSet<String>,
    committed_byte_boundary: u64,
    record_count: usize,
}

fn replay(session: &AppendAuthoritySession) -> Result<ReplayResult, AppendAuthorityError> {
    let file = File::open(log_path(session)).map_err(|error| io_error("open-append-log", error))?;
    validate_opened_authority_leaf(&file, "canonical append log")?;
    let file_len = file
        .metadata()
        .map_err(|error| io_error("stat-append-log", error))?
        .len();
    let mut reader = BufReader::new(file);
    let mut line = Vec::new();
    let mut expected_sequence = 1_u64;
    let mut record_count = 0_usize;
    let mut pending: Option<(u64, CurrentContractState)> = None;
    let mut committed: Option<CurrentContractState> = None;
    let mut fingerprints = BTreeSet::new();
    let mut byte_offset = 0_u64;
    let mut committed_byte_boundary = 0_u64;
    loop {
        let (read, terminated) = read_bounded_record(&mut reader, &mut line)?;
        if !read {
            break;
        }
        if !terminated {
            break;
        }
        byte_offset += line.len() as u64;
        record_count += 1;
        if record_count > MAX_RECORD_COUNT {
            return Err(AppendAuthorityError::new(
                "record-count-exceeded",
                "append record count exceeds bound",
            ));
        }
        let record: AppendRecord = serde_json::from_slice(&line).map_err(|error| {
            AppendAuthorityError::new("malformed-authoritative-record", error.to_string())
        })?;
        match record {
            AppendRecord::State { sequence, state } => {
                if pending.is_some() || sequence != expected_sequence {
                    return Err(AppendAuthorityError::new(
                        "invalid-append-order",
                        "state record is out of authoritative order",
                    ));
                }
                let state = canonicalized_state(*state);
                validate_state(&state)?;
                pending = Some((sequence, state));
            }
            AppendRecord::Commit {
                sequence,
                canonical_fingerprint: persisted_fingerprint,
            } => {
                let Some((pending_sequence, state)) = pending.take() else {
                    return Err(AppendAuthorityError::new(
                        "commit-without-state",
                        "commit has no preceding state record",
                    ));
                };
                if sequence != pending_sequence {
                    return Err(AppendAuthorityError::new(
                        "commit-sequence-mismatch",
                        "commit does not acknowledge its state record",
                    ));
                }
                let fingerprint = canonical_fingerprint(&state.canonical_projection());
                if persisted_fingerprint != fingerprint {
                    return Err(AppendAuthorityError::new(
                        "commit-fingerprint-mismatch",
                        "commit does not bind the preceding canonical state",
                    ));
                }
                if !fingerprints.insert(fingerprint) {
                    return Err(AppendAuthorityError::new(
                        "duplicate-canonical-identity",
                        "committed append records repeat canonical identity",
                    ));
                }
                committed = Some(state);
                expected_sequence += 1;
                committed_byte_boundary = byte_offset;
            }
        }
    }
    let Some(state) = committed else {
        return Err(AppendAuthorityError::new(
            "missing-committed-canonical-state",
            "no committed canonical state exists",
        ));
    };
    let committed_sequence = expected_sequence - 1;
    let tail_status = if let Some((sequence, _)) = pending {
        AppendTailStatus::IncompleteUncommitted { sequence }
    } else if byte_offset < file_len {
        AppendTailStatus::IncompleteUncommitted {
            sequence: expected_sequence,
        }
    } else {
        AppendTailStatus::Clean
    };
    Ok(ReplayResult {
        committed_sequence,
        tail_status,
        state,
        canonical_fingerprints: fingerprints,
        committed_byte_boundary,
        record_count,
    })
}

fn recover_committed_prefix(
    session: &AppendAuthoritySession,
    replayed: &ReplayResult,
) -> Result<(), AppendAuthorityError> {
    if replayed.tail_status == AppendTailStatus::Clean {
        return Ok(());
    }
    let file = OpenOptions::new()
        .write(true)
        .open(log_path(session))
        .map_err(|error| io_error("open-recovery-log", error))?;
    validate_opened_authority_leaf(&file, "canonical append log")?;
    file.set_len(replayed.committed_byte_boundary)
        .map_err(|error| io_error("truncate-incomplete-tail", error))?;
    file.sync_all()
        .map_err(|error| io_error("sync-recovered-prefix", error))
}

fn canonicalized_state(mut state: CurrentContractState) -> CurrentContractState {
    finalize_derived_fields(&mut state);
    state.normalize()
}

fn validate_state(state: &CurrentContractState) -> Result<(), AppendAuthorityError> {
    let result = CurrentContractOracle::validate(&canonicalized_state(state.clone()));
    if result.passed {
        Ok(())
    } else {
        Err(AppendAuthorityError::new(
            "current-contract-oracle-rejected",
            format!("{} oracle violations", result.violations.len()),
        ))
    }
}

fn validate_session_id(session_id: &str) -> Result<(), AppendAuthorityError> {
    if session_id.is_empty()
        || session_id.len() > 128
        || session_id.contains('/')
        || session_id.contains('\\')
        || session_id == "."
        || session_id == ".."
    {
        return Err(AppendAuthorityError::new(
            "unsafe-session-id",
            "session id escapes candidate root",
        ));
    }
    Ok(())
}

fn physical_storage_key(session_id: &str) -> String {
    let digest = Sha256::digest(session_id.as_bytes());
    let mut encoded = String::with_capacity("vp-session-v1-".len() + (digest.len() * 2));
    encoded.push_str("vp-session-v1-");
    for byte in digest {
        use std::fmt::Write as _;
        write!(&mut encoded, "{byte:02x}").expect("writing to String cannot fail");
    }
    encoded
}

fn validate_temporary_name(name: &str) -> Result<(), AppendAuthorityError> {
    if name.is_empty() || name.contains('/') || name.contains('\\') || name == "." || name == ".." {
        return Err(AppendAuthorityError::new(
            "unsafe-temporary-name",
            "temporary artifact name escapes candidate root",
        ));
    }
    Ok(())
}

fn manifest_path(session: &AppendAuthoritySession) -> PathBuf {
    session.root.join("manifest.json")
}

fn log_path(session: &AppendAuthoritySession) -> PathBuf {
    session.root.join("canonical.append.jsonl")
}

fn writer_lock_path(session: &AppendAuthoritySession) -> PathBuf {
    session.root.join("authoritative.writer.lock")
}

fn creation_marker_path(session: &AppendAuthoritySession) -> PathBuf {
    session.root.join(CREATION_MARKER_NAME)
}

fn read_manifest(session: &AppendAuthoritySession) -> Result<AppendManifest, AppendAuthorityError> {
    let path = manifest_path(session);
    validate_regular_file(&path, "aliased-manifest-path")?;
    let mut file = File::open(&path).map_err(|error| io_error("read-manifest", error))?;
    validate_opened_authority_leaf(&file, "manifest")?;
    let mut bytes = Vec::with_capacity(MAX_MANIFEST_BYTES.min(8 * 1024));
    Read::by_ref(&mut file)
        .take((MAX_MANIFEST_BYTES as u64) + 1)
        .read_to_end(&mut bytes)
        .map_err(|error| io_error("read-manifest", error))?;
    if bytes.len() > MAX_MANIFEST_BYTES {
        return Err(AppendAuthorityError::new(
            "manifest-too-large",
            "manifest exceeds the bounded input size",
        ));
    }
    serde_json::from_slice(&bytes)
        .map_err(|error| AppendAuthorityError::new("malformed-manifest", error.to_string()))
}

fn write_manifest(
    session: &AppendAuthoritySession,
    manifest: &AppendManifest,
) -> Result<(), AppendAuthorityError> {
    let path = manifest_path(session);
    let temporary = path.with_extension("tmp");
    validate_optional_regular_file(&path, "aliased-manifest-path")?;
    match fs::symlink_metadata(&temporary) {
        Ok(metadata) if is_static_filesystem_alias(&metadata) => {
            return Err(AppendAuthorityError::new(
                "aliased-manifest-temporary-path",
                "candidate authority path must not be a filesystem alias",
            ));
        }
        Ok(metadata) if !metadata.is_file() => {
            return Err(AppendAuthorityError::new(
                "canonical-path-not-regular-file",
                "candidate authority path is not a regular file",
            ));
        }
        Ok(_) => {
            return Err(AppendAuthorityError::new(
                "manifest-temporary-exists",
                "manifest checkpoint temporary already exists",
            ));
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(io_error("stat-candidate-path", error)),
    }
    let mut temporary_file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temporary)
        .map_err(|error| io_error("create-manifest-temporary", error))?;
    validate_opened_authority_leaf(&temporary_file, "manifest checkpoint temporary")?;
    let bytes = serde_json::to_vec(manifest)
        .map_err(|error| AppendAuthorityError::new("serialize-manifest", error.to_string()))?;
    temporary_file
        .write_all(&bytes)
        .map_err(|error| io_error("write-manifest", error))?;
    temporary_file
        .sync_all()
        .map_err(|error| io_error("sync-manifest-temporary", error))?;
    if path.exists() {
        let destination =
            File::open(&path).map_err(|error| io_error("open-manifest-destination", error))?;
        validate_opened_authority_leaf(&destination, "manifest")?;
    }
    fs::rename(&temporary, &path).map_err(|error| io_error("commit-manifest", error))
}

fn repair_manifest_checkpoint(
    session: &AppendAuthoritySession,
    manifest: &AppendManifest,
    committed_sequence: u64,
) -> Result<(), AppendAuthorityError> {
    let mut repaired = manifest.clone();
    repaired.committed_sequence = committed_sequence;
    write_manifest(session, &repaired)
}

fn validate_manifest(
    session: &AppendAuthoritySession,
    manifest: &AppendManifest,
    mode: AppendOpenMode,
) -> Result<(), AppendAuthorityError> {
    if manifest.session_id != session.session_id {
        return Err(AppendAuthorityError::new(
            "manifest-session-identity-mismatch",
            "manifest session identity differs from reference",
        ));
    }
    if manifest.format_version > APPEND_AUTHORITATIVE_FORMAT_VERSION
        && mode == AppendOpenMode::Writable
    {
        return Err(AppendAuthorityError::new(
            "unsupported-newer-format",
            "unknown newer format cannot open writable",
        ));
    }
    if manifest.format_version != APPEND_AUTHORITATIVE_FORMAT_VERSION {
        return Err(AppendAuthorityError::new(
            "unsupported-format",
            "candidate cannot safely interpret this format",
        ));
    }
    Ok(())
}

fn append_record(
    session: &AppendAuthoritySession,
    record: &AppendRecord,
) -> Result<(), AppendAuthorityError> {
    let bytes = serialize_append_record(record)?;
    append_serialized_record(session, &bytes)
}

fn serialize_append_record(record: &AppendRecord) -> Result<Vec<u8>, AppendAuthorityError> {
    let mut bounded = BoundedRecordBuffer::new(MAX_RECORD_BYTES - 1);
    if let Err(error) = serde_json::to_writer(&mut bounded, record) {
        if bounded.overflowed {
            return Err(AppendAuthorityError::new(
                "outbound-record-too-large",
                "append record exceeds the replayable record bound",
            ));
        }
        return Err(AppendAuthorityError::new(
            "serialize-append-record",
            error.to_string(),
        ));
    }
    if bounded.overflowed {
        return Err(AppendAuthorityError::new(
            "outbound-record-too-large",
            "append record exceeds the replayable record bound",
        ));
    }
    let mut bytes = bounded.bytes;
    bytes.push(b'\n');
    Ok(bytes)
}

struct BoundedRecordBuffer {
    bytes: Vec<u8>,
    limit: usize,
    overflowed: bool,
}

impl BoundedRecordBuffer {
    fn new(limit: usize) -> Self {
        Self {
            bytes: Vec::with_capacity(limit.min(8 * 1024)),
            limit,
            overflowed: false,
        }
    }
}

impl Write for BoundedRecordBuffer {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        if self
            .bytes
            .len()
            .checked_add(bytes.len())
            .is_none_or(|size| size > self.limit)
        {
            self.overflowed = true;
            return Err(std::io::Error::other("bounded append record exceeded"));
        }
        self.bytes.extend_from_slice(bytes);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

fn validate_record_capacity(current_record_count: usize) -> Result<(), AppendAuthorityError> {
    if current_record_count
        .checked_add(2)
        .is_none_or(|count| count > MAX_RECORD_COUNT)
    {
        Err(AppendAuthorityError::new(
            "record-count-exhausted",
            "append transition would exceed the bounded record count",
        ))
    } else {
        Ok(())
    }
}

fn append_serialized_record(
    session: &AppendAuthoritySession,
    bytes: &[u8],
) -> Result<(), AppendAuthorityError> {
    validate_regular_file(&log_path(session), "aliased-append-log-path")?;
    let mut file = OpenOptions::new()
        .append(true)
        .open(log_path(session))
        .map_err(|error| io_error("open-append-log", error))?;
    validate_opened_authority_leaf(&file, "canonical append log")?;
    file.write_all(bytes)
        .map_err(|error| io_error("write-append-record", error))?;
    file.sync_all()
        .map_err(|error| io_error("sync-append-record", error))
}

fn validate_opened_authority_leaf(
    file: &File,
    leaf_category: &'static str,
) -> Result<(), AppendAuthorityError> {
    let metadata = file
        .metadata()
        .map_err(|error| io_error("stat-opened-authority-leaf", error))?;
    if metadata.file_type().is_symlink() {
        return Err(AppendAuthorityError::new(
            "authority-leaf-not-regular",
            format!("{leaf_category} must be a regular file"),
        ));
    }
    if !metadata.is_file() {
        return Err(AppendAuthorityError::new(
            "authority-leaf-not-regular",
            format!("{leaf_category} is not a regular file"),
        ));
    }
    let link_count = authority_hard_link_count(file, &metadata)?;
    if link_count != 1 {
        return Err(AppendAuthorityError::new(
            "authority-leaf-hard-linked",
            format!("{leaf_category} must have exactly one hard link"),
        ));
    }
    Ok(())
}

fn authority_hard_link_count(
    _file: &File,
    _metadata: &fs::Metadata,
) -> Result<u64, AppendAuthorityError> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        Ok(_metadata.nlink())
    }
    #[cfg(windows)]
    {
        authority_hard_link_count_windows(_file)
    }
    #[cfg(not(any(unix, windows)))]
    {
        Err(AppendAuthorityError::new(
            "authority-link-count-unavailable",
            "authority leaf link count cannot be verified on this platform",
        ))
    }
}

#[cfg(windows)]
fn authority_hard_link_count_windows(file: &File) -> Result<u64, AppendAuthorityError> {
    use std::os::windows::io::AsRawHandle;
    use windows_sys::Win32::Foundation::HANDLE;
    use windows_sys::Win32::Storage::FileSystem::{
        BY_HANDLE_FILE_INFORMATION, GetFileInformationByHandle,
    };

    let mut information = std::mem::MaybeUninit::<BY_HANDLE_FILE_INFORMATION>::zeroed();
    // SAFETY: `file` is a live, borrowed `std::fs::File` for the exact authority
    // operation being validated. `AsRawHandle` yields its valid Windows HANDLE,
    // and `information` points to writable storage of the documented ABI type.
    let succeeded = unsafe {
        GetFileInformationByHandle(file.as_raw_handle() as HANDLE, information.as_mut_ptr())
    };
    if succeeded == 0 {
        return Err(AppendAuthorityError::new(
            "authority-link-count-unavailable",
            std::io::Error::last_os_error().to_string(),
        ));
    }
    // SAFETY: a successful GetFileInformationByHandle initializes every field.
    Ok(unsafe { information.assume_init() }.nNumberOfLinks as u64)
}

fn validate_regular_file(
    path: &std::path::Path,
    alias_code: &'static str,
) -> Result<(), AppendAuthorityError> {
    let metadata =
        fs::symlink_metadata(path).map_err(|error| io_error("stat-candidate-path", error))?;
    if is_static_filesystem_alias(&metadata) {
        return Err(AppendAuthorityError::new(
            alias_code,
            "candidate authority path must not be a filesystem alias",
        ));
    }
    if !metadata.is_file() {
        return Err(AppendAuthorityError::new(
            "canonical-path-not-regular-file",
            "candidate authority path is not a regular file",
        ));
    }
    Ok(())
}

fn validate_optional_regular_file(
    path: &std::path::Path,
    alias_code: &'static str,
) -> Result<(), AppendAuthorityError> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if is_static_filesystem_alias(&metadata) => Err(AppendAuthorityError::new(
            alias_code,
            "candidate authority path must not be a filesystem alias",
        )),
        Ok(metadata) if !metadata.is_file() => Err(AppendAuthorityError::new(
            "canonical-path-not-regular-file",
            "candidate authority path is not a regular file",
        )),
        Ok(_) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(io_error("stat-candidate-path", error)),
    }
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

fn read_bounded_record(
    reader: &mut BufReader<File>,
    line: &mut Vec<u8>,
) -> Result<(bool, bool), AppendAuthorityError> {
    line.clear();
    loop {
        let available = reader
            .fill_buf()
            .map_err(|error| io_error("read-append-log", error))?;
        if available.is_empty() {
            return Ok((!line.is_empty(), false));
        }
        let take = available
            .iter()
            .position(|byte| *byte == b'\n')
            .map(|position| position + 1)
            .unwrap_or(available.len());
        if line.len() + take > MAX_RECORD_BYTES {
            return Err(AppendAuthorityError::new(
                "record-too-large",
                "append record exceeds bound before allocation",
            ));
        }
        line.extend_from_slice(&available[..take]);
        reader.consume(take);
        if line.last() == Some(&b'\n') {
            return Ok((true, true));
        }
    }
}

fn io_error(code: &'static str, error: std::io::Error) -> AppendAuthorityError {
    AppendAuthorityError::new(code, error.to_string())
}
