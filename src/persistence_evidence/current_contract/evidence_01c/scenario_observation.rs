//! Observed scenario outcome fields — never echo frozen contract expectations.

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
}

impl ScenarioOutcome {
    pub fn passed_with_oracle() -> Self {
        Self {
            status: ScenarioExecutionStatus::Passed,
            oracle_compare: true,
            recovery_class: OBSERVED_RECOVERY_NONE.to_owned(),
            open_state: OBSERVED_OPEN_NORMAL.to_owned(),
            failure_code: None,
            limitations: Vec::new(),
        }
    }

    pub fn passed_interface() -> Self {
        Self {
            status: ScenarioExecutionStatus::Passed,
            oracle_compare: false,
            recovery_class: OBSERVED_RECOVERY_NONE.to_owned(),
            open_state: OBSERVED_OPEN_NORMAL.to_owned(),
            failure_code: None,
            limitations: Vec::new(),
        }
    }

    pub fn passed_with_limitations(limitations: Vec<String>) -> Self {
        Self {
            status: ScenarioExecutionStatus::Passed,
            oracle_compare: false,
            recovery_class: OBSERVED_RECOVERY_NONE.to_owned(),
            open_state: OBSERVED_OPEN_NORMAL.to_owned(),
            failure_code: None,
            limitations,
        }
    }

    pub fn unsupported(limitations: Vec<String>) -> Self {
        Self {
            status: ScenarioExecutionStatus::Unsupported,
            oracle_compare: false,
            recovery_class: OBSERVED_RECOVERY_NONE.to_owned(),
            open_state: OBSERVED_OPEN_NORMAL.to_owned(),
            failure_code: None,
            limitations,
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
}

pub fn corruption_tamper_limitation(candidate: &str, scenario_id: &str, tamper: &str) -> String {
    format!("{candidate}: harness tamper `{tamper}` exercises refusal for `{scenario_id}`")
}
