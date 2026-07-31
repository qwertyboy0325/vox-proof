use vox_proof::application_service::{
    ApplicationMaterialUseDeclaration, DeclaredApplicationMaterialUseBasis,
    DeclaredSessionAuthority, DeclaredSessionOperatorRole,
};
use vox_proof::persistence_evidence::{
    CURRENT_CONTRACT_FIXTURE_ID, CURRENT_CONTRACT_FIXTURE_VERSION, CurrentContractOracle,
    build_golden_small_session, build_golden_small_state, canonical_fingerprint, golden_small,
    project_current_contract_state,
};

#[test]
fn fixture_v3_is_deterministic() {
    let first = golden_small().expected_state;
    let second = golden_small().expected_state;
    assert_eq!(first, second);
}

#[test]
fn canonical_fingerprint_is_stable() {
    let state = build_golden_small_state();
    let fp1 = canonical_fingerprint(&state.canonical_projection());
    let fp2 = canonical_fingerprint(&state.canonical_projection());
    assert_eq!(fp1, fp2);
}

#[test]
fn presentation_only_state_does_not_affect_canonical_identity() {
    let mut state = build_golden_small_state();
    let baseline = canonical_fingerprint(&state.canonical_projection());
    state.derived_queue_projection = "presentation-only-mutation".to_owned();
    assert_eq!(
        baseline,
        canonical_fingerprint(&state.canonical_projection())
    );
}

#[test]
fn mutable_project_display_name_does_not_affect_canonical_identity() {
    let mut state = build_golden_small_state();
    let baseline = canonical_fingerprint(&state.canonical_projection());
    state.project_scope.display_name = "Renamed Display Only".to_owned();
    assert_eq!(
        baseline,
        canonical_fingerprint(&state.canonical_projection())
    );
}

#[test]
fn session_actor_label_changes_canonical_identity() {
    let mut state = build_golden_small_state();
    let baseline = canonical_fingerprint(&state.canonical_projection());
    state.session_authority.display_label = "Other Reviewer".to_owned();
    assert_ne!(
        baseline,
        canonical_fingerprint(&state.canonical_projection())
    );
}

#[test]
fn material_use_declaration_changes_canonical_identity() {
    let mut state = build_golden_small_state();
    let baseline = canonical_fingerprint(&state.canonical_projection());
    state.material_use_declaration.basis = "explicit_permission".to_owned();
    assert_ne!(
        baseline,
        canonical_fingerprint(&state.canonical_projection())
    );
}

#[test]
fn manual_replacement_byte_change_changes_canonical_identity() {
    let mut state = build_golden_small_state();
    let baseline = canonical_fingerprint(&state.canonical_projection());
    state.review_ledger_events[0].manual_replacement_bytes = Some("Different".to_owned());
    assert_ne!(
        baseline,
        canonical_fingerprint(&state.canonical_projection())
    );
}

#[test]
fn human_raised_is_absent_from_fixture() {
    let state = build_golden_small_state();
    assert!(
        state
            .review_cases
            .iter()
            .all(|case| case.origin != "human_raised")
    );
}

#[test]
fn fixture_metadata_matches_contract() {
    let fixture = golden_small();
    assert_eq!(fixture.fixture_id, CURRENT_CONTRACT_FIXTURE_ID);
    assert_eq!(fixture.fixture_version, CURRENT_CONTRACT_FIXTURE_VERSION);
}

#[test]
fn projection_from_session_matches_golden_state() {
    let session = build_golden_small_session();
    let material_use =
        ApplicationMaterialUseDeclaration::new(DeclaredApplicationMaterialUseBasis::SelfOwned);
    let projected = project_current_contract_state(&session, &material_use);
    let golden = build_golden_small_state();
    let oracle = CurrentContractOracle::compare(&golden, &projected);
    assert!(oracle.passed, "{:?}", oracle.violations);
}

#[test]
fn fixture_oracle_production_isolation() {
    let state = build_golden_small_state();
    assert!(!state.session_id.is_empty());
    assert!(
        state
            .reusable_snapshot_identity
            .contains("reusable-influence-snapshot")
    );
    let _authority = DeclaredSessionAuthority::new(
        DeclaredSessionOperatorRole::DeclaredLocalOwnerOperator,
        "Ezra",
    )
    .expect("authority remains separate type");
}
