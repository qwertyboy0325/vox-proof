use crate::analysis::ReuseEnabledAnalysisSnapshot;
use crate::application_export::{escape_export_text, render_application_decision_log};
use crate::application_reuse::ApplicationReuseState;
use crate::application_service::ApplicationReviewExportBundle;
use crate::pipeline::ReuseEnabledTermReviewRun;
use crate::reusable_influence::{
    EffectiveReusableInfluenceRecord, ReusableGovernanceEvent, ReusableInfluenceLedger,
    ReusableInfluenceSnapshot, ReuseCandidate,
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
}

pub fn build_export_bundle_v3(
    base: ApplicationReviewExportBundle,
    reuse_state: &ApplicationReuseState,
    review_ledger: &ReviewLedger,
    derived_candidates: Vec<ReuseCandidate>,
    reusable_snapshot: ReusableInfluenceSnapshot,
    reuse_enabled_run: Option<&ReuseEnabledTermReviewRun>,
) -> Result<ApplicationReviewExportBundleV3, crate::application_reuse::ApplicationReuseError> {
    let project_scope = reuse_state
        .project_scope
        .clone()
        .ok_or(crate::application_reuse::ApplicationReuseError::MissingProjectScope)?;
    let effective = reuse_state.effective_state(review_ledger);
    Ok(ApplicationReviewExportBundleV3 {
        base,
        export_posture_v3:
            ApplicationExportPostureV3::DeclaredOperatorUnauthenticatedInMemoryGate3V0_2,
        project_scope,
        governance_ledger: reuse_state.governance_ledger.clone(),
        effective_active_records: effective.active_records,
        historical_records: effective.historical_records,
        derived_candidates_non_authoritative: derived_candidates,
        reusable_snapshot,
        reuse_enabled_analysis: reuse_enabled_run.map(|run| run.reuse_enabled_snapshot()),
    })
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
    output.push_str(
        "export_posture_v3: declared_operator_unauthenticated_in_memory_gate3_v0_2\n",
    );
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
        bundle.reusable_snapshot.identity.to_tagged_string()
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
