//! Direct SQL assembly for independent oracle observation (no derived-cache rebuild).

use rusqlite::{Connection, OptionalExtension};

use super::adapter::AdapterError;
use super::model::NormalizedSemanticState;
use super::oracle::SemanticOracle;

fn has_legacy_blob_schema(connection: &Connection) -> bool {
    connection
        .query_row(
            "SELECT 1 FROM sqlite_master WHERE type='table' AND name='canonical_state'",
            [],
            |_| Ok(()),
        )
        .is_ok()
}

fn read_meta(connection: &Connection, key: &str) -> Result<Option<String>, AdapterError> {
    connection
        .query_row(
            "SELECT value FROM session_meta WHERE key = ?1",
            [key],
            |row| row.get(0),
        )
        .optional()
        .map_err(|error| AdapterError::new("sqlite-read-meta", error.to_string()))
}

pub fn load_from_connection(
    connection: &Connection,
) -> Result<NormalizedSemanticState, AdapterError> {
    if has_legacy_blob_schema(connection) {
        return Err(AdapterError::new(
            "unsupported-older-format",
            "legacy blob schema cannot be loaded as relational authority",
        ));
    }

    let session_id = read_meta(connection, "session_id")?
        .ok_or_else(|| AdapterError::new("canonical-corruption", "missing session_id"))?;
    let duplicated_from = read_meta(connection, "duplicated_from")?;
    let session_format_version =
        read_meta(connection, "session_format_version")?.ok_or_else(|| {
            AdapterError::new("canonical-corruption", "missing session_format_version")
        })?;
    let interpretation_version =
        read_meta(connection, "interpretation_version")?.ok_or_else(|| {
            AdapterError::new("canonical-corruption", "missing interpretation_version")
        })?;

    let mut source_revisions = Vec::new();
    let mut revision_rows = connection
        .prepare("SELECT revision_id, payload_json FROM source_revisions ORDER BY revision_id ASC")
        .map_err(|e| AdapterError::new("sqlite-load-state", e.to_string()))?;
    for row in revision_rows
        .query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })
        .map_err(|e| AdapterError::new("sqlite-load-state", e.to_string()))?
    {
        let (row_revision_id, json) =
            row.map_err(|e| AdapterError::new("sqlite-load-state", e.to_string()))?;
        let revision: super::model::NormalizedSourceRevision = serde_json::from_str(&json)
            .map_err(|e| AdapterError::new("canonical-corruption", e.to_string()))?;
        if revision.revision_id != row_revision_id {
            return Err(AdapterError::new(
                "canonical-corruption",
                "source_revisions key does not match payload revision_id",
            ));
        }
        source_revisions.push(revision);
    }

    let mut review_cases = Vec::new();
    let mut case_rows = connection
        .prepare("SELECT case_id, payload_json FROM review_cases ORDER BY case_id ASC")
        .map_err(|e| AdapterError::new("sqlite-load-state", e.to_string()))?;
    for row in case_rows
        .query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })
        .map_err(|e| AdapterError::new("sqlite-load-state", e.to_string()))?
    {
        let (row_case_id, json) =
            row.map_err(|e| AdapterError::new("sqlite-load-state", e.to_string()))?;
        let review_case: super::model::NormalizedReviewCase = serde_json::from_str(&json)
            .map_err(|e| AdapterError::new("canonical-corruption", e.to_string()))?;
        if review_case.case_id != row_case_id {
            return Err(AdapterError::new(
                "canonical-corruption",
                "review_cases key does not match payload case_id",
            ));
        }
        review_cases.push(review_case);
    }

    let mut review_case_raised_events = Vec::new();
    let mut raised_rows = connection
            .prepare(
                "SELECT event_id, sequence, payload_json FROM review_case_raised_events ORDER BY sequence ASC",
            )
            .map_err(|e| AdapterError::new("sqlite-load-state", e.to_string()))?;
    for row in raised_rows
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, i64>(1)?,
                row.get::<_, String>(2)?,
            ))
        })
        .map_err(|e| AdapterError::new("sqlite-load-state", e.to_string()))?
    {
        let (row_event_id, row_sequence, json) =
            row.map_err(|e| AdapterError::new("sqlite-load-state", e.to_string()))?;
        let event: super::model::ReviewCaseRaisedEventState = serde_json::from_str(&json)
            .map_err(|e| AdapterError::new("canonical-corruption", e.to_string()))?;
        if event.event_id != row_event_id {
            return Err(AdapterError::new(
                "canonical-corruption",
                "review_case_raised_events key does not match payload event_id",
            ));
        }
        if event.sequence != row_sequence as u64 {
            return Err(AdapterError::new(
                "canonical-corruption",
                "review_case_raised_events sequence does not match payload sequence",
            ));
        }
        review_case_raised_events.push(event);
    }

    let mut review_ledger_events = Vec::new();
    let mut ledger_rows = connection
            .prepare(
                "SELECT event_id, sequence, payload_json FROM review_ledger_events ORDER BY sequence ASC",
            )
            .map_err(|e| AdapterError::new("sqlite-load-state", e.to_string()))?;
    for row in ledger_rows
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, i64>(1)?,
                row.get::<_, String>(2)?,
            ))
        })
        .map_err(|e| AdapterError::new("sqlite-load-state", e.to_string()))?
    {
        let (row_event_id, row_sequence, json) =
            row.map_err(|e| AdapterError::new("sqlite-load-state", e.to_string()))?;
        let event: super::model::ReviewLedgerEventState = serde_json::from_str(&json)
            .map_err(|e| AdapterError::new("canonical-corruption", e.to_string()))?;
        if event.event_id != row_event_id {
            return Err(AdapterError::new(
                "canonical-corruption",
                "review_ledger_events key does not match payload event_id",
            ));
        }
        if event.sequence != row_sequence as u64 {
            return Err(AdapterError::new(
                "canonical-corruption",
                "review_ledger_events sequence does not match payload sequence",
            ));
        }
        review_ledger_events.push(event);
    }

    let mut analysis_results = Vec::new();
    let mut analysis_rows = connection
            .prepare(
                "SELECT analysis_result_id, payload_json FROM analysis_results ORDER BY analysis_result_id ASC",
            )
            .map_err(|e| AdapterError::new("sqlite-load-state", e.to_string()))?;
    for row in analysis_rows
        .query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })
        .map_err(|e| AdapterError::new("sqlite-load-state", e.to_string()))?
    {
        let (row_analysis_result_id, json) =
            row.map_err(|e| AdapterError::new("sqlite-load-state", e.to_string()))?;
        let analysis_result: super::model::AnalysisResultState = serde_json::from_str(&json)
            .map_err(|e| AdapterError::new("canonical-corruption", e.to_string()))?;
        if analysis_result.analysis_result_id != row_analysis_result_id {
            return Err(AdapterError::new(
                "canonical-corruption",
                "analysis_results key does not match payload analysis_result_id",
            ));
        }
        analysis_results.push(analysis_result);
    }

    let active_analysis_selection = connection
        .query_row(
            "SELECT selection_json FROM active_analysis_selection WHERE id = 1",
            [],
            |row| row.get::<_, String>(0),
        )
        .optional()
        .map_err(|e| AdapterError::new("sqlite-load-state", e.to_string()))?
        .map(|json| {
            serde_json::from_str(&json)
                .map_err(|e| AdapterError::new("canonical-corruption", e.to_string()))
        })
        .transpose()?;

    let mut knowledge_snapshot_references = Vec::new();
    let mut ks_rows = connection
            .prepare("SELECT knowledge_snapshot_id, payload_json FROM knowledge_snapshot_references ORDER BY knowledge_snapshot_id ASC")
            .map_err(|e| AdapterError::new("sqlite-load-state", e.to_string()))?;
    for row in ks_rows
        .query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })
        .map_err(|e| AdapterError::new("sqlite-load-state", e.to_string()))?
    {
        let (row_knowledge_snapshot_id, json) =
            row.map_err(|e| AdapterError::new("sqlite-load-state", e.to_string()))?;
        let reference: super::model::KnowledgeSnapshotReference = serde_json::from_str(&json)
            .map_err(|e| AdapterError::new("canonical-corruption", e.to_string()))?;
        if reference.knowledge_snapshot_id != row_knowledge_snapshot_id {
            return Err(AdapterError::new(
                "canonical-corruption",
                "knowledge_snapshot_references key does not match payload knowledge_snapshot_id",
            ));
        }
        knowledge_snapshot_references.push(reference);
    }

    let mut lineage_conflicts = Vec::new();
    let mut conflict_rows = connection
        .prepare("SELECT conflict_id, payload_json FROM lineage_conflicts ORDER BY conflict_id ASC")
        .map_err(|e| AdapterError::new("sqlite-load-state", e.to_string()))?;
    for row in conflict_rows
        .query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })
        .map_err(|e| AdapterError::new("sqlite-load-state", e.to_string()))?
    {
        let (row_conflict_id, json) =
            row.map_err(|e| AdapterError::new("sqlite-load-state", e.to_string()))?;
        let conflict: super::model::LineageConflict = serde_json::from_str(&json)
            .map_err(|e| AdapterError::new("canonical-corruption", e.to_string()))?;
        if conflict.conflict_id != row_conflict_id {
            return Err(AdapterError::new(
                "canonical-corruption",
                "lineage_conflicts key does not match payload conflict_id",
            ));
        }
        lineage_conflicts.push(conflict);
    }

    let mut artifacts = Vec::new();
    let mut artifact_rows = connection
            .prepare("SELECT artifact_id, retention_class, payload_json FROM artifacts ORDER BY artifact_id ASC")
            .map_err(|e| AdapterError::new("sqlite-load-state", e.to_string()))?;
    for row in artifact_rows
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
            ))
        })
        .map_err(|e| AdapterError::new("sqlite-load-state", e.to_string()))?
    {
        let (row_artifact_id, row_retention_class, json) =
            row.map_err(|e| AdapterError::new("sqlite-load-state", e.to_string()))?;
        let artifact: super::model::ArtifactState = serde_json::from_str(&json)
            .map_err(|e| AdapterError::new("canonical-corruption", e.to_string()))?;
        if artifact.artifact_id != row_artifact_id {
            return Err(AdapterError::new(
                "canonical-corruption",
                "artifacts key does not match payload artifact_id",
            ));
        }
        if format!("{:?}", artifact.class) != row_retention_class {
            return Err(AdapterError::new(
                "canonical-corruption",
                "artifacts retention_class does not match payload class",
            ));
        }
        artifacts.push(artifact);
    }

    let mut retention_references = Vec::new();
    let mut retention_rows = connection
        .prepare(
            "SELECT reference_id, payload_json FROM retention_references ORDER BY reference_id ASC",
        )
        .map_err(|e| AdapterError::new("sqlite-load-state", e.to_string()))?;
    for row in retention_rows
        .query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })
        .map_err(|e| AdapterError::new("sqlite-load-state", e.to_string()))?
    {
        let (row_reference_id, json) =
            row.map_err(|e| AdapterError::new("sqlite-load-state", e.to_string()))?;
        let reference: super::model::RetentionReference = serde_json::from_str(&json)
            .map_err(|e| AdapterError::new("canonical-corruption", e.to_string()))?;
        let expected_reference_id = format!(
            "{}:{}:{:?}",
            reference.root_id, reference.artifact_id, reference.relation
        );
        if row_reference_id != expected_reference_id {
            return Err(AdapterError::new(
                "canonical-corruption",
                "retention_references key does not match payload identity",
            ));
        }
        retention_references.push(reference);
    }

    Ok({
        let normalized = NormalizedSemanticState {
            session: super::model::SessionIdentityState {
                session_id,
                duplicated_from_session_id: duplicated_from.filter(|value| !value.is_empty()),
            },
            session_format_version,
            interpretation_version,
            source_revisions,
            review_cases,
            review_case_raised_events,
            review_ledger_events,
            analysis_results,
            active_analysis_selection,
            knowledge_snapshot_references,
            lineage_conflicts,
            artifacts,
            retention_references,
        }
        .normalize();
        let validation = SemanticOracle::validate(&normalized);
        if !validation.passed {
            let detail = validation
                .violations
                .first()
                .map(|violation| violation.message.clone())
                .unwrap_or_else(|| "canonical semantic validation failed".to_string());
            return Err(AdapterError::new("canonical-corruption", detail));
        }
        normalized
    })
}
