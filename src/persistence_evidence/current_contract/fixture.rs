use crate::application_service::{
    ApplicationMaterialUseDeclaration, ApplicationReviewSession,
    DeclaredApplicationMaterialUseBasis, DeclaredSessionAuthority, DeclaredSessionOperatorRole,
    begin_application_review,
};
use crate::candidate::SessionTermEntry;
use crate::reuse_primitives::ReusableInfluenceRecordId;
use crate::srt::parse_srt;

use super::model::{
    CURRENT_CONTRACT_FIXTURE_ID, CURRENT_CONTRACT_FIXTURE_VERSION, CurrentContractFixture,
    CurrentContractFixtureScale, CurrentContractState,
};
use super::projection::project_current_contract_state;

pub const GOLDEN_SMALL_TRANSCRIPT: &str = "1\n00:00:00,000 --> 00:00:01,000\nKafak\n";
pub const OFFSET_ANCHOR_TRANSCRIPT: &str = "1\n00:00:00,000 --> 00:00:01,000\nThe Kafak stream\n";
pub const SUPERSESSION_TRANSCRIPT: &str =
    "1\n00:00:00,000 --> 00:00:01,000\nKafak\n\n2\n00:00:01,000 --> 00:00:02,000\nKafak\n";

pub const VARIANT_BASE_MANUAL_REPLACEMENT: &str = "base_manual_replacement";
pub const VARIANT_CANDIDATE_REJECTED: &str = "candidate_rejected";
pub const VARIANT_PROMOTED_ACTIVE: &str = "promoted_active";
pub const VARIANT_REVOKED_HISTORICAL: &str = "revoked_historical";
pub const VARIANT_SUPERSEDED: &str = "superseded_predecessor_and_active_successor";
pub const VARIANT_DUPLICATED_LINEAGE: &str = "duplicated_session_lineage";
pub const VARIANT_OFFSET_ANCHOR: &str = "offset_anchor_manual_replacement";

pub fn golden_small() -> CurrentContractFixture {
    fixture_variant(VARIANT_PROMOTED_ACTIVE, build_promoted_active_state())
}

pub fn build_golden_small_state() -> CurrentContractState {
    build_promoted_active_state()
}

pub fn build_golden_small_session() -> ApplicationReviewSession {
    build_promoted_active_session()
}

pub fn all_fixture_variants() -> Vec<CurrentContractFixture> {
    vec![
        fixture_variant(
            VARIANT_BASE_MANUAL_REPLACEMENT,
            build_base_manual_replacement_state(),
        ),
        fixture_variant(VARIANT_CANDIDATE_REJECTED, build_candidate_rejected_state()),
        fixture_variant(VARIANT_PROMOTED_ACTIVE, build_promoted_active_state()),
        fixture_variant(VARIANT_REVOKED_HISTORICAL, build_revoked_historical_state()),
        fixture_variant(VARIANT_SUPERSEDED, build_superseded_state()),
        fixture_variant(
            VARIANT_DUPLICATED_LINEAGE,
            build_duplicated_session_lineage_state(),
        ),
        fixture_variant(
            VARIANT_OFFSET_ANCHOR,
            build_offset_anchor_manual_replacement_state(),
        ),
    ]
}

fn fixture_variant(
    variant_id: &str,
    expected_state: CurrentContractState,
) -> CurrentContractFixture {
    CurrentContractFixture {
        fixture_id: CURRENT_CONTRACT_FIXTURE_ID.to_owned(),
        fixture_version: CURRENT_CONTRACT_FIXTURE_VERSION.to_owned(),
        variant_id: variant_id.to_owned(),
        scale: CurrentContractFixtureScale::Small,
        expected_state,
    }
}

pub fn build_base_manual_replacement_state() -> CurrentContractState {
    project_session(
        &build_base_manual_replacement_session(),
        "session:current-contract:base",
        "writer:base",
    )
}

pub fn build_candidate_rejected_state() -> CurrentContractState {
    project_session(
        &build_candidate_rejected_session(),
        "session:current-contract:rejected",
        "writer:rejected",
    )
}

pub fn build_promoted_active_state() -> CurrentContractState {
    project_session(
        &build_promoted_active_session(),
        "session:current-contract:promoted",
        "writer:promoted",
    )
}

