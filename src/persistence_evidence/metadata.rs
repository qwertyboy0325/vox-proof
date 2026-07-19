use std::collections::BTreeSet;

use serde_json::Value;

use super::readiness_result::{MetadataValidationFailure, ReadinessBlockerCode};

pub const SUPPORTED_CONTRACT_VERSION: &str = "4";
pub const SUPPORTED_SCENARIO_CATALOG_VERSION: &str = "1";

const REQUIRED_SCENARIO_FIELDS: &[&str] = &[
    "scenario_id",
    "scenario_version",
    "claim_structure",
    "intended_claim",
    "intended_claims",
    "current_demonstrated_claim",
    "pre_state",
    "operation",
    "fault_point",
    "fault_layer",
    "persisted_observation",
    "reopen_or_recovery_step",
    "expected_result",
    "required_error_classification",
    "forbidden_shortcuts",
    "current_assertion_summary",
    "demonstrated_observation_kind",
    "current_evidence_strength",
    "target_evidence_strength_for_this_scenario",
    "related_higher_level_claims_requiring_separate_scenarios",
    "current_gap",
    "correction_required",
    "current_status",
    "invariant_kind",
    "invariant_metadata",
];
const REQUIRED_CORRECTION_FALSE_SEMANTICS: &str = "no_immediate_correction_not_readiness_credit";
const REQUIRED_AGGREGATION_EXCLUSIONS: &[&str] =
    &["Unsupported", "NotRun", "deferred", "capability_missing"];

const REQUIRED_V4_RESOLVED_CONFLICT_IDS: &[&str] = &[
    "V3-C001", "V3-C002", "V3-C003", "V3-C004", "V3-C005", "V3-C006",
];
const REQUIRED_PRIOR_REVIEW_COMMIT: &str = "f9c746e2f09eb589e5a767d236432b90865d8779";

const BLOCKING_CANDIDATE_CLASSIFICATIONS: &[&str] = &[
    "not_demonstrated",
    "not_demonstrated_unsafe_under_wal",
    "insufficiently_demonstrated",
    "blocking_defect",
    "logical_transition_only",
    "authority_model_not_demonstrated",
];

struct CandidateBlockerRequirement {
    candidate_id: &'static str,
    field_key: &'static str,
    blocker_id: &'static str,
    label: &'static str,
}

const REQUIRED_CANDIDATE_BLOCKERS: &[CandidateBlockerRequirement] = &[
    CandidateBlockerRequirement {
        candidate_id: "embedded-relational-sqlite-spike",
        field_key: "duplication",
        blocker_id: "embedded-relational-wal-safe-duplication-not-demonstrated",
        label: "WAL-safe duplication or backup protocol not demonstrated",
    },
    CandidateBlockerRequirement {
        candidate_id: "embedded-relational-sqlite-spike",
        field_key: "writer_crash_recovery",
        blocker_id: "embedded-relational-writer-crash-takeover-not-demonstrated",
        label: "writer crash/takeover recovery not demonstrated",
    },
    CandidateBlockerRequirement {
        candidate_id: "embedded-relational-sqlite-spike",
        field_key: "relational_model_conformance",
        blocker_id: "embedded-relational-authority-model-not-demonstrated",
        label: "declared relational authority model not demonstrated",
    },
    CandidateBlockerRequirement {
        candidate_id: "embedded-relational-sqlite-spike",
        field_key: "durability_claim_strength",
        blocker_id: "embedded-relational-filesystem-durability-not-demonstrated",
        label: "filesystem or power-loss durability not demonstrated",
    },
    CandidateBlockerRequirement {
        candidate_id: "append-bundle-log-spike",
        field_key: "replay_behavior",
        blocker_id: "append-bundle-authoritative-replay-not-demonstrated",
        label: "authoritative replay not demonstrated",
    },
    CandidateBlockerRequirement {
        candidate_id: "append-bundle-log-spike",
        field_key: "compaction_recovery",
        blocker_id: "append-bundle-checkpoint-log-consistency-not-demonstrated",
        label: "checkpoint/log consistency or compaction recovery not demonstrated",
    },
    CandidateBlockerRequirement {
        candidate_id: "append-bundle-log-spike",
        field_key: "writer_crash_recovery",
        blocker_id: "append-bundle-writer-crash-takeover-not-demonstrated",
        label: "writer crash/takeover recovery not demonstrated",
    },
    CandidateBlockerRequirement {
        candidate_id: "append-bundle-log-spike",
        field_key: "durable_acknowledgement",
        blocker_id: "append-bundle-filesystem-durability-not-demonstrated",
        label: "filesystem or power-loss durability not demonstrated",
    },
];

const REQUIRED_SCENARIO_SEMANTIC_STRINGS: &[&str] = &[
    "pre_state",
    "operation",
    "fault_point",
    "fault_layer",
    "persisted_observation",
    "reopen_or_recovery_step",
    "expected_result",
    "required_error_classification",
    "current_assertion_summary",
    "current_gap",
];

const EVIDENCE_STRENGTHS: &[&str] = &[
    "InterfaceBehavior",
    "LogicalStateTransition",
    "ProcessCrashRecovery",
    "FilesystemDurability",
    "HardwarePowerLoss",
    "CrossPlatform",
];

const INVARIANT_KINDS: &[&str] = &[
    "rejected_operation_no_mutation",
    "successful_read_after_external_artifact_mutation",
    "pre_write_abort_no_authoritative_mutation",
    "unsupported_capability_not_executed",
    "not_applicable",
];

