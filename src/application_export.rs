use crate::analysis::AnalysisSnapshot;
use crate::application_service::{
    ApplicationDecisionCoverage, ApplicationExportPosture, ApplicationResolutionStatus,
    ApplicationReviewExportBundle, DeclaredApplicationMaterialUseBasis,
    DeclaredSessionOperatorRole,
};
use crate::candidate::DetectionKind;

const DECISION_LOG_HEADER: &str = "voxproof application decision log v2";
const SESSION_SUMMARY_HEADER: &str = "voxproof application session summary v2";
const EXPORT_DISCLAIMER: &str = "Human-readable export; not machine re-import; not persistence; \
not authenticated identity; not legal authorization; not validation evidence by itself.";

pub fn render_application_decision_log(bundle: &ApplicationReviewExportBundle) -> String {
    let mut output = String::from(DECISION_LOG_HEADER);
    output.push('\n');
    output.push_str(EXPORT_DISCLAIMER);
    output.push('\n');
    output.push_str(&format!(
        "declared_session_authority_role: {}\n",
        operator_role_name(bundle.declared_session_authority.role())
    ));
    output.push_str(&format!(
        "declared_session_authority_label: {}\n",
        bundle.declared_session_authority.display_label()
    ));

    for record in &bundle.decision_records {
        output.push('\n');
        output.push_str(&format!("event {}\n", record.event_index + 1));
        output.push_str("type: decision_recorded\n");
        output.push_str(&format!(
            "session_authority_role: {}\n",
            operator_role_name(record.session_authority.role())
        ));
        output.push_str(&format!(
            "session_authority_label: {}\n",
            record.session_authority.display_label()
        ));
        output.push_str(&format!(
            "case_id: {}\n",
            record
                .case_id
                .map(|case_id| format!("local:{}", case_id.local_index()))
                .or_else(|| {
                    record
                        .reuse_proposal_target_identity
                        .map(|identity| identity.to_tagged_string())
                })
                .or_else(|| {
                    record
                        .terminology_proposal_target_identity
                        .map(|identity| identity.to_tagged_string())
                })
                .unwrap_or_else(|| "unknown".to_owned())
        ));
        output.push_str(&format!(
            "observed_revision: {}\n",
            record.observed_revision.to_tagged_string()
        ));
        render_decision_lines(&record.decision, &mut output);
    }

    output
}

