#![allow(clippy::too_many_arguments)]

use vox_proof::artifact_bundle::{ArtifactBundleId, ArtifactId};
use vox_proof::candidate::DetectionKind;
use vox_proof::detector_reference_join::{
    DetectorReferenceJoinContext, DetectorReferenceJoinId, DetectorReferenceJoinRevisionId,
};
use vox_proof::detector_snapshot::{
    DETECTOR_PROPOSAL_SNAPSHOT_SCHEMA, DetectorAnalysisIdentity, DetectorComponentIdentity,
    DetectorProposalAlternative, DetectorProposalEvidence, DetectorProposalId,
    DetectorProposalRecord, DetectorProposalSemanticKey, DetectorProposalSnapshot,
    DetectorProposalSnapshotState, DetectorProposalSourceAnchor, DetectorSessionTermEntry,
    DetectorSnapshotRevisionId,
};
use vox_proof::human_final_reference::{
    HUMAN_FINAL_REFERENCE_SCHEMA, HumanFinalReference, HumanFinalReferenceState, ReferenceClass,
    ReferenceErrorId, ReferenceErrorRecord, ReferenceSourceAnchor,
};
use vox_proof::join_adjudication::{
    OVERLAP_ADJUDICATION_SCHEMA, OverlapAdjudicationId, OverlapAdjudicationRecord,
    OverlapAdjudicationResult, OverlapAdjudicationSet, OverlapAdjudicationSetId,
    OverlapAdjudicationSetState, OverlapAdjudicatorRole,
};
use vox_proof::join_metric_aggregation::{MetricAggregateRevisionId, MetricAggregateSetId};
use vox_proof::join_metric_contribution::{MetricContributionRevisionId, MetricContributionSetId};
use vox_proof::reference_coverage::{
    CueReferenceId, CueReviewCompletionRecord, ExpectedCueUniverse, REFERENCE_COVERAGE_SCHEMA,
    ReferenceCoverage, ReferenceCoverageId, ReferenceCoveragePurpose, ReferenceCoverageState,
    ReferenceCueDisposition,
};
use vox_proof::reference_identity::{
    CueSourceTextDigest, ReferenceReviewerIdentityClass, ReferenceRevisionId, VerificationBasis,
};
use vox_proof::reference_seal::{
    CalibrationValidityImpact, REFERENCE_SEAL_SCHEMA, ReferenceCalibrationValidity,
    ReferenceProducerClass, ReferenceSeal, ReferenceSealId, ReferenceSealState,
};
use vox_proof::run_manifest::{CalibrationValidityMode, InputClass, InputIdentityReference, RunId};
use vox_proof::synthetic_evaluation_harness::{
    SyntheticEvaluationArtifactIds, SyntheticEvaluationFixture, SyntheticEvaluationFixtureId,
    SyntheticEvaluationRevisionIds, SyntheticEvaluationTimestamps,
};

pub const SAMPLE_REVISION: &str =
    "rev:sha256-v1:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
pub const SAMPLE_REFERENCE_REVISION: &str = "ref-rev-synth-001";
pub const SAMPLE_DIGEST: &str =
    "sha256:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
pub const SAMPLE_SESSION_TERMS: &str =
    "session-terms:sha256-v1:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
pub const JOIN_CONTRACT_REVISION: &str = "voxproof-detector-reference-join-v1";
const RUN_ID: &str = "run-synth-eval-001";
const TIMESTAMP_MS: u64 = 1_700_000_000_000;

pub enum FixtureMutation {
    RealInputClass,
    RealMaterialQualification,
    HumanSealProducer,
    PrimaryCoveragePurpose,
    OwnerAdjudicator,
    MismatchedRunId,
    MismatchedInputIdentity,
    MismatchedReferenceRevision,
}

