use std::fs;
use std::path::PathBuf;

use vox_proof::persistence_evidence::{
    READINESS_RESULT_VERSION, ReadinessBlockerCode, ReadinessOutcome, ReadinessResultKind,
    SUPPORTED_CONTRACT_VERSION, aggregate_readiness_from_json, contract_json_with_mutation,
    parse_and_validate_contract, serialize_readiness_result,
};

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn accepted_contract_json() -> String {
    fs::read_to_string(
        repo_root().join("evidence/persistence/spike-v1-review/scenario-claim-contracts.json"),
    )
    .expect("accepted v4 contract must exist")
}

fn accepted_reclassification_json() -> String {
    fs::read_to_string(
        repo_root().join("evidence/persistence/spike-v1-review/reclassification.json"),
    )
    .expect("reclassification must exist")
}

fn aggregate(
    contract_json: &str,
    reclassification_json: &str,
) -> vox_proof::persistence_evidence::ReadinessAggregationOutput {
    aggregate_readiness_from_json(contract_json, reclassification_json)
}

#[test]
fn golden_baseline_validates_and_aggregates_fail_closed() {
    let contract = accepted_contract_json();
    let reclassification = accepted_reclassification_json();

    let validated =
        parse_and_validate_contract(&contract).expect("accepted contract must validate");
    assert_eq!(validated.contract_version, SUPPORTED_CONTRACT_VERSION);
    assert_eq!(validated.scenarios.len(), 14);

    let derived = validated
        .scenarios
        .iter()
        .find(|scenario| scenario.scenario_id == "derived-state-corruption")
        .expect("derived scenario present");
    assert_eq!(derived.claim_structure, "multiple");
    assert_eq!(derived.intended_claims.len(), 3);

    let cleanup = validated
        .scenarios
        .iter()
        .find(|scenario| scenario.scenario_id == "interrupted-cleanup")
        .expect("cleanup scenario present");
    assert!(cleanup.demonstrated_observation_kind.is_none());
    assert_eq!(cleanup.current_status, "Unsupported");

    let result = aggregate(&contract, &reclassification);
    assert_eq!(result.result_version, READINESS_RESULT_VERSION);
    assert_eq!(result.outcome_kind, ReadinessResultKind::ValidNotReady);
    assert_eq!(result.scenario_count, 14);
    assert_eq!(result.subclaim_count, 3);
    assert_eq!(result.validated_scenario_count, 14);
    assert_eq!(
        result.mechanism_comparison_readiness,
        ReadinessOutcome::NotReady
    );
    assert_eq!(
        result.mechanism_selection_readiness,
        ReadinessOutcome::NotReady
    );
    assert_eq!(result.selection_status, "none");
    assert!(
        result
            .unsupported_scenario_ids
            .contains(&"interrupted-cleanup".to_string())
    );
    assert!(
        result
            .positive_evidence_by_scenario_or_subclaim
            .iter()
            .all(|entry| entry.scenario_id != "interrupted-cleanup")
    );
    assert!(
        result
            .blocking_reasons
            .iter()
            .any(|blocker| { blocker.code == ReadinessBlockerCode::UnresolvedCorrectionRequired })
    );
}

#[test]
fn deterministic_serialized_result_ordering() {
    let contract = accepted_contract_json();
    let reclassification = accepted_reclassification_json();
    let first =
        serialize_readiness_result(&aggregate(&contract, &reclassification)).expect("serialize");
    let second =
        serialize_readiness_result(&aggregate(&contract, &reclassification)).expect("serialize");
    assert_eq!(first, second);
}

#[test]
fn rejects_duplicate_scenario_id() {
    let contract = contract_json_with_mutation(&accepted_contract_json(), |value| {
        let contracts = value["contracts"].as_array_mut().expect("contracts");
        contracts.push(contracts[0].clone());
    });
    let error = parse_and_validate_contract(&contract).expect_err("duplicate scenario");
    assert_eq!(
        error.primary_code(),
        ReadinessBlockerCode::DuplicateScenarioId
    );
}

