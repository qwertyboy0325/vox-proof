#![cfg(feature = "persistence-spike")]

use vox_proof::persistence_evidence::{
    EvidenceFixture, EvidenceManifest, KnownOrUnavailable, ORACLE_VERSION, SMALL_FIXTURE_ID,
    SMALL_FIXTURE_VERSION, SQLITE_EVIDENCE_HARNESS_VERSION, SqliteScenarioRunner,
};

#[test]
fn sqlite_scenario_runner_executes_catalog_without_panic() {
    let fixture = EvidenceFixture::small();
    let manifest = EvidenceManifest {
        evidence_protocol_version: "md-015-accepted-v1".to_string(),
        repository_commit: "test".to_string(),
        candidate_id: "embedded-relational-sqlite-spike".to_string(),
        candidate_version: "1".to_string(),
        fixture_id: SMALL_FIXTURE_ID.to_string(),
        fixture_version: SMALL_FIXTURE_VERSION.to_string(),
        harness_version: SQLITE_EVIDENCE_HARNESS_VERSION.to_string(),
        oracle_version: ORACLE_VERSION.to_string(),
        scenario_ids: Vec::new(),
        operating_system: KnownOrUnavailable::Known("test".to_string()),
        operating_system_version: KnownOrUnavailable::Unavailable {
            reason: "test".to_string(),
        },
        filesystem: KnownOrUnavailable::Unavailable {
            reason: "test".to_string(),
        },
        hardware_summary: KnownOrUnavailable::Unavailable {
            reason: "test".to_string(),
        },
        runtime_versions: Default::default(),
        configuration: Default::default(),
        start_timestamp: KnownOrUnavailable::Unavailable {
            reason: "test".to_string(),
        },
        end_timestamp: KnownOrUnavailable::Unavailable {
            reason: "test".to_string(),
        },
        known_limitations: Vec::new(),
    };
    let runner = SqliteScenarioRunner::new();
    let (result, artifacts) = runner.run_catalog(&fixture, manifest);
    assert!(
        result.summary.passed >= 10,
        "expected most catalog scenarios to pass: {:?}",
        result.summary
    );
    assert!(
        !artifacts.fault_executions.is_empty() || !artifacts.oracle_observations.is_empty(),
        "expected evidence artifacts"
    );
}
