use crate::application_service::{
    ApplicationMaterialUseDeclaration, ApplicationReviewSession,
    DeclaredApplicationMaterialUseBasis, DeclaredSessionAuthority, DeclaredSessionOperatorRole,
    begin_application_review,
};
use crate::candidate::SessionTermEntry;
use crate::srt::parse_srt;

use super::model::{
    CURRENT_CONTRACT_FIXTURE_ID, CURRENT_CONTRACT_FIXTURE_VERSION, CurrentContractFixture,
    CurrentContractFixtureScale, CurrentContractState,
};
use super::projection::project_current_contract_state;

pub const GOLDEN_SMALL_TRANSCRIPT: &str = "1\n00:00:00,000 --> 00:00:01,000\nKafak\n";

pub fn golden_small() -> CurrentContractFixture {
    CurrentContractFixture {
        fixture_id: CURRENT_CONTRACT_FIXTURE_ID.to_owned(),
        fixture_version: CURRENT_CONTRACT_FIXTURE_VERSION.to_owned(),
        scale: CurrentContractFixtureScale::Small,
        expected_state: build_golden_small_state(),
    }
}

pub fn build_golden_small_state() -> CurrentContractState {
    let material_use =
        ApplicationMaterialUseDeclaration::new(DeclaredApplicationMaterialUseBasis::SelfOwned);
    let authority = DeclaredSessionAuthority::new(
        DeclaredSessionOperatorRole::DeclaredLocalOwnerOperator,
        "Ezra",
    )
    .expect("valid authority label");
    let transcript = parse_srt(GOLDEN_SMALL_TRANSCRIPT).expect("golden transcript");
    let terms = vec![SessionTermEntry::new(
        "Kafka",
        vec!["Kafak".to_owned()],
        Vec::new(),
    )];
    let mut session =
        begin_application_review(transcript, terms, material_use, authority).expect("session");
    session
        .initialize_project_scope("proj-a", "Project A")
        .expect("scope");
    let target = session.review_items()[0].target;
    session
        .record_manual_replacement(target, "Kafka")
        .expect("manual replacement");
    let candidates = session.reuse_candidates().expect("candidates");
    assert_eq!(candidates.len(), 1);
    session
        .accept_reuse_candidate(&candidates[0].key)
        .expect("promotion");
    session
        .run_reuse_enabled_review()
        .expect("reuse-enabled run");
    project_current_contract_state(&session, &material_use)
}

pub fn build_golden_small_session() -> ApplicationReviewSession {
    let material_use =
        ApplicationMaterialUseDeclaration::new(DeclaredApplicationMaterialUseBasis::SelfOwned);
    let authority = DeclaredSessionAuthority::new(
        DeclaredSessionOperatorRole::DeclaredLocalOwnerOperator,
        "Ezra",
    )
    .expect("valid authority label");
    let transcript = parse_srt(GOLDEN_SMALL_TRANSCRIPT).expect("golden transcript");
    let terms = vec![SessionTermEntry::new(
        "Kafka",
        vec!["Kafak".to_owned()],
        Vec::new(),
    )];
    let mut session =
        begin_application_review(transcript, terms, material_use, authority).expect("session");
    session
        .initialize_project_scope("proj-a", "Project A")
        .expect("scope");
    let target = session.review_items()[0].target;
    session
        .record_manual_replacement(target, "Kafka")
        .expect("manual replacement");
    let candidates = session.reuse_candidates().expect("candidates");
    session
        .accept_reuse_candidate(&candidates[0].key)
        .expect("promotion");
    session.run_reuse_enabled_review().expect("reuse run");
    session
}