pub fn build_revoked_historical_state() -> CurrentContractState {
    let session_before_revoke = build_promoted_active_session();
    let binding = session_before_revoke.reuse_enabled_run().map(|reuse_run| {
        let revision = session_before_revoke
            .source()
            .revision_id()
            .to_tagged_string();
        let analysis_snapshot = super::projection::map_analysis_snapshot_for_export(
            reuse_run.analysis_run().snapshot(),
            revision,
        );
        super::model::EvidenceReuseEnabledAnalysisBinding {
            analysis_snapshot_identity: analysis_snapshot.identity.clone(),
            analysis_snapshot,
            reusable_snapshot_identity: reuse_run.reusable_snapshot_identity().to_tagged_string(),
            governance_event_boundary: reuse_run.governance_event_boundary_at_run(),
            projection_version: super::model::REUSABLE_INFLUENCE_PROJECTION_VERSION.to_owned(),
        }
    });
    let mut session = session_before_revoke;
    let record_id = ReusableInfluenceRecordId::from_promotion_event_index(0);
    session
        .revoke_reusable_influence(record_id)
        .expect("revoke");
    let mut state = project_session(
        &session,
        "session:current-contract:revoked",
        "writer:revoked",
    );
    state.reuse_enabled_analysis_binding = binding;
    if let Some(binding) = &state.reuse_enabled_analysis_binding {
        state
            .analysis_snapshots
            .push(binding.analysis_snapshot.clone());
    }
    super::derivation::finalize_derived_fields(&mut state);
    state.normalize()
}

pub fn build_superseded_state() -> CurrentContractState {
    project_session(
        &build_superseded_session(),
        "session:current-contract:superseded",
        "writer:superseded",
    )
}

pub fn build_superseded_with_reuse_run_state() -> CurrentContractState {
    let mut session = build_superseded_session();
    session
        .run_reuse_enabled_review()
        .expect("reuse-enabled run after supersession");
    project_session(
        &session,
        "session:current-contract:superseded-fresh-run",
        "writer:superseded-fresh-run",
    )
}

pub fn build_promoted_then_run_then_superseded_state() -> CurrentContractState {
    let mut session = build_pre_supersession_run_session();
    let binding = session.reuse_enabled_run().map(|reuse_run| {
        let revision = session.source().revision_id().to_tagged_string();
        let analysis_snapshot = super::projection::map_analysis_snapshot_for_export(
            reuse_run.analysis_run().snapshot(),
            revision,
        );
        super::model::EvidenceReuseEnabledAnalysisBinding {
            analysis_snapshot_identity: analysis_snapshot.identity.clone(),
            analysis_snapshot,
            reusable_snapshot_identity: reuse_run.reusable_snapshot_identity().to_tagged_string(),
            governance_event_boundary: reuse_run.governance_event_boundary_at_run(),
            projection_version: super::model::REUSABLE_INFLUENCE_PROJECTION_VERSION.to_owned(),
        }
    });
    let candidates = session.reuse_candidates().expect("candidates");
    session
        .supersede_reusable_influence(
            ReusableInfluenceRecordId::from_promotion_event_index(0),
            &candidates[0].key,
        )
        .expect("supersede");
    let mut state = project_session(
        &session,
        "session:current-contract:superseded-historical-run",
        "writer:superseded-historical-run",
    );
    state.reuse_enabled_analysis_binding = binding;
    if let Some(binding) = &state.reuse_enabled_analysis_binding {
        state
            .analysis_snapshots
            .push(binding.analysis_snapshot.clone());
    }
    super::derivation::finalize_derived_fields(&mut state);
    state.normalize()
}

pub fn build_offset_anchor_manual_replacement_state() -> CurrentContractState {
    project_session(
        &build_offset_anchor_session(),
        "session:current-contract:offset-anchor",
        "writer:offset-anchor",
    )
}

pub fn build_duplicated_session_lineage_state() -> CurrentContractState {
    let original = build_promoted_active_state();
    let mut duplicate = original.clone();
    duplicate.session_id = "session:current-contract:duplicate".to_owned();
    duplicate.duplicated_from_session_id = Some(original.session_id.clone());
    duplicate.durable_command_tokens.evidence_writer_token = "writer:duplicate".to_owned();
    duplicate.normalize()
}

