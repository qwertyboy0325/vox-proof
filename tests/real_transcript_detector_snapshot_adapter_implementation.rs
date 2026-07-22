#[path = "support/real_transcript_detector_snapshot_adapter_fixtures.rs"]
mod fixtures;

fn observed_surface_byte_len(surface: &str) -> u32 {
    #[allow(clippy::needless_as_bytes)] // anchor contract uses byte offsets
    {
        surface.as_bytes().len() as u32
    }
}

use fixtures::{
    FROZEN_AT_MS, RUN_ID, adapter_request_for, aligned_run_request, combined_canonical_fixture,
    execution_input_for_materialized, input_identity_for, reversed_proposal_ids,
    single_cue_zero_run_request, zero_candidate_fixture,
};
use std::fs;
use vox_proof::detector_snapshot::{DetectorProposalEvidence, DetectorProposalSnapshotState};
use vox_proof::pipeline::run_canonical_term_review;
use vox_proof::real_transcript_detector_snapshot_adapter::{
    RealTranscriptDetectorSnapshotAdapterContractError,
    RealTranscriptDetectorSnapshotMaterializationError,
    materialize_real_transcript_detector_snapshot,
};
use vox_proof::real_transcript_evaluation_execution::{
    RealTranscriptEvaluationCompletionStage, RealTranscriptEvaluationExecutionOutcome,
    RealTranscriptEvaluationStage, execute_real_transcript_evaluation,
};
use vox_proof::run_manifest::InputClass;
use vox_proof::srt::parse_srt;

#[test]
fn successful_combined_materialization_maps_all_proposals() {
    let (transcript, canonical_run) = combined_canonical_fixture();
    let run_request = aligned_run_request(&transcript, &canonical_run, InputClass::SelfOwnedReal);
    let adapter_request = adapter_request_for(&canonical_run);

    let result = materialize_real_transcript_detector_snapshot(
        &run_request,
        &adapter_request,
        &transcript,
        &canonical_run,
    )
    .expect("materialize");

    assert_eq!(result.detector_snapshot.proposals.len(), 3);
    assert_eq!(
        result.detector_snapshot.state,
        DetectorProposalSnapshotState::Frozen
    );

    for (index, (review_case, proposal)) in canonical_run
        .review_cases()
        .iter()
        .zip(result.detector_snapshot.proposals.iter())
        .enumerate()
    {
        let candidate = review_case.candidate_span();
        let anchor = candidate.anchor();
        let segment = &transcript.segments()[anchor.segment_position()];

        assert_eq!(
            proposal.detector_proposal_id.as_str(),
            format!("det-prop-adapter-{index:03}")
        );
        assert_eq!(
            proposal.snapshot_revision,
            result.validated_plan.snapshot_revision
        );
        assert_eq!(
            proposal.input_identity,
            result.validated_plan.input_identity
        );
        assert_eq!(proposal.detector.id, candidate.provenance().detector_id());
        assert_eq!(
            proposal.detector.version,
            candidate.provenance().detector_version()
        );
        assert_eq!(proposal.detection_kind, candidate.kind());
        assert_eq!(proposal.source_anchor.cue_id.value(), segment.index());
        assert_eq!(
            proposal.source_anchor.segment_position as usize,
            anchor.segment_position()
        );
        assert_eq!(
            observed_surface_byte_len(&proposal.observed_surface),
            proposal.source_anchor.end_byte - proposal.source_anchor.start_byte
        );
        assert_eq!(
            proposal.observed_surface,
            transcript.resolve(anchor).expect("resolve")
        );
        assert_eq!(proposal.semantic_key, proposal.derive_semantic_key());
        proposal
            .validate(
                &result.validated_plan.snapshot_revision,
                &result.validated_plan.input_identity,
                &result.validated_plan.detector_analysis_identity,
            )
            .expect("record validate");
    }
}

