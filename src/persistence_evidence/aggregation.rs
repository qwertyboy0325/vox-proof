use std::collections::BTreeMap;

use super::metadata::{MetadataValidationError, parse_and_validate_metadata_bundle};
use super::metadata::{ScenarioContract, ValidatedMetadataBundle};
use super::readiness_result::{
    PositiveEvidenceEntry, READINESS_RESULT_VERSION, ReadinessAggregationOutput, ReadinessBlocker,
    ReadinessBlockerCode, ReadinessOutcome, ReadinessResultKind,
};

pub fn aggregate_readiness_from_json(
    contract_json: &str,
    reclassification_json: &str,
) -> ReadinessAggregationOutput {
    evaluate_persistence_readiness(contract_json, reclassification_json)
}

pub fn evaluate_persistence_readiness(
    contract_json: &str,
    reclassification_json: &str,
) -> ReadinessAggregationOutput {
    match parse_and_validate_metadata_bundle(contract_json, reclassification_json) {
        Ok(bundle) => aggregate_validated_metadata(&bundle),
        Err(error) => invalid_input_output(error),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AggregationInternalMode {
    Normal,
    ForcePostValidationScenarioCountMismatch,
}

pub(crate) fn aggregate_validated_metadata(
    bundle: &ValidatedMetadataBundle,
) -> ReadinessAggregationOutput {
    aggregate_validated_metadata_with_mode(bundle, AggregationInternalMode::Normal)
}

fn aggregate_validated_metadata_with_mode(
    bundle: &ValidatedMetadataBundle,
    mode: AggregationInternalMode,
) -> ReadinessAggregationOutput {
    if mode == AggregationInternalMode::ForcePostValidationScenarioCountMismatch {
        return internal_validation_error(
            "post-validation scenario count inconsistency: validated index missing",
        );
    }

    let contract = &bundle.contract;
    let reclassification = &bundle.reclassification;

    let scenario_count = contract.scenarios.len();
    let subclaim_count = contract
        .scenarios
        .iter()
        .map(|scenario| scenario.intended_claims.len())
        .sum();

    let mut status_counts = BTreeMap::new();
    let mut evidence_strength_counts = BTreeMap::new();
    let mut unsupported_scenario_ids = Vec::new();
    let mut deferred_scenario_ids = Vec::new();
    let mut correction_required_scenario_ids = Vec::new();
    let mut partially_demonstrated_scenario_ids = Vec::new();
    let mut not_demonstrated_subclaim_ids = Vec::new();
    let mut positive_evidence = Vec::new();
    let mut blockers = Vec::new();
    let warnings = Vec::new();

    for scenario in &contract.scenarios {
        *status_counts
            .entry(scenario.current_status.clone())
            .or_insert(0) += 1;

        if scenario.current_status == "Unsupported" {
            unsupported_scenario_ids.push(scenario.scenario_id.clone());
        }
        if scenario.deferred_until_capability_exists {
            deferred_scenario_ids.push(scenario.scenario_id.clone());
            blockers.push(blocker(
                ReadinessBlockerCode::DeferredClaimNoCredit,
                Some(scenario.scenario_id.clone()),
                None,
                "deferred claim excluded from readiness aggregation",
            ));
        }
        if scenario.correction_required {
            correction_required_scenario_ids.push(scenario.scenario_id.clone());
        }
        if scenario.current_status == "partially_demonstrated" {
            partially_demonstrated_scenario_ids.push(scenario.scenario_id.clone());
        }

        let excluded = is_excluded_from_readiness(
            scenario,
            &contract.aggregation_rules.aggregation_exclusions,
        );

        if excluded {
            if !scenario.current_evidence_strength.is_empty() {
                blockers.push(blocker(
                    ReadinessBlockerCode::UnsupportedScenarioNoCredit,
                    Some(scenario.scenario_id.clone()),
                    None,
                    "excluded scenario must not carry scenario-level positive evidence",
                ));
            }
            if scenario.current_status == "Unsupported" {
                blockers.push(blocker(
                    ReadinessBlockerCode::UnsupportedScenarioNoCredit,
                    Some(scenario.scenario_id.clone()),
                    None,
                    "unsupported scenario contributes no readiness credit",
                ));
            }
        } else if scenario.claim_structure == "single" {
            if scenario.current_status == "not_demonstrated" {
                blockers.push(blocker(
                    ReadinessBlockerCode::NotDemonstratedSubclaim,
                    Some(scenario.scenario_id.clone()),
                    None,
                    "single-claim scenario remains not_demonstrated",
                ));
            } else {
                credit_scenario_evidence(
                    scenario,
                    &mut positive_evidence,
                    &mut evidence_strength_counts,
                );
            }
        }

        if !excluded {
            for subclaim in &scenario.intended_claims {
                if subclaim.current_status == "not_demonstrated" {
                    not_demonstrated_subclaim_ids.push(format!(
                        "{}::{}",
                        scenario.scenario_id, subclaim.subclaim_id
                    ));
                    blockers.push(blocker(
                        ReadinessBlockerCode::NotDemonstratedSubclaim,
                        Some(scenario.scenario_id.clone()),
                        Some(subclaim.subclaim_id.clone()),
                        "required subclaim remains not_demonstrated",
                    ));
                } else if subclaim.current_status == "Unsupported" {
                    blockers.push(blocker(
                        ReadinessBlockerCode::UnsupportedScenarioNoCredit,
                        Some(scenario.scenario_id.clone()),
                        Some(subclaim.subclaim_id.clone()),
                        "unsupported subclaim contributes no readiness credit",
                    ));
                } else if subclaim.current_status == "partially_demonstrated"
                    && !subclaim.current_evidence_strength.is_empty()
                {
                    for strength in &subclaim.current_evidence_strength {
                        *evidence_strength_counts
                            .entry(strength.clone())
                            .or_insert(0) += 1;
                    }
                    positive_evidence.push(PositiveEvidenceEntry {
                        scenario_id: scenario.scenario_id.clone(),
                        subclaim_id: Some(subclaim.subclaim_id.clone()),
                        evidence_strength: subclaim.current_evidence_strength.clone(),
                    });
                }
            }
        }
    }

    unsupported_scenario_ids.sort();
    deferred_scenario_ids.sort();
    correction_required_scenario_ids.sort();
    partially_demonstrated_scenario_ids.sort();
    not_demonstrated_subclaim_ids.sort();
    positive_evidence.sort_by(|left, right| {
        (&left.scenario_id, &left.subclaim_id).cmp(&(&right.scenario_id, &right.subclaim_id))
    });

    if !correction_required_scenario_ids.is_empty() {
        for scenario_id in &correction_required_scenario_ids {
            blockers.push(blocker(
                ReadinessBlockerCode::UnresolvedCorrectionRequired,
                Some(scenario_id.clone()),
                None,
                "correction_required remains true",
            ));
        }
    }

    if reclassification.fail_closed_invariant_present {
        blockers.push(blocker(
            ReadinessBlockerCode::FailClosedInvariantActive,
            None,
            None,
            "reclassification fail_closed_readiness_invariant remains active until corrected evidence is re-executed",
        ));
    }

    if reclassification.mechanism_comparison_readiness == "not_ready" {
        blockers.push(blocker(
            ReadinessBlockerCode::MechanismBlockerRetained,
            None,
            None,
            "reclassification retains mechanism_comparison_readiness not_ready",
        ));
    }

    if reclassification.mechanism_selection_readiness == "not_ready" {
        blockers.push(blocker(
            ReadinessBlockerCode::MechanismBlockerRetained,
            None,
            None,
            "reclassification retains mechanism_selection_readiness not_ready",
        ));
    }

    if reclassification.selection_status != "none" {
        blockers.push(blocker(
            ReadinessBlockerCode::SelectionProhibitedWhileNotReady,
            None,
            None,
            "selection_status must remain none while fail-closed readiness applies",
        ));
    }

    for retained in &reclassification.retained_blockers {
        blockers.push(blocker(
            ReadinessBlockerCode::MechanismBlockerRetained,
            None,
            None,
            format!(
                "{}: {} ({}/{})",
                retained.blocker_id, retained.label, retained.candidate_id, retained.source_field
            ),
        ));
    }

    blockers.sort_by(|left, right| {
        (
            left.code,
            left.scenario_id.as_deref(),
            left.subclaim_id.as_deref(),
            left.message.as_str(),
        )
            .cmp(&(
                right.code,
                right.scenario_id.as_deref(),
                right.subclaim_id.as_deref(),
                right.message.as_str(),
            ))
    });
    blockers.dedup_by(|left, right| {
        left.code == right.code
            && left.scenario_id == right.scenario_id
            && left.subclaim_id == right.subclaim_id
            && left.message == right.message
    });

    let mechanism_comparison_readiness = if blockers.is_empty() {
        ReadinessOutcome::Ready
    } else {
        ReadinessOutcome::NotReady
    };

    let mechanism_selection_readiness = if mechanism_comparison_readiness
        == ReadinessOutcome::NotReady
        || reclassification.mechanism_selection_readiness == "not_ready"
    {
        ReadinessOutcome::NotReady
    } else {
        ReadinessOutcome::Ready
    };

    let selection_status = if mechanism_comparison_readiness == ReadinessOutcome::NotReady
        || reclassification.selection_status != "none"
    {
        "none".to_string()
    } else {
        reclassification.selection_status.clone()
    };

    let outcome_kind = if mechanism_comparison_readiness == ReadinessOutcome::Ready
        && mechanism_selection_readiness == ReadinessOutcome::Ready
    {
        ReadinessResultKind::ValidReady
    } else {
        ReadinessResultKind::ValidNotReady
    };

    ReadinessAggregationOutput {
        result_version: READINESS_RESULT_VERSION.to_string(),
        outcome_kind,
        contract_version: Some(contract.contract_version.clone()),
        scenario_count,
        subclaim_count,
        validated_scenario_count: scenario_count,
        invalid_scenario_count: 0,
        status_counts,
        evidence_strength_counts,
        unsupported_scenario_ids,
        deferred_scenario_ids,
        correction_required_scenario_ids,
        partially_demonstrated_scenario_ids,
        not_demonstrated_subclaim_ids,
        positive_evidence_by_scenario_or_subclaim: positive_evidence,
        mechanism_comparison_readiness,
        mechanism_selection_readiness,
        selection_status,
        blocking_reasons: blockers,
        warnings,
    }
}

fn credit_scenario_evidence(
    scenario: &ScenarioContract,
    positive_evidence: &mut Vec<PositiveEvidenceEntry>,
    evidence_strength_counts: &mut BTreeMap<String, usize>,
) {
    if scenario.current_evidence_strength.is_empty() {
        return;
    }
    for strength in &scenario.current_evidence_strength {
        *evidence_strength_counts
            .entry(strength.clone())
            .or_insert(0) += 1;
    }
    positive_evidence.push(PositiveEvidenceEntry {
        scenario_id: scenario.scenario_id.clone(),
        subclaim_id: None,
        evidence_strength: scenario.current_evidence_strength.clone(),
    });
}

fn is_excluded_from_readiness(
    scenario: &ScenarioContract,
    aggregation_exclusions: &[String],
) -> bool {
    if scenario.current_status == "Unsupported"
        && aggregation_exclusions
            .iter()
            .any(|item| item == "Unsupported")
    {
        return true;
    }
    if scenario.deferred_until_capability_exists
        && aggregation_exclusions.iter().any(|item| item == "deferred")
    {
        return true;
    }
    if scenario.invariant_kind == "unsupported_capability_not_executed"
        && aggregation_exclusions
            .iter()
            .any(|item| item == "capability_missing")
    {
        return true;
    }
    scenario.current_status == "Unsupported"
        || scenario.deferred_until_capability_exists
        || scenario.invariant_kind == "unsupported_capability_not_executed"
}

fn blocker(
    code: ReadinessBlockerCode,
    scenario_id: Option<String>,
    subclaim_id: Option<String>,
    message: impl Into<String>,
) -> ReadinessBlocker {
    ReadinessBlocker {
        code,
        scenario_id,
        subclaim_id,
        message: message.into(),
    }
}

fn internal_validation_error(message: impl Into<String>) -> ReadinessAggregationOutput {
    ReadinessAggregationOutput {
        result_version: READINESS_RESULT_VERSION.to_string(),
        outcome_kind: ReadinessResultKind::InternalValidationError,
        contract_version: None,
        scenario_count: 0,
        subclaim_count: 0,
        validated_scenario_count: 0,
        invalid_scenario_count: 0,
        status_counts: BTreeMap::new(),
        evidence_strength_counts: BTreeMap::new(),
        unsupported_scenario_ids: Vec::new(),
        deferred_scenario_ids: Vec::new(),
        correction_required_scenario_ids: Vec::new(),
        partially_demonstrated_scenario_ids: Vec::new(),
        not_demonstrated_subclaim_ids: Vec::new(),
        positive_evidence_by_scenario_or_subclaim: Vec::new(),
        mechanism_comparison_readiness: ReadinessOutcome::NotReady,
        mechanism_selection_readiness: ReadinessOutcome::NotReady,
        selection_status: "none".to_string(),
        blocking_reasons: vec![ReadinessBlocker {
            code: ReadinessBlockerCode::InternalInvariantViolation,
            scenario_id: None,
            subclaim_id: None,
            message: message.into(),
        }],
        warnings: Vec::new(),
    }
}

#[doc(hidden)]
pub fn aggregate_validated_metadata_internal_error_test(
    bundle: &ValidatedMetadataBundle,
) -> ReadinessAggregationOutput {
    aggregate_validated_metadata_with_mode(
        bundle,
        AggregationInternalMode::ForcePostValidationScenarioCountMismatch,
    )
}

fn invalid_input_output(error: MetadataValidationError) -> ReadinessAggregationOutput {
    let primary = error.primary_code();
    let outcome_kind = match primary {
        ReadinessBlockerCode::UnsupportedContractVersion => {
            ReadinessResultKind::UnsupportedContractVersion
        }
        ReadinessBlockerCode::MissingReclassificationAuthority
        | ReadinessBlockerCode::MissingCandidateSpecificBlockers
        | ReadinessBlockerCode::MissingCandidateBlocker
        | ReadinessBlockerCode::MalformedCandidateBlocker => {
            ReadinessResultKind::AggregationBlocked
        }
        _ => ReadinessResultKind::InvalidInput,
    };

    let blocking_reasons = error
        .failures
        .into_iter()
        .map(|failure| ReadinessBlocker {
            code: failure.code,
            scenario_id: failure.scenario_id,
            subclaim_id: failure.subclaim_id,
            message: failure.message,
        })
        .collect();

    ReadinessAggregationOutput {
        result_version: READINESS_RESULT_VERSION.to_string(),
        outcome_kind,
        contract_version: None,
        scenario_count: 0,
        subclaim_count: 0,
        validated_scenario_count: 0,
        invalid_scenario_count: 0,
        status_counts: BTreeMap::new(),
        evidence_strength_counts: BTreeMap::new(),
        unsupported_scenario_ids: Vec::new(),
        deferred_scenario_ids: Vec::new(),
        correction_required_scenario_ids: Vec::new(),
        partially_demonstrated_scenario_ids: Vec::new(),
        not_demonstrated_subclaim_ids: Vec::new(),
        positive_evidence_by_scenario_or_subclaim: Vec::new(),
        mechanism_comparison_readiness: ReadinessOutcome::NotReady,
        mechanism_selection_readiness: ReadinessOutcome::NotReady,
        selection_status: "none".to_string(),
        blocking_reasons,
        warnings: Vec::new(),
    }
}

pub fn serialize_readiness_result(
    result: &ReadinessAggregationOutput,
) -> Result<String, serde_json::Error> {
    serde_json::to_string_pretty(result)
}
