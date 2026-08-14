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

/// Scoped command lineage for genuine stale-command FCR-03 scenarios (R2-02).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GenuineStaleScenarioFixture {
    pub prepare_authority: CurrentContractState,
    pub prepared_target: CurrentContractState,
    pub competing_target: CurrentContractState,
    pub scope: GenuineStaleCommandScope,
    pub expected_failure_code: &'static str,
    pub analysis_details: Option<GenuineStaleAnalysisDetails>,
}

/// Pairwise-distinct analysis identities for R2-02 stale-analysis observation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GenuineStaleAnalysisDetails {
    pub analysis_identity_before: String,
    pub competing_identity_after: String,
    pub stale_command_target_identity: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GenuineStaleCommandScope {
    ReviewLedger,
    ReuseGovernance,
    ActiveAnalysis,
}

/// Returns S0 + prepared command target for a genuine stale scenario.
///
/// Competing transition applies the same scoped command at S0; a subsequently
/// executed prepared command retains S0 preconditions and must be rejected.
pub fn genuine_stale_scenario_fixture(
    scenario_id: &str,
    scale: MeasurementFixtureScale,
) -> Option<GenuineStaleScenarioFixture> {
    match scenario_id {
        "stale-review-ledger-command" => {
            let (prepare_authority, prepared_target) =
                measurement_transition_states("append_review_decision", scale)?;
            Some(GenuineStaleScenarioFixture {
                prepare_authority,
                prepared_target: prepared_target.clone(),
                competing_target: prepared_target,
                scope: GenuineStaleCommandScope::ReviewLedger,
                expected_failure_code: "stale-review-ledger-precondition",
                analysis_details: None,
            })
        }
        "stale-reuse-governance-command" => {
            let (prepare_authority, prepared_target) =
                measurement_transition_states("append_reusable_revocation", scale)?;
            Some(GenuineStaleScenarioFixture {
                prepare_authority,
                prepared_target: prepared_target.clone(),
                competing_target: prepared_target,
                scope: GenuineStaleCommandScope::ReuseGovernance,
                expected_failure_code: "stale-reuse-governance-precondition",
                analysis_details: None,
            })
        }
        "stale-analysis-attachment-or-selection" => Some(genuine_stale_analysis_fixture(scale)?),
        _ => None,
    }
}

/// R2-02 evidence-only analysis stale fixture: review+reuse-advanced medium authority
/// with two oracle-valid reuse-enabled analysis selections prepared at the same S0.
fn genuine_stale_analysis_fixture(
    scale: MeasurementFixtureScale,
) -> Option<GenuineStaleScenarioFixture> {
    let prepare_authority = genuine_stale_analysis_prepare_authority(scale);
    let (_, promotion_medium) = measurement_transition_states(
        "append_reusable_promotion",
        MeasurementFixtureScale::Medium,
    )?;
    let (_, promotion_small) = measurement_transition_states("append_reusable_promotion", scale)?;

    let analysis_identity_before = prepare_authority
        .durable_command_tokens
        .active_analysis_snapshot_identity
        .clone();
    let competing_target = analysis_selection_target_from(
        &prepare_authority,
        &promotion_medium,
        reuse_attachment_identity(&promotion_medium).as_deref(),
    );
    let prepared_target = analysis_selection_target_from(
        &prepare_authority,
        &promotion_small,
        reuse_attachment_identity(&promotion_small).as_deref(),
    );
    let competing_identity_after = competing_target
        .durable_command_tokens
        .active_analysis_snapshot_identity
        .clone();
    let stale_command_target_identity = prepared_target
        .durable_command_tokens
        .active_analysis_snapshot_identity
        .clone();

    Some(GenuineStaleScenarioFixture {
        prepare_authority,
        prepared_target,
        competing_target,
        scope: GenuineStaleCommandScope::ActiveAnalysis,
        expected_failure_code: "stale-analysis-selection-precondition",
        analysis_details: Some(GenuineStaleAnalysisDetails {
            analysis_identity_before,
            competing_identity_after,
            stale_command_target_identity,
        }),
    })
}

