use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};
use std::fs::{self, File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
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

#[derive(Debug, Clone, Serialize, Deserialize)]
struct BundleManifest {
    format_version: u32,
    generation: u64,
    committed_seq: u64,
    session_id: String,
    duplicated_from_session_id: Option<String>,
    session_format_version: String,
    interpretation_version: String,
    writer_token: Option<String>,
    writer_epoch: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct AppendRecord {
    seq: u64,
    command_kind: String,
    checkpoint_generation: u64,
}

pub struct AppendBundleAdapter {
    storage_root: PathBuf,
    faults: FaultRegistry,
    handles: RefCell<BTreeMap<String, (String, SemanticOpenMode)>>,
    next_handle: RefCell<u64>,
}

impl AppendBundleAdapter {
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

    fn manifest_path(locator: &str) -> PathBuf {
        PathBuf::from(locator).join("manifest.json")
    }

    fn log_path(locator: &str) -> PathBuf {
        PathBuf::from(locator).join("canonical.log")
    }

    fn checkpoint_path(locator: &str, generation: u64) -> PathBuf {
        PathBuf::from(locator)
            .join("checkpoints")
            .join(format!("gen-{generation}"))
            .join("state.json")
    }

    fn derived_index_path(locator: &str) -> PathBuf {
        PathBuf::from(locator).join("derived").join("index.cache")
    }

    fn read_manifest(locator: &str) -> Result<BundleManifest, AdapterError> {
        let bytes = fs::read(Self::manifest_path(locator))
            .map_err(|error| AdapterError::new("filesystem-read-manifest", error.to_string()))?;
        serde_json::from_slice(&bytes)
            .map_err(|error| AdapterError::new("manifest-deserialize-failed", error.to_string()))
    }

    fn write_manifest_atomic(locator: &str, manifest: &BundleManifest) -> Result<(), AdapterError> {
        let path = Self::manifest_path(locator);
        let temp = path.with_extension("tmp");
        let json = serde_json::to_vec_pretty(manifest)
            .map_err(|error| AdapterError::new("manifest-serialize-failed", error.to_string()))?;
        fs::write(&temp, &json).map_err(|error| {
            AdapterError::new("filesystem-write-manifest-temp", error.to_string())
        })?;
        fs::rename(&temp, &path)
            .map_err(|error| AdapterError::new("filesystem-rename-manifest", error.to_string()))?;
        Ok(())
    }

    fn load_checkpoint_state(
        locator: &str,
        generation: u64,
    ) -> Result<NormalizedSemanticState, AdapterError> {
        let bytes = fs::read(Self::checkpoint_path(locator, generation))
            .map_err(|error| AdapterError::new("filesystem-read-checkpoint", error.to_string()))?;
        let state: NormalizedSemanticState = serde_json::from_slice(&bytes).map_err(|error| {
            AdapterError::new("checkpoint-deserialize-failed", error.to_string())
        })?;
        Ok(state.normalize())
    }

    fn write_checkpoint_atomic(
        locator: &str,
        generation: u64,
        state: &NormalizedSemanticState,
    ) -> Result<(), AdapterError> {
        let path = Self::checkpoint_path(locator, generation);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|error| {
                AdapterError::new("filesystem-create-checkpoint-dir", error.to_string())
            })?;
        }
        let temp = path.with_extension("tmp");
        let json = serde_json::to_vec_pretty(&state.clone().normalize())
            .map_err(|error| AdapterError::new("checkpoint-serialize-failed", error.to_string()))?;
        fs::write(&temp, &json).map_err(|error| {
            AdapterError::new("filesystem-write-checkpoint-temp", error.to_string())
        })?;
        fs::rename(&temp, &path).map_err(|error| {
            AdapterError::new("filesystem-rename-checkpoint", error.to_string())
        })?;
        Ok(())
    }

    fn append_log_record(locator: &str, record: &AppendRecord) -> Result<(), AdapterError> {
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(Self::log_path(locator))
            .map_err(|error| AdapterError::new("filesystem-open-log", error.to_string()))?;
        serde_json::to_writer(&mut file, record)
            .map_err(|error| AdapterError::new("log-serialize-failed", error.to_string()))?;
        file.write_all(b"\n")
            .map_err(|error| AdapterError::new("log-write-newline", error.to_string()))?;
        file.sync_all()
            .map_err(|error| AdapterError::new("log-fsync-failed", error.to_string()))?;
        Ok(())
    }

    fn replay_log(
        locator: &str,
        state: NormalizedSemanticState,
    ) -> Result<NormalizedSemanticState, AdapterError> {
        let path = Self::log_path(locator);
        if !path.exists() {
            return Ok(state);
        }
        let file = File::open(path)
            .map_err(|error| AdapterError::new("filesystem-open-log", error.to_string()))?;
        let mut expected_seq = 1_u64;
        for line in BufReader::new(file).lines() {
            let line =
                line.map_err(|error| AdapterError::new("log-read-line", error.to_string()))?;
            if line.trim().is_empty() {
                continue;
            }
            let record: AppendRecord = serde_json::from_str(&line).map_err(|error| {
                AdapterError::new("log-record-deserialize-failed", error.to_string())
            })?;
            if record.command_kind == "truncate-simulated" {
                return Err(AdapterError::new(
                    "canonical-log-corruption",
                    "truncated append log detected during replay",
                ));
            }
            if record.seq != expected_seq {
                return Err(AdapterError::new(
                    "canonical-log-sequence-gap",
                    "append log sequence is not contiguous",
                ));
            }
            expected_seq += 1;
        }
        Ok(state.normalize())
    }

    fn validate_format(
        manifest: &BundleManifest,
        mode: SemanticOpenMode,
    ) -> Result<(), AdapterError> {
        if manifest.format_version > CURRENT_FORMAT_VERSION && mode == SemanticOpenMode::Writable {
            return Err(AdapterError::new(
                "unsupported-newer-format",
                "unknown newer bundle format cannot open writable",
            ));
        }
        if manifest.format_version < CURRENT_FORMAT_VERSION && mode == SemanticOpenMode::Writable {
            return Err(AdapterError::new(
                "unsupported-older-format",
                "writable open requires migration for older bundle format",
            ));
        }
        Ok(())
    }

    fn rebuild_derived_index(
        locator: &str,
        state: &NormalizedSemanticState,
    ) -> Result<(), AdapterError> {
        let derived_dir = PathBuf::from(locator).join("derived");
        fs::create_dir_all(&derived_dir)
            .map_err(|error| AdapterError::new("filesystem-create-derived", error.to_string()))?;
        let marker = format!(
            "derived-index:{}:{}",
            state.review_ledger_events.len(),
            state.analysis_results.len()
        );
        fs::write(Self::derived_index_path(locator), marker)
            .map_err(|error| AdapterError::new("filesystem-write-derived", error.to_string()))?;
        Ok(())
    }

    fn load_openable_state(locator: &str) -> Result<NormalizedSemanticState, AdapterError> {
        let manifest = Self::read_manifest(locator)?;
        let mut state = Self::load_checkpoint_state(locator, manifest.generation)?;
        state = Self::replay_log(locator, state)?;
        Ok(state)
    }
}

