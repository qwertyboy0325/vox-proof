use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum CandidateEligibilityStatus {
    EligibleForEquivalentExecution,
    IneligibleContractFailure,
    ImplementationNotYetEvaluated,
    DisqualifiedByDemonstratedFailure,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CandidateEquivalenceRequirements {
    pub contract_version: String,
    pub fixture_id: String,
    pub fixture_version: String,
    pub oracle_version: String,
    pub scenario_contract_version: String,
    pub measurement_contract_version: String,
    pub sqlite_candidate_id: String,
    pub append_candidate_id: String,
    pub sqlite_status: CandidateEligibilityStatus,
    pub append_status: CandidateEligibilityStatus,
    pub shared_required_scenarios: Vec<String>,
    pub append_authoritative_requirements: Vec<String>,
    pub sqlite_obligations: Vec<String>,
    pub cross_platform_labels: Vec<String>,
}

pub fn candidate_equivalence_requirements() -> CandidateEquivalenceRequirements {
    CandidateEquivalenceRequirements {
        contract_version: "1".to_owned(),
        fixture_id: super::model::CURRENT_CONTRACT_FIXTURE_ID.to_owned(),
        fixture_version: super::model::CURRENT_CONTRACT_FIXTURE_VERSION.to_owned(),
        oracle_version: super::oracle::CURRENT_CONTRACT_ORACLE_VERSION.to_owned(),
        scenario_contract_version: super::scenario_contract::SCENARIO_CONTRACT_VERSION.to_owned(),
        measurement_contract_version: super::measurement::MEASUREMENT_CONTRACT_VERSION.to_owned(),
        sqlite_candidate_id: "embedded-relational-sqlite-spike".to_owned(),
        append_candidate_id: "append-bundle-log-spike".to_owned(),
        sqlite_status: CandidateEligibilityStatus::ImplementationNotYetEvaluated,
        append_status: CandidateEligibilityStatus::ImplementationNotYetEvaluated,
        shared_required_scenarios:
            super::scenario_contract::required_scenario_ids_for_both_candidates(),
        append_authoritative_requirements: vec![
            "append history contains sufficient canonical command payloads".to_owned(),
            "state can be reconstructed from append authority".to_owned(),
            "checkpoint is not sole hidden authority".to_owned(),
            "record ordering and lineage are validated".to_owned(),
            "truncation and malformed records fail closed".to_owned(),
            "compaction preserves event identity, order, and provenance".to_owned(),
            "interrupted compaction does not expose mixed generations".to_owned(),
            "writer ownership and takeover are validated".to_owned(),
            "unknown newer format refuses writable open".to_owned(),
            "semantic duplication creates new identity and independent ownership".to_owned(),
        ],
        sqlite_obligations: vec![
            "same fixture version".to_owned(),
            "same oracle version".to_owned(),
            "same scenario versions".to_owned(),
            "same platform matrix".to_owned(),
            "same measurement contract".to_owned(),
            "same normalized result schema".to_owned(),
        ],
        cross_platform_labels: vec![
            "macos_native".to_owned(),
            "windows_github_actions".to_owned(),
        ],
    }
}