pub fn build_original_for_duplication_fixture() -> CurrentContractState {
    build_promoted_active_state()
}

fn project_session(
    session: &ApplicationReviewSession,
    session_id: &str,
    writer_token: &str,
) -> CurrentContractState {
    let material_use =
        ApplicationMaterialUseDeclaration::new(DeclaredApplicationMaterialUseBasis::SelfOwned);
    project_current_contract_state(session, &material_use, session_id, writer_token)
}

fn session_authority() -> DeclaredSessionAuthority {
    DeclaredSessionAuthority::new(
        DeclaredSessionOperatorRole::DeclaredLocalOwnerOperator,
        "Ezra",
    )
    .expect("valid authority label")
}

fn material_use() -> ApplicationMaterialUseDeclaration {
    ApplicationMaterialUseDeclaration::new(DeclaredApplicationMaterialUseBasis::SelfOwned)
}

fn build_base_manual_replacement_session() -> ApplicationReviewSession {
    let transcript = parse_srt(GOLDEN_SMALL_TRANSCRIPT).expect("golden transcript");
    let terms = vec![SessionTermEntry::new(
        "Kafka",
        vec!["Kafak".to_owned()],
        Vec::new(),
    )];
    let mut session =
        begin_application_review(transcript, terms, material_use(), session_authority())
            .expect("session");
    let target = session.review_items()[0].target;
    session
        .record_manual_replacement(target, "Kafka")
        .expect("manual replacement");
    session
}

fn build_offset_anchor_session() -> ApplicationReviewSession {
    let transcript = parse_srt(OFFSET_ANCHOR_TRANSCRIPT).expect("offset transcript");
    let terms = vec![SessionTermEntry::new(
        "Kafka",
        vec!["Kafak".to_owned()],
        Vec::new(),
    )];
    let mut session =
        begin_application_review(transcript, terms, material_use(), session_authority())
            .expect("session");
    let target = session.review_items()[0].target;
    session
        .record_manual_replacement(target, "Kafka")
        .expect("manual replacement");
    session
}

fn build_candidate_rejected_session() -> ApplicationReviewSession {
    let mut session = build_base_manual_replacement_session();
    session
        .initialize_project_scope("proj-a", "Project A")
        .expect("scope");
    let key = session.reuse_candidates().expect("candidates")[0]
        .key
        .clone();
    session.reject_reuse_candidate(&key).expect("reject");
    session
}

pub fn build_promoted_active_session() -> ApplicationReviewSession {
    let mut session = build_base_manual_replacement_session();
    session
        .initialize_project_scope("proj-a", "Project A")
        .expect("scope");
    let key = session.reuse_candidates().expect("candidates")[0]
        .key
        .clone();
    session.accept_reuse_candidate(&key).expect("promotion");
    session
        .run_reuse_enabled_review()
        .expect("reuse-enabled run");
    session
}

pub fn build_pre_supersession_run_session() -> ApplicationReviewSession {
    let transcript = parse_srt(SUPERSESSION_TRANSCRIPT).expect("supersession transcript");
    let terms = vec![SessionTermEntry::new(
        "Kafka",
        vec!["Kafak".to_owned()],
        Vec::new(),
    )];
    let mut session =
        begin_application_review(transcript, terms, material_use(), session_authority())
            .expect("session");
    for target in session
        .review_items()
        .iter()
        .map(|item| item.target)
        .collect::<Vec<_>>()
    {
        session
            .record_manual_replacement(target, "Kafka")
            .expect("replacement");
    }
    session
        .initialize_project_scope("proj-a", "Project A")
        .expect("scope");
    let candidates = session.reuse_candidates().expect("candidates");
    session
        .accept_reuse_candidate(&candidates[0].key)
        .expect("accept first");
    session
        .run_reuse_enabled_review()
        .expect("reuse-enabled run before supersession");
    session
}

pub fn build_superseded_session() -> ApplicationReviewSession {
    let mut session = build_pre_supersession_run_session();
    let candidates = session.reuse_candidates().expect("candidates");
    let predecessor = ReusableInfluenceRecordId::from_promotion_event_index(0);
    session
        .supersede_reusable_influence(predecessor, &candidates[0].key)
        .expect("supersede");
    session
}