#[test]
fn rejects_duplicate_derived_subclaim_id() {
    let contract = contract_json_with_mutation(&accepted_contract_json(), |value| {
        let contracts = value["contracts"].as_array_mut().expect("contracts");
        let derived = contracts
            .iter_mut()
            .find(|scenario| scenario["scenario_id"] == "derived-state-corruption")
            .expect("derived");
        let subclaims = derived["intended_claims"]
            .as_array_mut()
            .expect("subclaims");
        subclaims.push(subclaims[0].clone());
    });
    let error = parse_and_validate_contract(&contract).expect_err("duplicate subclaim");
    assert_eq!(
        error.primary_code(),
        ReadinessBlockerCode::DuplicateSubclaimId
    );
}

#[test]
fn rejects_single_claim_with_null_intended_claim() {
    let contract = contract_json_with_mutation(&accepted_contract_json(), |value| {
        let contracts = value["contracts"].as_array_mut().expect("contracts");
        contracts[0]["intended_claim"] = serde_json::Value::Null;
    });
    let error = parse_and_validate_contract(&contract).expect_err("invalid single claim");
    assert_eq!(
        error.primary_code(),
        ReadinessBlockerCode::InvalidClaimStructure
    );
}

#[test]
fn rejects_multiple_claim_with_scenario_level_demonstrated_claim() {
    let contract = contract_json_with_mutation(&accepted_contract_json(), |value| {
        let contracts = value["contracts"].as_array_mut().expect("contracts");
        let derived = contracts
            .iter_mut()
            .find(|scenario| scenario["scenario_id"] == "derived-state-corruption")
            .expect("derived");
        derived["current_demonstrated_claim"] = serde_json::json!("must remain null");
    });
    let error = parse_and_validate_contract(&contract).expect_err("invalid multiple claim");
    assert_eq!(
        error.primary_code(),
        ReadinessBlockerCode::InvalidClaimStructure
    );
}

#[test]
fn rejects_multiple_claim_with_empty_subclaims() {
    let contract = contract_json_with_mutation(&accepted_contract_json(), |value| {
        let contracts = value["contracts"].as_array_mut().expect("contracts");
        let derived = contracts
            .iter_mut()
            .find(|scenario| scenario["scenario_id"] == "derived-state-corruption")
            .expect("derived");
        derived["intended_claims"] = serde_json::json!([]);
    });
    let error = parse_and_validate_contract(&contract).expect_err("empty subclaims");
    assert_eq!(
        error.primary_code(),
        ReadinessBlockerCode::InvalidClaimStructure
    );
}

#[test]
fn rejects_unsupported_scenario_with_positive_evidence() {
    let contract = contract_json_with_mutation(&accepted_contract_json(), |value| {
        let contracts = value["contracts"].as_array_mut().expect("contracts");
        let cleanup = contracts
            .iter_mut()
            .find(|scenario| scenario["scenario_id"] == "interrupted-cleanup")
            .expect("cleanup");
        cleanup["current_evidence_strength"] = serde_json::json!(["InterfaceBehavior"]);
    });
    let error = parse_and_validate_contract(&contract).expect_err("unsupported with evidence");
    assert_eq!(
        error.primary_code(),
        ReadinessBlockerCode::InvalidStatusEvidenceCombination
    );
}

#[test]
fn rejects_unsupported_scenario_with_non_null_observation_kind() {
    let contract = contract_json_with_mutation(&accepted_contract_json(), |value| {
        let contracts = value["contracts"].as_array_mut().expect("contracts");
        let cleanup = contracts
            .iter_mut()
            .find(|scenario| scenario["scenario_id"] == "interrupted-cleanup")
            .expect("cleanup");
        cleanup["demonstrated_observation_kind"] = serde_json::json!("must_be_null");
    });
    let error = parse_and_validate_contract(&contract).expect_err("unsupported with observation");
    assert_eq!(
        error.primary_code(),
        ReadinessBlockerCode::InvalidStatusEvidenceCombination
    );
}

