use vox_proof::persistence_evidence::{
    build_platform_matrix, compare_scenario, normalize_platform_label,
    PlatformEquivalenceResult, ScenarioIdentity, ScenarioResult, ScenarioStatus,
    FailureModel, ScenarioCategory, ScenarioEvidenceKind, ScenarioRequirement,
};

fn passed_scenario(id: &str, fingerprint: &str) -> ScenarioResult {
    ScenarioResult {
        scenario_identity: ScenarioIdentity {
            scenario_id: id.to_string(),
            scenario_version: 1,
            category: ScenarioCategory::Baseline,
            description: "test".to_string(),
            failure_model: FailureModel::None,
            required_capabilities: vec![],
            requirement: ScenarioRequirement::Required,
            evidence_kind: ScenarioEvidenceKind::SemanticCorrectness,
        },
        status: ScenarioStatus::Passed,
        oracle_result: Some(vox_proof::persistence_evidence::OracleResult {
            passed: true,
            violations: vec![],
            warnings: vec![],
            expected_fingerprint: Some(fingerprint.to_string()),
            actual_fingerprint: fingerprint.to_string(),
            oracle_version: "2".to_string(),
        }),
        measurements: Default::default(),
        failure_classification: None,
        limitations: vec![],
        raw_artifact_references: vec![],
        achieved_evidence_strength: vec!["InterfaceBehavior".to_string()],
        process_interruption_performed: None,
        reopen_performed: Some(true),
        observed_error_code: None,
    }
}

#[test]
fn equivalent_passed_scenarios_credit_cross_platform() {
    let fp = "semantic:sha256-v1:abc";
    let macos = passed_scenario("baseline-create-open-close", fp);
    let windows = passed_scenario("baseline-create-open-close", fp);
    let row = compare_scenario("baseline-create-open-close", Some(&macos), Some(&windows));
    assert_eq!(row.equivalence_result, PlatformEquivalenceResult::Equivalent);
    assert!(row.cross_platform_credited);
}

#[test]
fn missing_platform_does_not_credit_cross_platform() {
    let macos = passed_scenario("append-correction-event", "fp");
    let row = compare_scenario("append-correction-event", Some(&macos), None);
    assert_eq!(
        row.equivalence_result,
        PlatformEquivalenceResult::MissingPlatform
    );
    assert!(!row.cross_platform_credited);
}

#[test]
fn platform_matrix_marks_fsd_and_hpl_not_demonstrated() {
    let matrix = build_platform_matrix(
        Some("macos-run"),
        Some("windows-run"),
        &[passed_scenario("baseline-create-open-close", "fp")],
        &[passed_scenario("baseline-create-open-close", "fp")],
    );
    assert_eq!(matrix.filesystem_durability_status, "not_demonstrated");
    assert_eq!(matrix.hardware_power_loss_status, "not_demonstrated");
}

#[test]
fn normalize_platform_label_distinguishes_github_actions() {
    let label = normalize_platform_label("windows", "github-actions");
    assert!(label.contains("github"));
}
