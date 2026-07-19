use std::collections::{BTreeMap, BTreeSet};

use super::adapter::{AdapterError, PersistenceCandidateAdapter, SemanticOpenMode};
use super::fixture::EvidenceFixture;
use super::model::{
    EvidenceAggregationIssue, EvidenceAggregationIssueCode, EvidenceManifest,
    EvidenceRunEligibility, EvidenceRunResult, EvidenceRunSummary, ScenarioResult, ScenarioStatus,
};
use super::oracle::SemanticOracle;
use super::scenario::{
    REQUIRED_SCENARIO_IDS, ScenarioEvidenceKind, ScenarioIdentity, ScenarioRequirement,
    scenario_catalog,
};

pub const HARNESS_VERSION: &str = "2";

pub struct EvidenceHarness;

impl EvidenceHarness {
    /// Minimal foundation entry point. It exercises only the candidate-neutral
    /// baseline scenario and makes no durability or performance claim.
    pub fn run_baseline(
        adapter: &mut impl PersistenceCandidateAdapter,
        fixture: &EvidenceFixture,
        mut manifest: EvidenceManifest,
    ) -> EvidenceRunResult {
        let scenario = scenario_catalog()
            .into_iter()
            .find(|scenario| scenario.scenario_id == "baseline-create-open-close")
            .expect("baseline scenario must remain catalogued");
        manifest.scenario_ids = vec![format!(
            "{}@{}",
            scenario.scenario_id, scenario.scenario_version
        )];

        let mut limitations = adapter.capabilities().limitations;
        let operation: Result<_, AdapterError> = (|| {
            let session = adapter.create(fixture)?;
            let handle = adapter.open(&session, SemanticOpenMode::Writable)?;
            let actual = adapter.read_normalized_state(&handle)?;
            adapter.close(&handle)?;
            Ok(actual)
        })();

        let result = match operation {
            Ok(actual) => {
                let oracle_result = SemanticOracle::compare(&fixture.normalized_state(), &actual);
                let status = if oracle_result.passed {
                    ScenarioStatus::Passed
                } else {
                    ScenarioStatus::Failed
                };
                ScenarioResult {
                    scenario_identity: scenario,
                    status,
                    oracle_result: Some(oracle_result),
                    measurements: BTreeMap::new(),
                    failure_classification: None,
                    limitations,
                    raw_artifact_references: Vec::new(),
                }
            }
            Err(error) => {
                limitations.push(error.to_string());
                ScenarioResult {
                    scenario_identity: scenario,
                    status: ScenarioStatus::Failed,
                    oracle_result: None,
                    measurements: BTreeMap::new(),
                    failure_classification: None,
                    limitations,
                    raw_artifact_references: Vec::new(),
                }
            }
        };

        Self::aggregate(manifest, vec![result])
    }

    pub fn aggregate(
        manifest: EvidenceManifest,
        scenario_results: Vec<ScenarioResult>,
    ) -> EvidenceRunResult {
        let catalog = scenario_catalog();
        let mut aggregation_issues = validate_catalog(&catalog);
        let catalog_by_id: BTreeMap<_, _> = catalog
            .iter()
            .map(|scenario| (scenario.scenario_id.as_str(), scenario))
            .collect();
        let mut seen_result_identities = BTreeSet::new();
        let mut valid_result_identities = BTreeSet::new();

        for result in &scenario_results {
            let identity = (
                result.scenario_identity.scenario_id.as_str(),
                result.scenario_identity.scenario_version,
            );
            if !seen_result_identities.insert(identity) {
                aggregation_issues.push(aggregation_issue(
                    EvidenceAggregationIssueCode::DuplicateScenarioIdentity,
                    Some(&result.scenario_identity),
                    "duplicate scenario result identity",
                ));
            }

            let Some(canonical) = catalog_by_id.get(result.scenario_identity.scenario_id.as_str())
            else {
                aggregation_issues.push(aggregation_issue(
                    EvidenceAggregationIssueCode::UnknownScenario,
                    Some(&result.scenario_identity),
                    "scenario ID is not present in the canonical catalog",
                ));
                validate_result_status(result, &mut aggregation_issues);
                continue;
            };

            if result.scenario_identity.scenario_version != canonical.scenario_version {
                aggregation_issues.push(aggregation_issue(
                    EvidenceAggregationIssueCode::ScenarioVersionMismatch,
                    Some(&result.scenario_identity),
                    format!(
                        "scenario version {} does not match catalog version {}",
                        result.scenario_identity.scenario_version, canonical.scenario_version
                    ),
                ));
            } else if &result.scenario_identity != *canonical {
                aggregation_issues.push(aggregation_issue(
                    EvidenceAggregationIssueCode::ScenarioDefinitionMismatch,
                    Some(&result.scenario_identity),
                    "scenario metadata does not match the canonical catalog definition",
                ));
            } else {
                valid_result_identities.insert(identity);
            }

            validate_result_status(result, &mut aggregation_issues);
        }

        for required_id in REQUIRED_SCENARIO_IDS {
            let Some(required) = catalog_by_id.get(required_id) else {
                aggregation_issues.push(EvidenceAggregationIssue {
                    code: EvidenceAggregationIssueCode::MissingRequiredScenario,
                    scenario_id: Some((*required_id).to_string()),
                    message: "required scenario definition is missing from the catalog".to_string(),
                });
                continue;
            };
            if !valid_result_identities
                .contains(&(required.scenario_id.as_str(), required.scenario_version))
            {
                aggregation_issues.push(aggregation_issue(
                    EvidenceAggregationIssueCode::MissingRequiredScenario,
                    Some(required),
                    "required canonical scenario result is missing",
                ));
            }
        }

        aggregation_issues.sort_by(|left, right| {
            (&left.code, &left.scenario_id, &left.message).cmp(&(
                &right.code,
                &right.scenario_id,
                &right.message,
            ))
        });
        aggregation_issues.dedup();

        let summary = EvidenceRunSummary {
            passed: count_status(&scenario_results, ScenarioStatus::Passed),
            failed: count_status(&scenario_results, ScenarioStatus::Failed),
            unsupported: count_status(&scenario_results, ScenarioStatus::Unsupported),
            not_run: count_status(&scenario_results, ScenarioStatus::NotRun),
            inconclusive: count_status(&scenario_results, ScenarioStatus::Inconclusive),
        };
        let negative_results = scenario_results
            .iter()
            .filter(|result| result.status != ScenarioStatus::Passed)
            .cloned()
            .collect();

        let required_unsupported = scenario_results.iter().any(|result| {
            catalog_by_id
                .get(result.scenario_identity.scenario_id.as_str())
                .is_some_and(|scenario| {
                    scenario.requirement == ScenarioRequirement::Required
                        && result.status == ScenarioStatus::Unsupported
                })
        });
        let invalid_evidence = aggregation_issues
            .iter()
            .any(|issue| issue.code != EvidenceAggregationIssueCode::MissingRequiredScenario);
        let missing_required_scenario = aggregation_issues
            .iter()
            .any(|issue| issue.code == EvidenceAggregationIssueCode::MissingRequiredScenario);
        let eligibility = if summary.failed > 0 || required_unsupported || invalid_evidence {
            EvidenceRunEligibility::NotEligible
        } else if scenario_results.is_empty()
            || summary.not_run > 0
            || summary.inconclusive > 0
            || missing_required_scenario
        {
            EvidenceRunEligibility::Inconclusive
        } else {
            EvidenceRunEligibility::EligibleForComparison
        };

        EvidenceRunResult {
            manifest,
            scenario_results,
            negative_results,
            aggregation_issues,
            summary,
            eligibility,
        }
    }
}