#[test]
fn rejects_not_demonstrated_subclaim_with_evidence_strength() {
    let contract = contract_json_with_mutation(&accepted_contract_json(), |value| {
        let contracts = value["contracts"].as_array_mut().expect("contracts");
        let derived = contracts
            .iter_mut()
            .find(|scenario| scenario["scenario_id"] == "derived-state-corruption")
            .expect("derived");
        derived["intended_claims"][0]["current_evidence_strength"] =
            serde_json::json!(["InterfaceBehavior"]);
    });
    let error = parse_and_validate_contract(&contract).expect_err("not demonstrated with strength");
    assert_eq!(
        error.primary_code(),
        ReadinessBlockerCode::InvalidStatusEvidenceCombination
    );
}

#[test]
fn rejects_unknown_evidence_strength_enum() {
    let contract = contract_json_with_mutation(&accepted_contract_json(), |value| {
        value["contracts"][0]["current_evidence_strength"] =
            serde_json::json!(["ImaginaryStrength"]);
    });
    let error = parse_and_validate_contract(&contract).expect_err("unknown strength");
    assert_eq!(error.primary_code(), ReadinessBlockerCode::InvalidContract);
}

#[test]
fn rejects_unknown_invariant_kind() {
    let contract = contract_json_with_mutation(&accepted_contract_json(), |value| {
        value["contracts"][0]["invariant_kind"] = serde_json::json!("unknown_kind");
    });
    let error = parse_and_validate_contract(&contract).expect_err("unknown invariant");
    assert_eq!(error.primary_code(), ReadinessBlockerCode::InvalidContract);
}

#[test]
fn rejects_future_unsupported_contract_version() {
    let contract = contract_json_with_mutation(&accepted_contract_json(), |value| {
        value["contract_version"] = serde_json::json!("99");
    });
    let result = aggregate(&contract, &accepted_reclassification_json());
    assert_eq!(
        result.outcome_kind,
        ReadinessResultKind::UnsupportedContractVersion
    );
    assert_eq!(
        result.mechanism_comparison_readiness,
        ReadinessOutcome::NotReady
    );
}

#[test]
fn excluded_deferred_parent_subclaim_evidence_contributes_zero_credit() {
    let contract = contract_json_with_mutation(&accepted_contract_json(), |value| {
        let contracts = value["contracts"].as_array_mut().expect("contracts");
        let derived = contracts
            .iter_mut()
            .find(|scenario| scenario["scenario_id"] == "derived-state-corruption")
            .expect("derived");
        derived["deferred_until_capability_exists"] = serde_json::json!(true);
    });
    parse_and_validate_contract(&contract)
        .expect("deferred parent with subclaim evidence validates");
    let mutated = aggregate(&contract, &accepted_reclassification_json());
    assert_eq!(mutated.outcome_kind, ReadinessResultKind::ValidNotReady);
    assert_eq!(
        mutated.mechanism_comparison_readiness,
        ReadinessOutcome::NotReady
    );
    assert_eq!(mutated.selection_status, "none");
    assert!(
        mutated
            .positive_evidence_by_scenario_or_subclaim
            .iter()
            .all(|entry| entry.scenario_id != "derived-state-corruption"),
        "excluded deferred parent must not credit subclaim evidence"
    );
    assert!(
        mutated
            .deferred_scenario_ids
            .contains(&"derived-state-corruption".to_string())
    );
    assert!(mutated.blocking_reasons.iter().any(|blocker| {
        blocker.code == ReadinessBlockerCode::DeferredClaimNoCredit
            && blocker.scenario_id.as_deref() == Some("derived-state-corruption")
    }));
}

