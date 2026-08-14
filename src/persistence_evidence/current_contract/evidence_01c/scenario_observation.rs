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