impl PersistenceCandidateAdapter for AppendBundleAdapter {
    fn candidate_id(&self) -> &str {
        "append-bundle-log-spike"
    }

    fn candidate_version(&self) -> &str {
        "1"
    }

    fn capabilities(&self) -> CandidateCapabilities {
        CandidateCapabilities {
            optional: BTreeSet::from([OptionalCapability::Compaction]),
            limitations: vec![
                "spike-only append bundle; not production layout".to_string(),
                "destructive historical GC not implemented".to_string(),
            ],
        }
    }

    fn create(&mut self, fixture: &EvidenceFixture) -> Result<EvidenceSessionRef, AdapterError> {
        let state = fixture.normalized_state();
        let session_id = state.session.session_id.clone();
        let locator_path = self.session_dir(&session_id);
        if locator_path.exists() {
            return Err(AdapterError::new(
                "already-created",
                "session directory already exists",
            ));
        }
        fs::create_dir_all(&locator_path)
            .map_err(|error| AdapterError::new("filesystem-create-session", error.to_string()))?;
        fs::create_dir_all(locator_path.join("checkpoints")).map_err(|error| {
            AdapterError::new("filesystem-create-checkpoints", error.to_string())
        })?;
        let locator = locator_path.to_string_lossy().to_string();
        Self::write_checkpoint_atomic(&locator, 0, &state)?;
        Self::rebuild_derived_index(&locator, &state)?;
        let manifest = BundleManifest {
            format_version: CURRENT_FORMAT_VERSION,
            generation: 0,
            committed_seq: 0,
            session_id: session_id.clone(),
            duplicated_from_session_id: None,
            session_format_version: state.session_format_version.clone(),
            interpretation_version: state.interpretation_version.clone(),
            writer_token: None,
            writer_epoch: 0,
        };
        Self::write_manifest_atomic(&locator, &manifest)?;
        File::create(Self::log_path(&locator))
            .map_err(|error| AdapterError::new("filesystem-create-log", error.to_string()))?;
        Ok(EvidenceSessionRef::new(session_id, locator))
    }