#[test]
fn evidence_fidelity_preserves_all_variants() {
    let (transcript, canonical_run) = combined_canonical_fixture();
    let run_request = aligned_run_request(&transcript, &canonical_run, InputClass::SelfOwnedReal);
    let adapter_request = adapter_request_for(&canonical_run);
    let result = materialize_real_transcript_detector_snapshot(
        &run_request,
        &adapter_request,
        &transcript,
        &canonical_run,
    )
    .expect("materialize");

    for (review_case, proposal) in canonical_run
        .review_cases()
        .iter()
        .zip(result.detector_snapshot.proposals.iter())
    {
        let candidate = review_case.candidate_span();
        match (candidate.evidence(), &proposal.evidence) {
            (
                vox_proof::candidate::Evidence::GlossaryAlias(source),
                DetectorProposalEvidence::GlossaryAlias {
                    entry,
                    matched_form,
                },
            ) => {
                assert_eq!(entry.canonical_term, source.entry.canonical_term);
                assert_eq!(entry.aliases, source.entry.aliases);
                assert_eq!(
                    entry.observed_error_forms,
                    source.entry.observed_error_forms
                );
                assert_eq!(matched_form, &source.matched_form);
            }
            (
                vox_proof::candidate::Evidence::ObservedErrorForm(source),
                DetectorProposalEvidence::ObservedErrorForm {
                    entry,
                    matched_form,
                },
            ) => {
                assert_eq!(entry.canonical_term, source.entry.canonical_term);
                assert_eq!(entry.aliases, source.entry.aliases);
                assert_eq!(
                    entry.observed_error_forms,
                    source.entry.observed_error_forms
                );
                assert_eq!(matched_form, &source.matched_form);
            }
            (
                vox_proof::candidate::Evidence::PhoneticSimilarity(source),
                DetectorProposalEvidence::PhoneticSimilarity {
                    observed_surface,
                    target_surface,
                    target_kind,
                    canonical_term,
                    source_representation,
                    target_representation,
                    comparison,
                    detector_config,
                    algorithm,
                },
            ) => {
                assert_eq!(observed_surface, &source.observed_surface);
                assert_eq!(target_surface, &source.target_surface);
                assert_eq!(
                    format!("{target_kind:?}"),
                    format!("{:?}", source.target_kind)
                );
                assert_eq!(canonical_term, &source.canonical_term);
                assert_eq!(
                    source_representation.normalized_letters,
                    source.source_representation.normalized_letters
                );
                assert_eq!(
                    source_representation.primary_key,
                    source.source_representation.primary_key
                );
                assert_eq!(
                    source_representation.alternate_key,
                    source.source_representation.alternate_key
                );
                assert_eq!(
                    target_representation.normalized_letters,
                    source.target_representation.normalized_letters
                );
                assert_eq!(
                    target_representation.primary_key,
                    source.target_representation.primary_key
                );
                assert_eq!(
                    target_representation.alternate_key,
                    source.target_representation.alternate_key
                );
                assert_eq!(
                    comparison.edit_distance,
                    u32::try_from(source.comparison.edit_distance).expect("edit_distance")
                );
                assert_eq!(
                    comparison.ratio_numerator,
                    u32::try_from(source.comparison.ratio_numerator).expect("ratio_numerator")
                );
                assert_eq!(
                    comparison.ratio_denominator,
                    u32::try_from(source.comparison.ratio_denominator).expect("ratio_denominator")
                );
                assert_eq!(
                    comparison.ratio_permille,
                    u32::try_from(source.comparison.ratio_permille).expect("ratio_permille")
                );
                assert_eq!(comparison.matched_key, source.comparison.matched_key);
                assert_eq!(detector_config.id, source.detector_config.id());
                assert_eq!(detector_config.version, source.detector_config.version());
                assert_eq!(algorithm.id, source.algorithm.id());
                assert_eq!(algorithm.version, source.algorithm.version());
            }
            _ => panic!("evidence variant mismatch"),
        }
    }
}

#[test]
fn alternative_fidelity_preserves_order_and_indexes() {
    let (transcript, canonical_run) = combined_canonical_fixture();
    let run_request = aligned_run_request(&transcript, &canonical_run, InputClass::SelfOwnedReal);
    let adapter_request = adapter_request_for(&canonical_run);
    let result = materialize_real_transcript_detector_snapshot(
        &run_request,
        &adapter_request,
        &transcript,
        &canonical_run,
    )
    .expect("materialize");

    for (review_case, proposal) in canonical_run
        .review_cases()
        .iter()
        .zip(result.detector_snapshot.proposals.iter())
    {
        let source_alternatives = review_case.candidate_span().alternatives();
        assert_eq!(proposal.alternatives.len(), source_alternatives.len());
        for (index, (source, mapped)) in source_alternatives
            .iter()
            .zip(proposal.alternatives.iter())
            .enumerate()
        {
            assert_eq!(mapped.alternative_index, index as u32);
            assert_eq!(mapped.replacement_surface, source.replacement_text());
        }
    }
}

