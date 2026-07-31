use vox_proof::application_service::{
    ApplicationMaterialUseDeclaration, DeclaredApplicationMaterialUseBasis,
    DeclaredSessionAuthority, DeclaredSessionOperatorRole,
};
use vox_proof::persistence_evidence::{
    CURRENT_CONTRACT_FIXTURE_ID, CURRENT_CONTRACT_FIXTURE_VERSION, CurrentContractOracle,
    VARIANT_PROMOTED_ACTIVE, all_fixture_variants, build_duplicated_session_lineage_state,
    build_golden_small_state, build_offset_anchor_manual_replacement_state,
    build_original_for_duplication_fixture, build_promoted_active_session, canonical_fingerprint,
    golden_small, project_current_contract_state,
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
    assert_eq!(fixture.variant_id, VARIANT_PROMOTED_ACTIVE);
}

#[test]
fn projection_from_session_matches_golden_state() {
    let session = build_promoted_active_session();
    let material_use =
        ApplicationMaterialUseDeclaration::new(DeclaredApplicationMaterialUseBasis::SelfOwned);
    let projected = project_current_contract_state(
        &session,
        &material_use,
        "session:current-contract:promoted",
        "writer:promoted",
    );
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

#[test]
fn all_fixture_variants_validate() {
    for fixture in all_fixture_variants() {
        let result = CurrentContractOracle::validate(&fixture.expected_state);
        assert!(
            result.passed,
            "variant {} failed: {:?}",
            fixture.variant_id, result.violations
        );
    }
}

#[test]
fn non_zero_source_anchor_is_preserved() {
    let state = build_offset_anchor_manual_replacement_state();
    let review_case = &state.review_cases[0];
    assert!(review_case.anchor_start_byte > 0);
    assert_eq!(review_case.observed_source_bytes, "Kafak");
}

#[test]
fn duplicated_session_has_new_identity_and_lineage() {
    let duplicate = build_duplicated_session_lineage_state();
    let original = build_original_for_duplication_fixture();
    assert_ne!(duplicate.session_id, original.session_id);
    assert_eq!(
        duplicate.duplicated_from_session_id.as_deref(),
        Some(original.session_id.as_str())
    );
    assert_ne!(
        duplicate.durable_command_tokens.evidence_writer_token,
        original.durable_command_tokens.evidence_writer_token
    );
}

#[test]
fn original_session_unchanged_in_duplication_fixture() {
    let original = build_original_for_duplication_fixture();
    let duplicate = build_duplicated_session_lineage_state();
    assert_eq!(
        original.review_ledger_events,
        duplicate.review_ledger_events
    );
    assert_eq!(
        original.reuse_governance_events,
        duplicate.reuse_governance_events
    );
    assert_eq!(original.review_cases, duplicate.review_cases);
    assert_eq!(original.session_id, "session:current-contract:promoted");
    assert_ne!(duplicate.session_id, original.session_id);
}