pub fn render_application_session_summary(bundle: &ApplicationReviewExportBundle) -> String {
    let summary = &bundle.session_summary;
    let mut output = String::from(SESSION_SUMMARY_HEADER);
    output.push('\n');
    output.push_str(EXPORT_DISCLAIMER);
    output.push('\n');
    output.push_str(&format!(
        "{}\n",
        export_posture_claim_limitations(bundle.export_posture)
    ));

    output.push_str("\nRun identity\n");
    output.push_str(&format!(
        "source_revision: {}\n",
        summary.source_revision.to_tagged_string()
    ));
    output.push_str(&format!(
        "transcript_segments: {}\n",
        summary.transcript_segments
    ));
    output.push_str(&format!(
        "session_term_entry_count: {}\n",
        summary.session_term_entry_count
    ));
    render_analysis_snapshot_lines(bundle.analysis_snapshot, &mut output);

    output.push_str("\nDeclared session authority\n");
    output.push_str(&format!(
        "role: {}\n",
        operator_role_name(bundle.declared_session_authority.role())
    ));
    output.push_str(&format!(
        "display_label: {}\n",
        bundle.declared_session_authority.display_label()
    ));
    output.push_str(&format!(
        "material_use_basis: {}\n",
        material_use_basis_name(bundle.material_use_basis)
    ));

    output.push_str("\nCandidates\n");
    output.push_str(&format!(
        "review_cases_raised: {}\n",
        summary.review_cases_raised
    ));
    output.push_str("by_detection_kind:\n");
    if summary.cases_by_detection_kind.is_empty() {
        output.push_str("  (none)\n");
    } else {
        for item in &summary.cases_by_detection_kind {
            output.push_str(&format!(
                "  {}: {}\n",
                detection_kind_name(item.kind),
                item.count
            ));
        }
    }
    output.push_str("by_detector:\n");
    if summary.cases_by_detector.is_empty() {
        output.push_str("  (none)\n");
    } else {
        for item in &summary.cases_by_detector {
            output.push_str(&format!(
                "  {} @ {}: {}\n",
                item.detector_id, item.detector_version, item.count
            ));
        }
    }

    output.push_str("\nSession correction profile\n");
    output.push_str("decision_counts_basis: effective last-decision-wins status per review case\n");
    output.push_str(&format!(
        "accepted_alternatives: {}\n",
        bundle.decision_summary.accepted_alternatives
    ));
    output.push_str(&format!(
        "manual_replacements: {}\n",
        bundle.decision_summary.manual_replacements
    ));
    output.push_str(&format!("rejected: {}\n", bundle.decision_summary.rejected));
    output.push_str(&format!("deferred: {}\n", bundle.decision_summary.deferred));
    output.push_str(&format!(
        "needs_manual_correction: {}\n",
        bundle.decision_summary.needs_manual_correction
    ));
    output.push_str(&format!(
        "total_decisions_recorded: {}\n",
        bundle.decision_summary.total_recorded_events
    ));
    output.push_str(&format!(
        "undecided_review_cases: {}\n",
        bundle.decision_summary.undecided
    ));
    output.push_str("accepted_replacement_texts_this_session:\n");
    if summary.accepted_replacements.is_empty() {
        output.push_str("  (none)\n");
    } else {
        for item in &summary.accepted_replacements {
            output.push_str(&format!(
                "  {}: {}\n",
                escape_export_text(&item.replacement_text),
                item.count
            ));
        }
    }

    output.push_str("\nOutcomes\n");
    output.push_str(&format!(
        "accepted_replacements_materialized: {}\n",
        summary.outcomes.accepted_replacements_materialized
    ));
    output.push_str(&format!(
        "source_segments_affected: {}\n",
        summary.outcomes.source_segments_affected
    ));

    output.push_str("\nProgress\n");
    output.push_str(&format!(
        "decision_coverage: {}\n",
        decision_coverage_name(bundle.progress.decision_coverage)
    ));
    output.push_str(&format!(
        "resolution_status: {}\n",
        resolution_status_name(bundle.progress.resolution_status)
    ));
    output.push_str(&format!(
        "export_posture: {}\n",
        export_posture_name(bundle.export_posture)
    ));

    output
}

fn render_analysis_snapshot_lines(snapshot: AnalysisSnapshot, output: &mut String) {
    let configuration = snapshot.configuration();
    output.push_str("analysis_snapshot:\n");
    output.push_str(&format!(
        "  source_revision: {}\n",
        snapshot.source_revision().to_tagged_string()
    ));
    output.push_str(&format!(
        "  session_terms: {}\n",
        snapshot.session_terms().to_tagged_string()
    ));
    output.push_str("  detector_set:\n");

    let mut detectors = configuration.detector_set().detectors().to_vec();
    detectors.sort_by_key(|detector| (detector.id(), detector.version()));
    if detectors.is_empty() {
        output.push_str("    (none)\n");
    } else {
        for detector in detectors {
            output.push_str(&format!("    {} @ {}\n", detector.id(), detector.version()));
        }
    }

    let detector_config = configuration.detector_config();
    output.push_str(&format!(
        "  detector_config: {} @ {}\n",
        detector_config.id(),
        detector_config.version()
    ));
    let algorithm = configuration.algorithm();
    output.push_str(&format!(
        "  algorithm: {} @ {}\n",
        algorithm.id(),
        algorithm.version()
    ));
}

pub(crate) fn escape_export_text(text: &str) -> String {
    let mut output = String::with_capacity(text.len());
    for character in text.chars() {
        match character {
            '\\' => output.push_str("\\\\"),
            '\n' => output.push_str("\\n"),
            '\r' => output.push_str("\\r"),
            '\t' => output.push_str("\\t"),
            other if other.is_control() => {
                output.push_str(&format!("\\u{{{:x}}}", other as u32));
            }
            other => output.push(other),
        }
    }
    output
}