#[test]
fn valid_input_target_strength_inflation_does_not_create_credit() {
    let contract = contract_json_with_mutation(&accepted_contract_json(), |value| {
        let contracts = value["contracts"].as_array_mut().expect("contracts");
        let baseline = contracts
            .iter_mut()
            .find(|scenario| scenario["scenario_id"] == "baseline-create-open-close")
            .expect("baseline");
        baseline["target_evidence_strength_for_this_scenario"] =
            serde_json::json!(["HardwarePowerLoss", "CrossPlatform"]);
    });
    parse_and_validate_contract(&contract).expect("valid contract with inflated target");
    let inflated = aggregate(&contract, &accepted_reclassification_json());
    assert_eq!(inflated.outcome_kind, ReadinessResultKind::ValidNotReady);
    assert_eq!(
        inflated.mechanism_comparison_readiness,
        ReadinessOutcome::NotReady
    );
    assert_eq!(inflated.selection_status, "none");
    let baseline_entry = inflated
        .positive_evidence_by_scenario_or_subclaim
        .iter()
        .find(|entry| entry.scenario_id == "baseline-create-open-close")
        .expect("baseline remains credited");
    assert_eq!(baseline_entry.subclaim_id, None);
    assert_eq!(
        baseline_entry.evidence_strength,
        vec![
            "InterfaceBehavior".to_string(),
            "CrossPlatform".to_string()
        ]
    );
    assert!(
        !inflated
            .evidence_strength_counts
            .contains_key("HardwarePowerLoss")
    );
    assert!(
        inflated
            .evidence_strength_counts
            .get("CrossPlatform")
            .copied()
            .unwrap_or(0)
            >= 1,
        "CrossPlatform credit must come from demonstrated Package 2D fields, not target inflation alone"
    );
}

#[test]
fn rejects_null_forbidden_shortcuts_array() {
    let contract = contract_json_with_mutation(&accepted_contract_json(), |value| {
        value["contracts"][0]["forbidden_shortcuts"] = serde_json::Value::Null;
    });
    let error = parse_and_validate_contract(&contract).expect_err("null forbidden_shortcuts");
    assert_eq!(error.primary_code(), ReadinessBlockerCode::InvalidContract);
}

#[test]
fn internal_validation_error_is_reachable_from_test_seam() {
    use vox_proof::persistence_evidence::{
        aggregate_validated_metadata_internal_error_test, parse_and_validate_metadata_bundle,
    };

    let bundle = parse_and_validate_metadata_bundle(
        &accepted_contract_json(),
        &accepted_reclassification_json(),
    )
    .expect("valid bundle");
    let result = aggregate_validated_metadata_internal_error_test(&bundle);
    assert_eq!(
        result.outcome_kind,
        ReadinessResultKind::InternalValidationError
    );
    assert_eq!(
        result.blocking_reasons[0].code,
        ReadinessBlockerCode::InternalInvariantViolation
    );
    let serialized = serialize_readiness_result(&result).expect("serialize");
    let reserialized = serialize_readiness_result(
        &serde_json::from_str(&serialized).expect("parse serialized internal error"),
    )
    .expect("reserialize");
    assert_eq!(serialized, reserialized);
}

#[test]
fn golden_includes_retained_candidate_blockers_from_authority() {
    let result = aggregate(&accepted_contract_json(), &accepted_reclassification_json());
    let retained: Vec<_> = result
        .blocking_reasons
        .iter()
        .filter(|blocker| blocker.code == ReadinessBlockerCode::MechanismBlockerRetained)
        .collect();
    assert!(retained.len() >= 8, "expected retained candidate blockers");
    assert!(retained.iter().any(|blocker| {
        blocker
            .message
            .contains("embedded-relational-wal-safe-duplication-not-demonstrated")
    }));
    assert!(retained.iter().any(|blocker| {
        blocker
            .message
            .contains("append-bundle-authoritative-replay-not-demonstrated")
    }));
}

#[test]
fn golden_positive_evidence_credits_only_readability_subclaim() {
    let result = aggregate(&accepted_contract_json(), &accepted_reclassification_json());
    let derived_entries: Vec<_> = result
        .positive_evidence_by_scenario_or_subclaim
        .iter()
        .filter(|entry| entry.scenario_id == "derived-state-corruption")
        .collect();
    assert_eq!(derived_entries.len(), 1);
    assert_eq!(
        derived_entries[0].subclaim_id.as_deref(),
        Some("canonical_readability_after_derived_artifact_mutation")
    );
    assert_eq!(
        derived_entries[0].evidence_strength,
        vec![
            "InterfaceBehavior".to_string(),
            "CrossPlatform".to_string()
        ]
    );
}

