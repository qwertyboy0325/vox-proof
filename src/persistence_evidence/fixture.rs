use serde::{Deserialize, Serialize};

use crate::analysis::AnalysisRun;
use crate::candidate::SessionTermEntry;
use crate::pipeline::run_term_review;
use crate::srt::parse_srt;
use crate::transcript::Transcript;

use super::model::{
    ActiveAnalysisSelection, AnalysisResultState, ArtifactClass, ArtifactState,
    CanonicalEventProvenance, KnowledgeSnapshotReference, LineageConflict, LineageConflictKind,
    NormalizedAnchor, NormalizedReviewCase, NormalizedSemanticState, NormalizedSourceRevision,
    RetentionReference, RetentionRelation, RetentionRootClass, ReviewCaseOrigin,
    ReviewCaseRaisedEventState, ReviewLedgerAction, ReviewLedgerEventState, SessionIdentityState,
    SourceSegmentState,
};

pub const SMALL_FIXTURE_ID: &str = "voxproof-persistence-evidence-small";
pub const SMALL_FIXTURE_VERSION: &str = "2";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FixtureScale {
    Small,
    Medium,
    Stress,
}

impl FixtureScale {
    pub fn is_implemented(self) -> bool {
        matches!(self, Self::Small)
    }
}

/// Deterministic, candidate-neutral semantic workload.
///
/// Proposed v0.2 concepts that do not yet exist in the accepted domain model
/// are represented only inside the nested spike-only normalized state.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvidenceFixture {
    pub fixture_id: String,
    pub fixture_version: String,
    pub scale: FixtureScale,
    pub expected_state: NormalizedSemanticState,
}

