//! Lifecycle-derived precursor/target pairs for comparative transition measurements.
//!
//! Each pair shares one `session_id` and reflects a real authoritative command boundary.

use crate::application_service::{
    begin_application_review, ApplicationMaterialUseDeclaration, ApplicationReviewSession,
    DeclaredApplicationMaterialUseBasis, DeclaredSessionAuthority, DeclaredSessionOperatorRole,
};
use crate::candidate::SessionTermEntry;
use crate::review::CorrectionDecision;
use crate::reuse_primitives::ReusableInfluenceRecordId;
use crate::srt::parse_srt;

use super::super::fixture::{
    build_promoted_active_session, GOLDEN_SMALL_TRANSCRIPT, SUPERSESSION_TRANSCRIPT,
};
use super::super::measurement::MeasurementFixtureScale;
use super::super::model::CurrentContractState;
use super::super::projection::project_current_contract_state;

const MEDIUM_TRANSCRIPT_SEGMENT_COUNT: usize = 12;

pub fn measurement_transition_states(
    operation: &str,
    scale: MeasurementFixtureScale,
) -> Option<(CurrentContractState, CurrentContractState)> {
    match operation {
        "append_review_decision" => Some(review_decision_transition(scale)),
        "append_manual_replacement" => Some(manual_replacement_transition(scale)),
        "append_reusable_promotion" => Some(reusable_promotion_transition(scale)),
        "append_reusable_revocation" => Some(reusable_revocation_transition(scale)),
        "append_reusable_supersession" => Some(reusable_supersession_transition(scale)),
        _ => None,
    }
}

fn review_decision_transition(scale: MeasurementFixtureScale) -> (CurrentContractState, CurrentContractState) {
    let (session_id, writer_token) = measurement_identity(scale);
    let mut session = pending_review_session(scale);
    let precursor = project_measurement(&session, session_id, writer_token);
    let target = session.review_items()[0].target;
    session
        .record_human_decision(
            target,
            CorrectionDecision::AcceptAlternative { alternative_index: 0 },
        )
        .expect("review decision");
    let target_state = project_measurement(&session, session_id, writer_token);
    (precursor, target_state)
}

fn manual_replacement_transition(
    scale: MeasurementFixtureScale,
) -> (CurrentContractState, CurrentContractState) {
    let (session_id, writer_token) = measurement_identity(scale);
    let mut session = pending_review_session(scale);
    let precursor = project_measurement(&session, session_id, writer_token);
    let target = session.review_items()[0].target;
    session
        .record_manual_replacement(target, "Kafka")
        .expect("manual replacement");
    let target_state = project_measurement(&session, session_id, writer_token);
    (precursor, target_state)
}

fn reusable_promotion_transition(
    scale: MeasurementFixtureScale,
) -> (CurrentContractState, CurrentContractState) {
    let (session_id, writer_token) = measurement_identity(scale);
    let mut session = scope_initialized_session(scale);
    let precursor = project_measurement(&session, session_id, writer_token);
    let key = session.reuse_candidates().expect("candidates")[0]
        .key
        .clone();
    session
        .accept_reuse_candidate(&key)
        .expect("reuse promotion");
    session
        .run_reuse_enabled_review()
        .expect("reuse-enabled run");
    let target_state = project_measurement(&session, session_id, writer_token);
    (precursor, target_state)
}

fn reusable_revocation_transition(
    scale: MeasurementFixtureScale,
) -> (CurrentContractState, CurrentContractState) {
    let (session_id, writer_token) = measurement_identity(scale);
    let session_before_revoke = promoted_session(scale);
    let binding = reuse_binding_from_session(&session_before_revoke);
    let mut session = session_before_revoke;
    let precursor = project_measurement(&session, session_id, writer_token);
    session
        .revoke_reusable_influence(ReusableInfluenceRecordId::from_promotion_event_index(0))
        .expect("revocation");
    let mut target_state = project_measurement(&session, session_id, writer_token);
    target_state.reuse_enabled_analysis_binding = binding;
    if let Some(binding) = &target_state.reuse_enabled_analysis_binding
        && !target_state
            .analysis_snapshots
            .iter()
            .any(|snapshot| snapshot.identity == binding.analysis_snapshot_identity)
    {
        target_state
            .analysis_snapshots
            .push(binding.analysis_snapshot.clone());
    }
    super::super::derivation::finalize_derived_fields(&mut target_state);
    let target_state = target_state.normalize();
    (precursor, target_state)
}

