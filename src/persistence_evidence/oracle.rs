use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use super::model::{
    ArtifactClass, CanonicalEventProvenance, NormalizedAnchor, NormalizedSemanticState,
    RetentionRootClass, ReviewCaseOrigin, ReviewLedgerAction,
};

pub const ORACLE_VERSION: &str = "2";

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum OracleViolationCode {
    MissingCommittedEvent,
    UnexpectedCommittedEvent,
    ChangedEventOrder,
    ChangedDomainIdentity,
    ChangedTranscriptRevisionId,
    ChangedSourceRevisionBinding,
    ChangedLedgerActionPayload,
    LedgerSequenceChanged,
    LedgerProvenanceChanged,
    LedgerBindingChanged,
    ChangedAnalysisResultIdentity,
    MissingReferencedAnalysisResult,
    ChangedActiveAnalysisSelection,
    LostReferencedHistoricalArtifact,
    AutomaticDecisionMigration,
    FabricatedRecoveryHistory,
    DuplicateCanonicalIdentity,
    BrokenCanonicalReference,
    ConflatedReviewCaseOrigin,
    MissingReviewCaseRaisedEvent,
    UnexpectedReviewCaseRaisedEvent,
    MismatchedReviewCaseRaisedEvent,
    DetectorCaseHasHumanRaisedCreationEvent,
    BrokenRetentionRoot,
    BrokenRetentionTarget,
    InvalidRetentionRootClass,
    InvalidRetentionTargetClass,
    DuplicateRetentionReference,
    CanonicalFingerprintMismatch,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OracleDiagnostic {
    pub code: OracleViolationCode,
    pub path: String,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OracleResult {
    pub passed: bool,
    pub violations: Vec<OracleDiagnostic>,
    pub warnings: Vec<OracleDiagnostic>,
    pub expected_fingerprint: Option<String>,
    pub actual_fingerprint: String,
    pub oracle_version: String,
}

pub struct SemanticOracle;

impl SemanticOracle {
    pub fn validate(state: &NormalizedSemanticState) -> OracleResult {
        let state = state.clone().normalize();
        let mut violations = validate_state(&state);
        sort_diagnostics(&mut violations);
        OracleResult {
            passed: violations.is_empty(),
            violations,
            warnings: Vec::new(),
            expected_fingerprint: None,
            actual_fingerprint: semantic_fingerprint(&state),
            oracle_version: ORACLE_VERSION.to_string(),
        }
    }

    pub fn compare(
        expected: &NormalizedSemanticState,
        actual: &NormalizedSemanticState,
    ) -> OracleResult {
        let expected = expected.clone().normalize();
        let actual = actual.clone().normalize();
        let mut violations = validate_state(&actual);

        compare_session(&expected, &actual, &mut violations);
        compare_source_revisions(&expected, &actual, &mut violations);
        compare_review_cases(&expected, &actual, &mut violations);
        compare_review_case_raised_events(&expected, &actual, &mut violations);
        compare_ledger(&expected, &actual, &mut violations);
        compare_analysis(&expected, &actual, &mut violations);
        compare_historical_references(&expected, &actual, &mut violations);

        let expected_fingerprint = semantic_fingerprint(&expected);
        let actual_fingerprint = semantic_fingerprint(&actual);
        if expected_fingerprint != actual_fingerprint && violations.is_empty() {
            violations.push(diagnostic(
                OracleViolationCode::CanonicalFingerprintMismatch,
                "canonical_projection",
                "canonical semantic fingerprints differ without a more specific diagnostic",
            ));
        }
        sort_diagnostics(&mut violations);
        violations.dedup();

        OracleResult {
            passed: violations.is_empty(),
            violations,
            warnings: Vec::new(),
            expected_fingerprint: Some(expected_fingerprint),
            actual_fingerprint,
            oracle_version: ORACLE_VERSION.to_string(),
        }
    }
}

fn diagnostic(
    code: OracleViolationCode,
    path: impl Into<String>,
    message: impl Into<String>,
) -> OracleDiagnostic {
    OracleDiagnostic {
        code,
        path: path.into(),
        message: message.into(),
    }
}

fn validate_state(state: &NormalizedSemanticState) -> Vec<OracleDiagnostic> {
    let mut violations = Vec::new();

    check_unique(
        state
            .source_revisions
            .iter()
            .map(|item| item.revision_id.as_str()),
        "source_revisions",
        &mut violations,
    );
    check_unique(
        state.review_cases.iter().map(|item| item.case_id.as_str()),
        "review_cases",
        &mut violations,
    );
    check_unique(
        state
            .review_case_raised_events
            .iter()
            .map(|item| item.event_id.as_str()),
        "review_case_raised_events",
        &mut violations,
    );
    check_unique(
        state
            .review_ledger_events
            .iter()
            .map(|item| item.event_id.as_str()),
        "review_ledger_events",
        &mut violations,
    );
    check_unique(
        state
            .analysis_results
            .iter()
            .map(|item| item.analysis_result_id.as_str()),
        "analysis_results",
        &mut violations,
    );
    check_unique(
        state
            .knowledge_snapshot_references
            .iter()
            .map(|item| item.knowledge_snapshot_id.as_str()),
        "knowledge_snapshot_references",
        &mut violations,
    );
    check_unique(
        state.artifacts.iter().map(|item| item.artifact_id.as_str()),
        "artifacts",
        &mut violations,
    );

    let revisions: BTreeSet<&str> = state
        .source_revisions
        .iter()
        .map(|revision| revision.revision_id.as_str())
        .collect();
    let cases: BTreeSet<&str> = state
        .review_cases
        .iter()
        .map(|case| case.case_id.as_str())
        .collect();
    let raised_events: BTreeMap<&str, _> = state
        .review_case_raised_events
        .iter()
        .map(|event| (event.event_id.as_str(), event))
        .collect();
    let events: BTreeSet<&str> = state
        .review_ledger_events
        .iter()
        .map(|event| event.event_id.as_str())
        .collect();
    let analyses: BTreeSet<&str> = state
        .analysis_results
        .iter()
        .map(|analysis| analysis.analysis_result_id.as_str())
        .collect();
    let knowledge: BTreeSet<&str> = state
        .knowledge_snapshot_references
        .iter()
        .map(|reference| reference.knowledge_snapshot_id.as_str())
        .collect();
    for (index, revision) in state.source_revisions.iter().enumerate() {
        if let Some(predecessor) = &revision.predecessor_revision_id
            && !revisions.contains(predecessor.as_str())
        {
            violations.push(diagnostic(
                OracleViolationCode::BrokenCanonicalReference,
                format!("source_revisions[{index}].predecessor_revision_id"),
                format!("unknown predecessor revision {predecessor}"),
            ));
        }
    }

    for (index, review_case) in state.review_cases.iter().enumerate() {
        let path = format!("review_cases[{index}]");
        if !revisions.contains(review_case.observed_revision_id.as_str())
            || !revisions.contains(review_case.anchor.source_revision_id.as_str())
        {
            violations.push(diagnostic(
                OracleViolationCode::BrokenCanonicalReference,
                path.clone(),
                "review case references an unknown source revision",
            ));
        }
        if review_case.observed_revision_id != review_case.anchor.source_revision_id {
            violations.push(diagnostic(
                OracleViolationCode::ChangedSourceRevisionBinding,
                path.clone(),
                "case observation and anchor revisions differ",
            ));
        }
        match &review_case.origin {
            ReviewCaseOrigin::DetectorRaised { analysis_result_id } => {
                if !analyses.contains(analysis_result_id.as_str()) {
                    violations.push(diagnostic(
                        OracleViolationCode::BrokenCanonicalReference,
                        format!("{path}.origin"),
                        format!("unknown detector analysis result {analysis_result_id}"),
                    ));
                }
                if state
                    .review_case_raised_events
                    .iter()
                    .any(|event| event.case_id == review_case.case_id)
                {
                    violations.push(diagnostic(
                        OracleViolationCode::DetectorCaseHasHumanRaisedCreationEvent,
                        format!("{path}.origin"),
                        "detector-raised case carries HumanRaised creation provenance",
                    ));
                }
            }
            ReviewCaseOrigin::HumanRaised { creation_event_id } => {
                if let Some(creation_event) = raised_events.get(creation_event_id.as_str()) {
                    if creation_event.case_id != review_case.case_id
                        || creation_event.observed_revision_id != review_case.observed_revision_id
                        || creation_event.anchor != review_case.anchor
                        || creation_event.provenance != CanonicalEventProvenance::Human
                    {
                        violations.push(diagnostic(
                            OracleViolationCode::MismatchedReviewCaseRaisedEvent,
                            format!("{path}.origin.creation_event_id"),
                            "HumanRaised creation event does not match case identity, source, anchor, or human provenance",
                        ));
                    }
                } else {
                    violations.push(diagnostic(
                        OracleViolationCode::MissingReviewCaseRaisedEvent,
                        format!("{path}.origin.creation_event_id"),
                        format!("unknown HumanRaised creation event {creation_event_id}"),
                    ));
                }
            }
        }
        if review_case.copied_decision_from_case_id.is_some() {
            violations.push(diagnostic(
                OracleViolationCode::AutomaticDecisionMigration,
                format!("{path}.copied_decision_from_case_id"),
                "automatic decision copying is not authoritative",
            ));
        }
        validate_anchor(state, index, &mut violations);
    }

    let mut previous_raised_sequence = None;
    for (index, event) in state.review_case_raised_events.iter().enumerate() {
        let path = format!("review_case_raised_events[{index}]");
        if previous_raised_sequence.is_some_and(|previous| event.sequence <= previous) {
            violations.push(diagnostic(
                OracleViolationCode::ChangedEventOrder,
                format!("{path}.sequence"),
                "HumanRaised creation-event sequence is not strictly increasing",
            ));
        }
        previous_raised_sequence = Some(event.sequence);

        if !revisions.contains(event.observed_revision_id.as_str())
            || !revisions.contains(event.anchor.source_revision_id.as_str())
        {
            violations.push(diagnostic(
                OracleViolationCode::BrokenCanonicalReference,
                path.clone(),
                "HumanRaised creation event references an unknown source revision",
            ));
        }
        if event.observed_revision_id != event.anchor.source_revision_id {
            violations.push(diagnostic(
                OracleViolationCode::MismatchedReviewCaseRaisedEvent,
                path.clone(),
                "HumanRaised creation event source and anchor revisions differ",
            ));
        }

        match state
            .review_cases
            .iter()
            .find(|review_case| review_case.case_id == event.case_id)
        {
            None => violations.push(diagnostic(
                OracleViolationCode::BrokenCanonicalReference,
                format!("{path}.case_id"),
                format!("unknown review case {}", event.case_id),
            )),
            Some(review_case) => match &review_case.origin {
                ReviewCaseOrigin::HumanRaised { creation_event_id }
                    if creation_event_id == &event.event_id => {}
                ReviewCaseOrigin::HumanRaised { .. } => violations.push(diagnostic(
                    OracleViolationCode::MismatchedReviewCaseRaisedEvent,
                    path.clone(),
                    "HumanRaised case points to a different creation event",
                )),
                ReviewCaseOrigin::DetectorRaised { .. } => violations.push(diagnostic(
                    OracleViolationCode::DetectorCaseHasHumanRaisedCreationEvent,
                    path.clone(),
                    "HumanRaised creation event points to a detector-raised case",
                )),
            },
        }
        if event.provenance != CanonicalEventProvenance::Human {
            violations.push(diagnostic(
                OracleViolationCode::MismatchedReviewCaseRaisedEvent,
                format!("{path}.provenance"),
                "HumanRaised creation must preserve human creator provenance",
            ));
        }
        validate_anchor_value(state, &event.anchor, &path, &mut violations);
    }

    let mut previous_sequence = None;
    for (index, event) in state.review_ledger_events.iter().enumerate() {
        let path = format!("review_ledger_events[{index}]");
        if previous_sequence.is_some_and(|previous| event.sequence <= previous) {
            violations.push(diagnostic(
                OracleViolationCode::ChangedEventOrder,
                format!("{path}.sequence"),
                "canonical event sequence is not strictly increasing",
            ));
        }
        previous_sequence = Some(event.sequence);
        if !cases.contains(event.case_id.as_str())
            || !revisions.contains(event.observed_revision_id.as_str())
        {
            violations.push(diagnostic(
                OracleViolationCode::BrokenCanonicalReference,
                path.clone(),
                "ledger event references an unknown case or source revision",
            ));
        }
        match &event.action {
            ReviewLedgerAction::Withdraw { target_event_id }
            | ReviewLedgerAction::Supersede { target_event_id } => {
                if !events.contains(target_event_id.as_str()) {
                    violations.push(diagnostic(
                        OracleViolationCode::BrokenCanonicalReference,
                        format!("{path}.action"),
                        format!("unknown target event {target_event_id}"),
                    ));
                }
            }
            ReviewLedgerAction::AcceptAlternative { .. }
            | ReviewLedgerAction::ManualReplacement { .. } => {}
        }
        match event.provenance {
            CanonicalEventProvenance::Human | CanonicalEventProvenance::Recovery => {}
            CanonicalEventProvenance::AutomaticMigration => violations.push(diagnostic(
                OracleViolationCode::AutomaticDecisionMigration,
                format!("{path}.provenance"),
                "automatic migration must not create correction authority",
            )),
        }
    }

    for (index, analysis) in state.analysis_results.iter().enumerate() {
        let path = format!("analysis_results[{index}]");
        if !revisions.contains(analysis.source_revision_id.as_str()) {
            violations.push(diagnostic(
                OracleViolationCode::BrokenCanonicalReference,
                format!("{path}.source_revision_id"),
                "analysis result references an unknown source revision",
            ));
        }
        for snapshot_id in &analysis.knowledge_snapshot_ids {
            if !knowledge.contains(snapshot_id.as_str()) {
                violations.push(diagnostic(
                    OracleViolationCode::BrokenCanonicalReference,
                    format!("{path}.knowledge_snapshot_ids"),
                    format!("unknown knowledge snapshot {snapshot_id}"),
                ));
            }
        }
    }

    if let Some(selection) = &state.active_analysis_selection
        && !analyses.contains(selection.analysis_result_id.as_str())
    {
        violations.push(diagnostic(
            OracleViolationCode::BrokenCanonicalReference,
            "active_analysis_selection.analysis_result_id",
            "active selection references an unknown analysis result",
        ));
    }

    for (index, reference) in state.knowledge_snapshot_references.iter().enumerate() {
        for analysis_id in &reference.referenced_by_analysis_result_ids {
            if !analyses.contains(analysis_id.as_str()) {
                violations.push(diagnostic(
                    OracleViolationCode::BrokenCanonicalReference,
                    format!(
                        "knowledge_snapshot_references[{index}].referenced_by_analysis_result_ids"
                    ),
                    format!("unknown analysis result {analysis_id}"),
                ));
            }
        }
    }

    for (index, conflict) in state.lineage_conflicts.iter().enumerate() {
        for case_id in &conflict.related_case_ids {
            if !cases.contains(case_id.as_str()) {
                violations.push(diagnostic(
                    OracleViolationCode::BrokenCanonicalReference,
                    format!("lineage_conflicts[{index}].related_case_ids"),
                    format!("unknown review case {case_id}"),
                ));
            }
        }
    }

    let mut retention_identities = BTreeSet::new();
    for (index, reference) in state.retention_references.iter().enumerate() {
        let path = format!("retention_references[{index}]");
        if !retention_identities.insert((
            reference.root_class,
            reference.root_id.as_str(),
            reference.artifact_id.as_str(),
            reference.relation,
        )) {
            violations.push(diagnostic(
                OracleViolationCode::DuplicateRetentionReference,
                path.clone(),
                "duplicate retention reference",
            ));
        }

        if !retention_root_exists(state, reference.root_class, &reference.root_id) {
            violations.push(diagnostic(
                if retention_root_exists_in_any_class(state, &reference.root_id) {
                    OracleViolationCode::InvalidRetentionRootClass
                } else {
                    OracleViolationCode::BrokenRetentionRoot
                },
                format!("{path}.root_id"),
                format!(
                    "retention root {} does not resolve as {:?}",
                    reference.root_id, reference.root_class
                ),
            ));
        }

        match state
            .artifacts
            .iter()
            .find(|artifact| artifact.artifact_id == reference.artifact_id)
        {
            None => violations.push(diagnostic(
                OracleViolationCode::BrokenRetentionTarget,
                format!("{path}.artifact_id"),
                format!("unknown artifact {}", reference.artifact_id),
            )),
            Some(artifact)
                if !matches!(
                    artifact.class,
                    ArtifactClass::ReferencedHistorical | ArtifactClass::UnreferencedHistorical
                ) =>
            {
                violations.push(diagnostic(
                    OracleViolationCode::InvalidRetentionTargetClass,
                    format!("{path}.artifact_id"),
                    "retention target must be a historical artifact",
                ));
            }
            Some(_) => {}
        }
    }

    for artifact in &state.artifacts {
        if artifact.class == ArtifactClass::ReferencedHistorical
            && !state
                .retention_references
                .iter()
                .any(|reference| reference.artifact_id == artifact.artifact_id)
        {
            violations.push(diagnostic(
                OracleViolationCode::LostReferencedHistoricalArtifact,
                format!("artifacts[{}]", artifact.artifact_id),
                "referenced historical artifact has no canonical retention root",
            ));
        }
    }

    violations
}

fn validate_anchor(
    state: &NormalizedSemanticState,
    case_index: usize,
    violations: &mut Vec<OracleDiagnostic>,
) {
    let review_case = &state.review_cases[case_index];
    validate_anchor_value(
        state,
        &review_case.anchor,
        &format!("review_cases[{case_index}]"),
        violations,
    );
}

fn validate_anchor_value(
    state: &NormalizedSemanticState,
    anchor: &NormalizedAnchor,
    path: &str,
    violations: &mut Vec<OracleDiagnostic>,
) {
    let Some(revision) = state
        .source_revisions
        .iter()
        .find(|revision| revision.revision_id == anchor.source_revision_id)
    else {
        return;
    };
    let Some(segment) = revision.segments.get(anchor.segment_position) else {
        violations.push(diagnostic(
            OracleViolationCode::BrokenCanonicalReference,
            format!("{path}.anchor.segment_position"),
            "anchor segment does not exist",
        ));
        return;
    };
    let range = anchor.start_byte..anchor.end_byte;
    if range.start >= range.end
        || range.end > segment.text.len()
        || !segment.text.is_char_boundary(range.start)
        || !segment.text.is_char_boundary(range.end)
    {
        violations.push(diagnostic(
            OracleViolationCode::BrokenCanonicalReference,
            format!("{path}.anchor"),
            "anchor range is invalid for referenced source text",
        ));
    }
}

fn check_unique<'a>(
    identities: impl Iterator<Item = &'a str>,
    path: &str,
    violations: &mut Vec<OracleDiagnostic>,
) {
    let mut seen = BTreeSet::new();
    for identity in identities {
        if !seen.insert(identity) {
            violations.push(diagnostic(
                OracleViolationCode::DuplicateCanonicalIdentity,
                path,
                format!("duplicate canonical identity {identity}"),
            ));
        }
    }
}

fn retention_root_exists(
    state: &NormalizedSemanticState,
    class: RetentionRootClass,
    root_id: &str,
) -> bool {
    match class {
        RetentionRootClass::ReviewLedgerEvent => state
            .review_ledger_events
            .iter()
            .any(|event| event.event_id == root_id),
        RetentionRootClass::ReviewCaseRaisedEvent => state
            .review_case_raised_events
            .iter()
            .any(|event| event.event_id == root_id),
        RetentionRootClass::AnalysisResult => state
            .analysis_results
            .iter()
            .any(|analysis| analysis.analysis_result_id == root_id),
        RetentionRootClass::SourceRevision => state
            .source_revisions
            .iter()
            .any(|revision| revision.revision_id == root_id),
    }
}

fn retention_root_exists_in_any_class(state: &NormalizedSemanticState, root_id: &str) -> bool {
    [
        RetentionRootClass::ReviewLedgerEvent,
        RetentionRootClass::ReviewCaseRaisedEvent,
        RetentionRootClass::AnalysisResult,
        RetentionRootClass::SourceRevision,
    ]
    .into_iter()
    .any(|class| retention_root_exists(state, class, root_id))
}

fn compare_session(
    expected: &NormalizedSemanticState,
    actual: &NormalizedSemanticState,
    violations: &mut Vec<OracleDiagnostic>,
) {
    if expected.session != actual.session {
        violations.push(diagnostic(
            OracleViolationCode::ChangedDomainIdentity,
            "session",
            "session identity or duplication lineage changed",
        ));
    }
    if expected.session_format_version != actual.session_format_version
        || expected.interpretation_version != actual.interpretation_version
    {
        violations.push(diagnostic(
            OracleViolationCode::ChangedDomainIdentity,
            "format_interpretation",
            "format or interpretation metadata changed",
        ));
    }
}

fn compare_source_revisions(
    expected: &NormalizedSemanticState,
    actual: &NormalizedSemanticState,
    violations: &mut Vec<OracleDiagnostic>,
) {
    let expected_map: BTreeMap<_, _> = expected
        .source_revisions
        .iter()
        .map(|item| (item.revision_id.as_str(), item))
        .collect();
    let actual_map: BTreeMap<_, _> = actual
        .source_revisions
        .iter()
        .map(|item| (item.revision_id.as_str(), item))
        .collect();
    if expected_map.keys().collect::<Vec<_>>() != actual_map.keys().collect::<Vec<_>>() {
        violations.push(diagnostic(
            OracleViolationCode::ChangedTranscriptRevisionId,
            "source_revisions",
            "source revision identity set changed",
        ));
    }
    for (identity, expected_revision) in expected_map {
        if let Some(actual_revision) = actual_map.get(identity)
            && expected_revision != *actual_revision
        {
            violations.push(diagnostic(
                OracleViolationCode::ChangedSourceRevisionBinding,
                format!("source_revisions[{identity}]"),
                "source revision content or lineage changed",
            ));
        }
    }
}

fn compare_review_cases(
    expected: &NormalizedSemanticState,
    actual: &NormalizedSemanticState,
    violations: &mut Vec<OracleDiagnostic>,
) {
    let expected_map: BTreeMap<_, _> = expected
        .review_cases
        .iter()
        .map(|item| (item.case_id.as_str(), item))
        .collect();
    let actual_map: BTreeMap<_, _> = actual
        .review_cases
        .iter()
        .map(|item| (item.case_id.as_str(), item))
        .collect();
    if expected_map.keys().collect::<Vec<_>>() != actual_map.keys().collect::<Vec<_>>() {
        violations.push(diagnostic(
            OracleViolationCode::ChangedDomainIdentity,
            "review_cases",
            "review case identity set changed",
        ));
    }
    for (identity, expected_case) in expected_map {
        let Some(actual_case) = actual_map.get(identity) else {
            continue;
        };
        if std::mem::discriminant(&expected_case.origin)
            != std::mem::discriminant(&actual_case.origin)
        {
            violations.push(diagnostic(
                OracleViolationCode::ConflatedReviewCaseOrigin,
                format!("review_cases[{identity}].origin"),
                "DetectorRaised and HumanRaised origins were conflated",
            ));
        } else if expected_case.origin != actual_case.origin {
            violations.push(diagnostic(
                OracleViolationCode::ChangedDomainIdentity,
                format!("review_cases[{identity}].origin"),
                "review case origin provenance changed",
            ));
        }
        if expected_case.observed_revision_id != actual_case.observed_revision_id
            || expected_case.anchor.source_revision_id != actual_case.anchor.source_revision_id
        {
            violations.push(diagnostic(
                OracleViolationCode::ChangedSourceRevisionBinding,
                format!("review_cases[{identity}]"),
                "review case source revision binding changed",
            ));
        }
        if expected_case.anchor != actual_case.anchor {
            violations.push(diagnostic(
                OracleViolationCode::ChangedDomainIdentity,
                format!("review_cases[{identity}].anchor"),
                "review case anchor changed",
            ));
        }
    }
}

fn compare_review_case_raised_events(
    expected: &NormalizedSemanticState,
    actual: &NormalizedSemanticState,
    violations: &mut Vec<OracleDiagnostic>,
) {
    let expected_map: BTreeMap<_, _> = expected
        .review_case_raised_events
        .iter()
        .map(|event| (event.event_id.as_str(), event))
        .collect();
    let actual_map: BTreeMap<_, _> = actual
        .review_case_raised_events
        .iter()
        .map(|event| (event.event_id.as_str(), event))
        .collect();

    for identity in expected_map.keys() {
        if !actual_map.contains_key(identity) {
            violations.push(diagnostic(
                OracleViolationCode::MissingReviewCaseRaisedEvent,
                format!("review_case_raised_events[{identity}]"),
                "HumanRaised creation event is missing",
            ));
        }
    }
    for identity in actual_map.keys() {
        if !expected_map.contains_key(identity) {
            violations.push(diagnostic(
                OracleViolationCode::UnexpectedReviewCaseRaisedEvent,
                format!("review_case_raised_events[{identity}]"),
                "unexpected HumanRaised creation event is present",
            ));
        }
    }

    let expected_order: Vec<_> = expected
        .review_case_raised_events
        .iter()
        .map(|event| event.event_id.as_str())
        .collect();
    let actual_order: Vec<_> = actual
        .review_case_raised_events
        .iter()
        .map(|event| event.event_id.as_str())
        .collect();
    if expected_map.len() == actual_map.len()
        && expected_map.keys().eq(actual_map.keys())
        && expected_order != actual_order
    {
        violations.push(diagnostic(
            OracleViolationCode::ChangedEventOrder,
            "review_case_raised_events",
            "HumanRaised creation-event order changed",
        ));
    }

    for (identity, expected_event) in expected_map {
        if let Some(actual_event) = actual_map.get(identity)
            && expected_event != *actual_event
        {
            violations.push(diagnostic(
                OracleViolationCode::MismatchedReviewCaseRaisedEvent,
                format!("review_case_raised_events[{identity}]"),
                "HumanRaised creation-event canonical fields changed",
            ));
        }
    }
}

fn compare_ledger(
    expected: &NormalizedSemanticState,
    actual: &NormalizedSemanticState,
    violations: &mut Vec<OracleDiagnostic>,
) {
    let expected_map: BTreeMap<_, _> = expected
        .review_ledger_events
        .iter()
        .map(|event| (event.event_id.as_str(), event))
        .collect();
    let actual_map: BTreeMap<_, _> = actual
        .review_ledger_events
        .iter()
        .map(|event| (event.event_id.as_str(), event))
        .collect();

    for identity in expected_map.keys() {
        if !actual_map.contains_key(identity) {
            violations.push(diagnostic(
                OracleViolationCode::MissingCommittedEvent,
                format!("review_ledger_events[{identity}]"),
                "committed event is missing",
            ));
        }
    }
    for identity in actual_map.keys() {
        if !expected_map.contains_key(identity) {
            violations.push(diagnostic(
                OracleViolationCode::UnexpectedCommittedEvent,
                format!("review_ledger_events[{identity}]"),
                "unexpected committed event is exposed",
            ));
            if actual_map
                .get(identity)
                .is_some_and(|event| event.provenance == CanonicalEventProvenance::Recovery)
            {
                violations.push(diagnostic(
                    OracleViolationCode::FabricatedRecoveryHistory,
                    format!("review_ledger_events[{identity}].provenance"),
                    "recovery introduced canonical history absent from expected truth",
                ));
            }
        }
    }

    let expected_order: Vec<_> = expected
        .review_ledger_events
        .iter()
        .map(|event| event.event_id.as_str())
        .collect();
    let actual_order: Vec<_> = actual
        .review_ledger_events
        .iter()
        .map(|event| event.event_id.as_str())
        .collect();
    if expected_map.len() == actual_map.len()
        && expected_map.keys().eq(actual_map.keys())
        && expected_order != actual_order
    {
        violations.push(diagnostic(
            OracleViolationCode::ChangedEventOrder,
            "review_ledger_events",
            "canonical event order changed",
        ));
    }

    for (identity, expected_event) in expected_map {
        let Some(actual_event) = actual_map.get(identity) else {
            continue;
        };
        if expected_event.sequence != actual_event.sequence {
            violations.push(diagnostic(
                OracleViolationCode::LedgerSequenceChanged,
                format!("review_ledger_events[{identity}].sequence"),
                "ledger sequence value changed",
            ));
        }
        if expected_event.action != actual_event.action {
            violations.push(diagnostic(
                OracleViolationCode::ChangedLedgerActionPayload,
                format!("review_ledger_events[{identity}].action"),
                "ledger action or authoritative payload changed",
            ));
        }
        if expected_event.case_id != actual_event.case_id
            || expected_event.observed_revision_id != actual_event.observed_revision_id
        {
            violations.push(diagnostic(
                OracleViolationCode::LedgerBindingChanged,
                format!("review_ledger_events[{identity}]"),
                "ledger case or source revision binding changed",
            ));
        }
        if expected_event.provenance != actual_event.provenance {
            violations.push(diagnostic(
                OracleViolationCode::LedgerProvenanceChanged,
                format!("review_ledger_events[{identity}].provenance"),
                "ledger authority provenance changed",
            ));
        }
    }
}

fn compare_analysis(
    expected: &NormalizedSemanticState,
    actual: &NormalizedSemanticState,
    violations: &mut Vec<OracleDiagnostic>,
) {
    let expected_map: BTreeMap<_, _> = expected
        .analysis_results
        .iter()
        .map(|analysis| (analysis.analysis_result_id.as_str(), analysis))
        .collect();
    let actual_map: BTreeMap<_, _> = actual
        .analysis_results
        .iter()
        .map(|analysis| (analysis.analysis_result_id.as_str(), analysis))
        .collect();
    for identity in expected_map.keys() {
        if !actual_map.contains_key(identity) {
            let referenced = expected
                .active_analysis_selection
                .as_ref()
                .is_some_and(|selection| selection.analysis_result_id == **identity)
                || expected
                    .knowledge_snapshot_references
                    .iter()
                    .any(|reference| {
                        reference
                            .referenced_by_analysis_result_ids
                            .iter()
                            .any(|analysis_id| analysis_id == *identity)
                    });
            violations.push(diagnostic(
                if referenced {
                    OracleViolationCode::MissingReferencedAnalysisResult
                } else {
                    OracleViolationCode::ChangedAnalysisResultIdentity
                },
                format!("analysis_results[{identity}]"),
                "analysis result identity is missing",
            ));
        }
    }
    for identity in actual_map.keys() {
        if !expected_map.contains_key(identity) {
            violations.push(diagnostic(
                OracleViolationCode::ChangedAnalysisResultIdentity,
                format!("analysis_results[{identity}]"),
                "unexpected analysis result identity is present",
            ));
        }
    }
    for (identity, expected_analysis) in expected_map {
        if let Some(actual_analysis) = actual_map.get(identity)
            && expected_analysis != *actual_analysis
        {
            violations.push(diagnostic(
                OracleViolationCode::ChangedAnalysisResultIdentity,
                format!("analysis_results[{identity}]"),
                "analysis result source or knowledge provenance changed",
            ));
        }
    }
    if expected.active_analysis_selection != actual.active_analysis_selection {
        violations.push(diagnostic(
            OracleViolationCode::ChangedActiveAnalysisSelection,
            "active_analysis_selection",
            "active analysis selection changed",
        ));
    }
}

fn compare_historical_references(
    expected: &NormalizedSemanticState,
    actual: &NormalizedSemanticState,
    violations: &mut Vec<OracleDiagnostic>,
) {
    if expected.knowledge_snapshot_references != actual.knowledge_snapshot_references
        || expected.retention_references != actual.retention_references
    {
        violations.push(diagnostic(
            OracleViolationCode::LostReferencedHistoricalArtifact,
            "historical_references",
            "knowledge or retention reachability changed",
        ));
    }
    if expected.lineage_conflicts != actual.lineage_conflicts {
        violations.push(diagnostic(
            OracleViolationCode::ChangedDomainIdentity,
            "lineage_conflicts",
            "lineage or conflict provenance changed",
        ));
    }

    let expected_historical: BTreeMap<_, _> = expected
        .artifacts
        .iter()
        .filter(|artifact| {
            matches!(
                artifact.class,
                ArtifactClass::ReferencedHistorical | ArtifactClass::UnreferencedHistorical
            )
        })
        .map(|artifact| (artifact.artifact_id.as_str(), artifact))
        .collect();
    let actual_historical: BTreeMap<_, _> = actual
        .artifacts
        .iter()
        .filter(|artifact| {
            matches!(
                artifact.class,
                ArtifactClass::ReferencedHistorical | ArtifactClass::UnreferencedHistorical
            )
        })
        .map(|artifact| (artifact.artifact_id.as_str(), artifact))
        .collect();
    if expected_historical != actual_historical {
        violations.push(diagnostic(
            OracleViolationCode::LostReferencedHistoricalArtifact,
            "artifacts",
            "historical artifact identity or content changed",
        ));
    }
}

fn semantic_fingerprint(state: &NormalizedSemanticState) -> String {
    let encoded = serde_json::to_vec(&state.canonical_projection())
        .expect("normalized semantic state serialization must succeed");
    let digest: [u8; 32] = Sha256::digest(encoded).into();
    let mut output = String::with_capacity("semantic:sha256-v1:".len() + 64);
    output.push_str("semantic:sha256-v1:");
    for byte in digest {
        use std::fmt::Write;
        write!(&mut output, "{byte:02x}").expect("writing to String cannot fail");
    }
    output
}

fn sort_diagnostics(diagnostics: &mut [OracleDiagnostic]) {
    diagnostics.sort_by(|left, right| {
        (&left.code, &left.path, &left.message).cmp(&(&right.code, &right.path, &right.message))
    });
}
