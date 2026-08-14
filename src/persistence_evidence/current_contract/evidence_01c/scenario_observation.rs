//! Observed scenario outcome fields — never echo frozen contract expectations.

use serde::{Deserialize, Serialize};

use super::types::ScenarioExecutionStatus;

pub const OBSERVED_RECOVERY_NONE: &str = "none";
pub const OBSERVED_RECOVERY_LAST_COMMITTED: &str = "last_committed_state";
pub const OBSERVED_RECOVERY_SAFE_AUTOMATIC: &str = "safe_automatic_recovery";
pub const OBSERVED_RECOVERY_MANUAL_REVIEW: &str = "manual_review_required";
pub const OBSERVED_RECOVERY_UNRECOVERABLE: &str = "unrecoverable";
pub const OBSERVED_RECOVERY_UNSUPPORTED_VERSION: &str = "unsupported_version";

pub const OBSERVED_OPEN_NORMAL: &str = "normal";
pub const OBSERVED_OPEN_REFUSED: &str = "open_refused";
pub const OBSERVED_OPEN_UNSUPPORTED_VERSION: &str = "unsupported_version";
pub const OBSERVED_OPEN_UNRECOVERABLE: &str = "unrecoverable";

pub const APPEND_STALE_ASYMMETRY_LIMITATION: &str =
    "append: equivalent generation stale only; sqlite-specific precondition classes not exercised";

pub const APPEND_01B3_STALE_SCOPED_LIMITATION: &str =
    "append-01b-3: scoped stale precondition classes exercised; generation-only stale not used for semantic validity";

/// Persisted FCR-03 stale-rejection observation (scenario-results.json).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Fcr03StaleRejectionRecord {
    pub observed_failure_code: String,
    pub transition_applied: bool,
    pub post_rejection_oracle_compare: bool,
    pub post_rejection_authority_unchanged: bool,
}

/// Persisted FCR-03 unrelated-success observation (scenario-results.json).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Fcr03UnrelatedSuccessRecord {
    pub transition_applied: bool,
    pub post_apply_oracle_compare: bool,
    pub unrelated_scope_preserved: bool,
    pub stale_full_state_not_persisted: bool,
}

/// FCR-03 minimum observation fields for stale scoped-command rejection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Fcr03StaleRejectionObservation {
    pub observed_failure_code: String,
    pub transition_applied: bool,
    pub post_rejection_oracle_compare: bool,
    pub post_rejection_authority_unchanged: bool,
}

/// FCR-03 minimum observation fields for unrelated-scope command success.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Fcr03UnrelatedSuccessObservation {
    pub transition_applied: bool,
    pub post_apply_oracle_compare: bool,
    pub unrelated_scope_preserved: bool,
    pub stale_full_state_not_persisted: bool,
}

impl Fcr03StaleRejectionObservation {
    pub fn record(
        observed_failure_code: impl Into<String>,
        expected: &super::super::model::CurrentContractState,
        actual: &super::super::model::CurrentContractState,
        oracle_compare: bool,
    ) -> Self {
        Self {
            observed_failure_code: observed_failure_code.into(),
            transition_applied: false,
            post_rejection_oracle_compare: oracle_compare,
            post_rejection_authority_unchanged: expected == actual,
        }
    }

    pub fn to_record(&self) -> Fcr03StaleRejectionRecord {
        Fcr03StaleRejectionRecord {
            observed_failure_code: self.observed_failure_code.clone(),
            transition_applied: self.transition_applied,
            post_rejection_oracle_compare: self.post_rejection_oracle_compare,
            post_rejection_authority_unchanged: self.post_rejection_authority_unchanged,
        }
    }
}

impl Fcr03UnrelatedSuccessObservation {
    pub fn record(
        expected: &super::super::model::CurrentContractState,
        actual: &super::super::model::CurrentContractState,
        unrelated_scope_preserved: bool,
    ) -> Self {
        let oracle_compare =
            super::super::oracle::CurrentContractOracle::compare(expected, actual).passed;
        let stale_full_state_not_persisted = unrelated_scope_preserved
            && !would_rewind_unrelated_authority(expected, actual);
        Self {
            transition_applied: true,
            post_apply_oracle_compare: oracle_compare,
            unrelated_scope_preserved,
            stale_full_state_not_persisted,
        }
    }

    pub fn to_record(&self) -> Fcr03UnrelatedSuccessRecord {
        Fcr03UnrelatedSuccessRecord {
            transition_applied: self.transition_applied,
            post_apply_oracle_compare: self.post_apply_oracle_compare,
            unrelated_scope_preserved: self.unrelated_scope_preserved,
            stale_full_state_not_persisted: self.stale_full_state_not_persisted,
        }
    }
}

fn would_rewind_unrelated_authority(
    expected: &super::super::model::CurrentContractState,
    actual: &super::super::model::CurrentContractState,
) -> bool {
    actual.durable_command_tokens.reuse_governance_head
        < expected.durable_command_tokens.reuse_governance_head
        || actual.durable_command_tokens.review_ledger_head
            < expected.durable_command_tokens.review_ledger_head
}

#[derive(Debug, Clone)]
pub struct ScenarioOutcome {
    pub status: ScenarioExecutionStatus,
    pub oracle_compare: bool,
    pub recovery_class: String,
    pub open_state: String,
    pub failure_code: Option<String>,
    pub limitations: Vec<String>,
    pub fcr03_stale_rejection: Option<Fcr03StaleRejectionRecord>,
    pub fcr03_unrelated_success: Option<Fcr03UnrelatedSuccessRecord>,
}

