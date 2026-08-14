use serde::{Deserialize, Serialize};

pub const SCENARIO_CONTRACT_VERSION_V3: &str = "3";
pub const SCENARIO_CONTRACT_VERSION: &str = "4";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScenarioRequirementLevel {
    Required,
    CapabilityDependent,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FaultLayer {
    NoFault,
    LogicalReturnedError,
    MultiProcessConcurrency,
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
    UnsupportedVersion,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExpectedOpenState {
    Normal,
    UnsupportedVersion,
    ReadOnlySalvage,
    Unrecoverable,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReadOnlyOpenPolicy {
    Forbidden,
    Allowed,
    ConditionallyAllowed { required_condition: String },
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
    pub expected_open_state: ExpectedOpenState,
    pub writable_open: bool,
    pub read_only_open: ReadOnlyOpenPolicy,
    pub oracle_assertions: Vec<String>,
    pub allowed_evidence_strength: Vec<String>,
    pub prohibited_claims: Vec<String>,
    pub platform_requirement: Vec<String>,
    pub capability_requirement: Option<String>,
    pub measurement_fields: Vec<String>,
}

const PLATFORM: &[&str] = &["macos_native", "windows_github_actions"];
const NO_DURABILITY: &[&str] = &["FilesystemDurability", "HardwarePowerLoss", "OsCrash"];

pub fn scenario_contract_v3() -> Vec<ScenarioContractV3> {
    vec![
        baseline_create_open_close(),
        append_review_decision(),
        append_manual_replacement(),
        append_promotion_candidate_rejection(),
        append_reusable_promotion(),
        append_reusable_revocation(),
        append_reusable_supersession(),
        stale_review_ledger_command(),
        stale_reuse_governance_command(),
        stale_analysis_attachment_or_selection(),
        unrelated_scope_review_after_reuse_advance(),
        concurrent_writer_attempt(),
        writer_crash_and_takeover(),
        read_only_open_during_writer(),
        unknown_newer_format(),
        malformed_format_version(),
        canonical_reference_corruption(),
        review_ledger_order_corruption(),
        reuse_governance_order_corruption(),
        source_locator_corruption(),
        derived_state_corruption_and_rebuild(),
        semantic_duplication(),
        interrupted_authoritative_review_transition(),
        interrupted_authoritative_reuse_transition(),
        interrupted_compaction(),
        interrupted_cleanup(),
    ]
}

fn baseline_create_open_close() -> ScenarioContractV3 {
    ScenarioContractV3 {
        scenario_id: "baseline-create-open-close".to_owned(),
        scenario_version: 1,
        requirement: ScenarioRequirementLevel::Required,
        category: "baseline".to_owned(),
        preconditions: "empty storage; current-contract fixture v3 selected".to_owned(),
        authoritative_command: "create_open_close".to_owned(),
        fault_point: "none".to_owned(),
        fault_layer: FaultLayer::NoFault,
        expected_durable_boundary: "session_created_and_closed_without_partial_authority"
            .to_owned(),
        reopen_required: true,
        expected_recovery_class: ExpectedRecoveryClass::None,
        expected_open_state: ExpectedOpenState::Normal,
        writable_open: true,
        read_only_open: ReadOnlyOpenPolicy::Allowed,
        oracle_assertions: vec!["current_contract_oracle_v3.validate".to_owned()],
        allowed_evidence_strength: vec!["InterfaceBehavior".to_owned()],
        prohibited_claims: Vec::new(),
        platform_requirement: PLATFORM.iter().map(|s| (*s).to_owned()).collect(),
        capability_requirement: None,
        measurement_fields: vec!["scenario_elapsed_ms".to_owned()],
    }
}

fn append_review_decision() -> ScenarioContractV3 {
    ScenarioContractV3 {
        scenario_id: "append-review-decision".to_owned(),
        scenario_version: 1,
        requirement: ScenarioRequirementLevel::Required,
        category: "authoritative_command".to_owned(),
        preconditions: "open writable session with pending review case".to_owned(),
        authoritative_command: "append_review_decision".to_owned(),
        fault_point: "none".to_owned(),
        fault_layer: FaultLayer::NoFault,
        expected_durable_boundary: "review_ledger_event_committed".to_owned(),
        reopen_required: true,
        expected_recovery_class: ExpectedRecoveryClass::None,
        expected_open_state: ExpectedOpenState::Normal,
        writable_open: true,
        read_only_open: ReadOnlyOpenPolicy::Allowed,
        oracle_assertions: vec!["current_contract_oracle_v3.compare".to_owned()],
        allowed_evidence_strength: vec!["LogicalStateTransition".to_owned()],
        prohibited_claims: Vec::new(),
        platform_requirement: PLATFORM.iter().map(|s| (*s).to_owned()).collect(),
        capability_requirement: None,
        measurement_fields: vec!["scenario_elapsed_ms".to_owned()],
    }
}

fn append_manual_replacement() -> ScenarioContractV3 {
    ScenarioContractV3 {
        scenario_id: "append-manual-replacement".to_owned(),
        scenario_version: 1,
        requirement: ScenarioRequirementLevel::Required,
        category: "authoritative_command".to_owned(),
        preconditions: "MD-017 manual replacement fixture loaded".to_owned(),
        authoritative_command: "append_manual_replacement".to_owned(),
        fault_point: "none".to_owned(),
        fault_layer: FaultLayer::NoFault,
        expected_durable_boundary: "manual_replacement_bytes_committed".to_owned(),
        reopen_required: true,
        expected_recovery_class: ExpectedRecoveryClass::None,
        expected_open_state: ExpectedOpenState::Normal,
        writable_open: true,
        read_only_open: ReadOnlyOpenPolicy::Allowed,
        oracle_assertions: vec!["current_contract_oracle_v3.compare".to_owned()],
        allowed_evidence_strength: vec!["LogicalStateTransition".to_owned()],
        prohibited_claims: Vec::new(),
        platform_requirement: PLATFORM.iter().map(|s| (*s).to_owned()).collect(),
        capability_requirement: None,
        measurement_fields: vec!["scenario_elapsed_ms".to_owned()],
    }
}

fn append_promotion_candidate_rejection() -> ScenarioContractV3 {
    ScenarioContractV3 {
        scenario_id: "append-promotion-candidate-rejection".to_owned(),
        scenario_version: 1,
        requirement: ScenarioRequirementLevel::Required,
        category: "authoritative_command".to_owned(),
        preconditions: "candidate_rejected fixture variant".to_owned(),
        authoritative_command: "append_promotion_candidate_rejection".to_owned(),
        fault_point: "none".to_owned(),
        fault_layer: FaultLayer::NoFault,
        expected_durable_boundary: "rejection_identity_committed".to_owned(),
        reopen_required: true,
        expected_recovery_class: ExpectedRecoveryClass::None,
        expected_open_state: ExpectedOpenState::Normal,
        writable_open: true,
        read_only_open: ReadOnlyOpenPolicy::Allowed,
        oracle_assertions: vec!["current_contract_oracle_v3.compare".to_owned()],
        allowed_evidence_strength: vec!["LogicalStateTransition".to_owned()],
        prohibited_claims: Vec::new(),
        platform_requirement: PLATFORM.iter().map(|s| (*s).to_owned()).collect(),
        capability_requirement: None,
        measurement_fields: vec!["scenario_elapsed_ms".to_owned()],
    }
}

fn append_reusable_promotion() -> ScenarioContractV3 {
    ScenarioContractV3 {
        scenario_id: "append-reusable-promotion".to_owned(),
        scenario_version: 1,
        requirement: ScenarioRequirementLevel::Required,
        category: "authoritative_command".to_owned(),
        preconditions: "promoted_active fixture variant".to_owned(),
        authoritative_command: "append_reusable_promotion".to_owned(),
        fault_point: "none".to_owned(),
        fault_layer: FaultLayer::NoFault,
        expected_durable_boundary: "promotion_accepted_committed".to_owned(),
        reopen_required: true,
        expected_recovery_class: ExpectedRecoveryClass::None,
        expected_open_state: ExpectedOpenState::Normal,
        writable_open: true,
        read_only_open: ReadOnlyOpenPolicy::Allowed,
        oracle_assertions: vec!["current_contract_oracle_v3.compare".to_owned()],
        allowed_evidence_strength: vec!["LogicalStateTransition".to_owned()],
        prohibited_claims: Vec::new(),
        platform_requirement: PLATFORM.iter().map(|s| (*s).to_owned()).collect(),
        capability_requirement: None,
        measurement_fields: vec!["scenario_elapsed_ms".to_owned()],
    }
}

fn append_reusable_revocation() -> ScenarioContractV3 {
    ScenarioContractV3 {
        scenario_id: "append-reusable-revocation".to_owned(),
        scenario_version: 1,
        requirement: ScenarioRequirementLevel::Required,
        category: "authoritative_command".to_owned(),
        preconditions: "revoked_historical fixture variant".to_owned(),
        authoritative_command: "append_reusable_revocation".to_owned(),
        fault_point: "none".to_owned(),
        fault_layer: FaultLayer::NoFault,
        expected_durable_boundary: "revocation_committed".to_owned(),
        reopen_required: true,
        expected_recovery_class: ExpectedRecoveryClass::None,
        expected_open_state: ExpectedOpenState::Normal,
        writable_open: true,
        read_only_open: ReadOnlyOpenPolicy::Allowed,
        oracle_assertions: vec!["current_contract_oracle_v3.compare".to_owned()],
        allowed_evidence_strength: vec!["LogicalStateTransition".to_owned()],
        prohibited_claims: Vec::new(),
        platform_requirement: PLATFORM.iter().map(|s| (*s).to_owned()).collect(),
        capability_requirement: None,
        measurement_fields: vec!["scenario_elapsed_ms".to_owned()],
    }
}

fn append_reusable_supersession() -> ScenarioContractV3 {
    ScenarioContractV3 {
        scenario_id: "append-reusable-supersession".to_owned(),
        scenario_version: 1,
        requirement: ScenarioRequirementLevel::Required,
        category: "authoritative_command".to_owned(),
        preconditions: "superseded fixture variant".to_owned(),
        authoritative_command: "append_reusable_supersession".to_owned(),
        fault_point: "none".to_owned(),
        fault_layer: FaultLayer::NoFault,
        expected_durable_boundary: "supersession_batch_committed".to_owned(),
        reopen_required: true,
        expected_recovery_class: ExpectedRecoveryClass::None,
        expected_open_state: ExpectedOpenState::Normal,
        writable_open: true,
        read_only_open: ReadOnlyOpenPolicy::Allowed,
        oracle_assertions: vec!["current_contract_oracle_v3.compare".to_owned()],
        allowed_evidence_strength: vec!["LogicalStateTransition".to_owned()],
        prohibited_claims: Vec::new(),
        platform_requirement: PLATFORM.iter().map(|s| (*s).to_owned()).collect(),
        capability_requirement: None,
        measurement_fields: vec!["scenario_elapsed_ms".to_owned()],
    }
}

fn stale_review_ledger_command() -> ScenarioContractV3 {
    ScenarioContractV3 {
        scenario_id: "stale-review-ledger-command".to_owned(),
        scenario_version: 2,
        requirement: ScenarioRequirementLevel::Required,
        category: "concurrency".to_owned(),
        preconditions:
            "review command A prepared at S0; competing review command B advances review scope to S1"
                .to_owned(),
        authoritative_command: "stale_review_ledger_command".to_owned(),
        fault_point: "none".to_owned(),
        fault_layer: FaultLayer::LogicalReturnedError,
        expected_durable_boundary: "stale_command_rejected".to_owned(),
        reopen_required: true,
        expected_recovery_class: ExpectedRecoveryClass::None,
        expected_open_state: ExpectedOpenState::Normal,
        writable_open: true,
        read_only_open: ReadOnlyOpenPolicy::Allowed,
        oracle_assertions: vec![
            "current_contract_oracle_v3.validate".to_owned(),
            "persist_reopen_authority_unchanged".to_owned(),
        ],
        allowed_evidence_strength: vec!["InterfaceBehavior".to_owned()],
        prohibited_claims: Vec::new(),
        platform_requirement: PLATFORM.iter().map(|s| (*s).to_owned()).collect(),
        capability_requirement: None,
        measurement_fields: vec!["scenario_elapsed_ms".to_owned()],
    }
}

fn stale_reuse_governance_command() -> ScenarioContractV3 {
    ScenarioContractV3 {
        scenario_id: "stale-reuse-governance-command".to_owned(),
        scenario_version: 2,
        requirement: ScenarioRequirementLevel::Required,
        category: "concurrency".to_owned(),
        preconditions:
            "reuse command A prepared at S0; competing reuse command B advances reuse scope to S1"
                .to_owned(),
        authoritative_command: "stale_reuse_governance_command".to_owned(),
        fault_point: "none".to_owned(),
        fault_layer: FaultLayer::LogicalReturnedError,
        expected_durable_boundary: "stale_command_rejected".to_owned(),
        reopen_required: true,
        expected_recovery_class: ExpectedRecoveryClass::None,
        expected_open_state: ExpectedOpenState::Normal,
        writable_open: true,
        read_only_open: ReadOnlyOpenPolicy::Allowed,
        oracle_assertions: vec![
            "current_contract_oracle_v3.validate".to_owned(),
            "persist_reopen_authority_unchanged".to_owned(),
        ],
        allowed_evidence_strength: vec!["InterfaceBehavior".to_owned()],
        prohibited_claims: Vec::new(),
        platform_requirement: PLATFORM.iter().map(|s| (*s).to_owned()).collect(),
        capability_requirement: None,
        measurement_fields: vec!["scenario_elapsed_ms".to_owned()],
    }
}

fn stale_analysis_attachment_or_selection() -> ScenarioContractV3 {
    ScenarioContractV3 {
        scenario_id: "stale-analysis-attachment-or-selection".to_owned(),
        scenario_version: 2,
        requirement: ScenarioRequirementLevel::Required,
        category: "concurrency".to_owned(),
        preconditions:
            "analysis command A prepared at S0; competing analysis command B advances active analysis to S1"
                .to_owned(),
        authoritative_command: "stale_analysis_attachment_or_selection".to_owned(),
        fault_point: "none".to_owned(),
        fault_layer: FaultLayer::LogicalReturnedError,
        expected_durable_boundary: "stale_selection_rejected".to_owned(),
        reopen_required: true,
        expected_recovery_class: ExpectedRecoveryClass::None,
        expected_open_state: ExpectedOpenState::Normal,
        writable_open: true,
        read_only_open: ReadOnlyOpenPolicy::Allowed,
        oracle_assertions: vec![
            "current_contract_oracle_v3.validate".to_owned(),
            "persist_reopen_authority_unchanged".to_owned(),
        ],
        allowed_evidence_strength: vec!["InterfaceBehavior".to_owned()],
        prohibited_claims: Vec::new(),
        platform_requirement: PLATFORM.iter().map(|s| (*s).to_owned()).collect(),
        capability_requirement: None,
        measurement_fields: vec!["scenario_elapsed_ms".to_owned()],
    }
}

fn unrelated_scope_review_after_reuse_advance() -> ScenarioContractV3 {
    ScenarioContractV3 {
        scenario_id: "unrelated-scope-review-after-reuse-advance".to_owned(),
        scenario_version: 1,
        requirement: ScenarioRequirementLevel::Required,
        category: "concurrency".to_owned(),
        preconditions: "review command prepared; unrelated reuse-only advance".to_owned(),
        authoritative_command: "unrelated_scope_review_after_reuse_advance".to_owned(),
        fault_point: "none".to_owned(),
        fault_layer: FaultLayer::NoFault,
        expected_durable_boundary: "review_applied_with_unrelated_reuse_authority_preserved"
            .to_owned(),
        reopen_required: true,
        expected_recovery_class: ExpectedRecoveryClass::None,
        expected_open_state: ExpectedOpenState::Normal,
        writable_open: true,
        read_only_open: ReadOnlyOpenPolicy::Allowed,
        oracle_assertions: vec![
            "current_contract_oracle_v3.compare".to_owned(),
            "persist_reopen_authority_unchanged".to_owned(),
        ],
        allowed_evidence_strength: vec!["LogicalStateTransition".to_owned()],
        prohibited_claims: Vec::new(),
        platform_requirement: PLATFORM.iter().map(|s| (*s).to_owned()).collect(),
        capability_requirement: None,
        measurement_fields: vec!["scenario_elapsed_ms".to_owned()],
    }
}

fn concurrent_writer_attempt() -> ScenarioContractV3 {
    ScenarioContractV3 {
        scenario_id: "concurrent-writer-attempt".to_owned(),
        scenario_version: 1,
        requirement: ScenarioRequirementLevel::Required,
        category: "concurrency".to_owned(),
        preconditions: "one active writer already holds session ownership".to_owned(),
        authoritative_command: "concurrent_writer_attempt".to_owned(),
        fault_point: "none".to_owned(),
        fault_layer: FaultLayer::MultiProcessConcurrency,
        expected_durable_boundary: "second_writer_rejected_or_read_only".to_owned(),
        reopen_required: false,
        expected_recovery_class: ExpectedRecoveryClass::None,
        expected_open_state: ExpectedOpenState::Normal,
        writable_open: false,
        read_only_open: ReadOnlyOpenPolicy::Allowed,
        oracle_assertions: vec!["current_contract_oracle_v3.validate".to_owned()],
        allowed_evidence_strength: vec!["InterfaceBehavior".to_owned()],
        prohibited_claims: vec![
            "ProcessCrashRecovery".to_owned(),
            "FilesystemDurability".to_owned(),
            "HardwarePowerLoss".to_owned(),
            "OsCrash".to_owned(),
        ],
        platform_requirement: PLATFORM.iter().map(|s| (*s).to_owned()).collect(),
        capability_requirement: None,
        measurement_fields: vec!["scenario_elapsed_ms".to_owned()],
    }
}

fn writer_crash_and_takeover() -> ScenarioContractV3 {
    ScenarioContractV3 {
        scenario_id: "writer-crash-and-takeover".to_owned(),
        scenario_version: 1,
        requirement: ScenarioRequirementLevel::Required,
        category: "concurrency".to_owned(),
        preconditions: "writer lease expired or child process aborted".to_owned(),
        authoritative_command: "writer_crash_and_takeover".to_owned(),
        fault_point: "authoritative_command_ack_boundary".to_owned(),
        fault_layer: FaultLayer::ChildProcessAbortOrKill,
        expected_durable_boundary: "last_committed_state_preserved".to_owned(),
        reopen_required: true,
        expected_recovery_class: ExpectedRecoveryClass::SafeAutomaticRecovery,
        expected_open_state: ExpectedOpenState::Normal,
        writable_open: true,
        read_only_open: ReadOnlyOpenPolicy::Allowed,
        oracle_assertions: vec!["current_contract_oracle_v3.compare".to_owned()],
        allowed_evidence_strength: vec![
            "InterfaceBehavior".to_owned(),
            "ProcessCrashRecovery".to_owned(),
        ],
        prohibited_claims: NO_DURABILITY.iter().map(|s| (*s).to_owned()).collect(),
        platform_requirement: PLATFORM.iter().map(|s| (*s).to_owned()).collect(),
        capability_requirement: Some("verified_writer_ownership_loss".to_owned()),
        measurement_fields: vec!["scenario_elapsed_ms".to_owned()],
    }
}

fn read_only_open_during_writer() -> ScenarioContractV3 {
    ScenarioContractV3 {
        scenario_id: "read-only-open-during-writer".to_owned(),
        scenario_version: 1,
        requirement: ScenarioRequirementLevel::Required,
        category: "concurrency".to_owned(),
        preconditions: "active writer holds exclusive ownership".to_owned(),
        authoritative_command: "read_only_open_during_writer".to_owned(),
        fault_point: "none".to_owned(),
        fault_layer: FaultLayer::MultiProcessConcurrency,
        expected_durable_boundary: "read_only_view_without_write_authority".to_owned(),
        reopen_required: false,
        expected_recovery_class: ExpectedRecoveryClass::None,
        expected_open_state: ExpectedOpenState::Normal,
        writable_open: false,
        read_only_open: ReadOnlyOpenPolicy::Allowed,
        oracle_assertions: vec!["current_contract_oracle_v3.validate".to_owned()],
        allowed_evidence_strength: vec!["InterfaceBehavior".to_owned()],
        prohibited_claims: Vec::new(),
        platform_requirement: PLATFORM.iter().map(|s| (*s).to_owned()).collect(),
        capability_requirement: None,
        measurement_fields: vec!["scenario_elapsed_ms".to_owned()],
    }
}

fn unknown_newer_format() -> ScenarioContractV3 {
    ScenarioContractV3 {
        scenario_id: "unknown-newer-format".to_owned(),
        scenario_version: 1,
        requirement: ScenarioRequirementLevel::Required,
        category: "format_compatibility".to_owned(),
        preconditions: "storage reports newer unsupported format version".to_owned(),
        authoritative_command: "open_unknown_newer_format".to_owned(),
        fault_point: "open_boundary".to_owned(),
        fault_layer: FaultLayer::LogicalReturnedError,
        expected_durable_boundary: "writable_open_forbidden".to_owned(),
        reopen_required: false,
        expected_recovery_class: ExpectedRecoveryClass::UnsupportedVersion,
        expected_open_state: ExpectedOpenState::UnsupportedVersion,
        writable_open: false,
        read_only_open: ReadOnlyOpenPolicy::ConditionallyAllowed {
            required_condition: "interpretation_demonstrably_safe".to_owned(),
        },
        oracle_assertions: vec!["current_contract_oracle_v3.validate".to_owned()],
        allowed_evidence_strength: vec!["InterfaceBehavior".to_owned()],
        prohibited_claims: Vec::new(),
        platform_requirement: PLATFORM.iter().map(|s| (*s).to_owned()).collect(),
        capability_requirement: None,
        measurement_fields: vec!["scenario_elapsed_ms".to_owned()],
    }
}

fn malformed_format_version() -> ScenarioContractV3 {
    ScenarioContractV3 {
        scenario_id: "malformed-format-version".to_owned(),
        scenario_version: 1,
        requirement: ScenarioRequirementLevel::Required,
        category: "format_compatibility".to_owned(),
        preconditions: "malformed format header".to_owned(),
        authoritative_command: "open_malformed_format_version".to_owned(),
        fault_point: "open_boundary".to_owned(),
        fault_layer: FaultLayer::LogicalReturnedError,
        expected_durable_boundary: "open_refused".to_owned(),
        reopen_required: false,
        expected_recovery_class: ExpectedRecoveryClass::Unrecoverable,
        expected_open_state: ExpectedOpenState::Unrecoverable,
        writable_open: false,
        read_only_open: ReadOnlyOpenPolicy::Forbidden,
        oracle_assertions: vec!["current_contract_oracle_v3.validate".to_owned()],
        allowed_evidence_strength: vec!["InterfaceBehavior".to_owned()],
        prohibited_claims: Vec::new(),
        platform_requirement: PLATFORM.iter().map(|s| (*s).to_owned()).collect(),
        capability_requirement: None,
        measurement_fields: vec!["scenario_elapsed_ms".to_owned()],
    }
}

fn canonical_reference_corruption() -> ScenarioContractV3 {
    ScenarioContractV3 {
        scenario_id: "canonical-reference-corruption".to_owned(),
        scenario_version: 1,
        requirement: ScenarioRequirementLevel::Required,
        category: "corruption".to_owned(),
        preconditions: "canonical referenced payload checksum mismatch".to_owned(),
        authoritative_command: "detect_canonical_reference_corruption".to_owned(),
        fault_point: "open_boundary".to_owned(),
        fault_layer: FaultLayer::LogicalReturnedError,
        expected_durable_boundary: "corruption_detected_before_authority_exposed".to_owned(),
        reopen_required: false,
        expected_recovery_class: ExpectedRecoveryClass::ManualReviewRequired,
        expected_open_state: ExpectedOpenState::ReadOnlySalvage,
        writable_open: false,
        read_only_open: ReadOnlyOpenPolicy::Allowed,
        oracle_assertions: vec!["current_contract_oracle_v3.validate".to_owned()],
        allowed_evidence_strength: vec!["InterfaceBehavior".to_owned()],
        prohibited_claims: Vec::new(),
        platform_requirement: PLATFORM.iter().map(|s| (*s).to_owned()).collect(),
        capability_requirement: None,
        measurement_fields: vec!["scenario_elapsed_ms".to_owned()],
    }
}

fn review_ledger_order_corruption() -> ScenarioContractV3 {
    ScenarioContractV3 {
        scenario_id: "review-ledger-order-corruption".to_owned(),
        scenario_version: 1,
        requirement: ScenarioRequirementLevel::Required,
        category: "corruption".to_owned(),
        preconditions: "review ledger event order corrupted".to_owned(),
        authoritative_command: "detect_review_ledger_order_corruption".to_owned(),
        fault_point: "open_boundary".to_owned(),
        fault_layer: FaultLayer::LogicalReturnedError,
        expected_durable_boundary: "corruption_detected_before_authority_exposed".to_owned(),
        reopen_required: false,
        expected_recovery_class: ExpectedRecoveryClass::Unrecoverable,
        expected_open_state: ExpectedOpenState::Unrecoverable,
        writable_open: false,
        read_only_open: ReadOnlyOpenPolicy::Forbidden,
        oracle_assertions: vec!["current_contract_oracle_v3.validate".to_owned()],
        allowed_evidence_strength: vec!["InterfaceBehavior".to_owned()],
        prohibited_claims: Vec::new(),
        platform_requirement: PLATFORM.iter().map(|s| (*s).to_owned()).collect(),
        capability_requirement: None,
        measurement_fields: vec!["scenario_elapsed_ms".to_owned()],
    }
}

fn reuse_governance_order_corruption() -> ScenarioContractV3 {
    ScenarioContractV3 {
        scenario_id: "reuse-governance-order-corruption".to_owned(),
        scenario_version: 1,
        requirement: ScenarioRequirementLevel::Required,
        category: "corruption".to_owned(),
        preconditions: "reuse governance event order corrupted".to_owned(),
        authoritative_command: "detect_reuse_governance_order_corruption".to_owned(),
        fault_point: "open_boundary".to_owned(),
        fault_layer: FaultLayer::LogicalReturnedError,
        expected_durable_boundary: "corruption_detected_before_authority_exposed".to_owned(),
        reopen_required: false,
        expected_recovery_class: ExpectedRecoveryClass::Unrecoverable,
        expected_open_state: ExpectedOpenState::Unrecoverable,
        writable_open: false,
        read_only_open: ReadOnlyOpenPolicy::Forbidden,
        oracle_assertions: vec!["current_contract_oracle_v3.validate".to_owned()],
        allowed_evidence_strength: vec!["InterfaceBehavior".to_owned()],
        prohibited_claims: Vec::new(),
        platform_requirement: PLATFORM.iter().map(|s| (*s).to_owned()).collect(),
        capability_requirement: None,
        measurement_fields: vec!["scenario_elapsed_ms".to_owned()],
    }
}

fn source_locator_corruption() -> ScenarioContractV3 {
    ScenarioContractV3 {
        scenario_id: "source-locator-corruption".to_owned(),
        scenario_version: 1,
        requirement: ScenarioRequirementLevel::Required,
        category: "corruption".to_owned(),
        preconditions: "typed source locator field corrupted".to_owned(),
        authoritative_command: "detect_source_locator_corruption".to_owned(),
        fault_point: "open_boundary".to_owned(),
        fault_layer: FaultLayer::LogicalReturnedError,
        expected_durable_boundary: "locator_integrity_failure".to_owned(),
        reopen_required: false,
        expected_recovery_class: ExpectedRecoveryClass::ManualReviewRequired,
        expected_open_state: ExpectedOpenState::ReadOnlySalvage,
        writable_open: false,
        read_only_open: ReadOnlyOpenPolicy::Allowed,
        oracle_assertions: vec!["current_contract_oracle_v3.validate".to_owned()],
        allowed_evidence_strength: vec!["InterfaceBehavior".to_owned()],
        prohibited_claims: Vec::new(),
        platform_requirement: PLATFORM.iter().map(|s| (*s).to_owned()).collect(),
        capability_requirement: None,
        measurement_fields: vec!["scenario_elapsed_ms".to_owned()],
    }
}

fn derived_state_corruption_and_rebuild() -> ScenarioContractV3 {
    ScenarioContractV3 {
        scenario_id: "derived-state-corruption-and-rebuild".to_owned(),
        scenario_version: 1,
        requirement: ScenarioRequirementLevel::Required,
        category: "corruption".to_owned(),
        preconditions: "derived cache corrupted while canonical history intact".to_owned(),
        authoritative_command: "rebuild_derived_state".to_owned(),
        fault_point: "none".to_owned(),
        fault_layer: FaultLayer::NoFault,
        expected_durable_boundary: "derived_state_rebuilt_from_canonical_history".to_owned(),
        reopen_required: true,
        expected_recovery_class: ExpectedRecoveryClass::SafeAutomaticRecovery,
        expected_open_state: ExpectedOpenState::Normal,
        writable_open: true,
        read_only_open: ReadOnlyOpenPolicy::Allowed,
        oracle_assertions: vec!["current_contract_oracle_v3.compare".to_owned()],
        allowed_evidence_strength: vec!["InterfaceBehavior".to_owned()],
        prohibited_claims: Vec::new(),
        platform_requirement: PLATFORM.iter().map(|s| (*s).to_owned()).collect(),
        capability_requirement: None,
        measurement_fields: vec!["scenario_elapsed_ms".to_owned()],
    }
}

fn semantic_duplication() -> ScenarioContractV3 {
    ScenarioContractV3 {
        scenario_id: "semantic-duplication".to_owned(),
        scenario_version: 1,
        requirement: ScenarioRequirementLevel::Required,
        category: "duplication".to_owned(),
        preconditions: "duplicated_session_lineage fixture variant".to_owned(),
        authoritative_command: "semantic_duplication".to_owned(),
        fault_point: "none".to_owned(),
        fault_layer: FaultLayer::NoFault,
        expected_durable_boundary: "new_session_identity_and_independent_writer_token".to_owned(),
        reopen_required: true,
        expected_recovery_class: ExpectedRecoveryClass::None,
        expected_open_state: ExpectedOpenState::Normal,
        writable_open: true,
        read_only_open: ReadOnlyOpenPolicy::Allowed,
        oracle_assertions: vec!["current_contract_oracle_v3.compare".to_owned()],
        allowed_evidence_strength: vec!["LogicalStateTransition".to_owned()],
        prohibited_claims: Vec::new(),
        platform_requirement: PLATFORM.iter().map(|s| (*s).to_owned()).collect(),
        capability_requirement: None,
        measurement_fields: vec!["scenario_elapsed_ms".to_owned()],
    }
}

fn interrupted_authoritative_review_transition() -> ScenarioContractV3 {
    ScenarioContractV3 {
        scenario_id: "interrupted-authoritative-review-transition".to_owned(),
        scenario_version: 1,
        requirement: ScenarioRequirementLevel::Required,
        category: "recovery".to_owned(),
        preconditions: "review transition in flight".to_owned(),
        authoritative_command: "interrupt_review_transition".to_owned(),
        fault_point: "authoritative_command_ack_boundary".to_owned(),
        fault_layer: FaultLayer::ChildProcessAbortOrKill,
        expected_durable_boundary: "no_partial_review_authority_exposed".to_owned(),
        reopen_required: true,
        expected_recovery_class: ExpectedRecoveryClass::SafeAutomaticRecovery,
        expected_open_state: ExpectedOpenState::Normal,
        writable_open: true,
        read_only_open: ReadOnlyOpenPolicy::Allowed,
        oracle_assertions: vec!["current_contract_oracle_v3.compare".to_owned()],
        allowed_evidence_strength: vec![
            "InterfaceBehavior".to_owned(),
            "ProcessCrashRecovery".to_owned(),
        ],
        prohibited_claims: NO_DURABILITY.iter().map(|s| (*s).to_owned()).collect(),
        platform_requirement: PLATFORM.iter().map(|s| (*s).to_owned()).collect(),
        capability_requirement: None,
        measurement_fields: vec!["scenario_elapsed_ms".to_owned()],
    }
}

fn interrupted_authoritative_reuse_transition() -> ScenarioContractV3 {
    ScenarioContractV3 {
        scenario_id: "interrupted-authoritative-reuse-transition".to_owned(),
        scenario_version: 1,
        requirement: ScenarioRequirementLevel::Required,
        category: "recovery".to_owned(),
        preconditions: "reuse governance transition in flight".to_owned(),
        authoritative_command: "interrupt_reuse_transition".to_owned(),
        fault_point: "authoritative_command_ack_boundary".to_owned(),
        fault_layer: FaultLayer::ChildProcessAbortOrKill,
        expected_durable_boundary: "no_partial_reuse_authority_exposed".to_owned(),
        reopen_required: true,
        expected_recovery_class: ExpectedRecoveryClass::SafeAutomaticRecovery,
        expected_open_state: ExpectedOpenState::Normal,
        writable_open: true,
        read_only_open: ReadOnlyOpenPolicy::Allowed,
        oracle_assertions: vec!["current_contract_oracle_v3.compare".to_owned()],
        allowed_evidence_strength: vec![
            "InterfaceBehavior".to_owned(),
            "ProcessCrashRecovery".to_owned(),
        ],
        prohibited_claims: NO_DURABILITY.iter().map(|s| (*s).to_owned()).collect(),
        platform_requirement: PLATFORM.iter().map(|s| (*s).to_owned()).collect(),
        capability_requirement: None,
        measurement_fields: vec!["scenario_elapsed_ms".to_owned()],
    }
}

fn interrupted_compaction() -> ScenarioContractV3 {
    ScenarioContractV3 {
        scenario_id: "interrupted-compaction".to_owned(),
        scenario_version: 1,
        requirement: ScenarioRequirementLevel::CapabilityDependent,
        category: "maintenance".to_owned(),
        preconditions: "compaction capability declared by candidate".to_owned(),
        authoritative_command: "interrupt_compaction".to_owned(),
        fault_point: "authoritative_command_ack_boundary".to_owned(),
        fault_layer: FaultLayer::ChildProcessAbortOrKill,
        expected_durable_boundary: "no_mixed_generation_exposed".to_owned(),
        reopen_required: true,
        expected_recovery_class: ExpectedRecoveryClass::SafeAutomaticRecovery,
        expected_open_state: ExpectedOpenState::Normal,
        writable_open: true,
        read_only_open: ReadOnlyOpenPolicy::Allowed,
        oracle_assertions: vec!["current_contract_oracle_v3.compare".to_owned()],
        allowed_evidence_strength: vec!["InterfaceBehavior".to_owned()],
        prohibited_claims: NO_DURABILITY.iter().map(|s| (*s).to_owned()).collect(),
        platform_requirement: PLATFORM.iter().map(|s| (*s).to_owned()).collect(),
        capability_requirement: Some("compaction_supported".to_owned()),
        measurement_fields: vec!["scenario_elapsed_ms".to_owned()],
    }
}

fn interrupted_cleanup() -> ScenarioContractV3 {
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
        expected_open_state: ExpectedOpenState::Normal,
        writable_open: true,
        read_only_open: ReadOnlyOpenPolicy::Allowed,
        oracle_assertions: vec!["current_contract_oracle_v3.compare".to_owned()],
        allowed_evidence_strength: vec!["InterfaceBehavior".to_owned()],
        prohibited_claims: NO_DURABILITY.iter().map(|s| (*s).to_owned()).collect(),
        platform_requirement: PLATFORM.iter().map(|s| (*s).to_owned()).collect(),
        capability_requirement: Some("destructive_cleanup_supported".to_owned()),
        measurement_fields: vec!["scenario_elapsed_ms".to_owned()],
    }
}

const ALLOWED_STRENGTHS: &[&str] = &[
    "InterfaceBehavior",
    "LogicalStateTransition",
    "ProcessCrashRecovery",
    "FilesystemDurability",
    "HardwarePowerLoss",
    "CrossPlatform",
];

pub fn scenario_contract_v4() -> Vec<ScenarioContractV3> {
    scenario_contract_v3()
        .into_iter()
        .map(|mut scenario| {
            match scenario.scenario_id.as_str() {
                "canonical-reference-corruption" | "source-locator-corruption" => {
                    scenario.scenario_version = 2;
                    scenario.expected_open_state = ExpectedOpenState::Unrecoverable;
                    scenario.read_only_open = ReadOnlyOpenPolicy::Forbidden;
                    scenario.writable_open = false;
                    scenario.expected_recovery_class = ExpectedRecoveryClass::ManualReviewRequired;
                    scenario.expected_durable_boundary =
                        "corruption_detected_fail_closed".to_owned();
                }
                _ => {}
            }
            scenario
        })
        .collect()
}

pub fn validate_scenario_contract_v3() -> Result<(), String> {
    validate_scenario_contracts_v3(&scenario_contract_v3())
}

pub fn validate_scenario_contract_v4() -> Result<(), String> {
    let scenarios = scenario_contract_v4();
    validate_scenario_contracts_v4(&scenarios)?;
    validate_v4_authoritative_corruption_semantics(&scenarios)?;
    validate_scenario_contract_v3()?;
    Ok(())
}

pub fn validate_scenario_contracts_v3(scenarios: &[ScenarioContractV3]) -> Result<(), String> {
    validate_scenario_contract_catalog(scenarios, &scenario_contract_v3())
}

pub fn validate_scenario_contracts_v4(scenarios: &[ScenarioContractV3]) -> Result<(), String> {
    validate_scenario_contract_catalog(scenarios, &scenario_contract_v4())
}

fn validate_scenario_contract_catalog(
    scenarios: &[ScenarioContractV3],
    required_catalog: &[ScenarioContractV3],
) -> Result<(), String> {
    let mut ids = std::collections::BTreeSet::new();
    for scenario in scenarios {
        let key = format!("{}@{}", scenario.scenario_id, scenario.scenario_version);
        if !ids.insert(key) {
            return Err(format!(
                "duplicate scenario identity: {}",
                scenario.scenario_id
            ));
        }
        if scenario.preconditions.trim().is_empty() {
            return Err(format!(
                "scenario {} missing preconditions",
                scenario.scenario_id
            ));
        }
        if scenario.authoritative_command.trim().is_empty() {
            return Err(format!(
                "scenario {} missing authoritative command",
                scenario.scenario_id
            ));
        }
        if scenario.oracle_assertions.is_empty() {
            return Err(format!(
                "scenario {} missing oracle assertions",
                scenario.scenario_id
            ));
        }
        if scenario
            .oracle_assertions
            .iter()
            .any(|assertion| assertion.trim().is_empty())
        {
            return Err(format!(
                "scenario {} has an empty oracle assertion",
                scenario.scenario_id
            ));
        }
        if scenario.allowed_evidence_strength.is_empty() {
            return Err(format!(
                "scenario {} missing allowed evidence strength",
                scenario.scenario_id
            ));
        }
        if scenario
            .allowed_evidence_strength
            .iter()
            .any(|strength| strength.trim().is_empty())
        {
            return Err(format!(
                "scenario {} has an empty evidence strength",
                scenario.scenario_id
            ));
        }
        for strength in &scenario.allowed_evidence_strength {
            if !ALLOWED_STRENGTHS.contains(&strength.as_str()) {
                return Err(format!(
                    "scenario {} has unrecognized evidence strength {strength}",
                    scenario.scenario_id
                ));
            }
        }
        validate_fault_semantics(scenario)?;
        if scenario.expected_recovery_class == ExpectedRecoveryClass::SafeAutomaticRecovery
            && !scenario.reopen_required
        {
            return Err(format!(
                "recovery scenario {} must require reopen",
                scenario.scenario_id
            ));
        }
        if scenario.scenario_id == "unknown-newer-format" {
            if scenario.writable_open {
                return Err("unknown-newer-format must forbid writable open".to_owned());
            }
            match &scenario.read_only_open {
                ReadOnlyOpenPolicy::ConditionallyAllowed { required_condition } => {
                    if required_condition != "interpretation_demonstrably_safe" {
                        return Err(
                            "unknown-newer-format must declare a demonstrably-safe condition"
                                .to_owned(),
                        );
                    }
                }
                ReadOnlyOpenPolicy::Allowed => {
                    return Err(
                        "unknown-newer-format must not unconditionally allow read-only open"
                            .to_owned(),
                    );
                }
                ReadOnlyOpenPolicy::Forbidden => {
                    return Err(
                        "unknown-newer-format must declare conditionally safe read-only access"
                            .to_owned(),
                    );
                }
            }
        }
        if scenario.requirement == ScenarioRequirementLevel::CapabilityDependent
            && scenario
                .capability_requirement
                .as_deref()
                .is_none_or(|value| value.trim().is_empty())
        {
            return Err(format!(
                "capability-dependent scenario {} missing capability requirement",
                scenario.scenario_id
            ));
        }
        if scenario.platform_requirement.is_empty() {
            return Err(format!(
                "scenario {} missing platform requirement",
                scenario.scenario_id
            ));
        }
        for platform in &scenario.platform_requirement {
            if platform != "macos_native" && platform != "windows_github_actions" {
                return Err(format!(
                    "scenario {} has invalid platform label {platform}",
                    scenario.scenario_id
                ));
            }
        }
    }
    let required = required_catalog
        .iter()
        .filter(|scenario| scenario.requirement == ScenarioRequirementLevel::Required)
        .map(|scenario| format!("{}@{}", scenario.scenario_id, scenario.scenario_version))
        .collect::<std::collections::BTreeSet<_>>();
    if !required.is_subset(&ids) {
        return Err("scenario catalog is missing a required scenario".to_owned());
    }
    Ok(())
}

fn validate_v4_authoritative_corruption_semantics(
    scenarios: &[ScenarioContractV3],
) -> Result<(), String> {
    for scenario_id in ["canonical-reference-corruption", "source-locator-corruption"] {
        let scenario = scenarios
            .iter()
            .find(|scenario| scenario.scenario_id == scenario_id)
            .ok_or_else(|| format!("v4 catalog missing {scenario_id}"))?;
        if scenario.scenario_version != 2 {
            return Err(format!("{scenario_id} must be scenario_version 2 in v4"));
        }
        if scenario.expected_open_state != ExpectedOpenState::Unrecoverable {
            return Err(format!("{scenario_id} must expect unrecoverable open state in v4"));
        }
        if scenario.read_only_open != ReadOnlyOpenPolicy::Forbidden {
            return Err(format!("{scenario_id} must forbid read-only open in v4"));
        }
        if scenario.writable_open {
            return Err(format!("{scenario_id} must forbid writable open in v4"));
        }
        if scenario.expected_recovery_class != ExpectedRecoveryClass::ManualReviewRequired {
            return Err(format!(
                "{scenario_id} must expect manual_review_required recovery in v4"
            ));
        }
        if scenario.expected_durable_boundary != "corruption_detected_fail_closed" {
            return Err(format!("{scenario_id} must declare fail-closed durable boundary"));
        }
    }
    Ok(())
}

fn validate_fault_semantics(scenario: &ScenarioContractV3) -> Result<(), String> {
    match scenario.fault_layer {
        FaultLayer::NoFault => {
            if scenario.fault_point != "none" {
                return Err(format!(
                    "scenario {} with NoFault must use fault_point none",
                    scenario.scenario_id
                ));
            }
        }
        FaultLayer::ChildProcessAbortOrKill | FaultLayer::ProcessCrashRecovery => {
            if scenario.fault_point == "none" {
                return Err(format!(
                    "scenario {} requires explicit fault point",
                    scenario.scenario_id
                ));
            }
            for claim in ["FilesystemDurability", "HardwarePowerLoss", "OsCrash"] {
                if !scenario
                    .prohibited_claims
                    .iter()
                    .any(|prohibited| prohibited == claim)
                {
                    return Err(format!(
                        "process interruption scenario {} must prohibit {claim} claims",
                        scenario.scenario_id
                    ));
                }
            }
        }
        FaultLayer::MultiProcessConcurrency => {}
        _ => {}
    }
    if scenario
        .allowed_evidence_strength
        .contains(&"ProcessCrashRecovery".to_owned())
        && !matches!(
            scenario.fault_layer,
            FaultLayer::ChildProcessAbortOrKill | FaultLayer::ProcessCrashRecovery
        )
    {
        return Err(format!(
            "scenario {} claims ProcessCrashRecovery without process interruption fault",
            scenario.scenario_id
        ));
    }
    if scenario.scenario_id == "concurrent-writer-attempt"
        && scenario.fault_layer != FaultLayer::MultiProcessConcurrency
    {
        return Err("concurrent-writer-attempt must use MultiProcessConcurrency".to_owned());
    }
    if scenario.scenario_id == "writer-crash-and-takeover"
        && scenario.fault_layer != FaultLayer::ChildProcessAbortOrKill
    {
        return Err("writer-crash-and-takeover must use ChildProcessAbortOrKill".to_owned());
    }
    if scenario.scenario_id == "interrupted-compaction"
        && scenario.requirement != ScenarioRequirementLevel::CapabilityDependent
    {
        return Err("interrupted-compaction must be capability-dependent".to_owned());
    }
    Ok(())
}

pub fn required_scenario_ids_for_both_candidates() -> Vec<String> {
    scenario_contract_v4()
        .into_iter()
        .filter(|scenario| scenario.requirement == ScenarioRequirementLevel::Required)
        .map(|scenario| scenario.scenario_id)
        .collect()
}
