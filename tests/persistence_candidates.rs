#![cfg(feature = "persistence-spike")]

use std::collections::BTreeMap;

use vox_proof::persistence_evidence::{
    AppendBundleAdapter, EmbeddedRelationalAdapter, EvidenceFixture, EvidenceManifest,
    EvidenceRunEligibility, HARNESS_VERSION, KnownOrUnavailable, ORACLE_VERSION,
    PersistenceCandidateAdapter, SMALL_FIXTURE_ID, SMALL_FIXTURE_VERSION, ScenarioRunner,
    SemanticOracle, fresh_storage_root,
};

fn manifest(candidate_id: &str) -> EvidenceManifest {
    EvidenceManifest {
        evidence_protocol_version: "md-015-accepted-v1".to_string(),
        repository_commit: "test-commit".to_string(),
        candidate_id: candidate_id.to_string(),
        candidate_version: "1".to_string(),
        fixture_id: SMALL_FIXTURE_ID.to_string(),
        fixture_version: SMALL_FIXTURE_VERSION.to_string(),
        harness_version: HARNESS_VERSION.to_string(),
        oracle_version: ORACLE_VERSION.to_string(),
        scenario_ids: Vec::new(),
        operating_system: KnownOrUnavailable::Known(std::env::consts::OS.to_string()),
        operating_system_version: KnownOrUnavailable::Unavailable {
            reason: "unit test".to_string(),
        },
        filesystem: KnownOrUnavailable::Unavailable {
            reason: "unit test".to_string(),
        },
        hardware_summary: KnownOrUnavailable::Unavailable {
            reason: "unit test".to_string(),
        },
        runtime_versions: BTreeMap::new(),
        configuration: BTreeMap::new(),
        start_timestamp: KnownOrUnavailable::Unavailable {
            reason: "unit test".to_string(),
        },
        end_timestamp: KnownOrUnavailable::Unavailable {
            reason: "unit test".to_string(),
        },
        known_limitations: vec!["integration test host".to_string()],
    }
}

#[test]
fn embedded_candidate_passes_required_scenario_catalog() {
    let fixture = EvidenceFixture::small();
    let mut adapter = EmbeddedRelationalAdapter::new(fresh_storage_root("embedded-test"));
    let run = ScenarioRunner::run_catalog(
        &mut adapter,
        &fixture,
        manifest("embedded-relational-sqlite-spike"),
    );
    assert!(
        run.summary.failed == 0,
        "embedded failures: {:?}",
        run.negative_results
    );
    assert_ne!(run.eligibility, EvidenceRunEligibility::NotEligible);
}

#[test]
fn append_candidate_passes_required_scenario_catalog() {
    let fixture = EvidenceFixture::small();
    let mut adapter = AppendBundleAdapter::new(fresh_storage_root("append-test"));
    let run =
        ScenarioRunner::run_catalog(&mut adapter, &fixture, manifest("append-bundle-log-spike"));
    assert!(
        run.summary.failed == 0,
        "append failures: {:?}",
        run.negative_results
    );
    assert_ne!(run.eligibility, EvidenceRunEligibility::NotEligible);
}

#[test]
fn candidates_remain_semantically_equivalent_on_fixture_baseline() {
    let fixture = EvidenceFixture::small();
    let mut embedded = EmbeddedRelationalAdapter::new(fresh_storage_root("embedded-baseline"));
    let mut append = AppendBundleAdapter::new(fresh_storage_root("append-baseline"));
    let embedded_session = embedded.create(&fixture).expect("embedded create");
    let append_session = append.create(&fixture).expect("append create");
    let embedded_handle = embedded
        .open(
            &embedded_session,
            vox_proof::persistence_evidence::SemanticOpenMode::ReadOnly,
        )
        .expect("embedded open");
    let append_handle = append
        .open(
            &append_session,
            vox_proof::persistence_evidence::SemanticOpenMode::ReadOnly,
        )
        .expect("append open");
    let embedded_state = embedded
        .read_normalized_state(&embedded_handle)
        .expect("embedded state");
    let append_state = append
        .read_normalized_state(&append_handle)
        .expect("append state");
    assert!(SemanticOracle::compare(&fixture.normalized_state(), &embedded_state).passed);
    assert!(SemanticOracle::compare(&fixture.normalized_state(), &append_state).passed);
}

#[test]
fn evidence_manifest_is_deterministic_for_same_fixture() {
    let left = manifest("embedded-relational-sqlite-spike");
    let right = manifest("embedded-relational-sqlite-spike");
    assert_eq!(
        serde_json::to_vec(&left).expect("serialize"),
        serde_json::to_vec(&right).expect("serialize")
    );
}

#[test]
fn stale_write_rejection_is_shared_across_candidates() {
    let fixture = EvidenceFixture::small();
    let mut embedded = EmbeddedRelationalAdapter::new(fresh_storage_root("embedded-stale"));
    let embedded_run = ScenarioRunner::run_catalog(
        &mut embedded,
        &fixture,
        manifest("embedded-relational-sqlite-spike"),
    );
    assert_eq!(
        embedded_run.summary.failed, 0,
        "embedded stale scenario failed"
    );

    let mut append = AppendBundleAdapter::new(fresh_storage_root("append-stale"));
    let append_run =
        ScenarioRunner::run_catalog(&mut append, &fixture, manifest("append-bundle-log-spike"));
    assert_eq!(append_run.summary.failed, 0, "append stale scenario failed");
}
