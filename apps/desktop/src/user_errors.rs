use vox_proof::application_service::ApplicationServiceError;
use vox_proof::project_memory::ProjectMemoryError;
use vox_proof::review::ManualReplacementTextError;
use vox_proof::session_terms::SessionTermsError;

use crate::controller::ControllerError;

pub fn user_message(error: &ControllerError) -> String {
    match error {
        ControllerError::Io(error) => format!("Could not read the selected file: {error}"),
        ControllerError::Transcript(_) => {
            "This file does not look like a valid SRT transcript.".to_owned()
        }
        ControllerError::SessionTerms(error) => session_terms_message(error),
        ControllerError::Authority(_) => {
            "Enter a reviewer name without control characters or empty whitespace.".to_owned()
        }
        ControllerError::Service(error) => service_message(error),
        ControllerError::Export(error) => format!("Export failed: {error}"),
        ControllerError::Reuse(_) => {
            "That project-memory action could not be completed.".to_owned()
        }
        ControllerError::Persistence(error) => format!("Could not access the saved review: {error}"),
        ControllerError::NoActiveSession => "No review session is open.".to_owned(),
        ControllerError::StaleUiSessionEpoch { .. } => {
            "This screen is out of date. Close and reopen the review.".to_owned()
        }
        ControllerError::NoSelectedCase => "Select a review item first.".to_owned(),
        ControllerError::AlternativeOutOfRange { index, count } => {
            if *count == 0 {
                "No suggested correction is available for this item.".to_owned()
            } else {
                format!(
                    "Suggestion {} is not available. This item has {count} suggestion(s).",
                    index + 1
                )
            }
        }
        ControllerError::ReviewIncomplete { undecided } => {
            format!("Review {undecided} remaining item(s) before exporting.")
        }
        ControllerError::UnresolvedConfirmationRequired => {
            "Confirm that unresolved items keep their original subtitle text before exporting."
                .to_owned()
        }
        ControllerError::RecoveryRequired => {
            "This review needs recovery before you can continue.".to_owned()
        }
        ControllerError::SessionNotWritable => {
            "This review is open read-only.".to_owned()
        }
        ControllerError::WriterOwnershipHeld => {
            "This review is currently open for editing elsewhere.".to_owned()
        }
        ControllerError::WritableReuseBlocked | ControllerError::ProjectMemoryUnavailable => {
            "Project Memory isn't available for this review. You can still inspect saved decisions, but new reuse suggestions can't be applied until the project files are available again.".to_owned()
        }
        ControllerError::ProjectMemory(error) => project_memory_message(error),
        ControllerError::NoPromotionCandidate => {
            "This correction isn't ready to reuse in related reviews yet.".to_owned()
        }
    }
}

fn session_terms_message(error: &SessionTermsError) -> String {
    match error {
        SessionTermsError::EmptyCanonicalTerm { line } => {
            format!("Term on line {line} needs a name.")
        }
        SessionTermsError::MissingSourceForm { line } => {
            format!("Term on line {line} needs at least one alias or known misrecognition.")
        }
        SessionTermsError::UnknownPrefix { line, prefix, .. } => {
            format!("Term on line {line} uses unsupported prefix '{prefix}'.")
        }
        SessionTermsError::UnprefixedSourceForm { line, .. } => {
            format!("Term on line {line} must label aliases and misrecognitions clearly.")
        }
        SessionTermsError::EmptyAlias { line, .. } => {
            format!("Term on line {line} has an empty alias.")
        }
        SessionTermsError::EmptyObservedErrorForm { line, .. } => {
            format!("Term on line {line} has an empty misrecognition.")
        }
        SessionTermsError::DuplicateAlias { alias, .. } => {
            format!("The alias '{alias}' appears more than once.")
        }
        SessionTermsError::DuplicateObservedErrorForm {
            observed_error_form,
            ..
        } => format!("The misrecognition '{observed_error_form}' appears more than once."),
        SessionTermsError::ConflictingSourceFormKinds { source_form, .. } => {
            format!("'{source_form}' is listed as both an alias and a misrecognition.")
        }
        SessionTermsError::DuplicateCanonicalTerm { canonical_term, .. } => {
            format!("The term '{canonical_term}' appears more than once.")
        }
    }
}

fn service_message(error: &ApplicationServiceError) -> String {
    match error {
        ApplicationServiceError::HumanRaisedRequiresFormatV3 => {
            "This saved review cannot record a new unflagged correction. Start a new review.".to_owned()
        }
        ApplicationServiceError::HumanRaisedOverlap => {
            "That span overlaps another correction or an item still waiting for a decision.".to_owned()
        }
        ApplicationServiceError::HumanRaisedAnchorInvalid(_)
        | ApplicationServiceError::HumanRaisedRevisionStale => {
            "Select a contiguous span inside one subtitle line.".to_owned()
        }
        ApplicationServiceError::ManualReplacement(inner) => manual_replacement_message(inner),
        ApplicationServiceError::DecisionCoverageIncomplete { undecided } => {
            format!("Review {undecided} remaining item(s) before exporting.")
        }
        ApplicationServiceError::StaleReuseAnalysis => {
            "This suggestion is out of date because Project Memory changed. Close and reopen the review to see current suggestions.".to_owned()
        }
        ApplicationServiceError::ProjectEvidenceUnverifiable | ApplicationServiceError::UnknownReuseProposal => {
            "This previous-correction suggestion can't be applied right now.".to_owned()
        }
        other => format!("Review action failed: {other:?}"),
    }
}

fn project_memory_message(error: &ProjectMemoryError) -> String {
    match error {
        ProjectMemoryError::InvalidDisplayName => "Enter a project name.".to_owned(),
        ProjectMemoryError::ProjectNotFound => "That project could not be found.".to_owned(),
        ProjectMemoryError::WriterOwnershipHeld => {
            "This project is currently open elsewhere.".to_owned()
        }
        _ => "The project could not be opened.".to_owned(),
    }
}

fn manual_replacement_message(error: &ManualReplacementTextError) -> String {
    match error {
        ManualReplacementTextError::Empty => "Enter the corrected text.".to_owned(),
        ManualReplacementTextError::WhitespaceOnly => "Enter the corrected text.".to_owned(),
        ManualReplacementTextError::UnicodeControl { .. }
        | ManualReplacementTextError::UnicodeLineSeparator { .. } => {
            "Correction must not include hidden control characters.".to_owned()
        }
        ManualReplacementTextError::IdenticalToSelectedSource => {
            "Correction must be different from the current subtitle text.".to_owned()
        }
        ManualReplacementTextError::TooLong {
            maximum_utf8_bytes, ..
        } => format!("Correction is too long. Keep it under {maximum_utf8_bytes} characters."),
    }
}