pub fn mutate_fixture(fixture: &mut SyntheticEvaluationFixture, mutation: FixtureMutation) {
    match mutation {
        FixtureMutation::RealInputClass => {
            fixture.input_class = InputClass::SelfOwnedReal;
        }
        FixtureMutation::RealMaterialQualification => {
            fixture.qualifies_as_real_material_evidence = true;
        }
        FixtureMutation::HumanSealProducer => {
            fixture.reference_seal.producer_class = ReferenceProducerClass::HumanBlindReviewer;
        }
        FixtureMutation::PrimaryCoveragePurpose => {
            fixture.reference_coverage.coverage_purpose =
                ReferenceCoveragePurpose::PrimaryBlindCalibration;
        }
        FixtureMutation::OwnerAdjudicator => {
            if let Some(set) = fixture.assisted_review_adjudication_set.as_mut() {
                for record in &mut set.records {
                    record.adjudicator_role = OverlapAdjudicatorRole::OwnerAdjudicator;
                }
            }
        }
        FixtureMutation::MismatchedRunId => {
            fixture.detector_snapshot.run_id = RunId::new("run-mismatch").expect("run id");
        }
        FixtureMutation::MismatchedInputIdentity => {
            fixture.detector_snapshot.input_identity = InputIdentityReference {
                transcript_revision_id:
                    "rev:sha256-v1:ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff"
                        .to_string(),
            };
        }
        FixtureMutation::MismatchedReferenceRevision => {
            fixture.human_final_reference.reference_revision =
                ReferenceRevisionId::new("ref-rev-mismatch").expect("revision");
        }
    }
}

pub fn exact_only_multi_disposition_fixture() -> SyntheticEvaluationFixture {
    build_fixture(
        "fixture-exact-only-multi-disposition",
        vec![
            record(1, ReferenceCueDisposition::TranscriptionError),
            record(2, ReferenceCueDisposition::TranscriptionError),
            record(3, ReferenceCueDisposition::TranscriptionError),
            record(4, ReferenceCueDisposition::TranscriptionError),
            record(5, ReferenceCueDisposition::TranscriptionError),
            record(6, ReferenceCueDisposition::TranscriptionError),
        ],
        vec![
            reference_error_record("ref-err-exact", 1, 0, 0, 4, "wrong"),
            reference_error_record("ref-err-wrong", 2, 1, 0, 4, "wrong"),
            reference_error_record("ref-err-dup", 3, 2, 0, 4, "wrong"),
            reference_error_record("ref-err-unmatched", 4, 3, 8, 12, "wrong"),
            ReferenceErrorRecord {
                reference_error_id: ReferenceErrorId::new("ref-err-excluded").expect("error id"),
                reference_revision: ReferenceRevisionId::new(SAMPLE_REFERENCE_REVISION)
                    .expect("revision"),
                input_identity: input_identity(),
                source_anchor: reference_source_anchor(5, 4, 0, 4),
                original_surface: "wrng".to_string(),
                human_final_surface: "wrong".to_string(),
                reference_class: ReferenceClass::TranscriptionError,
                verification_basis: VerificationBasis::TranscriptContextOnly,
                reviewer_identity_class: ReferenceReviewerIdentityClass::SyntheticFixtureGenerator,
                reviewed_at_unix_ms: TIMESTAMP_MS,
            },
            reference_error_record("ref-err-extra", 6, 5, 0, 4, "wrong"),
        ],
        vec![
            glossary_proposal("det-prop-exact", 1, 0, 0, 4, "wrng", "wrong"),
            glossary_proposal("det-prop-wrong", 2, 1, 0, 4, "wrng", "wright"),
            observed_error_proposal("det-prop-dup-b", 3, 2, 0, 4, "wrng", "wrong"),
            glossary_proposal("det-prop-dup-a", 3, 2, 0, 4, "wrng", "wrong"),
            glossary_proposal("det-prop-unmatched", 5, 4, 0, 4, "wrng", "wrong"),
        ],
        None,
    )
}

pub fn overlap_pending_then_resolved_fixture() -> SyntheticEvaluationFixture {
    build_fixture(
        "fixture-overlap-pending-then-resolved",
        vec![record(1, ReferenceCueDisposition::TranscriptionError)],
        vec![reference_error_record(
            "ref-err-overlap",
            1,
            0,
            0,
            4,
            "wrong",
        )],
        vec![glossary_proposal(
            "det-prop-overlap",
            1,
            0,
            2,
            6,
            "wrng",
            "wrong",
        )],
        Some(vec![adjudication_record(
            "adj-overlap-001",
            "det-prop-overlap",
            "ref-err-overlap",
            OverlapAdjudicationResult::SameErrorSameCorrection,
        )]),
    )
}

pub fn zero_population_fixture() -> SyntheticEvaluationFixture {
    build_fixture(
        "fixture-zero-population",
        vec![record(1, ReferenceCueDisposition::NoTranscriptionError)],
        Vec::new(),
        Vec::new(),
        None,
    )
}

