#![cfg(feature = "persistence-spike")]

use vox_proof::persistence_evidence::current_contract::evidence_01c::comparison::build_comparison_report;
use vox_proof::persistence_evidence::current_contract::evidence_01c::measurement_ops::execute_measured_operation;
use vox_proof::persistence_evidence::current_contract::evidence_01c::readiness::{
    aggregate_is_valid, assess_readiness,
};
use vox_proof::persistence_evidence::current_contract::evidence_01c::{
    candidate::{
        measurement_transition_states, unique_session_id_for_sample, CurrentContractCandidate,
    },
    runner::{INVALID_PREDECESSOR_EVIDENCE_SHA, INVALID_PREDECESSOR_HARNESS_SHA},
    types::{CandidateRunArtifacts, MeasurementAggregate, MetricAvailability},
};
use vox_proof::persistence_evidence::{
    build_medium_fixture_state, build_promoted_active_state, comparative_measurement_contract,
    methodology_record, validate_measurement_contract, CurrentContractOracle,
    MeasurementFixtureScale, EVIDENCE_01C_HARNESS_VERSION,
};

#[test]
fn medium_fixture_passes_oracle_v3() {
    let state = build_medium_fixture_state();
    let oracle = CurrentContractOracle::validate(&state);
    assert!(
        oracle.passed,
        "medium fixture oracle failures: {:?}",
        oracle.violations
    );
}

#[test]
fn measurement_contract_v2_medium_is_materialized() {
    validate_measurement_contract().expect("measurement contract");
    let contract = comparative_measurement_contract();
    assert_eq!(contract.deferred_scales.len(), 1);
    assert_eq!(
        contract.deferred_scales[0].scale,
        MeasurementFixtureScale::Stress
    );
    for operation in &contract.operations {
        assert!(
            operation
                .fixture_scales
                .contains(&MeasurementFixtureScale::Medium),
            "operation {} missing Medium scale",
            operation.operation
        );
    }
}

#[test]
fn evidence_01c_methodology_is_frozen() {
    let record = methodology_record();
    assert_eq!(record.harness_version, EVIDENCE_01C_HARNESS_VERSION);
    assert_eq!(record.harness_version, "gate4-01c-evidence-v2");
    assert_eq!(record.candidates_order.len(), 2);
    assert!(record.freeze_id.contains("HARNESS-CORRECTION"));
}

#[test]
fn invalid_predecessor_lineage_is_recorded() {
    assert_eq!(
        INVALID_PREDECESSOR_HARNESS_SHA,
        "970b3c38bfbf07fd9750df93b285527b73beb43c"
    );
    assert_eq!(
        INVALID_PREDECESSOR_EVIDENCE_SHA,
        "5b446b430f358d3da821461775cc50efe0943033"
    );
}

#[test]
fn measurement_transition_states_are_distinct_per_operation() {
    let review =
        measurement_transition_states("append_review_decision", MeasurementFixtureScale::Small)
            .expect("review");
    let manual =
        measurement_transition_states("append_manual_replacement", MeasurementFixtureScale::Small)
            .expect("manual");
    let promotion =
        measurement_transition_states("append_reusable_promotion", MeasurementFixtureScale::Small)
            .expect("promotion");
    assert_ne!(review.0.session_id, manual.0.session_id);
    assert_ne!(review.1.session_id, promotion.1.session_id);
}

