//! Independent persisted-state observation for SQLite evidence.

use std::path::PathBuf;

use rusqlite::{Connection, OpenFlags};
use serde::{Deserialize, Serialize};

use super::adapter::AdapterError;
use super::candidates::EmbeddedRelationalAdapter;
use super::canonical_sql_reader;
use super::model::NormalizedSemanticState;
use super::oracle::{OracleResult, SemanticOracle};

pub const INDEPENDENT_ORACLE_VERSION: &str = "direct-sql-v1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WriterOwnershipObservation {
    pub token: Option<String>,
    pub owner_epoch: i64,
    pub lease_expires_at_unix_ms: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AppliedCommandObservation {
    pub command_operation_id: String,
    pub command_kind: String,
    pub outcome_status: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PersistedCountsObservation {
    pub review_ledger_event_count: usize,
    pub analysis_result_count: usize,
    pub review_case_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OracleObservationRecord {
    pub observation_id: String,
    pub locator: String,
    pub oracle_type: String,
    pub session_id: Option<String>,
    pub writer_ownership: Option<WriterOwnershipObservation>,
    pub applied_commands: Vec<AppliedCommandObservation>,
    pub counts: PersistedCountsObservation,
    pub semantic_oracle: Option<OracleResult>,
    pub reopen_performed: bool,
}

pub struct IndependentSqliteOracle;

impl IndependentSqliteOracle {
    fn open_readonly_connection(locator: &str) -> Result<Connection, AdapterError> {
        let db_path = PathBuf::from(locator).join("session.db");
        Connection::open_with_flags(
            db_path,
            OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
        )
        .map_err(|error| AdapterError::new("sqlite-open-failed", error.to_string()))
    }

    fn integrity_check(connection: &Connection) -> Result<(), AdapterError> {
        let result: String = connection
            .query_row("PRAGMA integrity_check", [], |row| row.get(0))
            .map_err(|error| AdapterError::new("integrity-check-failed", error.to_string()))?;
        if result != "ok" {
            return Err(AdapterError::new(
                "integrity-check-failed",
                format!("integrity_check returned {result}"),
            ));
        }
        Ok(())
    }

    pub fn load_state(locator: &str) -> Result<NormalizedSemanticState, AdapterError> {
        let connection = Self::open_readonly_connection(locator)?;
        Self::integrity_check(&connection)?;
        canonical_sql_reader::load_from_connection(&connection)
    }

    pub fn compare_expected(
        locator: &str,
        expected: &NormalizedSemanticState,
        observation_id: &str,
    ) -> Result<OracleObservationRecord, AdapterError> {
        let actual = Self::load_state(locator)?;
        let mut semantic_oracle =
            SemanticOracle::compare(&expected.clone().normalize(), &actual.clone().normalize());
        if let Ok(adapter_state) = EmbeddedRelationalAdapter::load_persisted_state(locator) {
            let adapter_oracle = SemanticOracle::compare(
                &expected.clone().normalize(),
                &adapter_state.clone().normalize(),
            );
            if adapter_oracle.passed != semantic_oracle.passed {
                semantic_oracle
                    .warnings
                    .push(super::oracle::OracleDiagnostic {
                        code: super::oracle::OracleViolationCode::CanonicalFingerprintMismatch,
                        path: "cross-check.adapter".to_string(),
                        message:
                            "direct-sql oracle disagrees with adapter persisted-state cross-check"
                                .to_string(),
                    });
            }
        }
        Self::build_record(
            observation_id,
            locator,
            &actual,
            Some(semantic_oracle),
            true,
            Vec::new(),
        )
    }

    pub fn observe(
        locator: &str,
        observation_id: &str,
        expected: Option<&NormalizedSemanticState>,
        command_ids: &[&str],
    ) -> Result<OracleObservationRecord, AdapterError> {
        let actual = Self::load_state(locator)?;
        let semantic_oracle = expected.map(|expected| {
            SemanticOracle::compare(&expected.clone().normalize(), &actual.clone().normalize())
        });
        let mut applied = Vec::new();
        for id in command_ids {
            if let Some((kind, status)) =
                EmbeddedRelationalAdapter::observe_applied_command(locator, id)?
            {
                applied.push(AppliedCommandObservation {
                    command_operation_id: (*id).to_string(),
                    command_kind: kind,
                    outcome_status: status,
                });
            }
        }
        Self::build_record(
            observation_id,
            locator,
            &actual,
            semantic_oracle,
            true,
            applied,
        )
    }

    fn build_record(
        observation_id: &str,
        locator: &str,
        state: &NormalizedSemanticState,
        semantic_oracle: Option<OracleResult>,
        reopen_performed: bool,
        applied_commands: Vec<AppliedCommandObservation>,
    ) -> Result<OracleObservationRecord, AdapterError> {
        let (token, owner_epoch, lease_expires) =
            EmbeddedRelationalAdapter::observe_writer_ownership(locator)?;
        Ok(OracleObservationRecord {
            observation_id: observation_id.to_string(),
            locator: locator.to_string(),
            oracle_type: INDEPENDENT_ORACLE_VERSION.to_string(),
            session_id: Some(state.session.session_id.clone()),
            writer_ownership: Some(WriterOwnershipObservation {
                token,
                owner_epoch,
                lease_expires_at_unix_ms: lease_expires,
            }),
            applied_commands,
            counts: PersistedCountsObservation {
                review_ledger_event_count: state.review_ledger_events.len(),
                analysis_result_count: state.analysis_results.len(),
                review_case_count: state.review_cases.len(),
            },
            semantic_oracle,
            reopen_performed,
        })
    }
}