fn build_fixture(
    fixture_id: &str,
    coverage_records: Vec<CueReviewCompletionRecord>,
    reference_records: Vec<ReferenceErrorRecord>,
    proposals: Vec<DetectorProposalRecord>,
    assisted_adjudication: Option<Vec<OverlapAdjudicationRecord>>,
) -> SyntheticEvaluationFixture {
    let seal = synthetic_seal();
    let mut coverage = synthetic_coverage(coverage_records);
    let human_reference = synthetic_human_reference(&coverage, reference_records);
    coverage.assessment.total_eligible_transcription_errors = human_reference
        .assessment
        .recall_eligible_transcription_error_count;
    let detector_snapshot = synthetic_snapshot(proposals);
    let detector_execution_adjudication_set = empty_adjudication_set("adj-set-detector-empty");
    let assisted_review_adjudication_set =
        assisted_adjudication.map(|records| adjudication_set("adj-set-assisted", records));

    SyntheticEvaluationFixture {
        fixture_id: SyntheticEvaluationFixtureId::new(fixture_id).expect("fixture id"),
        input_class: InputClass::SyntheticProtocolFixture,
        qualifies_as_real_material_evidence: false,
        reference_seal: seal,
        reference_coverage: coverage,
        human_final_reference: human_reference,
        detector_snapshot,
        detector_execution_adjudication_set,
        assisted_review_adjudication_set,
        artifact_ids: artifact_ids(),
        revision_ids: revision_ids(),
        fixed_timestamps: SyntheticEvaluationTimestamps {
            reference_sealed_unix_ms: TIMESTAMP_MS,
            detector_frozen_unix_ms: TIMESTAMP_MS,
            adjudication_unix_ms: TIMESTAMP_MS,
        },
    }
}

fn artifact_ids() -> SyntheticEvaluationArtifactIds {
    SyntheticEvaluationArtifactIds {
        reference_seal: ArtifactId::new("artifact-reference-seal").expect("artifact id"),
        human_final_reference: ArtifactId::new("artifact-human-reference").expect("artifact id"),
        cue_review_completion: ArtifactId::new("artifact-cue-coverage").expect("artifact id"),
        detector_output: ArtifactId::new("artifact-detector-output").expect("artifact id"),
        evaluation_join: ArtifactId::new("artifact-evaluation-join").expect("artifact id"),
        join_adjudication: ArtifactId::new("artifact-join-adjudication").expect("artifact id"),
        metric_contributions: ArtifactId::new("artifact-metric-contributions")
            .expect("artifact id"),
        metrics: ArtifactId::new("artifact-metrics").expect("artifact id"),
        bundle: ArtifactBundleId::new("bundle-synth-eval").expect("bundle id"),
    }
}

fn revision_ids() -> SyntheticEvaluationRevisionIds {
    SyntheticEvaluationRevisionIds {
        join_context: DetectorReferenceJoinContext {
            join_id: DetectorReferenceJoinId::new("join-synth-001").expect("join id"),
            join_revision: DetectorReferenceJoinRevisionId::new("join-rev-synth-001")
                .expect("join revision"),
            evaluation_join_artifact_id: ArtifactId::new("artifact-evaluation-join")
                .expect("artifact id"),
            join_adjudication_artifact_id: ArtifactId::new("artifact-join-adjudication")
                .expect("artifact id"),
        },
        contribution_context: vox_proof::join_metric_contribution::JoinMetricContributionContext {
            contribution_set_id: MetricContributionSetId::new("metric-contrib-set-synth-001")
                .expect("contribution set id"),
            contribution_revision: MetricContributionRevisionId::new(
                "metric-contrib-rev-synth-001",
            )
            .expect("contribution revision"),
            metric_contributions_artifact_id: ArtifactId::new("artifact-metric-contributions")
                .expect("artifact id"),
        },
        aggregate_context: vox_proof::join_metric_aggregation::JoinMetricAggregateContext {
            aggregate_set_id: MetricAggregateSetId::new("metric-aggregate-set-synth-001")
                .expect("aggregate set id"),
            aggregate_revision: MetricAggregateRevisionId::new("metric-aggregate-rev-synth-001")
                .expect("aggregate revision"),
            metrics_artifact_id: ArtifactId::new("artifact-metrics").expect("artifact id"),
        },
    }
}