impl EvidenceFixture {
    pub fn small() -> Self {
        let initial = parse_srt(
            "1\n00:00:00,000 --> 00:00:02,000\nUse Kafka for events.\n\n\
             2\n00:00:02,000 --> 00:00:04,000\nPostgre SQL stores records.",
        )
        .expect("static small fixture initial transcript must remain valid");
        let revised = parse_srt(
            "1\n00:00:00,000 --> 00:00:02,000\nUse Apache Kafka for events.\n\n\
             2\n00:00:02,000 --> 00:00:04,000\nPostgre SQL stores records.",
        )
        .expect("static small fixture revised transcript must remain valid");

        let terms = [SessionTermEntry::new(
            "Apache Kafka",
            vec!["Kafka".to_string()],
            Vec::new(),
        )];
        let initial_run = AnalysisRun::for_canonical_session_terms(&initial, &terms);
        let revised_run = AnalysisRun::for_canonical_session_terms(&revised, &terms);
        let detector_cases = run_term_review(&initial, &terms)
            .expect("static small fixture term configuration must remain valid");
        let detector_case = detector_cases
            .first()
            .expect("static small fixture must produce a detector-raised case");

        let initial_revision = initial.revision_id().to_tagged_string();
        let revised_revision = revised.revision_id().to_tagged_string();
        let analysis_one = analysis_identity("analysis-result-001", initial_run);
        let analysis_two = analysis_identity("analysis-result-002", revised_run);
        let detector_case_id = format!("review-case:detector:{}", detector_case.id().local_index());
        let human_case_id = "review-case:human:001".to_string();
        let human_creation_event_id = "human-case-created:001".to_string();
        let human_anchor = NormalizedAnchor {
            source_revision_id: revised_revision.clone(),
            segment_position: 1,
            start_byte: 0,
            end_byte: 11,
        };

        let state = NormalizedSemanticState {
            session: SessionIdentityState {
                session_id: "session:evidence:001".to_string(),
                duplicated_from_session_id: None,
            },
            session_format_version: "evidence-session-format:1".to_string(),
            interpretation_version: "evidence-semantics:1".to_string(),
            source_revisions: vec![
                source_revision(&initial, None),
                source_revision(&revised, Some(initial_revision.clone())),
            ],
            review_cases: vec![
                NormalizedReviewCase {
                    case_id: detector_case_id.clone(),
                    origin: ReviewCaseOrigin::DetectorRaised {
                        analysis_result_id: analysis_one.clone(),
                    },
                    observed_revision_id: initial_revision.clone(),
                    anchor: NormalizedAnchor {
                        source_revision_id: initial_revision.clone(),
                        segment_position: 0,
                        start_byte: 4,
                        end_byte: 9,
                    },
                    copied_decision_from_case_id: None,
                },
                NormalizedReviewCase {
                    case_id: human_case_id.clone(),
                    origin: ReviewCaseOrigin::HumanRaised {
                        creation_event_id: human_creation_event_id.clone(),
                    },
                    observed_revision_id: revised_revision.clone(),
                    anchor: human_anchor.clone(),
                    copied_decision_from_case_id: None,
                },
            ],
            review_case_raised_events: vec![ReviewCaseRaisedEventState {
                event_id: human_creation_event_id,
                sequence: 1,
                case_id: human_case_id.clone(),
                observed_revision_id: revised_revision.clone(),
                anchor: human_anchor,
                provenance: CanonicalEventProvenance::Human,
            }],
            review_ledger_events: vec![
                ReviewLedgerEventState {
                    event_id: "ledger-event:001".to_string(),
                    sequence: 1,
                    case_id: detector_case_id.clone(),
                    observed_revision_id: initial_revision.clone(),
                    action: ReviewLedgerAction::AcceptAlternative {
                        alternative_index: 0,
                    },
                    provenance: CanonicalEventProvenance::Human,
                },
                ReviewLedgerEventState {
                    event_id: "ledger-event:002".to_string(),
                    sequence: 2,
                    case_id: human_case_id.clone(),
                    observed_revision_id: revised_revision.clone(),
                    action: ReviewLedgerAction::ManualReplacement {
                        replacement_text: "PostgreSQL — production".to_string(),
                    },
                    provenance: CanonicalEventProvenance::Human,
                },
                ReviewLedgerEventState {
                    event_id: "ledger-event:003".to_string(),
                    sequence: 3,
                    case_id: detector_case_id.clone(),
                    observed_revision_id: initial_revision.clone(),
                    action: ReviewLedgerAction::Withdraw {
                        target_event_id: "ledger-event:001".to_string(),
                    },
                    provenance: CanonicalEventProvenance::Human,
                },
            ],
            analysis_results: vec![
                AnalysisResultState {
                    analysis_result_id: analysis_one.clone(),
                    source_revision_id: initial_revision,
                    knowledge_snapshot_ids: Vec::new(),
                },
                AnalysisResultState {
                    analysis_result_id: analysis_two.clone(),
                    source_revision_id: revised_revision,
                    knowledge_snapshot_ids: vec!["knowledge-snapshot:001".to_string()],
                },
            ],
            active_analysis_selection: Some(ActiveAnalysisSelection {
                analysis_result_id: analysis_two.clone(),
                selection_event_id: "active-analysis-selection:001".to_string(),
            }),
            knowledge_snapshot_references: vec![KnowledgeSnapshotReference {
                knowledge_snapshot_id: "knowledge-snapshot:001".to_string(),
                referenced_by_analysis_result_ids: vec![analysis_two],
            }],
            lineage_conflicts: vec![LineageConflict {
                conflict_id: "lineage-conflict:001".to_string(),
                kind: LineageConflictKind::AmbiguousCaseLineage,
                related_case_ids: vec![detector_case_id, human_case_id],
            }],
            artifacts: vec![
                ArtifactState {
                    artifact_id: "artifact:historical:referenced".to_string(),
                    class: ArtifactClass::ReferencedHistorical,
                    content_marker: "immutable-analysis-provenance".to_string(),
                },
                ArtifactState {
                    artifact_id: "artifact:historical:unreferenced".to_string(),
                    class: ArtifactClass::UnreferencedHistorical,
                    content_marker: "inactive-analysis-provenance".to_string(),
                },
                ArtifactState {
                    artifact_id: "artifact:derived:index".to_string(),
                    class: ArtifactClass::RebuildableDerived,
                    content_marker: "queue-index-v1".to_string(),
                },
                ArtifactState {
                    artifact_id: "artifact:temporary:cancelled-job".to_string(),
                    class: ArtifactClass::Temporary,
                    content_marker: "cancelled-analysis-output".to_string(),
                },
            ],
            retention_references: vec![RetentionReference {
                root_id: "ledger-event:002".to_string(),
                root_class: RetentionRootClass::ReviewLedgerEvent,
                artifact_id: "artifact:historical:referenced".to_string(),
                relation: RetentionRelation::PreservesHistoricalProvenance,
            }],
        }
        .normalize();

        Self {
            fixture_id: SMALL_FIXTURE_ID.to_string(),
            fixture_version: SMALL_FIXTURE_VERSION.to_string(),
            scale: FixtureScale::Small,
            expected_state: state,
        }
    }

    pub fn normalized_state(&self) -> NormalizedSemanticState {
        self.expected_state.clone().normalize()
    }
}

fn source_revision(
    transcript: &Transcript,
    predecessor_revision_id: Option<String>,
) -> NormalizedSourceRevision {
    NormalizedSourceRevision {
        revision_id: transcript.revision_id().to_tagged_string(),
        predecessor_revision_id,
        segments: transcript
            .segments()
            .iter()
            .map(|segment| SourceSegmentState {
                cue_index: segment.index(),
                start_ms: segment.start_ms(),
                end_ms: segment.end_ms(),
                text: segment.text().to_string(),
            })
            .collect(),
    }
}

fn analysis_identity(prefix: &str, run: AnalysisRun) -> String {
    let snapshot = run.snapshot();
    let detector_ids = snapshot
        .configuration()
        .detector_set()
        .detectors()
        .iter()
        .map(|detector| format!("{}@{}", detector.id(), detector.version()))
        .collect::<Vec<_>>()
        .join(",");

    format!(
        "{prefix}:{}:{}:{}:{}@{}:{}@{}",
        snapshot.source_revision().to_tagged_string(),
        snapshot.session_terms().to_tagged_string(),
        detector_ids,
        snapshot.configuration().detector_config().id(),
        snapshot.configuration().detector_config().version(),
        snapshot.configuration().algorithm().id(),
        snapshot.configuration().algorithm().version(),
    )
}