fn validate_catalog(catalog: &[ScenarioIdentity]) -> Vec<EvidenceAggregationIssue> {
    let mut issues = Vec::new();
    let mut identities = BTreeSet::new();
    let mut ids = BTreeSet::new();
    for scenario in catalog {
        if !identities.insert((scenario.scenario_id.as_str(), scenario.scenario_version))
            || !ids.insert(scenario.scenario_id.as_str())
        {
            issues.push(aggregation_issue(
                EvidenceAggregationIssueCode::DuplicateScenarioIdentity,
                Some(scenario),
                "duplicate canonical scenario identity",
            ));
        }
        if scenario.scenario_version == 0 {
            issues.push(aggregation_issue(
                EvidenceAggregationIssueCode::ScenarioDefinitionMismatch,
                Some(scenario),
                "scenario version must be non-zero",
            ));
        }
        match scenario.requirement {
            ScenarioRequirement::Required if !scenario.required_capabilities.is_empty() => {
                issues.push(aggregation_issue(
                    EvidenceAggregationIssueCode::ScenarioDefinitionMismatch,
                    Some(scenario),
                    "required scenario must not depend on an optional capability",
                ));
            }
            ScenarioRequirement::CapabilityDependent
                if scenario.required_capabilities.is_empty() =>
            {
                issues.push(aggregation_issue(
                    EvidenceAggregationIssueCode::ScenarioDefinitionMismatch,
                    Some(scenario),
                    "capability-dependent scenario must declare a capability",
                ));
            }
            ScenarioRequirement::Required | ScenarioRequirement::CapabilityDependent => {}
        }
    }
    issues
}

fn validate_result_status(result: &ScenarioResult, issues: &mut Vec<EvidenceAggregationIssue>) {
    match result.status {
        ScenarioStatus::Passed
            if result.scenario_identity.evidence_kind
                == ScenarioEvidenceKind::SemanticCorrectness =>
        {
            match &result.oracle_result {
                None => issues.push(aggregation_issue(
                    EvidenceAggregationIssueCode::PassedWithoutOracle,
                    Some(&result.scenario_identity),
                    "semantic correctness scenario passed without an oracle result",
                )),
                Some(oracle) if !oracle.passed => issues.push(aggregation_issue(
                    EvidenceAggregationIssueCode::PassedWithFailingOracle,
                    Some(&result.scenario_identity),
                    "semantic correctness scenario passed with a failing oracle result",
                )),
                Some(_) => {}
            }
        }
        ScenarioStatus::Unsupported
            if !result
                .limitations
                .iter()
                .any(|limitation| !limitation.trim().is_empty()) =>
        {
            issues.push(aggregation_issue(
                EvidenceAggregationIssueCode::UnsupportedWithoutLimitation,
                Some(&result.scenario_identity),
                "unsupported scenario must retain a non-empty limitation",
            ));
        }
        ScenarioStatus::Passed
        | ScenarioStatus::Failed
        | ScenarioStatus::Unsupported
        | ScenarioStatus::NotRun
        | ScenarioStatus::Inconclusive => {}
    }
}

fn aggregation_issue(
    code: EvidenceAggregationIssueCode,
    scenario: Option<&ScenarioIdentity>,
    message: impl Into<String>,
) -> EvidenceAggregationIssue {
    EvidenceAggregationIssue {
        code,
        scenario_id: scenario
            .map(|identity| format!("{}@{}", identity.scenario_id, identity.scenario_version)),
        message: message.into(),
    }
}

fn count_status(results: &[ScenarioResult], expected: ScenarioStatus) -> usize {
    results
        .iter()
        .filter(|result| result.status == expected)
        .count()
}
