mod canonical;
mod durable_session;
mod error;
mod hydrate;
mod reuse_canonical;
mod reuse_store;
mod store;

pub use durable_session::DurableApplicationSession;
pub use error::{AuthorityScope, SessionPersistenceError, StaleAuthorityPrecondition};
pub use hydrate::{arm_force_hydrate_failure_for_test, disarm_force_hydrate_failure_for_test};
pub use store::{
    DurableAck, OpenMode, ProductSessionStore, SessionListSummary,
    arm_fail_after_human_raise_before_decision_for_test,
    arm_fail_after_target_before_ledger_for_test, arm_fail_before_commit_for_test,
    disarm_fail_after_human_raise_before_decision_for_test,
    disarm_fail_after_target_before_ledger_for_test, disarm_fail_before_commit_for_test,
};
pub(crate) use store::{load_bound_project_id, load_optional_bound_project_id};

pub(crate) use reuse_canonical::{
    PersistedReuseGovernanceEventV1, persist_governance_event, restore_governance_event,
    restore_project_scope,
};
