use vox_proof::persistence_evidence::{
    CurrentContractOracle, EvidenceReuseGovernanceEvent, OracleViolationCodeV3,
    build_candidate_rejected_state, build_pre_supersession_run_session,
    build_promoted_active_session, build_promoted_active_state,
    build_promoted_then_run_then_superseded_state, build_revoked_historical_state,
    build_superseded_session, build_superseded_with_reuse_run_state, derive_contract_projection,
};

#[test]
fn synchronized_review_case_and_payload_forgery_cannot_authorize_governance() {
    let mut promoted = build_promoted_active_state();
    promoted.review_cases[0].observed_source_bytes = "forged".to_owned();
    if let EvidenceReuseGovernanceEvent::PromotionAccepted {
        candidate_key,
        payload,
        ..
    } = &mut promoted.reuse_governance_events[0]
    {
        candidate_key.exact_payload.observed_text = "forged".to_owned();
        payload.observed_text = "forged".to_owned();
    }
    let (derived, violations) = derive_contract_projection(&promoted);
    assert!(violations.iter().any(|violation| {
        violation.code == OracleViolationCodeV3::SourceAnchorResolvedBytesMismatch
    }));
    assert!(derived.effective_review_status.is_empty());
    assert!(derived.effective_reusable_records.is_empty());
    assert!(derived.historical_reusable_records.is_empty());
    assert!(!CurrentContractOracle::validate(&promoted).passed);

    let mut rejected = build_candidate_rejected_state();
    rejected.review_cases[0].observed_source_bytes = "forged".to_owned();
    if let EvidenceReuseGovernanceEvent::PromotionCandidateRejected { candidate_key, .. } =
        &mut rejected.reuse_governance_events[0]
    {
        candidate_key.exact_payload.observed_text = "forged".to_owned();
    }
    let (derived, violations) = derive_contract_projection(&rejected);
    assert!(violations.iter().any(|violation| {
        violation.code == OracleViolationCodeV3::SourceAnchorResolvedBytesMismatch
    }));
    assert!(derived.effective_review_status.is_empty());
    assert!(derived.rejected_candidate_identities.is_empty());
    assert!(derived.effective_reusable_records.is_empty());
    assert!(derived.historical_reusable_records.is_empty());
    assert!(!CurrentContractOracle::validate(&rejected).passed);
}

#[test]
fn lifecycle_bindings_match_production_and_document_latest_run_only_retention() {
    let promoted_session = build_promoted_active_session();
    let promoted_identity = promoted_session
        .reuse_enabled_run()
        .expect("promoted session run")
        .reuse_enabled_snapshot()
        .reusable_influence_snapshot()
        .to_tagged_string();
    assert_eq!(
        build_promoted_active_state().reusable_snapshot_identity,
        promoted_identity
    );

    let revoked = build_revoked_historical_state();
    let revoked_binding = revoked
        .reuse_enabled_analysis_binding
        .as_ref()
        .expect("binding retained after revoke");
    assert_eq!(
        revoked_binding.reusable_snapshot_identity,
        promoted_identity
    );
    assert_ne!(revoked.reusable_snapshot_identity, promoted_identity);
    assert!(CurrentContractOracle::validate(&revoked).passed);

    let pre_supersession_session = build_pre_supersession_run_session();
    let pre_supersession_identity = pre_supersession_session
        .reuse_enabled_run()
        .expect("run before supersession")
        .reuse_enabled_snapshot()
        .reusable_influence_snapshot()
        .to_tagged_string();
    let superseded = build_promoted_then_run_then_superseded_state();
    let superseded_binding = superseded
        .reuse_enabled_analysis_binding
        .as_ref()
        .expect("binding retained after supersession");
    assert_eq!(superseded_binding.governance_event_boundary, 1);
    assert_eq!(superseded.reuse_governance_events.len(), 3);
    assert_eq!(
        superseded_binding.reusable_snapshot_identity,
        pre_supersession_identity
    );
    assert_ne!(
        superseded.reusable_snapshot_identity,
        pre_supersession_identity
    );
    assert!(CurrentContractOracle::validate(&superseded).passed);

    let mut fresh_post_supersession_session = build_superseded_session();
    assert!(
        fresh_post_supersession_session
            .reuse_enabled_run()
            .is_none(),
        "the bounded application retains only the latest run and invalidates it on governance change"
    );
    let fresh_identity = fresh_post_supersession_session
        .run_reuse_enabled_review()
        .expect("fresh run after supersession")
        .reuse_enabled_snapshot()
        .reusable_influence_snapshot()
        .to_tagged_string();
    let fresh_evidence = build_superseded_with_reuse_run_state();
    assert_eq!(fresh_evidence.reusable_snapshot_identity, fresh_identity);
    assert_ne!(fresh_identity, pre_supersession_identity);
    assert_eq!(
        fresh_evidence
            .reuse_enabled_analysis_binding
            .as_ref()
            .expect("latest run binding")
            .reusable_snapshot_identity,
        fresh_identity
    );
    assert!(CurrentContractOracle::validate(&fresh_evidence).passed);
}
