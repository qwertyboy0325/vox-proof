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
fn dual_scoped_evidence_work_package_id_is_r2_bounded_correction() {
    use vox_proof::persistence_evidence::current_contract::evidence_01c::{
        AppendEvidenceVariant, R2_BOUNDED_CORRECTION_WORK_PACKAGE_ID, SqliteEvidenceVariant,
    };

    assert_eq!(
        AppendEvidenceVariant::Scoped01B3.work_package_id(SqliteEvidenceVariant::Scoped01CSqlite3),
        R2_BOUNDED_CORRECTION_WORK_PACKAGE_ID,
    );
    assert_eq!(
        R2_BOUNDED_CORRECTION_WORK_PACKAGE_ID,
        "VP-GATE4-FCR03-SOL-R2-BOUNDED-CORRECTION-01",
    );
}

#[test]
fn dual_scoped_methodology_work_package_id_matches_package_resolution() {
    use vox_proof::persistence_evidence::current_contract::evidence_01c::{
        AppendEvidenceVariant, methodology_record_for_work_package, SqliteEvidenceVariant,
    };

    let work_package_id =
        AppendEvidenceVariant::Scoped01B3.work_package_id(SqliteEvidenceVariant::Scoped01CSqlite3);
    let methodology = methodology_record_for_work_package(work_package_id);
    assert_eq!(methodology.work_package_id, work_package_id);
}

#[test]
fn append_01b3_only_evidence_work_package_id_remains_historical() {
    use vox_proof::persistence_evidence::current_contract::evidence_01c::{
        AppendEvidenceVariant, SqliteEvidenceVariant, WORK_PACKAGE_ID,
    };

    assert_eq!(
        AppendEvidenceVariant::Scoped01B3.work_package_id(SqliteEvidenceVariant::Historical01CSqlite2),
        "VP-GATE4-APPEND-01B-3-EVIDENCE-EXECUTION-01",
    );
    assert_eq!(
        AppendEvidenceVariant::Historical01B2.work_package_id(SqliteEvidenceVariant::Historical01CSqlite2),
        WORK_PACKAGE_ID,
    );
}