    fn open(
        &mut self,
        session: &EvidenceSessionRef,
        mode: SemanticOpenMode,
    ) -> Result<EvidenceSessionHandle, AdapterError> {
        let locator = session.adapter_locator();
        let mut manifest = Self::read_manifest(locator)?;
        Self::validate_format(&manifest, mode)?;
        Self::load_openable_state(locator)?;
        *self.next_handle.borrow_mut() += 1;
        let handle_id = format!("append-handle:{}", self.next_handle.borrow());
        if mode == SemanticOpenMode::Writable {
            if manifest.writer_token.is_some() {
                return Err(AdapterError::new(
                    "writer-already-open",
                    "append bundle permits one authoritative writer",
                ));
            }
            manifest.writer_token = Some(handle_id.clone());
            manifest.writer_epoch += 1;
            Self::write_manifest_atomic(locator, &manifest)?;
        }
        self.handles
            .borrow_mut()
            .insert(handle_id.clone(), (locator.to_string(), mode));
        Ok(EvidenceSessionHandle::new(session.clone(), mode, handle_id))
    }

    fn close(&mut self, handle: &EvidenceSessionHandle) -> Result<(), AdapterError> {
        let Some((locator, mode)) = self.handles.borrow_mut().remove(handle.adapter_handle())
        else {
            return Err(AdapterError::new("not-open", "handle is not open"));
        };
        if mode == SemanticOpenMode::Writable {
            let mut manifest = Self::read_manifest(&locator)?;
            if manifest.writer_token.as_deref() == Some(handle.adapter_handle()) {
                manifest.writer_token = None;
                Self::write_manifest_atomic(&locator, &manifest)?;
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
        let locator = handle.session.adapter_locator();
        let mut manifest = Self::read_manifest(locator)?;
        if manifest.writer_token.as_deref() != Some(handle.adapter_handle()) {
            return Err(AdapterError::new(
                "not-authoritative-writer",
                "writer token mismatch",
            ));
        }
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
        let mut state = Self::load_openable_state(locator)?;
        apply_command(&mut state, command)?;
        let next_seq = manifest.committed_seq + 1;
        let next_generation = manifest.generation + 1;
        let record = AppendRecord {
            seq: next_seq,
            command_kind: command_kind_label(command),
            checkpoint_generation: next_generation,
        };
        Self::append_log_record(locator, &record)?;
        Self::write_checkpoint_atomic(locator, next_generation, &state)?;
        Self::rebuild_derived_index(locator, &state)?;
        manifest.generation = next_generation;
        manifest.committed_seq = next_seq;
        Self::write_manifest_atomic(locator, &manifest)?;
        Ok(())
    }

    fn read_normalized_state(
        &self,
        handle: &EvidenceSessionHandle,
    ) -> Result<NormalizedSemanticState, AdapterError> {
        if !self.handles.borrow().contains_key(handle.adapter_handle()) {
            return Err(AdapterError::new("not-open", "handle is not open"));
        }
        Self::load_openable_state(handle.session.adapter_locator())
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
        let locator_path = self.session_dir(new_session_id);
        if locator_path.exists() {
            return Err(AdapterError::new(
                "duplicate-session-exists",
                "duplicate session identity already exists",
            ));
        }
        copy_dir_all(Path::new(source.session.adapter_locator()), &locator_path)
            .map_err(|error| AdapterError::new("filesystem-copy-bundle", error.to_string()))?;
        let locator = locator_path.to_string_lossy().to_string();
        let manifest = BundleManifest {
            format_version: CURRENT_FORMAT_VERSION,
            generation: Self::read_manifest(&locator)?.generation,
            committed_seq: Self::read_manifest(&locator)?.committed_seq,
            session_id: new_session_id.to_string(),
            duplicated_from_session_id: Some(state.session.session_id.clone()),
            session_format_version: state.session_format_version.clone(),
            interpretation_version: state.interpretation_version.clone(),
            writer_token: None,
            writer_epoch: 0,
        };
        Self::write_manifest_atomic(&locator, &manifest)?;
        Self::save_duplicate_state(&locator, &duplicate_state)?;
        Ok(DuplicatedSession {
            session: EvidenceSessionRef::new(new_session_id, locator),
            normalized_state: duplicate_state.normalize(),
        })
    }

    fn corrupt_or_fault_inject(
        &mut self,
        session: &EvidenceSessionRef,
        scenario: &ScenarioIdentity,
    ) -> Result<(), AdapterError> {
        let locator = session.adapter_locator();
        match scenario.scenario_id.as_str() {
            "derived-state-corruption" => {
                fs::write(Self::derived_index_path(locator), "corrupted-derived").map_err(
                    |error| AdapterError::new("filesystem-corrupt-derived", error.to_string()),
                )?;
            }
            "canonical-reference-corruption" => {
                let manifest = Self::read_manifest(locator)?;
                let path = Self::checkpoint_path(locator, manifest.generation);
                fs::write(path, "{invalid-canonical-json").map_err(|error| {
                    AdapterError::new("filesystem-corrupt-canonical", error.to_string())
                })?;
            }
            "unknown-newer-format" => {
                let mut manifest = Self::read_manifest(locator)?;
                manifest.format_version = CURRENT_FORMAT_VERSION + 1;
                Self::write_manifest_atomic(locator, &manifest)?;
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
                limitation: "operation not supported by append bundle spike".to_string(),
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
        let locator = handle.session.adapter_locator();
        let state = Self::load_openable_state(locator)?;
        let mut manifest = Self::read_manifest(locator)?;
        let next_generation = manifest.generation + 1;
        Self::write_checkpoint_atomic(locator, next_generation, &state)?;
        fs::write(Self::log_path(locator), [])
            .map_err(|error| AdapterError::new("filesystem-truncate-log", error.to_string()))?;
        manifest.generation = next_generation;
        Self::write_manifest_atomic(locator, &manifest)?;
        Ok(OptionalOperationOutcome::Completed)
    }
}

impl AppendBundleAdapter {
    fn save_duplicate_state(
        locator: &str,
        state: &NormalizedSemanticState,
    ) -> Result<(), AdapterError> {
        let manifest = Self::read_manifest(locator)?;
        Self::write_checkpoint_atomic(locator, manifest.generation, state)
    }
}

fn command_kind_label(command: &AuthoritativeCommand) -> String {
    match command {
        AuthoritativeCommand::AppendCorrectionEvent { .. } => "append-correction".to_string(),
        AuthoritativeCommand::AttachAnalysisResult { .. } => "attach-analysis".to_string(),
        AuthoritativeCommand::SelectActiveAnalysis { .. } => "select-active-analysis".to_string(),
        AuthoritativeCommand::ExecuteCleanupPlan { .. } => "execute-cleanup".to_string(),
    }
}

fn copy_dir_all(source: &Path, destination: &Path) -> std::io::Result<()> {
    fs::create_dir_all(destination)?;
    for entry in fs::read_dir(source)? {
        let entry = entry?;
        let target = destination.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_dir_all(&entry.path(), &target)?;
        } else {
            fs::copy(entry.path(), target)?;
        }
    }
    Ok(())
}