#[test]
fn snapshot_closure_validates_and_matches_plan() {
    let (transcript, canonical_run) = combined_canonical_fixture();
    let run_request = aligned_run_request(&transcript, &canonical_run, InputClass::SelfOwnedReal);
    let adapter_request = adapter_request_for(&canonical_run);
    let result = materialize_real_transcript_detector_snapshot(
        &run_request,
        &adapter_request,
        &transcript,
        &canonical_run,
    )
    .expect("materialize");

    let snapshot = &result.detector_snapshot;
    assert_eq!(snapshot.run_id.as_str(), RUN_ID);
    assert_eq!(snapshot.input_identity, input_identity_for(&transcript));
    assert_eq!(
        snapshot.snapshot_revision,
        result.validated_plan.snapshot_revision
    );
    assert_eq!(
        snapshot.detector_output_artifact_id,
        result.validated_plan.detector_output_artifact_id
    );
    assert_eq!(
        snapshot.analysis_identity,
        result.validated_plan.detector_analysis_identity
    );
    assert_eq!(snapshot.frozen_at_unix_ms, FROZEN_AT_MS);
    assert_eq!(snapshot.state, DetectorProposalSnapshotState::Frozen);
    assert_eq!(snapshot.proposals.len(), 3);
    snapshot.validate().expect("snapshot validate");
    snapshot
        .validate_for_freeze_against(&run_request.detector_execution_envelope)
        .expect("freeze validate");
}

#[test]
fn reversed_proposal_ids_preserve_canonical_case_order() {
    let (transcript, canonical_run) = combined_canonical_fixture();
    let run_request = aligned_run_request(&transcript, &canonical_run, InputClass::SelfOwnedReal);
    let mut adapter_request = adapter_request_for(&canonical_run);
    adapter_request.proposal_ids = reversed_proposal_ids(&adapter_request.proposal_ids);

    let result = materialize_real_transcript_detector_snapshot(
        &run_request,
        &adapter_request,
        &transcript,
        &canonical_run,
    )
    .expect("materialize");

    for (index, review_case) in canonical_run.review_cases().iter().enumerate() {
        let proposal = &result.detector_snapshot.proposals[index];
        assert_eq!(proposal.detection_kind, review_case.candidate_span().kind());
        assert_eq!(
            proposal.detector_proposal_id,
            adapter_request.proposal_ids[index]
        );
    }
}

#[test]
fn zero_candidate_materialization_produces_empty_frozen_snapshot() {
    let (transcript, canonical_run) = zero_candidate_fixture();
    let run_request = single_cue_zero_run_request(&transcript, &canonical_run);
    let adapter_request = adapter_request_for(&canonical_run);

    let result = materialize_real_transcript_detector_snapshot(
        &run_request,
        &adapter_request,
        &transcript,
        &canonical_run,
    )
    .expect("materialize");

    assert!(result.detector_snapshot.proposals.is_empty());
    assert_eq!(result.detector_snapshot.assessment.total_proposal_count, 0);
    assert!(
        result
            .detector_snapshot
            .assessment
            .duplicate_proposal_ids
            .is_empty()
    );
    assert!(result.detector_snapshot.assessment.context_consistent);
    assert_eq!(
        result.detector_snapshot.state,
        DetectorProposalSnapshotState::Frozen
    );
}