const CURRENT_STATUSES: &[&str] = &["partially_demonstrated", "not_demonstrated", "Unsupported"];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IntendedClaimObject {
    pub claim: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SubclaimContract {
    pub subclaim_id: String,
    pub intended_claim: String,
    pub current_status: String,
    pub current_demonstrated_claim: String,
    pub current_evidence_strength: Vec<String>,
    pub target_evidence_strength_for_this_subclaim: Vec<String>,
    pub current_gap: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScenarioContract {
    pub scenario_id: String,
    pub scenario_version: u64,
    pub claim_structure: String,
    pub intended_claim: Option<IntendedClaimObject>,
    pub intended_claims: Vec<SubclaimContract>,
    pub current_demonstrated_claim: Option<String>,
    pub demonstrated_observation_kind: Option<String>,
    pub current_evidence_strength: Vec<String>,
    pub target_evidence_strength_for_this_scenario: Vec<String>,
    pub current_status: String,
    pub invariant_kind: String,
    pub correction_required: bool,
    pub deferred_until_capability_exists: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AggregationRules {
    pub aggregation_exclusions: Vec<String>,
    pub correction_required_false_semantics: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidatedClaimContractDocument {
    pub contract_version: String,
    pub work_package_id: String,
    pub scenario_catalog_version: String,
    pub aggregation_rules: AggregationRules,
    pub scenarios: Vec<ScenarioContract>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RetainedCandidateBlocker {
    pub blocker_id: String,
    pub candidate_id: String,
    pub source_field: String,
    pub revised_classification: String,
    pub label: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReclassificationAuthority {
    pub mechanism_comparison_readiness: String,
    pub mechanism_selection_readiness: String,
    pub selection_status: String,
    pub fail_closed_invariant_present: bool,
    pub retained_blockers: Vec<RetainedCandidateBlocker>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidatedMetadataBundle {
    pub contract: ValidatedClaimContractDocument,
    pub reclassification: ReclassificationAuthority,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MetadataValidationError {
    pub failures: Vec<MetadataValidationFailure>,
}

impl MetadataValidationError {
    pub fn single(code: ReadinessBlockerCode, message: impl Into<String>) -> Self {
        Self {
            failures: vec![MetadataValidationFailure {
                code,
                message: message.into(),
                scenario_id: None,
                subclaim_id: None,
            }],
        }
    }

    pub fn primary_code(&self) -> ReadinessBlockerCode {
        self.failures
            .first()
            .map(|failure| failure.code)
            .unwrap_or(ReadinessBlockerCode::InvalidContract)
    }
}

pub fn parse_and_validate_contract(
    json: &str,
) -> Result<ValidatedClaimContractDocument, MetadataValidationError> {
    let root: Value = serde_json::from_str(json).map_err(|error| {
        MetadataValidationError::single(
            ReadinessBlockerCode::InvalidContract,
            format!("contract JSON parse failed: {error}"),
        )
    })?;

    let contract_version = required_string(&root, "contract_version").map_err(invalid_contract)?;
    if contract_version != SUPPORTED_CONTRACT_VERSION {
        return Err(MetadataValidationError::single(
            ReadinessBlockerCode::UnsupportedContractVersion,
            format!(
                "unsupported contract_version {contract_version}; supported {SUPPORTED_CONTRACT_VERSION}"
            ),
        ));
    }

    for field in [
        "work_package_id",
        "prior_contract_version",
        "review_commit_or_baseline",
        "scenario_catalog_version",
        "aggregation_rules",
        "claim_structure_schema_rules",
        "evidence_strength_taxonomy_invariant",
        "contracts",
        "invariant_kind_taxonomy",
        "final_conflict_correction",
    ] {
        if root.get(field).is_none() {
            return Err(invalid_contract(format!("missing top-level field {field}")));
        }
    }

    require_non_empty_string(&root, "work_package_id")?;
    require_non_empty_string(&root, "review_commit_or_baseline")?;

    validate_final_conflict_correction(root.get("final_conflict_correction").expect("checked"))?;

    let scenario_catalog_version =
        required_string(&root, "scenario_catalog_version").map_err(invalid_contract)?;
    if scenario_catalog_version != SUPPORTED_SCENARIO_CATALOG_VERSION {
        return Err(invalid_contract(format!(
            "unsupported scenario_catalog_version {scenario_catalog_version}"
        )));
    }

    let aggregation_rules =
        validate_aggregation_rules(root.get("aggregation_rules").ok_or_else(|| {
            MetadataValidationError::single(
                ReadinessBlockerCode::InvalidContract,
                "missing aggregation_rules",
            )
        })?)?;

    let contracts = root
        .get("contracts")
        .and_then(Value::as_array)
        .ok_or_else(|| {
            MetadataValidationError::single(
                ReadinessBlockerCode::InvalidContract,
                "missing contracts array",
            )
        })?;

    let mut scenarios = Vec::with_capacity(contracts.len());
    let mut seen_ids = BTreeSet::new();

    for contract in contracts {
        let scenario = validate_scenario_contract(contract)?;
        if !seen_ids.insert(scenario.scenario_id.clone()) {
            return Err(MetadataValidationError::single(
                ReadinessBlockerCode::DuplicateScenarioId,
                format!("duplicate scenario_id {}", scenario.scenario_id),
            ));
        }
        scenarios.push(scenario);
    }

    Ok(ValidatedClaimContractDocument {
        contract_version,
        work_package_id: required_string(&root, "work_package_id").map_err(invalid_contract)?,
        scenario_catalog_version: required_string(&root, "scenario_catalog_version")
            .map_err(invalid_contract)?,
        aggregation_rules,
        scenarios,
    })
}

pub fn parse_and_validate_reclassification(
    json: &str,
) -> Result<ReclassificationAuthority, MetadataValidationError> {
    let root: Value = serde_json::from_str(json).map_err(|error| {
        MetadataValidationError::single(
            ReadinessBlockerCode::MissingReclassificationAuthority,
            format!("reclassification JSON parse failed: {error}"),
        )
    })?;

    let revised = root.get("revised_classification").ok_or_else(|| {
        MetadataValidationError::single(
            ReadinessBlockerCode::MissingReclassificationAuthority,
            "missing revised_classification",
        )
    })?;

    let fail_closed = root.get("fail_closed_readiness_invariant").ok_or_else(|| {
        MetadataValidationError::single(
            ReadinessBlockerCode::MissingReclassificationAuthority,
            "missing fail_closed_readiness_invariant",
        )
    })?;
    if !fail_closed.is_object() || required_string(fail_closed, "statement").is_err() {
        return Err(MetadataValidationError::single(
            ReadinessBlockerCode::MissingReclassificationAuthority,
            "fail_closed_readiness_invariant must be an object with statement",
        ));
    }

    let fail_closed_invariant_present = true;

    let mechanism_comparison_readiness = required_string(revised, "mechanism_comparison_readiness")
        .map_err(|message| {
            MetadataValidationError::single(
                ReadinessBlockerCode::MissingReclassificationAuthority,
                message,
            )
        })?;
    let mechanism_selection_readiness = required_string(revised, "mechanism_selection_readiness")
        .map_err(|message| {
        MetadataValidationError::single(
            ReadinessBlockerCode::MissingReclassificationAuthority,
            message,
        )
    })?;
    let selection_status = required_string(revised, "selection_status").map_err(|message| {
        MetadataValidationError::single(
            ReadinessBlockerCode::MissingReclassificationAuthority,
            message,
        )
    })?;

    validate_reclassification_readiness_fields(
        &mechanism_comparison_readiness,
        &mechanism_selection_readiness,
        &selection_status,
        fail_closed_invariant_present,
    )?;

    let retained_blockers = parse_and_validate_candidate_specific_blockers(&root)?;

    Ok(ReclassificationAuthority {
        mechanism_comparison_readiness,
        mechanism_selection_readiness,
        selection_status,
        fail_closed_invariant_present,
        retained_blockers,
    })
}

fn validate_reclassification_readiness_fields(
    mechanism_comparison_readiness: &str,
    mechanism_selection_readiness: &str,
    selection_status: &str,
    fail_closed_invariant_present: bool,
) -> Result<(), MetadataValidationError> {
    const ALLOWED_READINESS: &[&str] = &["not_ready", "ready"];
    if !ALLOWED_READINESS.contains(&mechanism_comparison_readiness) {
        return Err(MetadataValidationError::single(
            ReadinessBlockerCode::MissingReclassificationAuthority,
            format!("unknown mechanism_comparison_readiness {mechanism_comparison_readiness}"),
        ));
    }
    if !ALLOWED_READINESS.contains(&mechanism_selection_readiness) {
        return Err(MetadataValidationError::single(
            ReadinessBlockerCode::MissingReclassificationAuthority,
            format!("unknown mechanism_selection_readiness {mechanism_selection_readiness}"),
        ));
    }
    if fail_closed_invariant_present {
        if mechanism_comparison_readiness != "not_ready" {
            return Err(MetadataValidationError::single(
                ReadinessBlockerCode::MissingReclassificationAuthority,
                "fail_closed_readiness_invariant requires mechanism_comparison_readiness not_ready",
            ));
        }
        if mechanism_selection_readiness != "not_ready" {
            return Err(MetadataValidationError::single(
                ReadinessBlockerCode::MissingReclassificationAuthority,
                "fail_closed_readiness_invariant requires mechanism_selection_readiness not_ready",
            ));
        }
        if selection_status != "none" {
            return Err(MetadataValidationError::single(
                ReadinessBlockerCode::MissingReclassificationAuthority,
                "fail_closed_readiness_invariant requires selection_status none",
            ));
        }
    }
    Ok(())
}

pub fn parse_and_validate_metadata_bundle(
    contract_json: &str,
    reclassification_json: &str,
) -> Result<ValidatedMetadataBundle, MetadataValidationError> {
    Ok(ValidatedMetadataBundle {
        contract: parse_and_validate_contract(contract_json)?,
        reclassification: parse_and_validate_reclassification(reclassification_json)?,
    })
}

fn validate_aggregation_rules(value: &Value) -> Result<AggregationRules, MetadataValidationError> {
    let exclusions = value
        .get("aggregation_exclusions")
        .and_then(Value::as_array)
        .ok_or_else(|| {
            MetadataValidationError::single(
                ReadinessBlockerCode::InvalidContract,
                "missing aggregation_exclusions",
            )
        })?;
    let mut parsed_exclusions = Vec::with_capacity(exclusions.len());
    for item in exclusions {
        let exclusion = item.as_str().ok_or_else(|| {
            MetadataValidationError::single(
                ReadinessBlockerCode::InvalidContract,
                "aggregation_exclusions entries must be strings",
            )
        })?;
        if exclusion.trim().is_empty() {
            return Err(MetadataValidationError::single(
                ReadinessBlockerCode::EmptyRequiredString,
                "aggregation_exclusions entries must be non-empty",
            ));
        }
        parsed_exclusions.push(exclusion.to_string());
    }

    for required in REQUIRED_AGGREGATION_EXCLUSIONS {
        if !parsed_exclusions.iter().any(|item| item == required) {
            return Err(MetadataValidationError::single(
                ReadinessBlockerCode::InvalidContract,
                format!("aggregation_exclusions missing required entry {required}"),
            ));
        }
    }

    let correction_required_false_semantics =
        required_string(value, "correction_required_false_semantics").map_err(invalid_contract)?;
    if correction_required_false_semantics != REQUIRED_CORRECTION_FALSE_SEMANTICS {
        return Err(MetadataValidationError::single(
            ReadinessBlockerCode::InvalidContract,
            "correction_required_false_semantics weakened or missing accepted value",
        ));
    }

    Ok(AggregationRules {
        aggregation_exclusions: parsed_exclusions,
        correction_required_false_semantics,
    })
}

fn validate_scenario_contract(value: &Value) -> Result<ScenarioContract, MetadataValidationError> {
    let scenario_id = require_non_empty_string(value, "scenario_id")?;

    if let Some(object) = value.as_object() {
        for field in REQUIRED_SCENARIO_FIELDS {
            if !object.contains_key(*field) {
                return Err(failure_for_scenario(
                    ReadinessBlockerCode::InvalidContract,
                    &scenario_id,
                    format!("missing required field {field}"),
                ));
            }
        }
    } else {
        return Err(failure_for_scenario(
            ReadinessBlockerCode::InvalidContract,
            &scenario_id,
            "scenario contract must be an object",
        ));
    }

    for field in REQUIRED_SCENARIO_SEMANTIC_STRINGS {
        require_non_empty_string(value, field).map_err(|error| {
            failure_for_scenario(
                error.primary_code(),
                &scenario_id,
                error.failures[0].message.clone(),
            )
        })?;
    }

    validate_scenario_string_array(value, "forbidden_shortcuts", true, &scenario_id)?;
    validate_scenario_string_array(
        value,
        "related_higher_level_claims_requiring_separate_scenarios",
        false,
        &scenario_id,
    )?;

    let scenario_version = value
        .get("scenario_version")
        .and_then(Value::as_u64)
        .ok_or_else(|| {
            MetadataValidationError::single(
                ReadinessBlockerCode::InvalidContract,
                "missing scenario_version",
            )
        })?;

    if !value
        .as_object()
        .map(|obj| obj.contains_key("demonstrated_observation_kind"))
        .unwrap_or(false)
    {
        return Err(failure_for_scenario(
            ReadinessBlockerCode::InvalidContract,
            &scenario_id,
            "missing demonstrated_observation_kind",
        ));
    }

    let claim_structure = require_non_empty_string(value, "claim_structure")?;
    if claim_structure != "single" && claim_structure != "multiple" {
        return Err(failure_for_scenario(
            ReadinessBlockerCode::InvalidClaimStructure,
            &scenario_id,
            format!("unknown claim_structure {claim_structure}"),
        ));
    }

    let current_status = require_non_empty_string(value, "current_status")?;
    validate_current_status(&current_status)?;

    let invariant_kind = require_non_empty_string(value, "invariant_kind")?;
    validate_invariant_kind(&invariant_kind)?;

    let invariant_metadata = value.get("invariant_metadata").ok_or_else(|| {
        failure_for_scenario(
            ReadinessBlockerCode::InvalidContract,
            &scenario_id,
            "missing invariant_metadata",
        )
    })?;
    validate_invariant_metadata(&scenario_id, &invariant_kind, invariant_metadata)?;

    let demonstrated_observation_kind = match value.get("demonstrated_observation_kind") {
        Some(Value::Null) => None,
        Some(Value::String(text)) if !text.trim().is_empty() => Some(text.clone()),
        Some(Value::String(_)) => {
            return Err(failure_for_scenario(
                ReadinessBlockerCode::EmptyRequiredString,
                &scenario_id,
                "demonstrated_observation_kind string must be non-empty when present",
            ));
        }
        _ => {
            return Err(failure_for_scenario(
                ReadinessBlockerCode::InvalidContract,
                &scenario_id,
                "demonstrated_observation_kind must be string or null",
            ));
        }
    };

    let current_evidence_strength =
        parse_evidence_strength_array(value.get("current_evidence_strength"), &scenario_id, None)?;
    let target_evidence_strength_for_this_scenario = parse_evidence_strength_array(
        value.get("target_evidence_strength_for_this_scenario"),
        &scenario_id,
        None,
    )?;

    let correction_required = value
        .get("correction_required")
        .and_then(Value::as_bool)
        .ok_or_else(|| {
            failure_for_scenario(
                ReadinessBlockerCode::InvalidContract,
                &scenario_id,
                "missing correction_required",
            )
        })?;

    let deferred_until_capability_exists = match value.get("deferred_until_capability_exists") {
        None => false,
        Some(Value::Bool(value)) => *value,
        Some(Value::Null) => {
            return Err(failure_for_scenario(
                ReadinessBlockerCode::InvalidContract,
                &scenario_id,
                "deferred_until_capability_exists must not be null",
            ));
        }
        Some(_) => {
            return Err(failure_for_scenario(
                ReadinessBlockerCode::InvalidContract,
                &scenario_id,
                "deferred_until_capability_exists must be boolean",
            ));
        }
    };

    if current_status == "Unsupported" {
        validate_unsupported_scenario_shape(value, &scenario_id)?;
    }

    validate_status_evidence_consistency(
        &scenario_id,
        None,
        &current_status,
        &current_evidence_strength,
        demonstrated_observation_kind.as_deref(),
        deferred_until_capability_exists,
        claim_structure == "multiple",
    )?;

    let (intended_claim, intended_claims, current_demonstrated_claim) = match claim_structure
        .as_str()
    {
        "single" => {
            let intended = value.get("intended_claim").ok_or_else(|| {
                failure_for_scenario(
                    ReadinessBlockerCode::InvalidClaimStructure,
                    &scenario_id,
                    "missing intended_claim",
                )
            })?;
            if intended.is_null() {
                return Err(failure_for_scenario(
                    ReadinessBlockerCode::InvalidClaimStructure,
                    &scenario_id,
                    "single claim_structure requires non-null intended_claim",
                ));
            }
            let claim = require_non_empty_string(intended, "claim").map_err(|error| {
                failure_for_scenario(
                    error.primary_code(),
                    &scenario_id,
                    error.failures[0].message.clone(),
                )
            })?;

            let subclaims = value
                .get("intended_claims")
                .and_then(Value::as_array)
                .ok_or_else(|| {
                    failure_for_scenario(
                        ReadinessBlockerCode::InvalidClaimStructure,
                        &scenario_id,
                        "missing intended_claims",
                    )
                })?;
            if !subclaims.is_empty() {
                return Err(failure_for_scenario(
                    ReadinessBlockerCode::InvalidClaimStructure,
                    &scenario_id,
                    "single claim_structure requires empty intended_claims",
                ));
            }

            let demonstrated = value
                .get("current_demonstrated_claim")
                .and_then(Value::as_str)
                .filter(|text| !text.trim().is_empty())
                .ok_or_else(|| {
                    failure_for_scenario(
                        ReadinessBlockerCode::EmptyRequiredString,
                        &scenario_id,
                        "single claim_structure requires non-empty current_demonstrated_claim string",
                    )
                })?;

            (
                Some(IntendedClaimObject { claim }),
                Vec::new(),
                Some(demonstrated.to_string()),
            )
        }
        "multiple" => {
            if !value
                .get("intended_claim")
                .map(|v| v.is_null())
                .unwrap_or(false)
            {
                return Err(failure_for_scenario(
                    ReadinessBlockerCode::InvalidClaimStructure,
                    &scenario_id,
                    "multiple claim_structure requires intended_claim null",
                ));
            }
            if !value
                .get("current_demonstrated_claim")
                .map(|v| v.is_null())
                .unwrap_or(false)
            {
                return Err(failure_for_scenario(
                    ReadinessBlockerCode::InvalidClaimStructure,
                    &scenario_id,
                    "multiple claim_structure requires scenario-level current_demonstrated_claim null",
                ));
            }

            if !current_evidence_strength.is_empty() {
                return Err(failure_for_scenario(
                    ReadinessBlockerCode::InvalidClaimStructure,
                    &scenario_id,
                    "multiple claim_structure requires empty scenario-level current_evidence_strength",
                ));
            }
            if !target_evidence_strength_for_this_scenario.is_empty() {
                return Err(failure_for_scenario(
                    ReadinessBlockerCode::InvalidClaimStructure,
                    &scenario_id,
                    "multiple claim_structure requires empty scenario-level target_evidence_strength_for_this_scenario",
                ));
            }

            let subclaims_value = value
                .get("intended_claims")
                .and_then(Value::as_array)
                .ok_or_else(|| {
                    failure_for_scenario(
                        ReadinessBlockerCode::InvalidClaimStructure,
                        &scenario_id,
                        "missing intended_claims",
                    )
                })?;
            if subclaims_value.is_empty() {
                return Err(failure_for_scenario(
                    ReadinessBlockerCode::InvalidClaimStructure,
                    &scenario_id,
                    "multiple claim_structure requires non-empty intended_claims",
                ));
            }

            let mut seen_subclaims = BTreeSet::new();
            let mut subclaims = Vec::with_capacity(subclaims_value.len());
            for subclaim in subclaims_value {
                let subclaim_id =
                    require_non_empty_string(subclaim, "subclaim_id").map_err(|error| {
                        failure_for_scenario(
                            error.primary_code(),
                            &scenario_id,
                            error.failures[0].message.clone(),
                        )
                    })?;
                if !seen_subclaims.insert(subclaim_id.clone()) {
                    return Err(MetadataValidationError {
                        failures: vec![MetadataValidationFailure {
                            code: ReadinessBlockerCode::DuplicateSubclaimId,
                            message: format!("duplicate subclaim_id {subclaim_id}"),
                            scenario_id: Some(scenario_id.clone()),
                            subclaim_id: Some(subclaim_id),
                        }],
                    });
                }

                let sub_status =
                    require_non_empty_string(subclaim, "current_status").map_err(|error| {
                        failure_for_scenario(
                            error.primary_code(),
                            &scenario_id,
                            error.failures[0].message.clone(),
                        )
                    })?;
                validate_current_status(&sub_status)?;

                let sub_strength = parse_evidence_strength_array(
                    subclaim.get("current_evidence_strength"),
                    &scenario_id,
                    Some(&subclaim_id),
                )?;
                let sub_target = parse_evidence_strength_array(
                    subclaim.get("target_evidence_strength_for_this_subclaim"),
                    &scenario_id,
                    Some(&subclaim_id),
                )?;

                validate_status_evidence_consistency(
                    &scenario_id,
                    Some(&subclaim_id),
                    &sub_status,
                    &sub_strength,
                    None,
                    false,
                    false,
                )?;

                subclaims.push(SubclaimContract {
                    subclaim_id,
                    intended_claim: require_non_empty_string(subclaim, "intended_claim").map_err(
                        |error| {
                            failure_for_scenario(
                                error.primary_code(),
                                &scenario_id,
                                error.failures[0].message.clone(),
                            )
                        },
                    )?,
                    current_status: sub_status,
                    current_demonstrated_claim: require_non_empty_string(
                        subclaim,
                        "current_demonstrated_claim",
                    )
                    .map_err(|error| {
                        failure_for_scenario(
                            error.primary_code(),
                            &scenario_id,
                            error.failures[0].message.clone(),
                        )
                    })?,
                    current_evidence_strength: sub_strength,
                    target_evidence_strength_for_this_subclaim: sub_target,
                    current_gap: require_non_empty_string(subclaim, "current_gap").map_err(
                        |error| {
                            failure_for_scenario(
                                error.primary_code(),
                                &scenario_id,
                                error.failures[0].message.clone(),
                            )
                        },
                    )?,
                });
            }

            (None, subclaims, None)
        }
        _ => unreachable!(),
    };

    Ok(ScenarioContract {
        scenario_id,
        scenario_version,
        claim_structure,
        intended_claim,
        intended_claims,
        current_demonstrated_claim,
        demonstrated_observation_kind,
        current_evidence_strength,
        target_evidence_strength_for_this_scenario,
        current_status,
        invariant_kind,
        correction_required,
        deferred_until_capability_exists,
    })
}

fn validate_current_status(status: &str) -> Result<(), MetadataValidationError> {
    if CURRENT_STATUSES.contains(&status) {
        Ok(())
    } else {
        Err(MetadataValidationError::single(
            ReadinessBlockerCode::InvalidContract,
            format!("unknown current_status {status}"),
        ))
    }
}

fn validate_invariant_kind(kind: &str) -> Result<(), MetadataValidationError> {
    if INVARIANT_KINDS.contains(&kind) {
        Ok(())
    } else {
        Err(MetadataValidationError::single(
            ReadinessBlockerCode::InvalidContract,
            format!("unknown invariant_kind {kind}"),
        ))
    }
}

fn parse_evidence_strength_array(
    value: Option<&Value>,
    scenario_id: &str,
    subclaim_id: Option<&str>,
) -> Result<Vec<String>, MetadataValidationError> {
    let array = value.and_then(Value::as_array).ok_or_else(|| {
        if let Some(subclaim_id) = subclaim_id {
            MetadataValidationError {
                failures: vec![MetadataValidationFailure {
                    code: ReadinessBlockerCode::InvalidContract,
                    message: "missing current_evidence_strength array".to_string(),
                    scenario_id: Some(scenario_id.to_string()),
                    subclaim_id: Some(subclaim_id.to_string()),
                }],
            }
        } else {
            failure_for_scenario(
                ReadinessBlockerCode::InvalidContract,
                scenario_id,
                "missing current_evidence_strength array",
            )
        }
    })?;

    let mut strengths = Vec::with_capacity(array.len());
    for item in array {
        let strength = item.as_str().ok_or_else(|| {
            failure_for_scenario(
                ReadinessBlockerCode::InvalidContract,
                scenario_id,
                "evidence strength entries must be strings",
            )
        })?;
        if !EVIDENCE_STRENGTHS.contains(&strength) {
            return Err(if let Some(subclaim_id) = subclaim_id {
                MetadataValidationError {
                    failures: vec![MetadataValidationFailure {
                        code: ReadinessBlockerCode::InvalidContract,
                        message: format!("unknown evidence strength {strength}"),
                        scenario_id: Some(scenario_id.to_string()),
                        subclaim_id: Some(subclaim_id.to_string()),
                    }],
                }
            } else {
                failure_for_scenario(
                    ReadinessBlockerCode::InvalidContract,
                    scenario_id,
                    format!("unknown evidence strength {strength}"),
                )
            });
        }
        strengths.push(strength.to_string());
    }
    Ok(strengths)
}

fn validate_status_evidence_consistency(
    scenario_id: &str,
    subclaim_id: Option<&str>,
    current_status: &str,
    current_evidence_strength: &[String],
    demonstrated_observation_kind: Option<&str>,
    deferred_until_capability_exists: bool,
    allow_empty_partial_at_scenario_level: bool,
) -> Result<(), MetadataValidationError> {
    if current_status == "not_demonstrated" && !current_evidence_strength.is_empty() {
        return Err(status_evidence_failure(
            scenario_id,
            subclaim_id,
            "not_demonstrated requires empty current_evidence_strength",
        ));
    }

    if current_status == "partially_demonstrated"
        && current_evidence_strength.is_empty()
        && !allow_empty_partial_at_scenario_level
    {
        return Err(status_evidence_failure(
            scenario_id,
            subclaim_id,
            "partially_demonstrated requires non-empty current_evidence_strength",
        ));
    }

    if current_status == "Unsupported" {
        if !current_evidence_strength.is_empty() {
            return Err(status_evidence_failure(
                scenario_id,
                subclaim_id,
                "Unsupported status requires empty current_evidence_strength",
            ));
        }
        if demonstrated_observation_kind.is_some() {
            return Err(status_evidence_failure(
                scenario_id,
                subclaim_id,
                "Unsupported status requires demonstrated_observation_kind null",
            ));
        }
    }

    if current_status == "not_demonstrated" && !current_evidence_strength.is_empty() {
        return Err(status_evidence_failure(
            scenario_id,
            subclaim_id,
            "not_demonstrated requires empty current_evidence_strength",
        ));
    }

    if deferred_until_capability_exists && !current_evidence_strength.is_empty() {
        return Err(status_evidence_failure(
            scenario_id,
            subclaim_id,
            "deferred_until_capability_exists requires empty current_evidence_strength",
        ));
    }

    Ok(())
}

fn invalid_contract(message: impl Into<String>) -> MetadataValidationError {
    MetadataValidationError::single(ReadinessBlockerCode::InvalidContract, message)
}

fn require_non_empty_string(value: &Value, field: &str) -> Result<String, MetadataValidationError> {
    match value.get(field) {
        None => Err(MetadataValidationError::single(
            ReadinessBlockerCode::InvalidContract,
            format!("missing string field {field}"),
        )),
        Some(Value::Null) => Err(MetadataValidationError::single(
            ReadinessBlockerCode::InvalidContract,
            format!("null string field {field}"),
        )),
        Some(Value::String(text)) if text.trim().is_empty() => {
            Err(MetadataValidationError::single(
                ReadinessBlockerCode::EmptyRequiredString,
                format!("empty required string field {field}"),
            ))
        }
        Some(Value::String(text)) => Ok(text.to_string()),
        Some(_) => Err(MetadataValidationError::single(
            ReadinessBlockerCode::InvalidContract,
            format!("invalid string field {field}"),
        )),
    }
}

fn validate_final_conflict_correction(value: &Value) -> Result<(), MetadataValidationError> {
    let object = value.as_object().ok_or_else(|| {
        MetadataValidationError::single(
            ReadinessBlockerCode::InvalidFinalConflictCorrection,
            "final_conflict_correction must be an object",
        )
    })?;

    let ids = object
        .get("resolved_conflict_ids")
        .and_then(Value::as_array)
        .ok_or_else(|| {
            MetadataValidationError::single(
                ReadinessBlockerCode::InvalidFinalConflictCorrection,
                "missing resolved_conflict_ids array",
            )
        })?;

    if ids.is_empty() {
        return Err(MetadataValidationError::single(
            ReadinessBlockerCode::InvalidFinalConflictCorrection,
            "resolved_conflict_ids must be non-empty",
        ));
    }

    let mut seen = BTreeSet::new();
    let mut parsed_ids = Vec::with_capacity(ids.len());
    for item in ids {
        let id = item.as_str().ok_or_else(|| {
            MetadataValidationError::single(
                ReadinessBlockerCode::InvalidFinalConflictCorrection,
                "resolved_conflict_ids entries must be strings",
            )
        })?;
        if id.trim().is_empty() {
            return Err(MetadataValidationError::single(
                ReadinessBlockerCode::EmptyRequiredString,
                "resolved_conflict_ids entries must be non-empty",
            ));
        }
        if !seen.insert(id.to_string()) {
            return Err(MetadataValidationError::single(
                ReadinessBlockerCode::InvalidFinalConflictCorrection,
                format!("duplicate resolved_conflict_id {id}"),
            ));
        }
        parsed_ids.push(id.to_string());
    }

    for required in REQUIRED_V4_RESOLVED_CONFLICT_IDS {
        if !parsed_ids.iter().any(|id| id == required) {
            return Err(MetadataValidationError::single(
                ReadinessBlockerCode::MissingResolvedConflictId,
                format!("missing resolved_conflict_id {required}"),
            ));
        }
    }

    for id in &parsed_ids {
        if !REQUIRED_V4_RESOLVED_CONFLICT_IDS.contains(&id.as_str()) {
            return Err(MetadataValidationError::single(
                ReadinessBlockerCode::UnexpectedResolvedConflictId,
                format!("unexpected resolved_conflict_id {id}"),
            ));
        }
    }

    let prior = object
        .get("prior_review_commit")
        .and_then(Value::as_str)
        .ok_or_else(|| {
            MetadataValidationError::single(
                ReadinessBlockerCode::InvalidFinalConflictCorrection,
                "missing prior_review_commit",
            )
        })?;
    if prior.trim().is_empty() {
        return Err(MetadataValidationError::single(
            ReadinessBlockerCode::EmptyRequiredString,
            "prior_review_commit must be non-empty",
        ));
    }
    if prior != REQUIRED_PRIOR_REVIEW_COMMIT {
        return Err(MetadataValidationError::single(
            ReadinessBlockerCode::InvalidPriorReviewCommit,
            format!("prior_review_commit must be {REQUIRED_PRIOR_REVIEW_COMMIT}"),
        ));
    }

    Ok(())
}

fn validate_unsupported_scenario_shape(
    value: &Value,
    scenario_id: &str,
) -> Result<(), MetadataValidationError> {
    let correction_required = value
        .get("correction_required")
        .and_then(Value::as_bool)
        .ok_or_else(|| {
            failure_for_scenario(
                ReadinessBlockerCode::InvalidUnsupportedScenarioShape,
                scenario_id,
                "missing correction_required",
            )
        })?;
    if correction_required {
        return Err(failure_for_scenario(
            ReadinessBlockerCode::InvalidUnsupportedScenarioShape,
            scenario_id,
            "Unsupported status requires correction_required false",
        ));
    }

    let deferred = value
        .get("deferred_until_capability_exists")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    if !deferred {
        return Err(failure_for_scenario(
            ReadinessBlockerCode::InvalidUnsupportedScenarioShape,
            scenario_id,
            "Unsupported status requires deferred_until_capability_exists true",
        ));
    }

    require_non_empty_string(value, "correction_required_false_note").map_err(|error| {
        failure_for_scenario(
            error.primary_code(),
            scenario_id,
            error.failures[0].message.clone(),
        )
    })?;

    let deferred_requirements = value
        .get("deferred_requirements_after_capability")
        .ok_or_else(|| {
            failure_for_scenario(
                ReadinessBlockerCode::InvalidUnsupportedScenarioShape,
                scenario_id,
                "missing deferred_requirements_after_capability",
            )
        })?;
    require_string_array(
        deferred_requirements,
        "deferred_requirements_after_capability",
    )
    .map_err(|error| {
        failure_for_scenario(
            error.primary_code(),
            scenario_id,
            error.failures[0].message.clone(),
        )
    })?;

    Ok(())
}

fn validate_invariant_metadata(
    scenario_id: &str,
    invariant_kind: &str,
    value: &Value,
) -> Result<(), MetadataValidationError> {
    let object = value.as_object().ok_or_else(|| {
        failure_for_scenario(
            ReadinessBlockerCode::InvalidInvariantMetadata,
            scenario_id,
            "invariant_metadata must be an object",
        )
    })?;

    match invariant_kind {
        "not_applicable" => {
            if !object.is_empty() {
                return Err(failure_for_scenario(
                    ReadinessBlockerCode::InvalidInvariantMetadata,
                    scenario_id,
                    "not_applicable invariant_metadata must be empty",
                ));
            }
        }
        "rejected_operation_no_mutation" => {
            for field in [
                "rejected_operation",
                "exact_error_or_policy_result",
                "independent_persisted_observation",
                "boundary_observed",
            ] {
                require_non_empty_string(value, field).map_err(|error| {
                    failure_for_scenario(
                        ReadinessBlockerCode::MissingInvariantField,
                        scenario_id,
                        error.failures[0].message.clone(),
                    )
                })?;
            }
            reject_unexpected_invariant_fields(
                scenario_id,
                object,
                &[
                    "rejected_operation",
                    "exact_error_or_policy_result",
                    "independent_persisted_observation",
                    "boundary_observed",
                    "does_not_describe_observation_as",
                ],
            )?;
            if let Some(extra) = object.get("does_not_describe_observation_as") {
                require_string_array(extra, "does_not_describe_observation_as").map_err(
                    |error| {
                        failure_for_scenario(
                            error.primary_code(),
                            scenario_id,
                            error.failures[0].message.clone(),
                        )
                    },
                )?;
            }
        }
        "successful_read_after_external_artifact_mutation" => {
            for field in [
                "mutated_artifact",
                "observation_boundary",
                "does_not_establish",
            ] {
                if value.get(field).is_none() {
                    return Err(failure_for_scenario(
                        ReadinessBlockerCode::MissingInvariantField,
                        scenario_id,
                        format!("missing invariant_metadata field {field}"),
                    ));
                }
            }
            require_non_empty_string(value, "mutated_artifact").map_err(|error| {
                failure_for_scenario(
                    error.primary_code(),
                    scenario_id,
                    error.failures[0].message.clone(),
                )
            })?;
            require_non_empty_string(value, "observation_boundary").map_err(|error| {
                failure_for_scenario(
                    error.primary_code(),
                    scenario_id,
                    error.failures[0].message.clone(),
                )
            })?;
            require_string_array(
                value.get("does_not_establish").expect("checked"),
                "does_not_establish",
            )
            .map_err(|error| {
                failure_for_scenario(
                    error.primary_code(),
                    scenario_id,
                    error.failures[0].message.clone(),
                )
            })?;
            reject_unexpected_invariant_fields(
                scenario_id,
                object,
                &[
                    "mutated_artifact",
                    "observation_boundary",
                    "does_not_establish",
                ],
            )?;
        }
        "pre_write_abort_no_authoritative_mutation" => {
            for field in [
                "aborted_operation",
                "exact_error_or_policy_result",
                "fault_point",
                "observation_boundary",
                "independent_persisted_observation",
            ] {
                require_non_empty_string(value, field).map_err(|error| {
                    failure_for_scenario(
                        ReadinessBlockerCode::MissingInvariantField,
                        scenario_id,
                        error.failures[0].message.clone(),
                    )
                })?;
            }
            reject_unexpected_invariant_fields(
                scenario_id,
                object,
                &[
                    "aborted_operation",
                    "exact_error_or_policy_result",
                    "fault_point",
                    "observation_boundary",
                    "independent_persisted_observation",
                ],
            )?;
        }
        "unsupported_capability_not_executed" => {
            for field in [
                "missing_capability",
                "aggregation_exclusion",
                "deferred_until_capability_exists",
            ] {
                if value.get(field).is_none() {
                    return Err(failure_for_scenario(
                        ReadinessBlockerCode::MissingInvariantField,
                        scenario_id,
                        format!("missing invariant_metadata field {field}"),
                    ));
                }
            }
            require_non_empty_string(value, "missing_capability").map_err(|error| {
                failure_for_scenario(
                    error.primary_code(),
                    scenario_id,
                    error.failures[0].message.clone(),
                )
            })?;
            require_non_empty_string(value, "aggregation_exclusion").map_err(|error| {
                failure_for_scenario(
                    error.primary_code(),
                    scenario_id,
                    error.failures[0].message.clone(),
                )
            })?;
            match value.get("deferred_until_capability_exists") {
                Some(Value::Bool(true)) => {}
                _ => {
                    return Err(failure_for_scenario(
                        ReadinessBlockerCode::InvalidUnsupportedScenarioShape,
                        scenario_id,
                        "unsupported_capability_not_executed requires deferred_until_capability_exists true in invariant_metadata",
                    ));
                }
            }
            reject_unexpected_invariant_fields(
                scenario_id,
                object,
                &[
                    "missing_capability",
                    "aggregation_exclusion",
                    "deferred_until_capability_exists",
                ],
            )?;
        }
        _ => {
            return Err(failure_for_scenario(
                ReadinessBlockerCode::InvalidInvariantMetadata,
                scenario_id,
                format!("unsupported invariant_kind {invariant_kind} for metadata validation"),
            ));
        }
    }

    Ok(())
}

fn validate_scenario_string_array(
    value: &Value,
    field: &str,
    require_non_empty: bool,
    scenario_id: &str,
) -> Result<(), MetadataValidationError> {
    let array_value = value.get(field).ok_or_else(|| {
        failure_for_scenario(
            ReadinessBlockerCode::InvalidContract,
            scenario_id,
            format!("missing {field}"),
        )
    })?;
    match array_value {
        Value::Null => Err(failure_for_scenario(
            ReadinessBlockerCode::InvalidContract,
            scenario_id,
            format!("{field} must not be null"),
        )),
        Value::Array(items) => {
            if require_non_empty && items.is_empty() {
                return Err(failure_for_scenario(
                    ReadinessBlockerCode::InvalidContract,
                    scenario_id,
                    format!("{field} must be non-empty"),
                ));
            }
            for item in items {
                match item {
                    Value::String(text) if !text.trim().is_empty() => {}
                    Value::String(_) => {
                        return Err(failure_for_scenario(
                            ReadinessBlockerCode::EmptyRequiredString,
                            scenario_id,
                            format!("{field} entries must be non-empty strings"),
                        ));
                    }
                    Value::Null => {
                        return Err(failure_for_scenario(
                            ReadinessBlockerCode::InvalidContract,
                            scenario_id,
                            format!("{field} entries must not be null"),
                        ));
                    }
                    _ => {
                        return Err(failure_for_scenario(
                            ReadinessBlockerCode::InvalidContract,
                            scenario_id,
                            format!("{field} entries must be strings"),
                        ));
                    }
                }
            }
            Ok(())
        }
        _ => Err(failure_for_scenario(
            ReadinessBlockerCode::InvalidContract,
            scenario_id,
            format!("{field} must be an array"),
        )),
    }
}

fn reject_unexpected_invariant_fields(
    scenario_id: &str,
    object: &serde_json::Map<String, Value>,
    allowed: &[&str],
) -> Result<(), MetadataValidationError> {
    for key in object.keys() {
        if !allowed.contains(&key.as_str()) {
            return Err(failure_for_scenario(
                ReadinessBlockerCode::UnexpectedInvariantField,
                scenario_id,
                format!("unexpected invariant_metadata field {key}"),
            ));
        }
    }
    Ok(())
}

fn require_string_array(
    value: &Value,
    field: &str,
) -> Result<Vec<String>, MetadataValidationError> {
    let array = value.as_array().ok_or_else(|| {
        MetadataValidationError::single(
            ReadinessBlockerCode::InvalidInvariantMetadata,
            format!("{field} must be an array"),
        )
    })?;
    if array.is_empty() {
        return Err(MetadataValidationError::single(
            ReadinessBlockerCode::MissingInvariantField,
            format!("{field} must be non-empty"),
        ));
    }
    array
        .iter()
        .map(|item| match item {
            Value::String(text) if !text.trim().is_empty() => Ok(text.to_string()),
            Value::String(_) => Err(MetadataValidationError::single(
                ReadinessBlockerCode::EmptyRequiredString,
                format!("{field} entries must be non-empty strings"),
            )),
            Value::Null => Err(MetadataValidationError::single(
                ReadinessBlockerCode::InvalidContract,
                format!("{field} entries must not be null"),
            )),
            _ => Err(MetadataValidationError::single(
                ReadinessBlockerCode::InvalidContract,
                format!("{field} entries must be strings"),
            )),
        })
        .collect()
}

fn parse_and_validate_candidate_specific_blockers(
    root: &Value,
) -> Result<Vec<RetainedCandidateBlocker>, MetadataValidationError> {
    let candidate_specific = root.get("candidate_specific").ok_or_else(|| {
        MetadataValidationError::single(
            ReadinessBlockerCode::MissingCandidateSpecificBlockers,
            "missing candidate_specific",
        )
    })?;
    let object = candidate_specific.as_object().ok_or_else(|| {
        MetadataValidationError::single(
            ReadinessBlockerCode::MissingCandidateSpecificBlockers,
            "candidate_specific must be an object",
        )
    })?;
    if object.is_empty() {
        return Err(MetadataValidationError::single(
            ReadinessBlockerCode::MissingCandidateSpecificBlockers,
            "candidate_specific must not be empty",
        ));
    }

    let mut retained = Vec::with_capacity(REQUIRED_CANDIDATE_BLOCKERS.len());
    for requirement in REQUIRED_CANDIDATE_BLOCKERS {
        let candidate = object.get(requirement.candidate_id).ok_or_else(|| {
            MetadataValidationError::single(
                ReadinessBlockerCode::MissingCandidateBlocker,
                format!(
                    "missing candidate_specific entry {}",
                    requirement.candidate_id
                ),
            )
        })?;
        let candidate_object = candidate.as_object().ok_or_else(|| {
            MetadataValidationError::single(
                ReadinessBlockerCode::MalformedCandidateBlocker,
                format!(
                    "candidate_specific.{} must be an object",
                    requirement.candidate_id
                ),
            )
        })?;
        let blocker = candidate_object.get(requirement.field_key).ok_or_else(|| {
            MetadataValidationError::single(
                ReadinessBlockerCode::MissingCandidateBlocker,
                format!(
                    "missing candidate blocker {}.{}",
                    requirement.candidate_id, requirement.field_key
                ),
            )
        })?;
        let blocker_object = blocker.as_object().ok_or_else(|| {
            MetadataValidationError::single(
                ReadinessBlockerCode::MalformedCandidateBlocker,
                format!(
                    "candidate_specific.{}.{} must be an object",
                    requirement.candidate_id, requirement.field_key
                ),
            )
        })?;
        let revised = blocker_object
            .get("revised_classification")
            .and_then(Value::as_str)
            .ok_or_else(|| {
                MetadataValidationError::single(
                    ReadinessBlockerCode::MalformedCandidateBlocker,
                    format!(
                        "candidate_specific.{}.{} missing revised_classification",
                        requirement.candidate_id, requirement.field_key
                    ),
                )
            })?;
        if !BLOCKING_CANDIDATE_CLASSIFICATIONS.contains(&revised) {
            return Err(MetadataValidationError::single(
                ReadinessBlockerCode::MalformedCandidateBlocker,
                format!(
                    "candidate_specific.{}.{} has non-blocking revised_classification {revised}",
                    requirement.candidate_id, requirement.field_key
                ),
            ));
        }
        retained.push(RetainedCandidateBlocker {
            blocker_id: requirement.blocker_id.to_string(),
            candidate_id: requirement.candidate_id.to_string(),
            source_field: requirement.field_key.to_string(),
            revised_classification: revised.to_string(),
            label: requirement.label.to_string(),
        });
    }

    retained.sort_by(|left, right| {
        (&left.candidate_id, &left.blocker_id).cmp(&(&right.candidate_id, &right.blocker_id))
    });
    Ok(retained)
}

fn required_string(value: &Value, field: &str) -> Result<String, String> {
    value
        .get(field)
        .and_then(Value::as_str)
        .map(str::to_string)
        .ok_or_else(|| format!("missing or invalid string field {field}"))
}

fn failure_for_scenario(
    code: ReadinessBlockerCode,
    scenario_id: &str,
    message: impl Into<String>,
) -> MetadataValidationError {
    MetadataValidationError {
        failures: vec![MetadataValidationFailure {
            code,
            message: message.into(),
            scenario_id: Some(scenario_id.to_string()),
            subclaim_id: None,
        }],
    }
}

fn status_evidence_failure(
    scenario_id: &str,
    subclaim_id: Option<&str>,
    message: impl Into<String>,
) -> MetadataValidationError {
    MetadataValidationError {
        failures: vec![MetadataValidationFailure {
            code: ReadinessBlockerCode::InvalidStatusEvidenceCombination,
            message: message.into(),
            scenario_id: Some(scenario_id.to_string()),
            subclaim_id: subclaim_id.map(str::to_string),
        }],
    }
}

/// Mutate contract JSON for adversarial tests without touching production files.
pub fn contract_json_with_mutation(base: &str, mutate: impl FnOnce(&mut Value)) -> String {
    let mut value: Value = serde_json::from_str(base).expect("base contract must parse");
    mutate(&mut value);
    serde_json::to_string(&value).expect("mutated contract must serialize")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_unsupported_contract_version() {
        let json = r#"{"contract_version":"99","aggregation_rules":{"aggregation_exclusions":["Unsupported","NotRun","deferred","capability_missing"],"correction_required_false_semantics":"no_immediate_correction_not_readiness_credit"},"contracts":[]}"#;
        let error = parse_and_validate_contract(json).expect_err("must reject");
        assert_eq!(
            error.primary_code(),
            ReadinessBlockerCode::UnsupportedContractVersion
        );
    }
}
