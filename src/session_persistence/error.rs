use std::fmt;

use crate::application_service::ApplicationServiceError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuthorityScope {
    ReviewLedger,
    ReuseGovernance,
    ActiveAnalysis,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StaleAuthorityPrecondition {
    pub scope: AuthorityScope,
}

#[derive(Debug, PartialEq, Eq)]
pub enum SessionPersistenceError {
    Io(String),
    Sqlite(String),
    SessionAlreadyExists,
    SessionNotFound,
    UnsupportedFormatVersion { found: u32, supported: u32 },
    CanonicalMismatch(String),
    ReviewCaseVerificationFailed,
    Replay(ApplicationServiceError),
    StaleAuthorityPrecondition(StaleAuthorityPrecondition),
    WriterOwnershipHeld,
    RecoveryRequired,
    SessionNotWritable,
    InvalidSessionId,
    ProjectMemoryUnavailable,
    WritableReuseBlocked,
    ProjectMemory(String),
}

impl fmt::Display for SessionPersistenceError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for SessionPersistenceError {}