#[test]
fn contract_failures_propagate_without_partial_output() {
    let (transcript, canonical_run) = combined_canonical_fixture();
    let run_request = aligned_run_request(&transcript, &canonical_run, InputClass::SelfOwnedReal);
    let adapter_request = adapter_request_for(&canonical_run);

    let mut invalid_runner = run_request.clone();
    invalid_runner.runner_policy_revision = "invalid-policy".to_string();
    assert!(matches!(
        materialize_real_transcript_detector_snapshot(
            &invalid_runner,
            &adapter_request,
            &transcript,
            &canonical_run,
        ),
        Err(RealTranscriptDetectorSnapshotMaterializationError::ContractValidationFailure(_))
    ));

    let (other_transcript, _) = zero_candidate_fixture();
    assert!(matches!(
        materialize_real_transcript_detector_snapshot(
            &run_request,
            &adapter_request,
            &other_transcript,
            &canonical_run,
        ),
        Err(
            RealTranscriptDetectorSnapshotMaterializationError::ContractValidationFailure(
                RealTranscriptDetectorSnapshotAdapterContractError::TranscriptInputIdentityMismatch
            )
        ) | Err(
            RealTranscriptDetectorSnapshotMaterializationError::ContractValidationFailure(
                RealTranscriptDetectorSnapshotAdapterContractError::AnalysisSourceRevisionMismatch
            )
        )
    ));

    let mut identity_mismatch = run_request.clone();
    let mut reordered = identity_mismatch
        .detector_analysis_identity
        .detector_set
        .clone();
    assert!(reordered.len() >= 2);
    reordered.swap(0, 1);
    identity_mismatch.detector_analysis_identity.detector_set = reordered;
    assert!(matches!(
        materialize_real_transcript_detector_snapshot(
            &identity_mismatch,
            &adapter_request,
            &transcript,
            &canonical_run,
        ),
        Err(
            RealTranscriptDetectorSnapshotMaterializationError::ContractValidationFailure(
                RealTranscriptDetectorSnapshotAdapterContractError::AnalysisIdentityMismatch { .. }
            )
        )
    ));

    let mut count_mismatch = adapter_request.clone();
    count_mismatch.proposal_ids.pop();
    assert!(matches!(
        materialize_real_transcript_detector_snapshot(
            &run_request,
            &count_mismatch,
            &transcript,
            &canonical_run,
        ),
        Err(
            RealTranscriptDetectorSnapshotMaterializationError::ContractValidationFailure(
                RealTranscriptDetectorSnapshotAdapterContractError::ProposalIdCountMismatch { .. }
            )
        )
    ));

    let mut duplicate_ids = adapter_request.clone();
    duplicate_ids.proposal_ids[1] = duplicate_ids.proposal_ids[0].clone();
    assert!(matches!(
        materialize_real_transcript_detector_snapshot(
            &run_request,
            &duplicate_ids,
            &transcript,
            &canonical_run,
        ),
        Err(
            RealTranscriptDetectorSnapshotMaterializationError::ContractValidationFailure(
                RealTranscriptDetectorSnapshotAdapterContractError::DuplicateProposalId { .. }
            )
        )
    ));
}

#[test]
fn record_validation_failure_is_wrapped() {
    let (transcript, canonical_run) = combined_canonical_fixture();
    let run_request = aligned_run_request(&transcript, &canonical_run, InputClass::SelfOwnedReal);
    let adapter_request = adapter_request_for(&canonical_run);
    let mut result = materialize_real_transcript_detector_snapshot(
        &run_request,
        &adapter_request,
        &transcript,
        &canonical_run,
    )
    .expect("materialize");
    result.detector_snapshot.proposals[0]
        .semantic_key
        .detector_id = "corrupt".to_string();
    let err = result.detector_snapshot.proposals[0]
        .validate(
            &result.validated_plan.snapshot_revision,
            &result.validated_plan.input_identity,
            &result.validated_plan.detector_analysis_identity,
        )
        .expect_err("semantic mismatch");
    assert!(matches!(
        err,
        vox_proof::detector_snapshot::DetectorProposalRecordValidationError::SemanticKeyMismatch
    ));
}

#[test]
fn repeated_materialization_is_deterministic_and_sources_remain_immutable() {
    let (transcript, canonical_run) = combined_canonical_fixture();
    let run_request = aligned_run_request(&transcript, &canonical_run, InputClass::SelfOwnedReal);
    let adapter_request = adapter_request_for(&canonical_run);

    let before_run = run_request.clone();
    let before_adapter = adapter_request.clone();
    let before_transcript_revision = transcript.revision_id();
    let before_cases: Vec<_> = canonical_run
        .review_cases()
        .iter()
        .map(|case| {
            (
                case.candidate_span().kind(),
                case.candidate_span().anchor().segment_position(),
            )
        })
        .collect();

    let first = materialize_real_transcript_detector_snapshot(
        &run_request,
        &adapter_request,
        &transcript,
        &canonical_run,
    )
    .expect("first");
    let second = materialize_real_transcript_detector_snapshot(
        &run_request,
        &adapter_request,
        &transcript,
        &canonical_run,
    )
    .expect("second");

    assert_eq!(first, second);
    assert_eq!(run_request, before_run);
    assert_eq!(adapter_request, before_adapter);
    assert_eq!(transcript.revision_id(), before_transcript_revision);
    let after_cases: Vec<_> = canonical_run
        .review_cases()
        .iter()
        .map(|case| {
            (
                case.candidate_span().kind(),
                case.candidate_span().anchor().segment_position(),
            )
        })
        .collect();
    assert_eq!(before_cases, after_cases);
}