impl ScenarioOutcome {
    fn base_passed(oracle_compare: bool) -> Self {
        Self {
            status: ScenarioExecutionStatus::Passed,
            oracle_compare,
            recovery_class: OBSERVED_RECOVERY_NONE.to_owned(),
            open_state: OBSERVED_OPEN_NORMAL.to_owned(),
            failure_code: None,
            limitations: Vec::new(),
            fcr03_stale_rejection: None,
            fcr03_unrelated_success: None,
        }
    }

    pub fn passed_with_oracle() -> Self {
        Self::base_passed(true)
    }

    pub fn passed_interface() -> Self {
        Self::base_passed(false)
    }

    pub fn passed_with_limitations(limitations: Vec<String>) -> Self {
        Self::base_passed(false).with_limitations(limitations)
    }

    pub fn unsupported(limitations: Vec<String>) -> Self {
        Self {
            status: ScenarioExecutionStatus::Unsupported,
            oracle_compare: false,
            recovery_class: OBSERVED_RECOVERY_NONE.to_owned(),
            open_state: OBSERVED_OPEN_NORMAL.to_owned(),
            failure_code: None,
            limitations,
            fcr03_stale_rejection: None,
            fcr03_unrelated_success: None,
        }
    }

    pub fn failed(code: impl Into<String>) -> Self {
        Self {
            status: ScenarioExecutionStatus::Failed,
            oracle_compare: false,
            recovery_class: OBSERVED_RECOVERY_NONE.to_owned(),
            open_state: OBSERVED_OPEN_NORMAL.to_owned(),
            failure_code: Some(code.into()),
            limitations: Vec::new(),
            fcr03_stale_rejection: None,
            fcr03_unrelated_success: None,
        }
    }

    pub fn fail_closed_refusal(code: impl Into<String>) -> Self {
        Self {
            status: ScenarioExecutionStatus::Passed,
            oracle_compare: false,
            recovery_class: OBSERVED_RECOVERY_MANUAL_REVIEW.to_owned(),
            open_state: OBSERVED_OPEN_UNRECOVERABLE.to_owned(),
            failure_code: Some(code.into()),
            limitations: Vec::new(),
            fcr03_stale_rejection: None,
            fcr03_unrelated_success: None,
        }
    }

    pub fn unrecoverable_refusal(code: impl Into<String>) -> Self {
        Self {
            status: ScenarioExecutionStatus::Passed,
            oracle_compare: false,
            recovery_class: OBSERVED_RECOVERY_UNRECOVERABLE.to_owned(),
            open_state: OBSERVED_OPEN_UNRECOVERABLE.to_owned(),
            failure_code: Some(code.into()),
            limitations: Vec::new(),
            fcr03_stale_rejection: None,
            fcr03_unrelated_success: None,
        }
    }

    pub fn malformed_format_refusal(code: impl Into<String>) -> Self {
        Self {
            status: ScenarioExecutionStatus::Passed,
            oracle_compare: false,
            recovery_class: OBSERVED_RECOVERY_UNRECOVERABLE.to_owned(),
            open_state: OBSERVED_OPEN_UNRECOVERABLE.to_owned(),
            failure_code: Some(code.into()),
            limitations: Vec::new(),
            fcr03_stale_rejection: None,
            fcr03_unrelated_success: None,
        }
    }

    pub fn refused_open(code: impl Into<String>) -> Self {
        Self {
            status: ScenarioExecutionStatus::Passed,
            oracle_compare: false,
            recovery_class: OBSERVED_RECOVERY_UNRECOVERABLE.to_owned(),
            open_state: OBSERVED_OPEN_REFUSED.to_owned(),
            failure_code: Some(code.into()),
            limitations: Vec::new(),
            fcr03_stale_rejection: None,
            fcr03_unrelated_success: None,
        }
    }

    pub fn unsupported_version_open(code: impl Into<String>) -> Self {
        Self {
            status: ScenarioExecutionStatus::Passed,
            oracle_compare: false,
            recovery_class: OBSERVED_RECOVERY_UNSUPPORTED_VERSION.to_owned(),
            open_state: OBSERVED_OPEN_UNSUPPORTED_VERSION.to_owned(),
            failure_code: Some(code.into()),
            limitations: Vec::new(),
            fcr03_stale_rejection: None,
            fcr03_unrelated_success: None,
        }
    }

    pub fn with_limitations(mut self, limitations: Vec<String>) -> Self {
        self.limitations = limitations;
        self
    }

    pub fn with_recovery(mut self, recovery_class: &str) -> Self {
        self.recovery_class = recovery_class.to_owned();
        self
    }

    pub fn with_fcr03_stale_rejection(mut self, observation: Fcr03StaleRejectionObservation) -> Self {
        self.failure_code = Some(observation.observed_failure_code.clone());
        self.fcr03_stale_rejection = Some(observation.to_record());
        self
    }

    pub fn with_fcr03_unrelated_success(mut self, observation: Fcr03UnrelatedSuccessObservation) -> Self {
        self.oracle_compare = observation.post_apply_oracle_compare;
        self.fcr03_unrelated_success = Some(observation.to_record());
        self
    }
}

pub fn corruption_tamper_limitation(candidate: &str, scenario_id: &str, tamper: &str) -> String {
    format!("{candidate}: harness tamper `{tamper}` exercises refusal for `{scenario_id}`")
}