#[test]
fn evidence_01c_methodology_is_frozen() {
    let record = methodology_record();
    assert_eq!(record.harness_version, EVIDENCE_01C_HARNESS_VERSION);
    assert_eq!(record.harness_version, "gate4-01c-evidence-v6");
    assert_eq!(record.candidates_order.len(), 2);
    assert!(record.freeze_id.contains("READINESS-SEPARATION"));
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
fn measurement_transition_pairs_share_session_id() {
    use vox_proof::persistence_evidence::MeasurementFixtureScale;
    for operation in [
        "append_review_decision",
        "append_manual_replacement",
        "append_reusable_promotion",
        "append_reusable_revocation",
        "append_reusable_supersession",
    ] {
        let (precursor, target) =
            measurement_transition_states(operation, MeasurementFixtureScale::Small)
                .unwrap_or_else(|| panic!("{operation}"));
        assert_eq!(precursor.session_id, target.session_id);
        assert_ne!(
            precursor.canonical_projection(),
            target.canonical_projection()
        );
    }
}

#[test]
fn append_stale_precondition_uses_generation_boundary() {
    use vox_proof::persistence_evidence::current_contract::evidence_01c::scenarios::{
        stale_preconditions_for_test, expected_stale_code_for_test,
    };
    use vox_proof::persistence_evidence::current_contract::CurrentContractPreconditions;
    use vox_proof::persistence_evidence::current_contract::evidence_01c::CurrentContractCandidateKind;

    let baseline = CurrentContractPreconditions {
        expected_generation: 3,
        review_ledger_head: 2,
        reuse_governance_head: 1,
        active_analysis_snapshot_identity: "analysis:live".to_owned(),
    };
    let stale = stale_preconditions_for_test(
        CurrentContractCandidateKind::Append,
        "stale-review-ledger-command",
        &baseline,
    );
    assert_eq!(stale.expected_generation, 2);
    assert_eq!(stale.review_ledger_head, baseline.review_ledger_head);
    assert_eq!(
        expected_stale_code_for_test(CurrentContractCandidateKind::Append, "stale-review-ledger-command"),
        "stale-append-precondition"
    );
}

#[test]
fn sqlite_stale_precondition_uses_ledger_identity() {
    use vox_proof::persistence_evidence::current_contract::evidence_01c::scenarios::{
        stale_preconditions_for_test, expected_stale_code_for_test,
    };
    use vox_proof::persistence_evidence::current_contract::CurrentContractPreconditions;
    use vox_proof::persistence_evidence::current_contract::evidence_01c::CurrentContractCandidateKind;

    let baseline = CurrentContractPreconditions {
        expected_generation: 3,
        review_ledger_head: 2,
        reuse_governance_head: 1,
        active_analysis_snapshot_identity: "analysis:live".to_owned(),
    };
    let stale = stale_preconditions_for_test(
        CurrentContractCandidateKind::Sqlite,
        "stale-review-ledger-command",
        &baseline,
    );
    assert_eq!(stale.review_ledger_head, 3);
    assert_eq!(
        expected_stale_code_for_test(
            CurrentContractCandidateKind::Sqlite,
            "stale-review-ledger-command"
        ),
        "stale-review-ledger-precondition"
    );
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
fn measurement_samples_mark_storage_before_unavailable() {
    let root =
        std::env::temp_dir().join(format!("voxproof-01c-storage-before-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).expect("root");
    let candidate = CurrentContractCandidate::append(&root).expect("append");
    let sample = execute_measured_operation(
        &candidate,
        "create_session",
        MeasurementFixtureScale::Small,
        0,
    );
    assert!(sample.storage_size_before.is_none());
    assert!(sample.storage_size_after > 0);
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn append_stale_asymmetry_downgrades_eligibility() {
    use vox_proof::persistence_evidence::current_contract::evidence_01c::readiness::compute_eligibility;
    use vox_proof::persistence_evidence::current_contract::evidence_01c::scenario_observation::APPEND_STALE_ASYMMETRY_LIMITATION;
    use vox_proof::persistence_evidence::current_contract::evidence_01c::types::{
        NormalizedScenarioResult, ScenarioExecutionStatus,
    };
    use vox_proof::persistence_evidence::current_contract::candidate_equivalence::CandidateEligibilityStatus;
    use vox_proof::persistence_evidence::scenario_contract_v4;

    let mut scenario_results: Vec<NormalizedScenarioResult> = scenario_contract_v4()
        .iter()
        .map(|scenario| NormalizedScenarioResult {
            scenario_id: scenario.scenario_id.clone(),
            scenario_version: scenario.scenario_version,
            candidate_id: "current-contract-append-authoritative-candidate".to_owned(),
            candidate_version: "01B-2".to_owned(),
            platform: "macos_native".to_owned(),
            fixture_scale: "small".to_owned(),
            status: ScenarioExecutionStatus::Passed,
            oracle_compare: false,
            recovery_class: "none".to_owned(),
            open_state: "normal".to_owned(),
            failure_code: None,
            elapsed_ms: 1,
            correctness_disqualification: None,
            limitations: Vec::new(),
            fcr03_stale_rejection: None,
            fcr03_unrelated_success: None,
        })
        .collect();
    let stale = scenario_results
        .iter_mut()
        .find(|row| row.scenario_id == "stale-review-ledger-command")
        .expect("stale scenario");
    stale.limitations = vec![APPEND_STALE_ASYMMETRY_LIMITATION.to_owned()];

    let artifacts = CandidateRunArtifacts {
        candidate_id: "current-contract-append-authoritative-candidate".to_owned(),
        candidate_version: "01B-2".to_owned(),
        scenario_results,
        measurements: Vec::new(),
        disqualifications: Vec::new(),
    };
    let eligibility = compute_eligibility(&artifacts);
    assert_eq!(
        eligibility.status,
        CandidateEligibilityStatus::ImplementationNotYetEvaluated
    );
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

#[test]
fn scoped_candidates_persist_fcr03_observations_in_harness() {
    use vox_proof::persistence_evidence::current_contract::evidence_01c::scenarios::run_fcr03_observation_scenarios;

    let root = std::env::temp_dir().join(format!(
        "voxproof-01c-fcr03-harness-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&root);
    for (label, candidate) in [
        (
            "append-01b3",
            CurrentContractCandidate::append_01b3(root.join("append-01b3"))
                .expect("append 01b3"),
        ),
        (
            "sqlite-01c3",
            CurrentContractCandidate::sqlite_01c3(root.join("sqlite-01c3"))
                .expect("sqlite 01c3"),
        ),
    ] {
        let results =
            run_fcr03_observation_scenarios(&candidate, "macos_native", candidate.storage_root());
        for scenario_id in [
            "stale-review-ledger-command",
            "stale-reuse-governance-command",
            "stale-analysis-attachment-or-selection",
        ] {
            let row = results
                .iter()
                .find(|row| row.scenario_id == scenario_id)
                .unwrap_or_else(|| panic!("{label} missing {scenario_id}"));
            assert_eq!(
                row.status,
                vox_proof::persistence_evidence::current_contract::evidence_01c::types::ScenarioExecutionStatus::Passed,
                "{label} {scenario_id} status"
            );
            let observation = row
                .fcr03_stale_rejection
                .as_ref()
                .unwrap_or_else(|| panic!("{label} {scenario_id} missing fcr03_stale_rejection"));
            assert!(!observation.transition_applied);
            assert!(observation.post_rejection_oracle_compare);
            assert!(observation.post_rejection_authority_unchanged);
            assert!(observation.persist_reopen_oracle_compare);
            assert!(observation.persist_reopen_authority_unchanged);
            assert_eq!(
                row.failure_code.as_deref(),
                Some(observation.observed_failure_code.as_str())
            );
        }
        let u1 = results
            .iter()
            .find(|row| row.scenario_id == "unrelated-scope-review-after-reuse-advance")
            .expect("u1 scenario");
        let observation = u1
            .fcr03_unrelated_success
            .as_ref()
            .expect("u1 fcr03_unrelated_success");
        assert!(observation.transition_applied);
        assert!(observation.post_apply_oracle_compare);
        assert!(observation.unrelated_scope_preserved);
        assert!(observation.stale_full_state_not_persisted);
        assert!(observation.persist_reopen_oracle_compare);
        assert!(observation.persist_reopen_authority_unchanged);
        assert!(observation.no_unrelated_scope_rewind_after_close_reopen);
    }
    let _ = std::fs::remove_dir_all(&root);
}
