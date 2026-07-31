use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CurrentContractReadiness {
    pub mechanism_comparison_readiness: String,
    pub mechanism_selection_readiness: String,
    pub selection_status: String,
    pub package: String,
    pub note: String,
}

pub fn current_contract_readiness() -> CurrentContractReadiness {
    CurrentContractReadiness {
        mechanism_comparison_readiness: "not_ready".to_owned(),
        mechanism_selection_readiness: "not_ready".to_owned(),
        selection_status: "none".to_owned(),
        package: "VP-GATE4-EVIDENCE-COMPLETION-01A".to_owned(),
        note: "Contract package only; equivalent candidate execution not completed".to_owned(),
    }
}

pub const EVIDENCE_PACKAGE_FILES: &[&str] = &[
    "manifest.json",
    "candidate.json",
    "fixture.json",
    "scenario-contract.json",
    "scenario-results.json",
    "oracle-results.json",
    "measurements.json",
    "environment.json",
    "limitations.json",
    "negative-results.json",
    "platform-matrix.json",
    "comparison.json",
    "readiness.json",
];