fn input_identity() -> InputIdentityReference {
    InputIdentityReference {
        transcript_revision_id: SAMPLE_REVISION.to_string(),
    }
}

fn synthetic_seal() -> ReferenceSeal {
    ReferenceSeal {
        schema_revision: REFERENCE_SEAL_SCHEMA.to_string(),
        seal_id: ReferenceSealId::new("seal-synth-001").expect("seal id"),
        run_id: RunId::new(RUN_ID).expect("run id"),
        input_identity: input_identity(),
        reference_revision: ReferenceRevisionId::new(SAMPLE_REFERENCE_REVISION).expect("revision"),
        producer_class: ReferenceProducerClass::SyntheticFixtureGenerator,
        reference_created_before_detector_run: true,
        prior_detector_run_on_same_input: false,
        prior_knowledge_of_detector_targets: false,
        session_terms_visible_during_reference: false,
        external_notes_encode_detector_targets: false,
        seal_state: ReferenceSealState::Sealed,
        calibration_classification: ReferenceCalibrationValidity::SyntheticProtocolOnly,
        calibration_validity_impact: CalibrationValidityImpact::ProtocolOnly,
    }
}

fn synthetic_coverage(records: Vec<CueReviewCompletionRecord>) -> ReferenceCoverage {
    let cue_ids: Vec<u32> = records.iter().map(|entry| entry.cue_id.value()).collect();
    let expected = universe(&cue_ids);
    let assessment =
        ReferenceCoverage::derive_assessment(&expected, &records).expect("derive assessment");
    ReferenceCoverage {
        schema_revision: REFERENCE_COVERAGE_SCHEMA.to_string(),
        coverage_id: ReferenceCoverageId::new("coverage-synth-001").expect("coverage id"),
        run_id: RunId::new(RUN_ID).expect("run id"),
        input_identity: input_identity(),
        seal_id: ReferenceSealId::new("seal-synth-001").expect("seal id"),
        reference_revision: ReferenceRevisionId::new(SAMPLE_REFERENCE_REVISION).expect("revision"),
        coverage_purpose: ReferenceCoveragePurpose::SyntheticProtocolValidation,
        expected_universe: expected,
        records,
        coverage_state: ReferenceCoverageState::Complete,
        assessment,
    }
}

fn synthetic_human_reference(
    coverage: &ReferenceCoverage,
    records: Vec<ReferenceErrorRecord>,
) -> HumanFinalReference {
    let assessment = HumanFinalReference::derive_assessment(
        &coverage.reference_revision,
        &coverage.input_identity,
        &records,
    )
    .expect("derive assessment");
    HumanFinalReference {
        schema_revision: HUMAN_FINAL_REFERENCE_SCHEMA.to_string(),
        run_id: coverage.run_id.clone(),
        input_identity: coverage.input_identity.clone(),
        seal_id: coverage.seal_id.clone(),
        reference_revision: coverage.reference_revision.clone(),
        records,
        state: HumanFinalReferenceState::Sealed,
        assessment,
    }
}

fn synthetic_snapshot(proposals: Vec<DetectorProposalRecord>) -> DetectorProposalSnapshot {
    let assessment = DetectorProposalSnapshot::derive_assessment(
        &DetectorSnapshotRevisionId::new("det-snap-rev-synth-001").expect("snapshot revision"),
        &input_identity(),
        &analysis_identity(),
        &proposals,
    )
    .expect("derive assessment");
    DetectorProposalSnapshot {
        schema_revision: DETECTOR_PROPOSAL_SNAPSHOT_SCHEMA.to_string(),
        run_id: RunId::new(RUN_ID).expect("run id"),
        input_identity: input_identity(),
        calibration_validity: CalibrationValidityMode::BlindReference,
        snapshot_revision: DetectorSnapshotRevisionId::new("det-snap-rev-synth-001")
            .expect("snapshot revision"),
        detector_output_artifact_id: ArtifactId::new("artifact-detector-output")
            .expect("artifact id"),
        analysis_identity: analysis_identity(),
        proposals,
        frozen_at_unix_ms: TIMESTAMP_MS,
        state: DetectorProposalSnapshotState::Frozen,
        assessment,
    }
}

fn empty_adjudication_set(set_id: &str) -> OverlapAdjudicationSet {
    adjudication_set(set_id, Vec::new())
}

