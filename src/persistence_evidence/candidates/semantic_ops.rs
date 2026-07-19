use super::super::adapter::{AdapterError, AuthoritativeCommand, SemanticPrecondition};
use super::super::model::{
    ActiveAnalysisSelection, AnalysisResultState, NormalizedSemanticState, ReviewLedgerEventState,
};

pub fn validate_preconditions(
    state: &NormalizedSemanticState,
    preconditions: &[SemanticPrecondition],
) -> Result<(), AdapterError> {
    for precondition in preconditions {
        let satisfied = match precondition {
            SemanticPrecondition::SourceRevisionExists {
                expected_revision_id,
            } => state
                .source_revisions
                .iter()
                .any(|revision| revision.revision_id == *expected_revision_id),
            SemanticPrecondition::ReviewLedgerHead { expected_event_id } => {
                state
                    .review_ledger_events
                    .last()
                    .map(|event| &event.event_id)
                    == expected_event_id.as_ref()
            }
            SemanticPrecondition::ActiveAnalysisSelection {
                expected_analysis_result_id,
            } => {
                state
                    .active_analysis_selection
                    .as_ref()
                    .map(|selection| &selection.analysis_result_id)
                    == expected_analysis_result_id.as_ref()
            }
            SemanticPrecondition::AnalysisAttachmentSet {
                expected_analysis_result_ids,
            } => {
                let mut actual: Vec<_> = state
                    .analysis_results
                    .iter()
                    .map(|analysis| analysis.analysis_result_id.clone())
                    .collect();
                let mut expected = expected_analysis_result_ids.clone();
                actual.sort();
                expected.sort();
                actual == expected
            }
        };
        if !satisfied {
            return Err(AdapterError::new(
                "stale-precondition",
                "semantic command precondition does not match current state",
            ));
        }
    }
    Ok(())
}

pub fn apply_command(
    state: &mut NormalizedSemanticState,
    command: &AuthoritativeCommand,
) -> Result<(), AdapterError> {
    let preconditions = match command {
        AuthoritativeCommand::AppendCorrectionEvent { preconditions, .. }
        | AuthoritativeCommand::AttachAnalysisResult { preconditions, .. }
        | AuthoritativeCommand::SelectActiveAnalysis { preconditions, .. }
        | AuthoritativeCommand::ExecuteCleanupPlan { preconditions, .. } => preconditions,
    };
    validate_preconditions(state, preconditions)?;

    match command {
        AuthoritativeCommand::AppendCorrectionEvent { event, .. } => {
            state.review_ledger_events.push(event.clone());
        }
        AuthoritativeCommand::AttachAnalysisResult {
            analysis_result, ..
        } => {
            state.analysis_results.push(analysis_result.clone());
        }
        AuthoritativeCommand::SelectActiveAnalysis { selection, .. } => {
            if !state
                .analysis_results
                .iter()
                .any(|analysis| analysis.analysis_result_id == selection.analysis_result_id)
            {
                return Err(AdapterError::new(
                    "unknown-analysis",
                    "active selection references an unknown analysis",
                ));
            }
            state.active_analysis_selection = Some(selection.clone());
        }
        AuthoritativeCommand::ExecuteCleanupPlan { plan_id, .. } => {
            state
                .artifacts
                .retain(|artifact| !artifact.artifact_id.contains(plan_id));
        }
    }
    Ok(())
}

pub fn sample_append_event(state: &NormalizedSemanticState) -> ReviewLedgerEventState {
    let next_sequence = state
        .review_ledger_events
        .iter()
        .map(|event| event.sequence)
        .max()
        .unwrap_or(0)
        + 1;
    let case_id = state
        .review_cases
        .first()
        .expect("fixture review case")
        .case_id
        .clone();
    let revision_id = state
        .review_cases
        .first()
        .expect("fixture review case")
        .observed_revision_id
        .clone();
    ReviewLedgerEventState {
        event_id: format!("ledger-event:{next_sequence:03}"),
        sequence: next_sequence,
        case_id,
        observed_revision_id: revision_id,
        action: super::super::model::ReviewLedgerAction::AcceptAlternative {
            alternative_index: 0,
        },
        provenance: super::super::model::CanonicalEventProvenance::Human,
    }
}

pub fn sample_attach_analysis(state: &NormalizedSemanticState) -> AnalysisResultState {
    AnalysisResultState {
        analysis_result_id: "analysis-result:spike:001".to_string(),
        source_revision_id: state
            .source_revisions
            .first()
            .expect("fixture source revision")
            .revision_id
            .clone(),
        knowledge_snapshot_ids: Vec::new(),
    }
}

pub fn sample_active_analysis_selection(analysis_result_id: &str) -> ActiveAnalysisSelection {
    ActiveAnalysisSelection {
        analysis_result_id: analysis_result_id.to_string(),
        selection_event_id: "active-analysis-selection:spike:001".to_string(),
    }
}
