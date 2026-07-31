use serde::{Deserialize, Serialize};

pub const SCENARIO_CONTRACT_VERSION: &str = "3";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScenarioRequirementLevel {
    Required,
    CapabilityDependent,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FaultLayer {
    LogicalReturnedError,
    InProcessPanic,
    ChildProcessAbortOrKill,
    ProcessCrashRecovery,
    OsCrash,
    FilesystemDurability,
    HardwarePowerLoss,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExpectedRecoveryClass {
    None,
    LastCommittedState,
    SafeAutomaticRecovery,
    ManualReviewRequired,
    ReadOnlySalvage,
    Unrecoverable,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScenarioContractV3 {
    pub scenario_id: String,
    pub scenario_version: u32,
    pub requirement: ScenarioRequirementLevel,
    pub category: String,
    pub preconditions: String,
    pub authoritative_command: String,
    pub fault_point: String,
    pub fault_layer: FaultLayer,
    pub expected_durable_boundary: String,
    pub reopen_required: bool,
    pub expected_recovery_class: ExpectedRecoveryClass,
    pub oracle_assertions: Vec<String>,
    pub allowed_evidence_strength: Vec<String>,
    pub prohibited_claims: Vec<String>,
    pub platform_requirement: Vec<String>,
    pub measurement_fields: Vec<String>,
}

#[allow(clippy::too_many_arguments)]
fn scenario(
    id: &str,
    category: &str,
    command: &str,
    fault_layer: FaultLayer,
    reopen: bool,
    recovery: ExpectedRecoveryClass,
    strengths: &[&str],
    prohibited: &[&str],
) -> ScenarioContractV3 {
    ScenarioContractV3 {
        scenario_id: id.to_owned(),
        scenario_version: 1,
        requirement: ScenarioRequirementLevel::Required,
        category: category.to_owned(),
        preconditions: "current-contract fixture v3 loaded".to_owned(),
        authoritative_command: command.to_owned(),
        fault_point: if matches!(
            fault_layer,
            FaultLayer::LogicalReturnedError | FaultLayer::InProcessPanic
        ) {
            "none".to_owned()
        } else {
            "authoritative_command_ack_boundary".to_owned()
        },
        fault_layer,
        expected_durable_boundary: "post_commit_or_explicit_rejection".to_owned(),
        reopen_required: reopen,
        expected_recovery_class: recovery,
        oracle_assertions: vec!["current_contract_oracle_v3.compare".to_owned()],
        allowed_evidence_strength: strengths.iter().map(|s| (*s).to_owned()).collect(),
        prohibited_claims: prohibited.iter().map(|s| (*s).to_owned()).collect(),
        platform_requirement: vec![
            "macos_native".to_owned(),
            "windows_github_actions".to_owned(),
        ],
        measurement_fields: vec!["scenario_elapsed_ms".to_owned()],
    }
}

pub fn scenario_contract_v3() -> Vec<ScenarioContractV3> {
    let no_durability = &["FilesystemDurability", "HardwarePowerLoss", "OsCrash"];

    let scenarios = vec![
        scenario(
            "baseline-create-open-close",
            "baseline",
            "create_open_close",
            FaultLayer::LogicalReturnedError,
            true,
            ExpectedRecoveryClass::None,
            &["InterfaceBehavior"],
            &[],
        ),
        scenario(
            "append-review-decision",
            "authoritative_command",
            "append_review_decision",
            FaultLayer::LogicalReturnedError,
            true,
            ExpectedRecoveryClass::None,
            &["LogicalStateTransition"],
            &[],
        ),
        scenario(
            "append-manual-replacement",
            "authoritative_command",
            "append_manual_replacement",
            FaultLayer::LogicalReturnedError,
            true,
            ExpectedRecoveryClass::None,
            &["LogicalStateTransition"],
            &[],
        ),
        scenario(
            "append-promotion-candidate-rejection",
            "authoritative_command",
            "append_promotion_candidate_rejection",
            FaultLayer::LogicalReturnedError,
            true,
            ExpectedRecoveryClass::None,
            &["LogicalStateTransition"],
            &[],
        ),
        scenario(
            "append-reusable-promotion",
            "authoritative_command",
            "append_reusable_promotion",
            FaultLayer::LogicalReturnedError,
            true,
            ExpectedRecoveryClass::None,
            &["LogicalStateTransition"],
            &[],
        ),
        scenario(
            "append-reusable-revocation",
            "authoritative_command",
            "append_reusable_revocation",
            FaultLayer::LogicalReturnedError,
            true,
            ExpectedRecoveryClass::None,
            &["LogicalStateTransition"],
            &[],
        ),
        scenario(
            "append-reusable-supersession",
            "authoritative_command",
            "append_reusable_supersession",
            FaultLayer::LogicalReturnedError,
            true,
            ExpectedRecoveryClass::None,
            &["LogicalStateTransition"],
            &[],
        ),
        scenario(
            "stale-review-ledger-command",
            "concurrency",
            "stale_review_ledger_command",
            FaultLayer::LogicalReturnedError,
            false,
            ExpectedRecoveryClass::None,
            &["InterfaceBehavior"],
            &[],
        ),
        scenario(
            "stale-reuse-governance-command",
            "concurrency",
            "stale_reuse_governance_command",
            FaultLayer::LogicalReturnedError,
            false,
            ExpectedRecoveryClass::None,
            &["InterfaceBehavior"],
            &[],
        ),
        scenario(
            "stale-analysis-attachment-or-selection",
            "concurrency",
            "stale_analysis_attachment_or_selection",
            FaultLayer::LogicalReturnedError,
            false,
            ExpectedRecoveryClass::None,
            &["InterfaceBehavior"],
            &[],
        ),
        scenario(
            "concurrent-writer-attempt",
            "concurrency",
            "concurrent_writer_attempt",
            FaultLayer::ChildProcessAbortOrKill,
            true,
            ExpectedRecoveryClass::SafeAutomaticRecovery,
            &["InterfaceBehavior", "ProcessCrashRecovery"],
            no_durability,
        ),
        scenario(
            "writer-crash-and-takeover",
            "concurrency",
            "writer_crash_and_takeover",
            FaultLayer::ChildProcessAbortOrKill,
            true,
            ExpectedRecoveryClass::SafeAutomaticRecovery,
            &["InterfaceBehavior", "ProcessCrashRecovery"],
            no_durability,
        ),
        scenario(
            "read-only-open-during-writer",
            "concurrency",
            "read_only_open_during_writer",
            FaultLayer::LogicalReturnedError,
            false,
            ExpectedRecoveryClass::None,
            &["InterfaceBehavior"],
            &[],
        ),
        scenario(
            "unknown-newer-format",
            "format_compatibility",
            "open_unknown_newer_format",
            FaultLayer::LogicalReturnedError,
            false,
            ExpectedRecoveryClass::Unrecoverable,
            &["InterfaceBehavior"],
            &[],
        ),
        scenario(
            "malformed-format-version",
            "format_compatibility",
            "open_malformed_format_version",
            FaultLayer::LogicalReturnedError,
            false,
            ExpectedRecoveryClass::Unrecoverable,
            &["InterfaceBehavior"],
            &[],
        ),
        scenario(
            "canonical-reference-corruption",
            "corruption",
            "detect_canonical_reference_corruption",
            FaultLayer::LogicalReturnedError,
            false,
            ExpectedRecoveryClass::Unrecoverable,
            &["InterfaceBehavior"],
            &[],
        ),
        scenario(
            "review-ledger-order-corruption",
            "corruption",
            "detect_review_ledger_order_corruption",
            FaultLayer::LogicalReturnedError,
            false,
            ExpectedRecoveryClass::Unrecoverable,
            &["InterfaceBehavior"],
            &[],
        ),
        scenario(
            "reuse-governance-order-corruption",
            "corruption",
            "detect_reuse_governance_order_corruption",
            FaultLayer::LogicalReturnedError,
            false,
            ExpectedRecoveryClass::Unrecoverable,
            &["InterfaceBehavior"],
            &[],
        ),
        scenario(
            "source-locator-corruption",
            "corruption",
            "detect_source_locator_corruption",
            FaultLayer::LogicalReturnedError,
            false,
            ExpectedRecoveryClass::Unrecoverable,
            &["InterfaceBehavior"],
            &[],
        ),
        scenario(
            "derived-state-corruption-and-rebuild",
            "corruption",
            "rebuild_derived_state",
            FaultLayer::LogicalReturnedError,
            true,
            ExpectedRecoveryClass::SafeAutomaticRecovery,
            &["InterfaceBehavior"],
            &[],
        ),
        scenario(
            "semantic-duplication",
            "duplication",
            "semantic_duplication",
            FaultLayer::LogicalReturnedError,
            true,
            ExpectedRecoveryClass::None,
            &["LogicalStateTransition"],
            &[],
        ),
        scenario(
            "interrupted-authoritative-review-transition",
            "recovery",
            "interrupt_review_transition",
            FaultLayer::ChildProcessAbortOrKill,
            true,
            ExpectedRecoveryClass::SafeAutomaticRecovery,
            &["InterfaceBehavior", "ProcessCrashRecovery"],
            no_durability,
        ),
        scenario(
            "interrupted-authoritative-reuse-transition",
            "recovery",
            "interrupt_reuse_transition",
            FaultLayer::ChildProcessAbortOrKill,
            true,
            ExpectedRecoveryClass::SafeAutomaticRecovery,
            &["InterfaceBehavior", "ProcessCrashRecovery"],
            no_durability,
        ),
        scenario(
            "interrupted-compaction",
            "maintenance",
            "interrupt_compaction",
            FaultLayer::ChildProcessAbortOrKill,
            true,
            ExpectedRecoveryClass::SafeAutomaticRecovery,
            &["InterfaceBehavior"],
            no_durability,
        ),
        ScenarioContractV3 {
            scenario_id: "interrupted-cleanup".to_owned(),
            scenario_version: 1,
            requirement: ScenarioRequirementLevel::CapabilityDependent,
            category: "maintenance".to_owned(),
            preconditions: "destructive historical cleanup capability declared".to_owned(),
            authoritative_command: "interrupt_cleanup".to_owned(),
            fault_point: "authoritative_command_ack_boundary".to_owned(),
            fault_layer: FaultLayer::ChildProcessAbortOrKill,
            expected_durable_boundary: "post_commit_or_explicit_rejection".to_owned(),
            reopen_required: true,
            expected_recovery_class: ExpectedRecoveryClass::SafeAutomaticRecovery,
            oracle_assertions: vec!["current_contract_oracle_v3.compare".to_owned()],
            allowed_evidence_strength: vec!["InterfaceBehavior".to_owned()],
            prohibited_claims: no_durability.iter().map(|s| (*s).to_owned()).collect(),
            platform_requirement: vec![
                "macos_native".to_owned(),
                "windows_github_actions".to_owned(),
            ],
            measurement_fields: vec!["scenario_elapsed_ms".to_owned()],
        },
    ];

    scenarios
}

pub fn validate_scenario_contract_v3() -> Result<(), String> {
    let scenarios = scenario_contract_v3();
    let mut ids = std::collections::BTreeSet::new();
    for scenario in &scenarios {
        let key = format!("{}@{}", scenario.scenario_id, scenario.scenario_version);
        if !ids.insert(key) {
            return Err(format!(
                "duplicate scenario identity: {}",
                scenario.scenario_id
            ));
        }
        if scenario.allowed_evidence_strength.is_empty() {
            return Err(format!(
                "scenario {} missing allowed evidence strength",
                scenario.scenario_id
            ));
        }
        if matches!(
            scenario.fault_layer,
            FaultLayer::ChildProcessAbortOrKill | FaultLayer::ProcessCrashRecovery
        ) && !scenario
            .prohibited_claims
            .iter()
            .any(|claim| claim == "FilesystemDurability")
        {
            return Err(format!(
                "process interruption scenario {} must prohibit filesystem durability claims",
                scenario.scenario_id
            ));
        }
    }
    Ok(())
}

pub fn required_scenario_ids_for_both_candidates() -> Vec<String> {
    scenario_contract_v3()
        .into_iter()
        .filter(|scenario| scenario.requirement == ScenarioRequirementLevel::Required)
        .map(|scenario| scenario.scenario_id)
        .collect()
}