fn adjudication_set(
    set_id: &str,
    records: Vec<OverlapAdjudicationRecord>,
) -> OverlapAdjudicationSet {
    let assessment = OverlapAdjudicationSet::derive_assessment(&records);
    OverlapAdjudicationSet {
        schema_revision: OVERLAP_ADJUDICATION_SCHEMA.to_string(),
        adjudication_set_id: OverlapAdjudicationSetId::new(set_id).expect("set id"),
        run_id: RunId::new(RUN_ID).expect("run id"),
        input_identity: input_identity(),
        reference_revision: ReferenceRevisionId::new(SAMPLE_REFERENCE_REVISION).expect("revision"),
        detector_snapshot_revision: DetectorSnapshotRevisionId::new("det-snap-rev-synth-001")
            .expect("snapshot revision"),
        join_contract_revision: JOIN_CONTRACT_REVISION.to_string(),
        overlap_rule_revision: "voxproof-overlap-v1".to_string(),
        join_adjudication_artifact_id: ArtifactId::new("artifact-join-adjudication")
            .expect("artifact id"),
        state: OverlapAdjudicationSetState::Frozen,
        records,
        assessment,
    }
}

fn record(cue_id: u32, disposition: ReferenceCueDisposition) -> CueReviewCompletionRecord {
    CueReviewCompletionRecord {
        cue_id: CueReferenceId::new(cue_id).expect("cue id"),
        segment_position: cue_id - 1,
        source_text_digest: CueSourceTextDigest::new(SAMPLE_DIGEST).expect("digest"),
        disposition,
        fully_reviewed: true,
        all_known_transcription_errors_enumerated: true,
        verification_source_used: VerificationBasis::AudioListened,
        reviewer_identity_class: ReferenceReviewerIdentityClass::SyntheticFixtureGenerator,
        completed_at_unix_ms: TIMESTAMP_MS,
    }
}

fn universe(cue_ids: &[u32]) -> ExpectedCueUniverse {
    ExpectedCueUniverse {
        total_cues: cue_ids.len() as u32,
        cue_ids: cue_ids
            .iter()
            .map(|id| CueReferenceId::new(*id).expect("cue id"))
            .collect(),
    }
}

fn reference_source_anchor(
    cue_id: u32,
    segment_position: u32,
    start: u32,
    end: u32,
) -> ReferenceSourceAnchor {
    ReferenceSourceAnchor {
        input_identity: input_identity(),
        cue_id: CueReferenceId::new(cue_id).expect("cue id"),
        segment_position,
        start_byte: start,
        end_byte: end,
    }
}

fn reference_error_record(
    error_id: &str,
    cue_id: u32,
    segment_position: u32,
    start: u32,
    end: u32,
    human_final_surface: &str,
) -> ReferenceErrorRecord {
    ReferenceErrorRecord {
        reference_error_id: ReferenceErrorId::new(error_id).expect("error id"),
        reference_revision: ReferenceRevisionId::new(SAMPLE_REFERENCE_REVISION).expect("revision"),
        input_identity: input_identity(),
        source_anchor: reference_source_anchor(cue_id, segment_position, start, end),
        original_surface: "wrng".to_string(),
        human_final_surface: human_final_surface.to_string(),
        reference_class: ReferenceClass::TranscriptionError,
        verification_basis: VerificationBasis::AudioListened,
        reviewer_identity_class: ReferenceReviewerIdentityClass::SyntheticFixtureGenerator,
        reviewed_at_unix_ms: TIMESTAMP_MS,
    }
}

fn source_anchor(
    cue_id: u32,
    segment_position: u32,
    start: u32,
    end: u32,
) -> DetectorProposalSourceAnchor {
    DetectorProposalSourceAnchor {
        input_identity: input_identity(),
        cue_id: CueReferenceId::new(cue_id).expect("cue id"),
        segment_position,
        start_byte: start,
        end_byte: end,
    }
}

fn detector_component(id: &str, version: &str) -> DetectorComponentIdentity {
    DetectorComponentIdentity {
        id: id.to_string(),
        version: version.to_string(),
    }
}