#[test]
fn production_materialization_source_has_no_forbidden_dependencies() {
    for path in [
        "src/real_transcript_detector_snapshot_adapter.rs",
        "src/real_transcript_detector_snapshot_materialization.rs",
    ] {
        let source = fs::read_to_string(path).expect("read adapter source");
        for forbidden in [
            "run_canonical_term_review(",
            "run_term_review(",
            "detect_glossary_matches(",
            "detect_observed_error_form_matches(",
            "detect_ascii_latin_phonetic_matches(",
            "std::fs",
            "std::path::Path",
            "tokio",
            "SystemTime",
            "UNIX_EPOCH",
            "rand",
            "EvaluationArtifactPacket",
            "EvaluationArtifactPacketFile",
        ] {
            assert!(
                !source.contains(forbidden),
                "forbidden dependency found in {path}: {forbidden}"
            );
        }
    }
}

#[test]
fn zero_candidate_runner_accepts_materialized_snapshot() {
    let (transcript, canonical_run) = zero_candidate_fixture();
    let run_request = single_cue_zero_run_request(&transcript, &canonical_run);
    let adapter_request = adapter_request_for(&canonical_run);
    let materialized = materialize_real_transcript_detector_snapshot(
        &run_request,
        &adapter_request,
        &transcript,
        &canonical_run,
    )
    .expect("materialize");
    let input = execution_input_for_materialized(&run_request, &materialized);
    let outcome =
        execute_real_transcript_evaluation(&run_request, &input).expect("runner accepts snapshot");

    let RealTranscriptEvaluationExecutionOutcome::Completed(result) = outcome else {
        panic!("expected completed outcome for zero-candidate artificial fixture");
    };
    assert_eq!(
        result.completion_stage,
        RealTranscriptEvaluationCompletionStage::DetectorExecution
    );
    assert!(
        result
            .execution_trace
            .iter()
            .any(|stage| stage.stage == RealTranscriptEvaluationStage::DetectorSnapshotValidated)
    );
}

#[test]
fn combined_candidate_runner_requires_human_adjudication_for_overlap() {
    let (transcript, canonical_run) = combined_canonical_fixture();
    let run_request = aligned_run_request(&transcript, &canonical_run, InputClass::SelfOwnedReal);
    let adapter_request = adapter_request_for(&canonical_run);
    let materialized = materialize_real_transcript_detector_snapshot(
        &run_request,
        &adapter_request,
        &transcript,
        &canonical_run,
    )
    .expect("materialize");
    let input = execution_input_for_materialized(&run_request, &materialized);
    let outcome =
        execute_real_transcript_evaluation(&run_request, &input).expect("runner accepts snapshot");

    match &outcome {
        RealTranscriptEvaluationExecutionOutcome::RequiresHumanAdjudication(pending) => {
            assert!(pending.execution_trace.iter().any(
                |stage| stage.stage == RealTranscriptEvaluationStage::DetectorSnapshotValidated
            ));
            assert_eq!(pending.required_human_adjudication.overlap_pairs.len(), 1);
            assert_eq!(
                pending.required_human_adjudication.overlap_pairs[0]
                    .detector_proposal_id
                    .as_str(),
                "det-prop-adapter-002"
            );
        }
        other => panic!("expected pending overlap outcome, got {other:?}"),
    }
}

#[test]
fn unicode_byte_anchors_are_preserved() {
    let transcript = parse_srt(
        "1\n00:00:00,000 --> 00:00:01,000\nPostgre SQL\n\n2\n00:00:01,000 --> 00:00:02,000\nPostgr\u{00e9}s",
    )
    .expect("unicode srt");
    let entries = vec![vox_proof::candidate::SessionTermEntry::new(
        "PostgreSQL",
        vec!["Postgres".to_string()],
        vec!["Postgre SQL".to_string()],
    )];
    let canonical_run = run_canonical_term_review(&transcript, &entries).expect("canonical");
    assert!(!canonical_run.review_cases().is_empty());
    let run_request = aligned_run_request(&transcript, &canonical_run, InputClass::SelfOwnedReal);
    let adapter_request = adapter_request_for(&canonical_run);
    let result = materialize_real_transcript_detector_snapshot(
        &run_request,
        &adapter_request,
        &transcript,
        &canonical_run,
    )
    .expect("materialize");

    let proposal = &result.detector_snapshot.proposals[0];
    let anchor = canonical_run.review_cases()[0].candidate_span().anchor();
    let resolved = transcript.resolve(anchor).expect("resolve");
    assert_eq!(proposal.observed_surface, resolved);
    assert_eq!(
        observed_surface_byte_len(&proposal.observed_surface),
        proposal.source_anchor.end_byte - proposal.source_anchor.start_byte
    );
}