#[test]
fn rejects_missing_demonstrated_observation_kind() {
    let contract = contract_json_with_mutation(&accepted_contract_json(), |value| {
        value["contracts"][0]
            .as_object_mut()
            .expect("object")
            .remove("demonstrated_observation_kind");
    });
    let error = parse_and_validate_contract(&contract).expect_err("missing observation kind");
    assert_eq!(error.primary_code(), ReadinessBlockerCode::InvalidContract);
}

#[test]
fn target_strength_does_not_create_readiness_credit_on_invalid_mutation() {
    let contract = contract_json_with_mutation(&accepted_contract_json(), |value| {
        value["contracts"][0]["target_evidence_strength_for_this_scenario"] =
            serde_json::json!(["HardwarePowerLoss"]);
        value["contracts"][0]["current_evidence_strength"] = serde_json::json!([]);
        value["contracts"][0]["correction_required"] = serde_json::json!(false);
    });
    let result = aggregate(&contract, &accepted_reclassification_json());
    assert_eq!(result.outcome_kind, ReadinessResultKind::InvalidInput);
}

#[test]
fn correction_required_false_does_not_create_readiness_credit() {
    let contract = accepted_contract_json();
    let reclassification = accepted_reclassification_json();
    let result = aggregate(&contract, &reclassification);
    assert_eq!(
        result.mechanism_comparison_readiness,
        ReadinessOutcome::NotReady
    );
    assert!(
        result
            .blocking_reasons
            .iter()
            .any(|blocker| { blocker.code == ReadinessBlockerCode::UnresolvedCorrectionRequired })
    );
}

#[test]
fn rejects_removed_aggregation_exclusions() {
    let contract = contract_json_with_mutation(&accepted_contract_json(), |value| {
        value["aggregation_rules"]["aggregation_exclusions"] = serde_json::json!(["Unsupported"]);
    });
    let error = parse_and_validate_contract(&contract).expect_err("weakened exclusions");
    assert_eq!(error.primary_code(), ReadinessBlockerCode::InvalidContract);
}

#[test]
fn sibling_subclaim_evidence_does_not_transfer() {
    let contract = accepted_contract_json();
    let validated = parse_and_validate_contract(&contract).expect("valid");
    let derived = validated
        .scenarios
        .iter()
        .find(|scenario| scenario.scenario_id == "derived-state-corruption")
        .expect("derived");
    let detection = derived
        .intended_claims
        .iter()
        .find(|subclaim| subclaim.subclaim_id == "derived_corruption_detection")
        .expect("detection");
    let readability = derived
        .intended_claims
        .iter()
        .find(|subclaim| {
            subclaim.subclaim_id == "canonical_readability_after_derived_artifact_mutation"
        })
        .expect("readability");
    assert!(detection.current_evidence_strength.is_empty());
    assert_eq!(
        readability.current_evidence_strength,
        vec![
            "InterfaceBehavior".to_string(),
            "CrossPlatform".to_string()
        ]
    );
}

#[test]
fn malformed_reclassification_fails_closed() {
    let contract = accepted_contract_json();
    let result = aggregate(&contract, "{}");
    assert_eq!(result.outcome_kind, ReadinessResultKind::AggregationBlocked);
    assert_eq!(
        result.mechanism_comparison_readiness,
        ReadinessOutcome::NotReady
    );
}

#[test]
fn selection_status_cannot_become_non_none_while_comparison_not_ready() {
    let contract = accepted_contract_json();
    let mut reclassification: serde_json::Value =
        serde_json::from_str(&accepted_reclassification_json()).expect("parse");
    reclassification["revised_classification"]["selection_status"] =
        serde_json::json!("candidate_a");
    let result = aggregate(
        &contract,
        &serde_json::to_string(&reclassification).expect("serialize"),
    );
    assert_eq!(result.outcome_kind, ReadinessResultKind::AggregationBlocked);
    assert_eq!(result.selection_status, "none");
    assert_eq!(
        result.mechanism_comparison_readiness,
        ReadinessOutcome::NotReady
    );
}