/// Oracle-valid S0 for analysis stale: unrelated review+reuse advances on medium
/// partial session, plus small-scale source revisions so a second reuse-enabled
/// analysis attachment remains oracle-valid without changing authority semantics.
pub fn genuine_stale_analysis_prepare_authority(
    scale: MeasurementFixtureScale,
) -> CurrentContractState {
    use super::super::CurrentContractOracle;
    use super::super::finalize_derived_fields;

    let (review_precursor, reuse_advanced, review_target) = unrelated_scope_success_fixture(scale);
    let (_, promotion_small) =
        measurement_transition_states("append_reusable_promotion", scale).expect("small promotion");

    let mut prepare_authority = review_precursor;
    prepare_authority.review_ledger_events = review_target.review_ledger_events.clone();
    prepare_authority.effective_review_status = review_target.effective_review_status.clone();
    prepare_authority.durable_command_tokens.review_ledger_head =
        review_target.durable_command_tokens.review_ledger_head;
    prepare_authority.reuse_governance_events = reuse_advanced.reuse_governance_events.clone();
    prepare_authority.effective_reusable_records = reuse_advanced.effective_reusable_records.clone();
    prepare_authority.historical_reusable_records =
        reuse_advanced.historical_reusable_records.clone();
    prepare_authority.reusable_snapshot_identity = reuse_advanced.reusable_snapshot_identity.clone();
    prepare_authority.reuse_enabled_analysis_binding =
        reuse_advanced.reuse_enabled_analysis_binding.clone();
    prepare_authority.durable_command_tokens.reuse_governance_head =
        reuse_advanced.durable_command_tokens.reuse_governance_head;
    for revision in &promotion_small.source_revisions {
        if !prepare_authority
            .source_revisions
            .iter()
            .any(|existing| existing.revision_id == revision.revision_id)
        {
            prepare_authority.source_revisions.push(revision.clone());
        }
    }
    finalize_derived_fields(&mut prepare_authority);
    let prepare_authority = prepare_authority.normalize();
    assert!(
        CurrentContractOracle::validate(&prepare_authority).passed,
        "analysis stale S0 must be oracle-valid"
    );
    prepare_authority
}

fn reuse_attachment_identity(state: &CurrentContractState) -> Option<String> {
    state
        .reuse_enabled_analysis_binding
        .as_ref()
        .map(|binding| binding.analysis_snapshot_identity.clone())
}

pub fn prepared_precondition_label(
    scope: GenuineStaleCommandScope,
    prepare_authority: &CurrentContractState,
) -> String {
    match scope {
        GenuineStaleCommandScope::ReviewLedger => format!(
            "review_ledger_head={}",
            prepare_authority
                .durable_command_tokens
                .review_ledger_head
        ),
        GenuineStaleCommandScope::ReuseGovernance => format!(
            "reuse_governance_head={}",
            prepare_authority
                .durable_command_tokens
                .reuse_governance_head
        ),
        GenuineStaleCommandScope::ActiveAnalysis => format!(
            "active_analysis_snapshot_identity={}",
            prepare_authority
                .durable_command_tokens
                .active_analysis_snapshot_identity
        ),
    }
}

pub fn authority_changed_in_relevant_scope(
    scope: GenuineStaleCommandScope,
    before: &CurrentContractState,
    after: &CurrentContractState,
) -> bool {
    match scope {
        GenuineStaleCommandScope::ReviewLedger => {
            before.durable_command_tokens.review_ledger_head
                != after.durable_command_tokens.review_ledger_head
        }
        GenuineStaleCommandScope::ReuseGovernance => {
            before.durable_command_tokens.reuse_governance_head
                != after.durable_command_tokens.reuse_governance_head
        }
        GenuineStaleCommandScope::ActiveAnalysis => {
            before.durable_command_tokens.active_analysis_snapshot_identity
                != after.durable_command_tokens.active_analysis_snapshot_identity
        }
    }
}

