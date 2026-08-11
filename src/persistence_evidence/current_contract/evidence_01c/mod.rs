//! Gate 4 package 01C — equivalent candidate evidence execution harness.
//!
//! Compares owner-accepted append 01B-2 and SQLite 01C-SQLITE-2 under frozen
//! fixture/oracle/scenario/measurement contracts. Spike-only; not production.

pub mod candidate;
pub mod comparison;
pub mod environment;
pub mod measurements;
pub mod memory;
pub mod methodology;
pub mod runner;
pub mod scenarios;
pub mod types;

pub use candidate::{CurrentContractCandidate, CurrentContractCandidateKind};
pub use comparison::build_comparison_report;
pub use environment::capture_environment;
pub use measurements::run_comparative_measurements;
pub use methodology::{
    EVIDENCE_01C_HARNESS_VERSION, INTERLEAVING_STRATEGY, METHODOLOGY_FREEZE_ID, methodology_record,
};
pub use runner::run_01c_evidence;
pub use types::{
    CandidateEligibilityRecord, CandidateRunArtifacts, ComparativeEvidencePackage,
    CorrectnessDisqualification, MeasurementAggregate, NormalizedScenarioResult,
    ScenarioExecutionStatus,
};
