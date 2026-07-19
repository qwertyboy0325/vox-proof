use std::collections::BTreeSet;
use std::fmt;

use serde::{Deserialize, Serialize};

use super::fixture::EvidenceFixture;
use super::model::{
    ActiveAnalysisSelection, AnalysisResultState, NormalizedSemanticState, ReviewLedgerEventState,
};
use super::scenario::ScenarioIdentity;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum OptionalCapability {
    Compaction,
    DestructiveHistoricalGc,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CandidateCapabilities {
    pub optional: BTreeSet<OptionalCapability>,
    pub limitations: Vec<String>,
}

impl CandidateCapabilities {
    pub fn supports(&self, capability: OptionalCapability) -> bool {
        self.optional.contains(&capability)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SemanticOpenMode {
    Writable,
    ReadOnly,
}

/// Test-only logical session address. `adapter_locator` is opaque adapter state:
/// it must never enter normalized semantic truth or evidence comparison.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EvidenceSessionRef {
    pub session_id: String,
    adapter_locator: String,
}

impl EvidenceSessionRef {
    pub fn new(session_id: impl Into<String>, adapter_locator: impl Into<String>) -> Self {
        Self {
            session_id: session_id.into(),
            adapter_locator: adapter_locator.into(),
        }
    }

    pub fn adapter_locator(&self) -> &str {
        &self.adapter_locator
    }
}

/// One adapter-owned open against a semantic session reference.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvidenceSessionHandle {
    pub session: EvidenceSessionRef,
    pub mode: SemanticOpenMode,
    adapter_handle: String,
}

impl EvidenceSessionHandle {
    pub fn new(
        session: EvidenceSessionRef,
        mode: SemanticOpenMode,
        adapter_handle: impl Into<String>,
    ) -> Self {
        Self {
            session,
            mode,
            adapter_handle: adapter_handle.into(),
        }
    }

    pub fn adapter_handle(&self) -> &str {
        &self.adapter_handle
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DuplicatedSession {
    pub session: EvidenceSessionRef,
    pub normalized_state: NormalizedSemanticState,
}

/// Candidate-neutral, scoped optimistic-concurrency expectations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SemanticPrecondition {
    SourceRevisionExists {
        expected_revision_id: String,
    },
    ReviewLedgerHead {
        expected_event_id: Option<String>,
    },
    ActiveAnalysisSelection {
        expected_analysis_result_id: Option<String>,
    },
    AnalysisAttachmentSet {
        expected_analysis_result_ids: Vec<String>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuthoritativeCommand {
    AppendCorrectionEvent {
        event: ReviewLedgerEventState,
        preconditions: Vec<SemanticPrecondition>,
    },
    AttachAnalysisResult {
        analysis_result: AnalysisResultState,
        preconditions: Vec<SemanticPrecondition>,
    },
    SelectActiveAnalysis {
        selection: ActiveAnalysisSelection,
        preconditions: Vec<SemanticPrecondition>,
    },
    ExecuteCleanupPlan {
        plan_id: String,
        preconditions: Vec<SemanticPrecondition>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MaintenanceOperation {
    Compact,
    DestructiveHistoricalGc,
}

impl MaintenanceOperation {
    pub fn required_capability(self) -> OptionalCapability {
        match self {
            Self::Compact => OptionalCapability::Compaction,
            Self::DestructiveHistoricalGc => OptionalCapability::DestructiveHistoricalGc,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OptionalOperationOutcome {
    Completed,
    Unsupported {
        capability: OptionalCapability,
        limitation: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdapterError {
    pub code: String,
    pub message: String,
}

impl AdapterError {
    pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
        }
    }
}

impl fmt::Display for AdapterError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}: {}", self.code, self.message)
    }
}

impl std::error::Error for AdapterError {}

/// Test-only semantic contract for future persistence-spike candidates.
///
/// Implementations expose observable session behavior, not storage primitives.
/// Implementing this trait does not approve a dependency or make an adapter a
/// production persistence implementation.
pub trait PersistenceCandidateAdapter {
    fn candidate_id(&self) -> &str;
    fn candidate_version(&self) -> &str;
    fn capabilities(&self) -> CandidateCapabilities;

    fn create(&mut self, fixture: &EvidenceFixture) -> Result<EvidenceSessionRef, AdapterError>;
    fn open(
        &mut self,
        session: &EvidenceSessionRef,
        mode: SemanticOpenMode,
    ) -> Result<EvidenceSessionHandle, AdapterError>;
    fn close(&mut self, handle: &EvidenceSessionHandle) -> Result<(), AdapterError>;
    fn apply_authoritative_command(
        &mut self,
        handle: &EvidenceSessionHandle,
        command: &AuthoritativeCommand,
    ) -> Result<(), AdapterError>;
    fn read_normalized_state(
        &self,
        handle: &EvidenceSessionHandle,
    ) -> Result<NormalizedSemanticState, AdapterError>;
    fn attempt_read_only_open(
        &mut self,
        session: &EvidenceSessionRef,
    ) -> Result<EvidenceSessionHandle, AdapterError>;
    fn duplicate_session(
        &mut self,
        source: &EvidenceSessionHandle,
        new_session_id: &str,
    ) -> Result<DuplicatedSession, AdapterError>;
    fn corrupt_or_fault_inject(
        &mut self,
        session: &EvidenceSessionRef,
        scenario: &ScenarioIdentity,
    ) -> Result<(), AdapterError>;
    fn cleanup_or_compact_if_supported(
        &mut self,
        handle: &EvidenceSessionHandle,
        operation: MaintenanceOperation,
    ) -> Result<OptionalOperationOutcome, AdapterError>;
}