fn analysis_selection_target_from(
    precursor: &CurrentContractState,
    attachment_source: &CurrentContractState,
    selected_identity: Option<&str>,
) -> CurrentContractState {
    let mut target = precursor.clone();
    for snapshot in &attachment_source.analysis_snapshots {
        if !target
            .analysis_snapshots
            .iter()
            .any(|existing| existing.identity == snapshot.identity)
        {
            target.analysis_snapshots.push(snapshot.clone());
        }
    }
    if let Some(binding) = &attachment_source.reuse_enabled_analysis_binding
        && !target
            .analysis_snapshots
            .iter()
            .any(|snapshot| snapshot.identity == binding.analysis_snapshot_identity)
    {
        target
            .analysis_snapshots
            .push(binding.analysis_snapshot.clone());
    }
    let identity = selected_identity
        .map(str::to_owned)
        .or_else(|| {
            attachment_source
                .reuse_enabled_analysis_binding
                .as_ref()
                .map(|binding| binding.analysis_snapshot_identity.clone())
        })
        .unwrap_or_else(|| {
            attachment_source
                .durable_command_tokens
                .active_analysis_snapshot_identity
                .clone()
        });
    target.durable_command_tokens.active_analysis_snapshot_identity = identity;
    super::super::finalize_derived_fields(&mut target);
    target.normalize()
}

/// MD-015 U1 proof fixture: reuse-only advance leaves review scope unchanged so a
/// review command prepared at the initial authority remains valid.
pub fn unrelated_scope_success_fixture(
    scale: MeasurementFixtureScale,
) -> (CurrentContractState, CurrentContractState, CurrentContractState) {
    let scale = match scale {
        MeasurementFixtureScale::Small => MeasurementFixtureScale::Medium,
        other => other,
    };
    let (session_id, writer_token) = measurement_identity(scale);
    let mut session = pending_review_session(scale);
    let first = session.review_items()[0].target;
    session
        .record_manual_replacement(first, "Kafka")
        .expect("partial review advance");
    session
        .initialize_project_scope("proj-a", "Project A")
        .expect("project scope");
    let initial = project_measurement(&session, session_id, writer_token);
    let key = session.reuse_candidates().expect("candidates")[0]
        .key
        .clone();
    session
        .accept_reuse_candidate(&key)
        .expect("reuse-only promotion");
    let reuse_only = project_measurement(&session, session_id, writer_token);
    assert_eq!(
        initial.durable_command_tokens.review_ledger_head,
        reuse_only.durable_command_tokens.review_ledger_head,
        "reuse-only advance must not change review scope"
    );
    assert_ne!(
        initial.durable_command_tokens.reuse_governance_head,
        reuse_only.durable_command_tokens.reuse_governance_head,
        "reuse scope must advance"
    );

    let mut review_session = pending_review_session(scale);
    review_session
        .record_manual_replacement(first, "Kafka")
        .expect("partial review advance");
    review_session
        .initialize_project_scope("proj-a", "Project A")
        .expect("project scope");
    let second = review_session.review_items()[1].target;
    review_session
        .record_manual_replacement(second, "Kafka")
        .expect("review-scope advance");
    let review_target = project_measurement(&review_session, session_id, writer_token);
    assert_ne!(
        initial.durable_command_tokens.review_ledger_head,
        review_target.durable_command_tokens.review_ledger_head,
        "review command must advance review scope"
    );
    (initial, reuse_only, review_target)
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
    fn genuine_stale_fixtures_cover_three_scopes_with_distinct_analysis_targets() {
        for scenario_id in [
            "stale-review-ledger-command",
            "stale-reuse-governance-command",
            "stale-analysis-attachment-or-selection",
        ] {
            let fixture = genuine_stale_scenario_fixture(scenario_id, MeasurementFixtureScale::Small)
                .unwrap_or_else(|| panic!("{scenario_id}"));
            assert!(
                authority_changed_in_relevant_scope(
                    fixture.scope,
                    &fixture.prepare_authority,
                    &fixture.competing_target,
                ),
                "{scenario_id}: competing target must advance relevant scope"
            );
            if fixture.scope == GenuineStaleCommandScope::ActiveAnalysis {
                let details = fixture
                    .analysis_details
                    .as_ref()
                    .expect("analysis stale fixture details");
                assert_ne!(
                    details.analysis_identity_before,
                    details.competing_identity_after
                );
                assert_ne!(
                    details.analysis_identity_before,
                    details.stale_command_target_identity
                );
                assert_ne!(
                    details.competing_identity_after,
                    details.stale_command_target_identity
                );
                assert!(CurrentContractOracle::validate(&fixture.prepare_authority).passed);
                assert!(CurrentContractOracle::validate(&fixture.competing_target).passed);
                assert!(CurrentContractOracle::validate(&fixture.prepared_target).passed);
            } else {
                assert_eq!(fixture.competing_target, fixture.prepared_target);
                assert!(fixture.analysis_details.is_none());
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