fn operator_role_name(role: DeclaredSessionOperatorRole) -> &'static str {
    match role {
        DeclaredSessionOperatorRole::DeclaredLocalOwnerOperator => "declared_local_owner_operator",
        DeclaredSessionOperatorRole::DeclaredAuthorizedHumanReviewer => {
            "declared_authorized_human_reviewer"
        }
    }
}

fn material_use_basis_name(basis: DeclaredApplicationMaterialUseBasis) -> &'static str {
    match basis {
        DeclaredApplicationMaterialUseBasis::SelfOwned => "self_owned",
        DeclaredApplicationMaterialUseBasis::ExplicitPermission => "explicit_permission",
    }
}

fn export_posture_name(posture: ApplicationExportPosture) -> &'static str {
    match posture {
        ApplicationExportPosture::DeclaredOperatorUnauthenticatedInMemoryV0_2 => {
            "declared_operator_unauthenticated_in_memory_v0_2"
        }
    }
}

fn export_posture_claim_limitations(posture: ApplicationExportPosture) -> &'static str {
    match posture {
        ApplicationExportPosture::DeclaredOperatorUnauthenticatedInMemoryV0_2 => {
            "Claim limitations: caller-declared, unauthenticated, single-session, single-process, \
             in-memory operator authority only. Does not establish legal authorization, durable \
             storage, cross-process replay, transport, hydration, or authenticated identity."
        }
    }
}

fn decision_coverage_name(coverage: ApplicationDecisionCoverage) -> String {
    match coverage {
        ApplicationDecisionCoverage::Complete => "complete".to_string(),
        ApplicationDecisionCoverage::Incomplete { undecided } => {
            format!("incomplete undecided={undecided}")
        }
    }
}

fn resolution_status_name(status: ApplicationResolutionStatus) -> String {
    match status {
        ApplicationResolutionStatus::Resolved => "resolved".to_string(),
        ApplicationResolutionStatus::Unresolved {
            deferred,
            needs_manual_correction,
        } => format!(
            "unresolved deferred={deferred} needs_manual_correction={needs_manual_correction}"
        ),
    }
}

fn detection_kind_name(kind: DetectionKind) -> &'static str {
    match kind {
        DetectionKind::GlossaryAliasMatch => "glossary_alias_match",
        DetectionKind::MixedLanguageAnomaly => "mixed_language_anomaly",
        DetectionKind::PhoneticSimilarity => "phonetic_similarity",
        DetectionKind::RepeatedPhrase => "repeated_phrase",
    }
}

fn render_decision_lines(decision: &crate::review::CorrectionDecision, output: &mut String) {
    match decision {
        crate::review::CorrectionDecision::Reject => output.push_str("decision: reject\n"),
        crate::review::CorrectionDecision::Defer => output.push_str("decision: defer\n"),
        crate::review::CorrectionDecision::AcceptAlternative { alternative_index } => {
            output.push_str("decision: accept_alternative\n");
            output.push_str(&format!("alternative_index: {alternative_index}\n"));
        }
        crate::review::CorrectionDecision::NeedsManualCorrection => {
            output.push_str("decision: needs_manual_correction\n");
        }
        crate::review::CorrectionDecision::ManualReplacement { replacement } => {
            output.push_str("decision: manual_replacement\n");
            output.push_str(&format!(
                "replacement_text: {}\n",
                escape_export_text(replacement.as_str())
            ));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn escape_export_text_handles_controls() {
        assert_eq!(escape_export_text("plain"), "plain");
        assert_eq!(escape_export_text("a\\b"), "a\\\\b");
        assert_eq!(escape_export_text("a\nb"), "a\\nb");
        assert_eq!(escape_export_text("a\rb"), "a\\rb");
        assert_eq!(escape_export_text("a\tb"), "a\\tb");
        assert_eq!(escape_export_text("a\u{0001}b"), "a\\u{1}b");
    }
}