fn analysis_identity() -> DetectorAnalysisIdentity {
    DetectorAnalysisIdentity {
        input_identity: input_identity(),
        session_terms_identity: SAMPLE_SESSION_TERMS.to_string(),
        detector_set: vec![
            detector_component("glossary-alias-match", "0.1.0"),
            detector_component("observed-error-form-match", "0.1.0"),
        ],
        detector_config: detector_component("detector-config", "0.1.0"),
        algorithm: detector_component("algorithm-v1", "0.1.0"),
    }
}

fn glossary_proposal(
    proposal_id: &str,
    cue_id: u32,
    segment_position: u32,
    start: u32,
    end: u32,
    observed: &str,
    correction: &str,
) -> DetectorProposalRecord {
    let detector = detector_component("glossary-alias-match", "0.1.0");
    let anchor = source_anchor(cue_id, segment_position, start, end);
    let mut record = DetectorProposalRecord {
        detector_proposal_id: DetectorProposalId::new(proposal_id).expect("proposal id"),
        snapshot_revision: DetectorSnapshotRevisionId::new("det-snap-rev-synth-001")
            .expect("snapshot revision"),
        input_identity: input_identity(),
        semantic_key: DetectorProposalSemanticKey {
            detector_id: detector.id.clone(),
            detection_kind: DetectionKind::GlossaryAliasMatch,
            source_anchor: anchor.clone(),
        },
        detector: detector.clone(),
        source_anchor: anchor,
        observed_surface: observed.to_string(),
        detection_kind: DetectionKind::GlossaryAliasMatch,
        evidence: DetectorProposalEvidence::GlossaryAlias {
            entry: DetectorSessionTermEntry {
                canonical_term: "widget".to_string(),
                aliases: vec!["wijet".to_string()],
                observed_error_forms: vec!["widgit".to_string()],
            },
            matched_form: observed.to_string(),
        },
        alternatives: vec![DetectorProposalAlternative {
            alternative_index: 0,
            replacement_surface: correction.to_string(),
        }],
    };
    record.semantic_key = record.derive_semantic_key();
    record
}

fn observed_error_proposal(
    proposal_id: &str,
    cue_id: u32,
    segment_position: u32,
    start: u32,
    end: u32,
    observed: &str,
    correction: &str,
) -> DetectorProposalRecord {
    let detector = detector_component("observed-error-form-match", "0.1.0");
    let anchor = source_anchor(cue_id, segment_position, start, end);
    let mut record = DetectorProposalRecord {
        detector_proposal_id: DetectorProposalId::new(proposal_id).expect("proposal id"),
        snapshot_revision: DetectorSnapshotRevisionId::new("det-snap-rev-synth-001")
            .expect("snapshot revision"),
        input_identity: input_identity(),
        semantic_key: DetectorProposalSemanticKey {
            detector_id: detector.id.clone(),
            detection_kind: DetectionKind::GlossaryAliasMatch,
            source_anchor: anchor.clone(),
        },
        detector: detector.clone(),
        source_anchor: anchor,
        observed_surface: observed.to_string(),
        detection_kind: DetectionKind::GlossaryAliasMatch,
        evidence: DetectorProposalEvidence::ObservedErrorForm {
            entry: DetectorSessionTermEntry {
                canonical_term: "widget".to_string(),
                aliases: vec!["wijet".to_string()],
                observed_error_forms: vec!["widgit".to_string()],
            },
            matched_form: observed.to_string(),
        },
        alternatives: vec![DetectorProposalAlternative {
            alternative_index: 0,
            replacement_surface: correction.to_string(),
        }],
    };
    record.semantic_key = record.derive_semantic_key();
    record
}

fn adjudication_record(
    adjudication_id: &str,
    proposal_id: &str,
    reference_error_id: &str,
    result: OverlapAdjudicationResult,
) -> OverlapAdjudicationRecord {
    OverlapAdjudicationRecord {
        adjudication_id: OverlapAdjudicationId::new(adjudication_id).expect("adjudication id"),
        detector_proposal_id: DetectorProposalId::new(proposal_id).expect("proposal id"),
        reference_error_id: ReferenceErrorId::new(reference_error_id).expect("reference error id"),
        join_contract_revision: JOIN_CONTRACT_REVISION.to_string(),
        adjudicator_role: OverlapAdjudicatorRole::SyntheticFixtureAdjudicator,
        adjudication_result: result,
        adjudication_reason: "synthetic fixture overlap adjudication".to_string(),
        adjudicated_at_unix_ms: TIMESTAMP_MS,
    }
}