#[test]
fn isolated_sample_roots_do_not_share_session_storage() {
    let root_a =
        std::env::temp_dir().join(format!("voxproof-01c-isolation-a-{}", std::process::id()));
    let root_b =
        std::env::temp_dir().join(format!("voxproof-01c-isolation-b-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root_a);
    let _ = std::fs::remove_dir_all(&root_b);
    std::fs::create_dir_all(&root_a).expect("root a");
    std::fs::create_dir_all(&root_b).expect("root b");
    let append_a = CurrentContractCandidate::append(&root_a).expect("append a");
    let append_b = CurrentContractCandidate::append(&root_b).expect("append b");
    let state = unique_session_id_for_sample(&build_promoted_active_state(), 0);
    let (session_a, _) = append_a.create_session(&state).expect("create a");
    let (session_b, _) = append_b.create_session(&state).expect("create b");
    assert_ne!(root_a, root_b);
    assert_eq!(session_a, session_b);
    let _ = std::fs::remove_dir_all(&root_a);
    let _ = std::fs::remove_dir_all(&root_b);
}

#[test]
fn failure_count_forces_not_ready() {
    let contract = comparative_measurement_contract();
    let sample_count = contract.operations[0].sample_count;
    let aggregate = MeasurementAggregate {
        operation: contract.operations[0].operation.clone(),
        fixture_scale: MeasurementFixtureScale::Small,
        candidate_id: "append".to_owned(),
        candidate_version: "01B-2".to_owned(),
        platform: "macos_native".to_owned(),
        count: sample_count,
        minimum_ms: 1,
        median_ms: 1,
        p95_ms: 1,
        maximum_ms: 1,
        failure_count: 1,
        peak_memory_bytes: MetricAvailability::available(1024),
        bytes_read: MetricAvailability::unavailable("not observed"),
        bytes_written: MetricAvailability::unavailable("not observed"),
        storage_size_before: MetricAvailability::available(1),
        storage_size_after: MetricAvailability::available(2),
    };
    assert!(!aggregate_is_valid(&aggregate, sample_count));
    let append = CandidateRunArtifacts {
        candidate_id: "append".to_owned(),
        candidate_version: "01B-2".to_owned(),
        scenario_results: Vec::new(),
        measurements: vec![aggregate],
        disqualifications: Vec::new(),
    };
    let sqlite = CandidateRunArtifacts {
        candidate_id: "sqlite".to_owned(),
        candidate_version: "01C-SQLITE-2".to_owned(),
        scenario_results: Vec::new(),
        measurements: Vec::new(),
        disqualifications: Vec::new(),
    };
    let readiness = assess_readiness(&append, &sqlite);
    assert_eq!(readiness.mechanism_comparison_readiness, "not_ready");
}

#[test]
fn missing_peak_memory_forces_not_ready() {
    let contract = comparative_measurement_contract();
    let sample_count = contract.operations[0].sample_count;
    let aggregate = MeasurementAggregate {
        operation: contract.operations[0].operation.clone(),
        fixture_scale: MeasurementFixtureScale::Small,
        candidate_id: "append".to_owned(),
        candidate_version: "01B-2".to_owned(),
        platform: "macos_native".to_owned(),
        count: sample_count,
        minimum_ms: 1,
        median_ms: 1,
        p95_ms: 1,
        maximum_ms: 1,
        failure_count: 0,
        peak_memory_bytes: MetricAvailability::available(0),
        bytes_read: MetricAvailability::unavailable("not observed"),
        bytes_written: MetricAvailability::unavailable("not observed"),
        storage_size_before: MetricAvailability::available(1),
        storage_size_after: MetricAvailability::available(2),
    };
    assert!(!aggregate_is_valid(&aggregate, sample_count));
}

#[test]
fn comparison_ignores_invalid_aggregates() {
    let append = CandidateRunArtifacts {
        candidate_id: "append".to_owned(),
        candidate_version: "01B-2".to_owned(),
        scenario_results: Vec::new(),
        measurements: vec![MeasurementAggregate {
            operation: "create_session".to_owned(),
            fixture_scale: MeasurementFixtureScale::Small,
            candidate_id: "append".to_owned(),
            candidate_version: "01B-2".to_owned(),
            platform: "macos_native".to_owned(),
            count: 5,
            minimum_ms: 1,
            median_ms: 1,
            p95_ms: 1,
            maximum_ms: 1,
            failure_count: 5,
            peak_memory_bytes: MetricAvailability::available(0),
            bytes_read: MetricAvailability::unavailable("not observed"),
            bytes_written: MetricAvailability::unavailable("not observed"),
            storage_size_before: MetricAvailability::available(0),
            storage_size_after: MetricAvailability::available(0),
        }],
        disqualifications: Vec::new(),
    };
    let sqlite = CandidateRunArtifacts {
        candidate_id: "sqlite".to_owned(),
        candidate_version: "01C-SQLITE-2".to_owned(),
        scenario_results: Vec::new(),
        measurements: vec![MeasurementAggregate {
            operation: "create_session".to_owned(),
            fixture_scale: MeasurementFixtureScale::Small,
            candidate_id: "sqlite".to_owned(),
            candidate_version: "01C-SQLITE-2".to_owned(),
            platform: "macos_native".to_owned(),
            count: 5,
            minimum_ms: 10,
            median_ms: 10,
            p95_ms: 10,
            maximum_ms: 10,
            failure_count: 0,
            peak_memory_bytes: MetricAvailability::available(2048),
            bytes_read: MetricAvailability::unavailable("not observed"),
            bytes_written: MetricAvailability::unavailable("not observed"),
            storage_size_before: MetricAvailability::available(1),
            storage_size_after: MetricAvailability::available(2),
        }],
        disqualifications: Vec::new(),
    };
    let comparison = build_comparison_report("macos_native", &append, &sqlite);
    assert!(comparison.tradeoffs.is_empty());
}

#[test]
fn create_session_and_unsupported_compaction_behave_distinctly() {
    let root =
        std::env::temp_dir().join(format!("voxproof-01c-op-dispatch-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).expect("root");
    let candidate = CurrentContractCandidate::append(&root).expect("append");
    let create = execute_measured_operation(
        &candidate,
        "create_session",
        MeasurementFixtureScale::Small,
        0,
    );
    let compaction = execute_measured_operation(
        &candidate,
        "compaction_where_supported",
        MeasurementFixtureScale::Small,
        0,
    );
    assert!(
        !create.failed,
        "create_session should succeed in isolated root"
    );
    assert!(
        !compaction.failed,
        "unsupported compaction must not count as measurement failure"
    );
    assert_ne!(create.elapsed_ms, compaction.elapsed_ms);
    let _ = std::fs::remove_dir_all(&root);
}
