use vox_proof::application_service::{
    ApplicationDecisionCoverage, ApplicationResolutionStatus, ApplicationReviewProgress,
};
use vox_proof::review::CorrectionDecision;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BottomTab {
    CurrentPreview,
    DecisionLog,
    SessionSummary,
}

pub fn coverage_label(progress: ApplicationReviewProgress, total: usize) -> String {
    match progress.decision_coverage {
        ApplicationDecisionCoverage::Complete => {
            if total == 0 {
                "Review complete".to_owned()
            } else {
                format!("Review complete ({total} items)")
            }
        }
        ApplicationDecisionCoverage::Incomplete { undecided } => {
            let reviewed = total.saturating_sub(undecided);
            format!("Progress: {reviewed} of {total} reviewed")
        }
    }
}

pub fn resolution_label(progress: ApplicationReviewProgress) -> String {
    match progress.resolution_status {
        ApplicationResolutionStatus::Resolved => "All items resolved".to_owned(),
        ApplicationResolutionStatus::Unresolved {
            deferred,
            needs_manual_correction,
        } => format!(
            "{deferred} deferred, {needs_manual_correction} still need manual correction"
        ),
    }
}

pub fn review_is_complete(progress: ApplicationReviewProgress) -> bool {
    matches!(
        progress.decision_coverage,
        ApplicationDecisionCoverage::Complete
    )
}

pub fn export_enabled(progress: ApplicationReviewProgress) -> bool {
    matches!(
        progress.decision_coverage,
        ApplicationDecisionCoverage::Complete
    )
}

pub fn unresolved_confirmation_needed(progress: ApplicationReviewProgress) -> bool {
    export_enabled(progress)
        && matches!(
            progress.resolution_status,
            ApplicationResolutionStatus::Unresolved { .. }
        )
}

pub fn accept_enabled(alternative_count: usize, selected_alternative: usize) -> bool {
    alternative_count > selected_alternative
}

pub fn review_shortcuts_suppressed(
    text_input_active: bool,
    ime_composing: bool,
    modal_open: bool,
) -> bool {
    text_input_active || ime_composing || modal_open
}

pub fn decision_shortcut(
    key: egui::Key,
    pressed: bool,
    repeat: bool,
    selected_alternative: usize,
) -> Option<CorrectionDecision> {
    if !pressed || repeat {
        return None;
    }
    match key {
        egui::Key::A => Some(CorrectionDecision::AcceptAlternative {
            alternative_index: selected_alternative,
        }),
        egui::Key::R => Some(CorrectionDecision::Reject),
        egui::Key::D => Some(CorrectionDecision::Defer),
        egui::Key::M => Some(CorrectionDecision::NeedsManualCorrection),
        _ => None,
    }
}

pub fn search_matches(source_text: &str, evidence: &str, query: &str) -> bool {
    let query = query.trim().to_lowercase();
    query.is_empty()
        || source_text.to_lowercase().contains(&query)
        || evidence.to_lowercase().contains(&query)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn coverage_and_resolution_are_presented_as_separate_axes() {
        let progress = ApplicationReviewProgress {
            decision_coverage: ApplicationDecisionCoverage::Complete,
            resolution_status: ApplicationResolutionStatus::Unresolved {
                deferred: 1,
                needs_manual_correction: 2,
            },
        };
        assert_eq!(coverage_label(progress, 3), "Review complete (3 items)");
        assert!(resolution_label(progress).contains("deferred"));
        assert!(export_enabled(progress));
        assert!(unresolved_confirmation_needed(progress));
    }

    #[test]
    fn incomplete_resolved_is_not_presented_as_complete_or_exportable() {
        let progress = ApplicationReviewProgress {
            decision_coverage: ApplicationDecisionCoverage::Incomplete { undecided: 2 },
            resolution_status: ApplicationResolutionStatus::Resolved,
        };
        assert!(coverage_label(progress, 3).starts_with("Progress:"));
        assert_eq!(resolution_label(progress), "All items resolved");
        assert!(!export_enabled(progress));
    }

    #[test]
    fn zero_case_session_is_honestly_complete_and_resolved() {
        let progress = ApplicationReviewProgress {
            decision_coverage: ApplicationDecisionCoverage::Complete,
            resolution_status: ApplicationResolutionStatus::Resolved,
        };
        assert_eq!(coverage_label(progress, 0), "Review complete");
        assert_eq!(resolution_label(progress), "All items resolved");
        assert!(export_enabled(progress));
        assert!(!unresolved_confirmation_needed(progress));
    }

    #[test]
    fn accept_requires_an_existing_selected_alternative() {
        assert!(!accept_enabled(0, 0));
        assert!(accept_enabled(2, 1));
        assert!(!accept_enabled(2, 2));
    }

    #[test]
    fn decision_shortcuts_are_suppressed_at_every_text_authoring_boundary() {
        assert!(review_shortcuts_suppressed(true, false, false));
        assert!(review_shortcuts_suppressed(false, true, false));
        assert!(review_shortcuts_suppressed(false, false, true));
        assert!(!review_shortcuts_suppressed(false, false, false));
    }

    #[test]
    fn decision_shortcuts_ignore_release_and_key_repeat_events() {
        assert_eq!(
            decision_shortcut(egui::Key::D, true, false, 0),
            Some(CorrectionDecision::Defer)
        );
        assert_eq!(decision_shortcut(egui::Key::D, true, true, 0), None);
        assert_eq!(decision_shortcut(egui::Key::D, false, false, 0), None);
    }

    #[test]
    fn search_filter_is_pure_and_matches_source_or_evidence() {
        let source = "華說 product";
        let evidence = "Glossary alias";
        assert!(search_matches(source, evidence, "華說"));
        assert!(search_matches(source, evidence, "ALIAS"));
        assert!(!search_matches(source, evidence, "unrelated"));
        assert_eq!(source, "華說 product");
        assert_eq!(evidence, "Glossary alias");
    }
}
