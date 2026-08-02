//! Bounded, spike-only append authority for current-contract v3 fixtures.
//!
//! This is a candidate test adapter, not a selected persistence mechanism or a
//! production session store.  Its append records contain complete semantic
//! transitions so the current-contract oracle remains the sole semantic oracle.

use std::collections::BTreeSet;
use std::fs::{self, File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

use serde::{Deserialize, Serialize};

use super::derivation::finalize_derived_fields;
use super::model::CurrentContractState;
use super::oracle::{CurrentContractOracle, canonical_fingerprint};

pub const APPEND_AUTHORITATIVE_CANDIDATE_ID: &str =
    "current-contract-append-authoritative-candidate";
pub const APPEND_AUTHORITATIVE_CANDIDATE_VERSION: &str = "01B-1";
pub const APPEND_AUTHORITATIVE_FORMAT_VERSION: u32 = 1;
const MAX_RECORD_BYTES: usize = 2 * 1024 * 1024;
const MAX_RECORD_COUNT: usize = 4_096;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppendAuthoritySession {
    session_id: String,
    root: PathBuf,
}

impl AppendAuthoritySession {
    pub fn session_id(&self) -> &str {
        &self.session_id
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppendTailStatus {
    Clean,
    IncompleteUncommitted { sequence: u64 },
}

#[derive(Debug, Clone)]
pub struct OpenedAppendAuthoritySession {
    pub session: AppendAuthoritySession,
    pub committed_sequence: u64,
    pub tail_status: AppendTailStatus,
    open_mode: AppendOpenMode,
    writer_token: Option<String>,
    normalized_state: CurrentContractState,
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
    next_writer_token: AtomicU64,
}

impl AppendAuthoritativeCandidateAdapter {
    pub fn new(storage_root: impl Into<PathBuf>) -> Result<Self, AppendAuthorityError> {
        let storage_root = storage_root.into();
        fs::create_dir_all(&storage_root)
            .map_err(|error| io_error("create-storage-root", error))?;
        Ok(Self {
            storage_root,
            next_writer_token: AtomicU64::new(0),
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
        let root = self.storage_root.join(&state.session_id);
        if root.exists() {
            return Err(AppendAuthorityError::new(
                "session-already-exists",
                "candidate session already exists",
            ));
        }
        fs::create_dir_all(root.join("temporary"))
            .map_err(|error| io_error("create-session-root", error))?;
        let session = AppendAuthoritySession {
            session_id: state.session_id.clone(),
            root,
        };
        write_manifest(
            &session,
            &AppendManifest {
                format_version: APPEND_AUTHORITATIVE_FORMAT_VERSION,
                session_id: session.session_id.clone(),
                committed_sequence: 0,
            },
        )?;
        File::create(log_path(&session)).map_err(|error| io_error("create-append-log", error))?;
        self.append_committed_state(&session, 0, state)?;
        Ok(session)
    }

    pub fn open(
        &self,
        session: &AppendAuthoritySession,
        mode: AppendOpenMode,
    ) -> Result<OpenedAppendAuthoritySession, AppendAuthorityError> {
        let manifest = read_manifest(session)?;
        validate_manifest(session, &manifest, mode)?;
        let replayed = replay(session)?;
        if manifest.committed_sequence > replayed.committed_sequence {
            return Err(AppendAuthorityError::new(
                "manifest-ahead-of-append-authority",
                "manifest claims an uncommitted authoritative boundary",
            ));
        }
        let writer_token = if mode == AppendOpenMode::Writable {
            Some(self.acquire_writer_lock(session)?)
        } else {
            None
        };
        Ok(OpenedAppendAuthoritySession {
            session: session.clone(),
            committed_sequence: replayed.committed_sequence,
            tail_status: replayed.tail_status,
            open_mode: mode,
            writer_token,
            normalized_state: replayed.state,
        })
    }

    pub fn append_authoritative_transition(
        &self,
        opened: &OpenedAppendAuthoritySession,
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
        if opened.tail_status != AppendTailStatus::Clean {
            return Err(AppendAuthorityError::new(
                "incomplete-authoritative-tail",
                "an incomplete tail must be recovered or rejected before another write",
            ));
        }
        let manifest = read_manifest(&opened.session)?;
        validate_manifest(&opened.session, &manifest, AppendOpenMode::Writable)?;
        let current = replay(&opened.session)?;
        if manifest.committed_sequence > current.committed_sequence {
            return Err(AppendAuthorityError::new(
                "manifest-ahead-of-append-authority",
                "manifest claims an uncommitted authoritative boundary",
            ));
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
        self.append_committed_state(
            &opened.session,
            current.committed_sequence,
            &canonical_next_state,
        )
    }

    pub fn close(&self, opened: OpenedAppendAuthoritySession) -> Result<(), AppendAuthorityError> {
        let Some(token) = opened.writer_token else {
            return Ok(());
        };
        let path = writer_lock_path(&opened.session);
        let actual =
            fs::read_to_string(&path).map_err(|error| io_error("read-writer-lock", error))?;
        if actual != token {
            return Err(AppendAuthorityError::new(
                "writer-lock-token-mismatch",
                "writer lock was replaced before close",
            ));
        }
        fs::remove_file(path).map_err(|error| io_error("release-writer-lock", error))?;
        Ok(())
    }

    pub fn duplicate(
        &self,
        source: &OpenedAppendAuthoritySession,
        new_session_id: impl Into<String>,
    ) -> Result<AppendAuthoritySession, AppendAuthorityError> {
        let new_session_id = new_session_id.into();
        validate_session_id(&new_session_id)?;
        let mut copied = source.normalized_state.clone();
        copied.duplicated_from_session_id = Some(copied.session_id.clone());
        copied.session_id = new_session_id;
        self.create(&copied)
    }

    pub fn plan_cleanup(
        &self,
        session: &AppendAuthoritySession,
    ) -> Result<CleanupPlan, AppendAuthorityError> {
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

    /// Harness hook: append a valid state record without its commit acknowledgement.
    pub fn append_incomplete_tail_for_test(
        &self,
        session: &AppendAuthoritySession,
        state: &CurrentContractState,
    ) -> Result<(), AppendAuthorityError> {
        validate_state(state)?;
        let sequence = read_manifest(session)?.committed_sequence + 1;
        append_record(
            session,
            &AppendRecord::State {
                sequence,
                state: Box::new(canonicalized_state(state.clone())),
            },
        )
    }

    /// Harness hook: append hostile bytes to test malformed-tail fail-closed behavior.
    pub fn append_raw_tail_for_test(
        &self,
        session: &AppendAuthoritySession,
        bytes: &[u8],
    ) -> Result<(), AppendAuthorityError> {
        let mut file = OpenOptions::new()
            .append(true)
            .open(log_path(session))
            .map_err(|error| io_error("open-append-log", error))?;
        file.write_all(bytes)
            .map_err(|error| io_error("write-hostile-tail", error))?;
        file.sync_all()
            .map_err(|error| io_error("sync-hostile-tail", error))
    }

    /// Harness hook: model an unsupported newer on-disk format without changing state.
    pub fn set_format_version_for_test(
        &self,
        session: &AppendAuthoritySession,
        format_version: u32,
    ) -> Result<(), AppendAuthorityError> {
        let mut manifest = read_manifest(session)?;
        manifest.format_version = format_version;
        write_manifest(session, &manifest)
    }

    /// Harness hook: create a temporary, non-authoritative artifact for cleanup planning.
    pub fn write_temporary_artifact_for_test(
        &self,
        session: &AppendAuthoritySession,
        name: &str,
    ) -> Result<(), AppendAuthorityError> {
        validate_temporary_name(name)?;
        fs::write(session.root.join("temporary").join(name), b"temporary")
            .map_err(|error| io_error("write-temporary", error))
    }

    /// Harness hook: prove canonical loss is not repaired from temporary or derived data.
    pub fn remove_canonical_log_for_test(
        &self,
        session: &AppendAuthoritySession,
    ) -> Result<(), AppendAuthorityError> {
        fs::remove_file(log_path(session)).map_err(|error| io_error("remove-canonical-log", error))
    }

    fn append_committed_state(
        &self,
        session: &AppendAuthoritySession,
        committed_sequence: u64,
        state: &CurrentContractState,
    ) -> Result<DurableAppendAck, AppendAuthorityError> {
        let sequence = committed_sequence + 1;
        let canonical_state = canonicalized_state(state.clone());
        append_record(
            session,
            &AppendRecord::State {
                sequence,
                state: Box::new(canonical_state.clone()),
            },
        )?;
        append_record(session, &AppendRecord::Commit { sequence })?;
        let mut manifest = read_manifest(session)?;
        manifest.committed_sequence = sequence;
        write_manifest(session, &manifest)?;
        Ok(DurableAppendAck {
            committed_sequence: sequence,
            canonical_fingerprint: canonical_fingerprint(&canonical_state.canonical_projection()),
        })
    }

    fn acquire_writer_lock(
        &self,
        session: &AppendAuthoritySession,
    ) -> Result<String, AppendAuthorityError> {
        let token = format!(
            "append-authoritative-writer:{}",
            self.next_writer_token.fetch_add(1, Ordering::Relaxed) + 1
        );
        let mut lock = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(writer_lock_path(session))
            .map_err(|error| {
                if error.kind() == std::io::ErrorKind::AlreadyExists {
                    AppendAuthorityError::new(
                        "writer-already-open",
                        "candidate session already has an authoritative writer",
                    )
                } else {
                    io_error("acquire-writer-lock", error)
                }
            })?;
        lock.write_all(token.as_bytes())
            .map_err(|error| io_error("write-writer-lock", error))?;
        lock.sync_all()
            .map_err(|error| io_error("sync-writer-lock", error))?;
        Ok(token)
    }

    fn validate_writer_lock(
        &self,
        opened: &OpenedAppendAuthoritySession,
    ) -> Result<(), AppendAuthorityError> {
        let Some(token) = opened.writer_token.as_deref() else {
            return Err(AppendAuthorityError::new(
                "not-authoritative-writer",
                "writable handle lacks a writer lock",
            ));
        };
        let actual = fs::read_to_string(writer_lock_path(&opened.session))
            .map_err(|error| io_error("read-writer-lock", error))?;
        if actual == token {
            Ok(())
        } else {
            Err(AppendAuthorityError::new(
                "writer-lock-token-mismatch",
                "writable handle no longer owns the candidate writer lock",
            ))
        }
    }
}

struct ReplayResult {
    committed_sequence: u64,
    tail_status: AppendTailStatus,
    state: CurrentContractState,
    canonical_fingerprints: BTreeSet<String>,
}

fn replay(session: &AppendAuthoritySession) -> Result<ReplayResult, AppendAuthorityError> {
    let file = File::open(log_path(session)).map_err(|error| io_error("open-append-log", error))?;
    let mut reader = BufReader::new(file);
    let mut line = Vec::new();
    let mut expected_sequence = 1_u64;
    let mut record_count = 0_usize;
    let mut pending: Option<(u64, CurrentContractState)> = None;
    let mut committed: Option<CurrentContractState> = None;
    let mut fingerprints = BTreeSet::new();
    loop {
        let read = read_bounded_record(&mut reader, &mut line)?;
        if !read {
            break;
        }
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
            AppendRecord::Commit { sequence } => {
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
                if !fingerprints.insert(fingerprint) {
                    return Err(AppendAuthorityError::new(
                        "duplicate-canonical-identity",
                        "committed append records repeat canonical identity",
                    ));
                }
                committed = Some(state);
                expected_sequence += 1;
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
    let tail_status = pending
        .map(|(sequence, _)| AppendTailStatus::IncompleteUncommitted { sequence })
        .unwrap_or(AppendTailStatus::Clean);
    Ok(ReplayResult {
        committed_sequence,
        tail_status,
        state,
        canonical_fingerprints: fingerprints,
    })
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

fn read_manifest(session: &AppendAuthoritySession) -> Result<AppendManifest, AppendAuthorityError> {
    let bytes =
        fs::read(manifest_path(session)).map_err(|error| io_error("read-manifest", error))?;
    serde_json::from_slice(&bytes)
        .map_err(|error| AppendAuthorityError::new("malformed-manifest", error.to_string()))
}

fn write_manifest(
    session: &AppendAuthoritySession,
    manifest: &AppendManifest,
) -> Result<(), AppendAuthorityError> {
    let path = manifest_path(session);
    let temporary = path.with_extension("tmp");
    let bytes = serde_json::to_vec(manifest)
        .map_err(|error| AppendAuthorityError::new("serialize-manifest", error.to_string()))?;
    fs::write(&temporary, bytes).map_err(|error| io_error("write-manifest", error))?;
    fs::rename(&temporary, &path).map_err(|error| io_error("commit-manifest", error))
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
    let mut file = OpenOptions::new()
        .append(true)
        .open(log_path(session))
        .map_err(|error| io_error("open-append-log", error))?;
    serde_json::to_writer(&mut file, record)
        .map_err(|error| AppendAuthorityError::new("serialize-append-record", error.to_string()))?;
    file.write_all(b"\n")
        .map_err(|error| io_error("write-append-record", error))?;
    file.sync_all()
        .map_err(|error| io_error("sync-append-record", error))
}

fn read_bounded_record(
    reader: &mut BufReader<File>,
    line: &mut Vec<u8>,
) -> Result<bool, AppendAuthorityError> {
    line.clear();
    loop {
        let available = reader
            .fill_buf()
            .map_err(|error| io_error("read-append-log", error))?;
        if available.is_empty() {
            return Ok(!line.is_empty());
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
            return Ok(true);
        }
    }
}

fn io_error(code: &'static str, error: std::io::Error) -> AppendAuthorityError {
    AppendAuthorityError::new(code, error.to_string())
}