fn reusable_supersession_transition(
    scale: MeasurementFixtureScale,
) -> (CurrentContractState, CurrentContractState) {
    let (session_id, writer_token) = measurement_identity(scale);
    let mut session = promoted_session_for_supersession(scale);
    let precursor = project_measurement(&session, session_id, writer_token);
    let candidates = session.reuse_candidates().expect("candidates");
    session
        .supersede_reusable_influence(
            ReusableInfluenceRecordId::from_promotion_event_index(0),
            &candidates[0].key,
        )
        .expect("supersession");
    let target_state = project_measurement(&session, session_id, writer_token);
    (precursor, target_state)
}

fn measurement_identity(scale: MeasurementFixtureScale) -> (&'static str, &'static str) {
    match scale {
        MeasurementFixtureScale::Small => (
            "session:current-contract:measurement-small",
            "writer:measurement-small",
        ),
        MeasurementFixtureScale::Medium => (
            "session:current-contract:measurement-medium",
            "writer:measurement-medium",
        ),
        MeasurementFixtureScale::Stress => (
            "session:current-contract:measurement-stress",
            "writer:measurement-stress",
        ),
    }
}

fn project_measurement(
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

fn glossary_terms() -> Vec<SessionTermEntry> {
    vec![SessionTermEntry::new(
        "Kafka",
        vec!["Kafak".to_owned()],
        Vec::new(),
    )]
}

fn pending_review_session(scale: MeasurementFixtureScale) -> ApplicationReviewSession {
    match scale {
        MeasurementFixtureScale::Small => {
            let transcript = parse_srt(GOLDEN_SMALL_TRANSCRIPT).expect("golden transcript");
            begin_application_review(
                transcript,
                glossary_terms(),
                material_use(),
                session_authority(),
            )
            .expect("small pending review")
        }
        MeasurementFixtureScale::Medium | MeasurementFixtureScale::Stress => {
            let transcript =
                parse_srt(&build_medium_transcript_srt(MEDIUM_TRANSCRIPT_SEGMENT_COUNT))
                    .expect("medium transcript");
            begin_application_review(
                transcript,
                glossary_terms(),
                material_use(),
                session_authority(),
            )
            .expect("medium pending review")
        }
    }
}

fn scope_initialized_session(scale: MeasurementFixtureScale) -> ApplicationReviewSession {
    let mut session = match scale {
        MeasurementFixtureScale::Small => {
            let mut session = pending_review_session(scale);
            let target = session.review_items()[0].target;
            session
                .record_manual_replacement(target, "Kafka")
                .expect("manual replacement");
            session
        }
        MeasurementFixtureScale::Medium | MeasurementFixtureScale::Stress => {
            let mut session = pending_review_session(scale);
            for target in session
                .review_items()
                .iter()
                .map(|item| item.target)
                .collect::<Vec<_>>()
            {
                session
                    .record_manual_replacement(target, "Kafka")
                    .expect("manual replacement");
            }
            session
        }
    };
    let (project_id, project_name) = match scale {
        MeasurementFixtureScale::Small => ("proj-a", "Project A"),
        MeasurementFixtureScale::Medium | MeasurementFixtureScale::Stress => {
            ("proj-medium", "Project Medium")
        }
    };
    session
        .initialize_project_scope(project_id, project_name)
        .expect("scope");
    session
}

fn promoted_session(scale: MeasurementFixtureScale) -> ApplicationReviewSession {
    match scale {
        MeasurementFixtureScale::Small => build_promoted_active_session(),
        MeasurementFixtureScale::Medium | MeasurementFixtureScale::Stress => {
            let mut session = scope_initialized_session(scale);
            let initial_candidates = session.reuse_candidates().expect("initial candidates");
            for candidate in &initial_candidates {
                session
                    .accept_reuse_candidate(&candidate.key)
                    .expect("promotion");
            }
            session
                .run_reuse_enabled_review()
                .expect("reuse-enabled run");
            session
        }
    }
}

fn promoted_session_for_supersession(scale: MeasurementFixtureScale) -> ApplicationReviewSession {
    match scale {
        MeasurementFixtureScale::Small => small_pre_supersession_session(),
        MeasurementFixtureScale::Medium | MeasurementFixtureScale::Stress => {
            medium_pre_supersession_session()
        }
    }
}

fn small_pre_supersession_session() -> ApplicationReviewSession {
    let transcript = parse_srt(SUPERSESSION_TRANSCRIPT).expect("supersession transcript");
    let mut session =
        begin_application_review(transcript, glossary_terms(), material_use(), session_authority())
            .expect("supersession session");
    for target in session
        .review_items()
        .iter()
        .map(|item| item.target)
        .collect::<Vec<_>>()
    {
        session
            .record_manual_replacement(target, "Kafka")
            .expect("manual replacement");
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

fn medium_pre_supersession_session() -> ApplicationReviewSession {
    let transcript =
        parse_srt(&build_medium_transcript_srt(MEDIUM_TRANSCRIPT_SEGMENT_COUNT)).expect("medium");
    let mut session =
        begin_application_review(transcript, glossary_terms(), material_use(), session_authority())
            .expect("medium session");
    for target in session
        .review_items()
        .iter()
        .map(|item| item.target)
        .collect::<Vec<_>>()
    {
        session
            .record_manual_replacement(target, "Kafka")
            .expect("manual replacement");
    }
    session
        .initialize_project_scope("proj-medium", "Project Medium")
        .expect("scope");
    let candidates = session.reuse_candidates().expect("initial candidates");
    session
        .accept_reuse_candidate(&candidates[0].key)
        .expect("accept first promotion");
    session
        .run_reuse_enabled_review()
        .expect("initial reuse-enabled run");
    session
}

fn reuse_binding_from_session(
    session: &ApplicationReviewSession,
) -> Option<super::super::model::EvidenceReuseEnabledAnalysisBinding> {
    session.reuse_enabled_run().map(|reuse_run| {
        let revision = session.source().revision_id().to_tagged_string();
        let analysis_snapshot = super::super::projection::map_analysis_snapshot_for_export(
            reuse_run.analysis_run().snapshot(),
            revision,
        );
        super::super::model::EvidenceReuseEnabledAnalysisBinding {
            analysis_snapshot_identity: analysis_snapshot.identity.clone(),
            analysis_snapshot,
            reusable_snapshot_identity: reuse_run.reusable_snapshot_identity().to_tagged_string(),
            governance_event_boundary: reuse_run.governance_event_boundary_at_run(),
            projection_version: super::super::model::REUSABLE_INFLUENCE_PROJECTION_VERSION
                .to_owned(),
        }
    })
}

fn build_medium_transcript_srt(segment_count: usize) -> String {
    let mut out = String::new();
    for index in 0..segment_count {
        let cue = index + 1;
        let start = format_srt_timestamp(index as u64 * 1_000);
        let end = format_srt_timestamp((index as u64 + 1) * 1_000);
        out.push_str(&format!("{cue}\n{start} --> {end}\nKafak\n\n"));
    }
    out
}

fn format_srt_timestamp(ms: u64) -> String {
    let hours = ms / 3_600_000;
    let minutes = (ms % 3_600_000) / 60_000;
    let seconds = (ms % 60_000) / 1_000;
    let millis = ms % 1_000;
    format!("{hours:02}:{minutes:02}:{seconds:02},{millis:03}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::super::CurrentContractOracle;

    #[test]
    fn transition_pairs_share_session_id_per_operation() {
        for operation in [
            "append_review_decision",
            "append_manual_replacement",
            "append_reusable_promotion",
            "append_reusable_revocation",
            "append_reusable_supersession",
        ] {
            for scale in [MeasurementFixtureScale::Small, MeasurementFixtureScale::Medium] {
                let (precursor, target) = measurement_transition_states(operation, scale)
                    .unwrap_or_else(|| panic!("{operation} {scale:?}"));
                assert_eq!(
                    precursor.session_id, target.session_id,
                    "{operation} {scale:?}"
                );
                assert_ne!(
                    precursor.canonical_projection(),
                    target.canonical_projection(),
                    "{operation} {scale:?}"
                );
            }
        }
    }

    #[test]
    fn supersession_small_uses_fixture_semantics() {
        let mut session = small_pre_supersession_session();
        let candidates = session.reuse_candidates().expect("candidates");
        let (_, target) =
            measurement_transition_states("append_reusable_supersession", MeasurementFixtureScale::Small)
                .expect("supersession");
        session
            .supersede_reusable_influence(
                ReusableInfluenceRecordId::from_promotion_event_index(0),
                &candidates[0].key,
            )
            .expect("supersede");
        let expected = project_measurement(
            &session,
            "session:current-contract:measurement-small",
            "writer:measurement-small",
        );
        assert!(CurrentContractOracle::compare(&expected, &target).passed);
    }
}
