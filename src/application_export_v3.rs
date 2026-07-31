use crate::analysis::ReuseEnabledAnalysisSnapshot;
use crate::application_export::{escape_export_text, render_application_decision_log};
use crate::application_reuse::ApplicationReuseState;
use crate::application_service::ApplicationReviewExportBundle;
use crate::candidate::ResolvedExactInputContributionEvidence;
use crate::pipeline::{CanonicalTermReviewRun, ReuseEnabledTermReviewRun};
use crate::reusable_influence::{
    EffectiveReusableInfluenceRecord, ReusableGovernanceEvent, ReusableInfluenceLedger,
    ReusableInfluenceSnapshot, ReuseCandidate, assert_snapshot_identity_matches_contents,
};
use crate::reuse_primitives::ProjectScope;
use crate::review::ReviewLedger;

pub const DECISION_LOG_V3_HEADER: &str = "voxproof application decision log v3";
pub const SESSION_SUMMARY_V3_HEADER: &str = "voxproof application session summary v3";
const EXPORT_V3_DISCLAIMER: &str = "Human-readable export; not machine re-import; not persistence; \
not authenticated identity; not legal authorization; not validation evidence by itself; \
not a public Experience Pack or Language Pack format.";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ApplicationExportPostureV3 {
    DeclaredOperatorUnauthenticatedInMemoryGate3V0_2,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReuseEnabledProposalProjectionRecord {
    pub detector_id: String,
    pub detector_version: String,
    pub snapshot_identity: crate::reuse_primitives::ReusableInfluenceSnapshotIdentity,
    pub project_scope_id: crate::reuse_primitives::ProjectScopeId,
    pub observed_text: String,
    pub confirmed_replacement: String,
    pub promotion_event_indices: Vec<usize>,
    pub source_locators: Vec<crate::reuse_primitives::SourceDecisionLocator>,
    pub record_ids: Vec<crate::reuse_primitives::ReusableInfluenceRecordId>,
    pub exact_input_contributions: Vec<ResolvedExactInputContributionEvidence>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApplicationReviewExportBundleV3 {
    pub base: ApplicationReviewExportBundle,
    pub export_posture_v3: ApplicationExportPostureV3,
    pub project_scope: ProjectScope,
    pub governance_ledger: ReusableInfluenceLedger,
    pub effective_active_records: Vec<EffectiveReusableInfluenceRecord>,
    pub historical_records: Vec<EffectiveReusableInfluenceRecord>,
    pub derived_candidates_non_authoritative: Vec<ReuseCandidate>,
    pub reusable_snapshot: ReusableInfluenceSnapshot,
    pub reuse_enabled_analysis: Option<ReuseEnabledAnalysisSnapshot>,
    pub reuse_enabled_proposal_projections: Vec<ReuseEnabledProposalProjectionRecord>,
}

pub fn build_export_bundle_v3(
    base: ApplicationReviewExportBundle,
    reuse_state: &ApplicationReuseState,
    review_ledger: &ReviewLedger,
    canonical_run: &CanonicalTermReviewRun,
    derived_candidates: Vec<ReuseCandidate>,
    reusable_snapshot: ReusableInfluenceSnapshot,
    reuse_enabled_run: Option<&ReuseEnabledTermReviewRun>,
) -> Result<ApplicationReviewExportBundleV3, crate::application_reuse::ApplicationReuseError> {
    let project_scope = reuse_state
        .project_scope()
        .cloned()
        .ok_or(crate::application_reuse::ApplicationReuseError::MissingProjectScope)?;
    let effective = reuse_state.effective_state(review_ledger, canonical_run);
    assert_snapshot_identity_matches_contents(&reusable_snapshot)?;
    let reuse_enabled_proposal_projections =
        collect_reuse_enabled_proposal_projections(reuse_enabled_run);
    Ok(ApplicationReviewExportBundleV3 {
        base,
        export_posture_v3:
            ApplicationExportPostureV3::DeclaredOperatorUnauthenticatedInMemoryGate3V0_2,
        project_scope,
        governance_ledger: reuse_state.governance_ledger().clone(),
        effective_active_records: effective.active_records().to_vec(),
        historical_records: effective.historical_records().to_vec(),
        derived_candidates_non_authoritative: derived_candidates,
        reusable_snapshot,
        reuse_enabled_analysis: reuse_enabled_run.map(|run| run.reuse_enabled_snapshot()),
        reuse_enabled_proposal_projections,
    })
}

fn collect_reuse_enabled_proposal_projections(
    reuse_enabled_run: Option<&ReuseEnabledTermReviewRun>,
) -> Vec<ReuseEnabledProposalProjectionRecord> {
    let Some(run) = reuse_enabled_run else {
        return Vec::new();
    };
    let mut records = Vec::new();
    for review_case in run.review_cases() {
        let candidate = review_case.candidate_span();
        let crate::candidate::Evidence::ReusableExactObservedForm(evidence) = candidate.evidence()
        else {
            continue;
        };
        records.push(ReuseEnabledProposalProjectionRecord {
            detector_id: candidate.provenance().detector_id().to_owned(),
            detector_version: candidate.provenance().detector_version().to_owned(),
            snapshot_identity: evidence.snapshot_identity,
            project_scope_id: evidence.project_scope_id.clone(),
            observed_text: evidence.observed_text.clone(),
            confirmed_replacement: evidence.confirmed_replacement.clone(),
            promotion_event_indices: evidence.promotion_event_indices.clone(),
            source_locators: evidence
                .contributions
                .iter()
                .map(|contribution| contribution.source_locator.clone())
                .collect(),
            record_ids: evidence
                .contributions
                .iter()
                .map(|contribution| contribution.record_id)
                .collect(),
            exact_input_contributions: evidence.exact_input_contributions.clone(),
        });
    }
    records
}

pub fn render_application_decision_log_v3(bundle: &ApplicationReviewExportBundleV3) -> String {
    let mut output = String::from(DECISION_LOG_V3_HEADER);
    output.push('\n');
    output.push_str(EXPORT_V3_DISCLAIMER);
    output.push('\n');
    output.push_str(&format!(
        "project_scope_id: {}\n",
        escape_export_text(bundle.project_scope.stable_id.as_str())
    ));
    output.push_str(&format!(
        "project_scope_display_name: {}\n",
        escape_export_text(bundle.project_scope.display_name.as_str())
    ));
    output.push_str("review_decision_events:\n");
    output.push_str(&render_application_decision_log(&bundle.base));
    output.push_str("\nreuse_governance_events:\n");
    for (index, event) in bundle.governance_ledger.events().iter().enumerate() {
        render_governance_event(index, event, &mut output);
    }
    output
}

pub fn render_application_session_summary_v3(bundle: &ApplicationReviewExportBundleV3) -> String {
    let mut output = String::from(SESSION_SUMMARY_V3_HEADER);
    output.push('\n');
    output.push_str(EXPORT_V3_DISCLAIMER);
    output.push('\n');
    output.push_str("export_posture_v3: declared_operator_unauthenticated_in_memory_gate3_v0_2\n");
    output.push_str(&format!(
        "project_scope_id: {}\n",
        escape_export_text(bundle.project_scope.stable_id.as_str())
    ));
    output.push_str(&format!(
        "project_scope_display_name: {}\n",
        escape_export_text(bundle.project_scope.display_name.as_str())
    ));
    output.push_str(&format!(
        "reusable_snapshot_identity: {}\n",
        bundle.reusable_snapshot.identity().to_tagged_string()
    ));
    if let Some(analysis) = &bundle.reuse_enabled_analysis {
        output.push_str(&format!(
            "reuse_enabled_analysis_base_revision: {}\n",
            analysis.base().source_revision().to_tagged_string()
        ));
        output.push_str(&format!(
            "reuse_enabled_analysis_reusable_snapshot: {}\n",
            analysis.reusable_influence_snapshot().to_tagged_string()
        ));
    } else {
        output.push_str("reuse_enabled_analysis: (none)\n");
    }

    output.push_str("\nEffective active reusable records\n");
    if bundle.effective_active_records.is_empty() {
        output.push_str("  (none)\n");
    } else {
        for record in &bundle.effective_active_records {
            render_active_record(record, &mut output);
        }
    }

    output.push_str("\nHistorical reusable records\n");
    if bundle.historical_records.is_empty() {
        output.push_str("  (none)\n");
    } else {
        for record in &bundle.historical_records {
            render_historical_record(record, &mut output);
        }
    }

    output.push_str("\nDerived promotion candidates (non-authoritative)\n");
    if bundle.derived_candidates_non_authoritative.is_empty() {
        output.push_str("  (none)\n");
    } else {
        for candidate in &bundle.derived_candidates_non_authoritative {
            output.push_str("  candidate:\n");
            output.push_str(&format!(
                "    observed_text: {}\n",
                escape_export_text(&candidate.exact_payload.observed_text)
            ));
            output.push_str(&format!(
                "    confirmed_replacement: {}\n",
                escape_export_text(&candidate.exact_payload.confirmed_replacement)
            ));
            output.push_str(&format!(
                "    source_case: local:{}\n",
                candidate
                    .key
                    .source_locator
                    .source_review_case_id
                    .local_index()
            ));
            output.push_str(&format!(
                "    source_decision_still_effective: {}\n",
                candidate.source_decision_still_effective
            ));
        }
    }

    output.push_str("\nReuse-enabled proposal projections (non-authoritative)\n");
    if bundle.reuse_enabled_proposal_projections.is_empty() {
        output.push_str("  (none)\n");
    } else {
        for record in &bundle.reuse_enabled_proposal_projections {
            output.push_str("  proposal:\n");
            output.push_str(&format!(
                "    detector_id: {}\n",
                escape_export_text(&record.detector_id)
            ));
            output.push_str(&format!(
                "    detector_version: {}\n",
                escape_export_text(&record.detector_version)
            ));
            output.push_str(&format!(
                "    snapshot_identity: {}\n",
                record.snapshot_identity.to_tagged_string()
            ));
            output.push_str(&format!(
                "    project_scope_id: {}\n",
                escape_export_text(record.project_scope_id.as_str())
            ));
            output.push_str(&format!(
                "    observed_text: {}\n",
                escape_export_text(&record.observed_text)
            ));
            output.push_str(&format!(
                "    confirmed_replacement: {}\n",
                escape_export_text(&record.confirmed_replacement)
            ));
            output.push_str(&format!(
                "    promotion_event_indices: {:?}\n",
                record.promotion_event_indices
            ));
            output.push_str(&format!(
                "    record_ids: {:?}\n",
                record
                    .record_ids
                    .iter()
                    .map(|id| id.promotion_event_index())
                    .collect::<Vec<_>>()
            ));
            output.push_str("    exact_input_contributions:\n");
            for contribution in &record.exact_input_contributions {
                match contribution {
                    ResolvedExactInputContributionEvidence::BaseObservedErrorForm {
                        session_term_canonical,
                        observed_form,
                    } => {
                        output.push_str("      - kind: base_observed_error_form\n");
                        output.push_str(&format!(
                            "        session_term_canonical: {}\n",
                            escape_export_text(session_term_canonical)
                        ));
                        output.push_str(&format!(
                            "        observed_form: {}\n",
                            escape_export_text(observed_form)
                        ));
                    }
                    ResolvedExactInputContributionEvidence::ReusableInfluenceRecord {
                        record_id,
                        promotion_event_index,
                        source_locator,
                    } => {
                        output.push_str("      - kind: reusable_influence_record\n");
                        output.push_str(&format!(
                            "        record_id: promotion_event:{}\n",
                            record_id.promotion_event_index()
                        ));
                        output.push_str(&format!(
                            "        promotion_event: {}\n",
                            promotion_event_index
                        ));
                        output.push_str(&format!(
                            "        source_case_local: {}\n",
                            source_locator.source_review_case_id.local_index() + 1
                        ));
                    }
                }
            }
        }
    }

    output.push_str("\nReviewed output posture\n");
    output.push_str(&format!(
        "decision_coverage: {:?}\n",
        bundle.base.progress.decision_coverage
    ));
    output.push_str(&format!(
        "resolution_status: {:?}\n",
        bundle.base.progress.resolution_status
    ));
    output.push_str(&format!(
        "export_posture_v2: {:?}\n",
        bundle.base.export_posture
    ));
    output
}

fn render_governance_event(index: usize, event: &ReusableGovernanceEvent, output: &mut String) {
    output.push_str(&format!("event {}\n", index + 1));
    match event {
        ReusableGovernanceEvent::PromotionCandidateRejected {
            candidate_key,
            actor,
        } => {
            output.push_str("type: promotion_candidate_rejected\n");
            output.push_str(&format!(
                "actor_role: {}\n",
                escape_export_text(&actor.role_label)
            ));
            output.push_str(&format!(
                "actor_label: {}\n",
                escape_export_text(&actor.display_label)
            ));
            output.push_str(&format!(
                "source_case: local:{}\n",
                candidate_key
                    .source_locator
                    .source_review_case_id
                    .local_index()
            ));
        }
        ReusableGovernanceEvent::PromotionAccepted {
            payload,
            source_locator,
            actor,
            project_scope,
            ..
        } => {
            output.push_str("type: promotion_accepted\n");
            output.push_str(&format!(
                "project_scope_id: {}\n",
                escape_export_text(project_scope.stable_id.as_str())
            ));
            output.push_str(&format!(
                "observed_text: {}\n",
                escape_export_text(&payload.observed_text)
            ));
            output.push_str(&format!(
                "confirmed_replacement: {}\n",
                escape_export_text(&payload.confirmed_replacement)
            ));
            output.push_str(&format!(
                "actor_role: {}\n",
                escape_export_text(&actor.role_label)
            ));
            output.push_str(&format!(
                "actor_label: {}\n",
                escape_export_text(&actor.display_label)
            ));
            output.push_str(&format!(
                "source_case: local:{}\n",
                source_locator.source_review_case_id.local_index()
            ));
            output.push_str(&format!(
                "review_ledger_position: {}\n",
                source_locator.review_ledger_position
            ));
        }
        ReusableGovernanceEvent::ReusableInfluenceRevoked { record_id, actor } => {
            output.push_str("type: reusable_influence_revoked\n");
            output.push_str(&format!(
                "record_id: promotion_event:{}\n",
                record_id.promotion_event_index()
            ));
            output.push_str(&format!(
                "actor_role: {}\n",
                escape_export_text(&actor.role_label)
            ));
            output.push_str(&format!(
                "actor_label: {}\n",
                escape_export_text(&actor.display_label)
            ));
        }
        ReusableGovernanceEvent::ReusableInfluenceSuperseded {
            predecessor_id,
            successor_id,
            actor,
        } => {
            output.push_str("type: reusable_influence_superseded\n");
            output.push_str(&format!(
                "predecessor_id: promotion_event:{}\n",
                predecessor_id.promotion_event_index()
            ));
            output.push_str(&format!(
                "successor_id: promotion_event:{}\n",
                successor_id.promotion_event_index()
            ));
            output.push_str(&format!(
                "actor_role: {}\n",
                escape_export_text(&actor.role_label)
            ));
            output.push_str(&format!(
                "actor_label: {}\n",
                escape_export_text(&actor.display_label)
            ));
        }
    }
}

fn render_active_record(record: &EffectiveReusableInfluenceRecord, output: &mut String) {
    output.push_str(&format!(
        "  record promotion_event:{}\n",
        record.record_id.promotion_event_index()
    ));
    output.push_str(&format!(
        "    observed_text: {}\n",
        escape_export_text(&record.payload.observed_text)
    ));
    output.push_str(&format!(
        "    confirmed_replacement: {}\n",
        escape_export_text(&record.payload.confirmed_replacement)
    ));
    output.push_str(&format!(
        "    source_decision_still_effective: {}\n",
        record.source_decision_still_effective
    ));
}

fn render_historical_record(record: &EffectiveReusableInfluenceRecord, output: &mut String) {
    render_active_record(record, output);
    if let Some(successor) = record.superseded_by {
        output.push_str(&format!(
            "    superseded_by: promotion_event:{}\n",
            successor.promotion_event_index()
        ));
    }
}

pub fn unescape_export_text(encoded: &str) -> Result<String, String> {
    let mut output = String::new();
    let mut chars = encoded.chars().peekable();
    while let Some(character) = chars.next() {
        if character == '\\' {
            let next = chars.next().ok_or_else(|| "trailing escape".to_string())?;
            match next {
                '\\' => output.push('\\'),
                'n' => output.push('\n'),
                'r' => output.push('\r'),
                't' => output.push('\t'),
                'u' => {
                    if chars.next() != Some('{') {
                        return Err("invalid unicode escape".to_string());
                    }
                    let mut hex = String::new();
                    while let Some(&peek) = chars.peek() {
                        if peek == '}' {
                            chars.next();
                            break;
                        }
                        hex.push(peek);
                        chars.next();
                    }
                    let code = u32::from_str_radix(&hex, 16).map_err(|_| "invalid unicode hex")?;
                    let decoded = char::from_u32(code).ok_or("invalid unicode codepoint")?;
                    output.push(decoded);
                }
                other => return Err(format!("unknown escape: {other}")),
            }
        } else {
            output.push(character);
        }
    }
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn escape_round_trip_is_deterministic() {
        let samples = [
            "plain",
            "a\\b",
            "a\nb",
            "a\rb",
            "a\tb",
            "a\u{0001}b",
            "我們",
        ];
        for sample in samples {
            assert_eq!(
                unescape_export_text(&escape_export_text(sample)).expect("round trip"),
                sample
            );
        }
    }
}
