use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use rusqlite::{Connection, OpenFlags, TransactionBehavior, params};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::project_memory::error::{ProjectMemoryError, ProjectMemoryOpenMode};
use crate::project_memory::identity::{
    PROJECT_MEMORY_FORMAT_VERSION, PROJECT_MEMORY_FORMAT_VERSION_V2,
    PROJECT_MEMORY_FORMAT_VERSION_V3, ProjectMemoryRecord, ProjectMemorySnapshotIdentity,
    compute_project_memory_snapshot_identity, is_supported_project_memory_format_version,
    record_has_explicit_allowed_effects, record_is_human_raised,
    required_project_memory_format_version,
};
use crate::reusable_influence::ReusableGovernanceEvent;
use crate::reuse_primitives::{ProjectScope, ProjectScopeDisplayName, ProjectScopeId};
use crate::session_persistence::{
    PersistedReuseGovernanceEventV1, persist_governance_event, restore_governance_event,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct PersistedProjectMemoryEventV1 {
    source_session_id: String,
    event: PersistedReuseGovernanceEventV1,
}

pub struct ProductProjectMemoryStore {
    root: PathBuf,
}

pub struct DurableProjectMemory {
    project_id: ProjectScopeId,
    display_name: ProjectScopeDisplayName,
    format_version: u32,
    opened: OpenedProject,
    records: Vec<ProjectMemoryRecord>,
}

struct OpenedProject {
    _project_id: String,
    _db_path: PathBuf,
    connection: Connection,
    mode: ProjectMemoryOpenMode,
    writer_token: Option<String>,
}

impl ProductProjectMemoryStore {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn database_path(&self, project_id: &str) -> PathBuf {
        self.root
            .join("projects")
            .join(project_id)
            .join("project.db")
    }

    pub fn create(
        &self,
        display_name: impl Into<String>,
    ) -> Result<DurableProjectMemory, ProjectMemoryError> {
        let display_name = ProjectScopeDisplayName::new(display_name.into())
            .map_err(|_| ProjectMemoryError::InvalidDisplayName)?;
        let project_id = ProjectScopeId::new(Uuid::new_v4().to_string())
            .map_err(|_| ProjectMemoryError::InvalidProjectId)?;
        let db_path = self.database_path(project_id.as_str());
        if db_path.exists() {
            return Err(ProjectMemoryError::ProjectAlreadyExists);
        }
        fs::create_dir_all(db_path.parent().expect("project parent"))
            .map_err(|error| ProjectMemoryError::Io(error.to_string()))?;
        let mut connection = Connection::open(&db_path)
            .map_err(|error| ProjectMemoryError::Sqlite(error.to_string()))?;
        configure_connection(&connection)?;
        initialize_schema(&connection)?;
        let writer_token = Uuid::new_v4().to_string();
        let now = unix_ms()?;
        let tx = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|error| ProjectMemoryError::Sqlite(error.to_string()))?;
        tx.execute(
            "INSERT INTO project_metadata (project_id, format_version, display_name, created_at_unix_ms) VALUES (?1, ?2, ?3, ?4)",
            params![
                project_id.as_str(),
                PROJECT_MEMORY_FORMAT_VERSION,
                display_name.as_str(),
                now
            ],
        )
        .map_err(|error| ProjectMemoryError::Sqlite(error.to_string()))?;
        tx.execute(
            "INSERT INTO command_tokens (project_id, governance_head) VALUES (?1, 0)",
            [project_id.as_str()],
        )
        .map_err(|error| ProjectMemoryError::Sqlite(error.to_string()))?;
        tx.execute(
            "INSERT INTO authority_transitions (project_id, generation, acknowledgement_status) VALUES (?1, 1, 'committed')",
            [project_id.as_str()],
        )
        .map_err(|error| ProjectMemoryError::Sqlite(error.to_string()))?;
        let pid = std::process::id() as i64;
        tx.execute(
            "INSERT INTO writer_ownership (project_id, writer_token, holder_pid, lease_expires_at_unix_ms) VALUES (?1, ?2, ?3, 0)",
            params![project_id.as_str(), writer_token.clone(), pid],
        )
        .map_err(|error| ProjectMemoryError::Sqlite(error.to_string()))?;
        tx.commit()
            .map_err(|error| ProjectMemoryError::Sqlite(error.to_string()))?;
        Ok(DurableProjectMemory {
            project_id: project_id.clone(),
            display_name,
            format_version: PROJECT_MEMORY_FORMAT_VERSION,
            opened: OpenedProject {
                _project_id: project_id.as_str().to_owned(),
                _db_path: db_path,
                connection,
                mode: ProjectMemoryOpenMode::Writable,
                writer_token: Some(writer_token),
            },
            records: Vec::new(),
        })
    }

    pub fn open(
        &self,
        project_id: &ProjectScopeId,
        mode: ProjectMemoryOpenMode,
    ) -> Result<DurableProjectMemory, ProjectMemoryError> {
        validate_project_id(project_id.as_str())?;
        let db_path = self.database_path(project_id.as_str());
        if !db_path.exists() {
            return Err(ProjectMemoryError::ProjectNotFound);
        }
        let flags = match mode {
            ProjectMemoryOpenMode::Writable => OpenFlags::SQLITE_OPEN_READ_WRITE,
            ProjectMemoryOpenMode::ReadOnly => OpenFlags::SQLITE_OPEN_READ_ONLY,
        };
        let mut connection = Connection::open_with_flags(&db_path, flags)
            .map_err(|error| ProjectMemoryError::Sqlite(error.to_string()))?;
        configure_connection(&connection)?;
        let format_version: u32 = connection
            .query_row(
                "SELECT format_version FROM project_metadata WHERE project_id = ?1",
                [project_id.as_str()],
                |row| row.get(0),
            )
            .map_err(|error| ProjectMemoryError::Sqlite(error.to_string()))?;
        if !is_supported_project_memory_format_version(format_version) {
            return Err(ProjectMemoryError::UnsupportedFormatVersion {
                found: format_version,
                supported: PROJECT_MEMORY_FORMAT_VERSION_V3,
            });
        }
        let display_name_raw: String = connection
            .query_row(
                "SELECT display_name FROM project_metadata WHERE project_id = ?1",
                [project_id.as_str()],
                |row| row.get(0),
            )
            .map_err(|error| ProjectMemoryError::Sqlite(error.to_string()))?;
        let display_name = ProjectScopeDisplayName::new(display_name_raw)
            .map_err(|_| ProjectMemoryError::InvalidDisplayName)?;
        let writer_token = if mode == ProjectMemoryOpenMode::Writable {
            let token = Uuid::new_v4().to_string();
            let pid = std::process::id() as i64;
            let tx = connection
                .transaction_with_behavior(TransactionBehavior::Immediate)
                .map_err(|error| ProjectMemoryError::Sqlite(error.to_string()))?;
            let updated = tx
                .execute(
                    "UPDATE writer_ownership SET writer_token = ?1, holder_pid = ?2, lease_expires_at_unix_ms = 0 WHERE project_id = ?3 AND writer_token IS NULL",
                    params![token, pid, project_id.as_str()],
                )
                .map_err(|error| ProjectMemoryError::Sqlite(error.to_string()))?;
            if updated != 1 {
                return Err(ProjectMemoryError::WriterOwnershipHeld);
            }
            tx.commit()
                .map_err(|error| ProjectMemoryError::Sqlite(error.to_string()))?;
            Some(token)
        } else {
            None
        };
        let records = load_records(&connection, project_id.as_str())?;
        if required_project_memory_format_version(&records) > format_version {
            return Err(ProjectMemoryError::CanonicalMismatch(
                "human-raised promotion records require format version 2".to_owned(),
            ));
        }
        Ok(DurableProjectMemory {
            project_id: project_id.clone(),
            display_name,
            format_version,
            opened: OpenedProject {
                _project_id: project_id.as_str().to_owned(),
                _db_path: db_path,
                connection,
                mode,
                writer_token,
            },
            records,
        })
    }

    pub fn list_projects(&self) -> Result<Vec<ProjectListSummary>, ProjectMemoryError> {
        let projects_root = self.root.join("projects");
        if !projects_root.exists() {
            return Ok(Vec::new());
        }
        let mut summaries = Vec::new();
        let entries = fs::read_dir(&projects_root)
            .map_err(|error| ProjectMemoryError::Io(error.to_string()))?;
        for entry in entries {
            let entry = entry.map_err(|error| ProjectMemoryError::Io(error.to_string()))?;
            let project_id_raw = entry.file_name().to_string_lossy().into_owned();
            if validate_project_id(&project_id_raw).is_err() {
                continue;
            }
            let db_path = self.database_path(&project_id_raw);
            if !db_path.is_file() {
                continue;
            }
            let connection =
                Connection::open_with_flags(&db_path, OpenFlags::SQLITE_OPEN_READ_ONLY)
                    .map_err(|error| ProjectMemoryError::Sqlite(error.to_string()))?;
            configure_connection(&connection)?;
            let format_version: u32 = match connection.query_row(
                "SELECT format_version FROM project_metadata WHERE project_id = ?1",
                [&project_id_raw],
                |row| row.get(0),
            ) {
                Ok(value) => value,
                Err(_) => continue,
            };
            if !is_supported_project_memory_format_version(format_version) {
                continue;
            }
            let display_name: String = match connection.query_row(
                "SELECT display_name FROM project_metadata WHERE project_id = ?1",
                [&project_id_raw],
                |row| row.get(0),
            ) {
                Ok(value) => value,
                Err(_) => continue,
            };
            let created_at_unix_ms: i64 = match connection.query_row(
                "SELECT created_at_unix_ms FROM project_metadata WHERE project_id = ?1",
                [&project_id_raw],
                |row| row.get(0),
            ) {
                Ok(value) => value,
                Err(_) => continue,
            };
            let Ok(project_id) = ProjectScopeId::new(project_id_raw) else {
                continue;
            };
            summaries.push(ProjectListSummary {
                project_id,
                display_name,
                created_at_unix_ms,
            });
        }
        summaries.sort_by(|left, right| {
            left.display_name
                .cmp(&right.display_name)
                .then_with(|| left.created_at_unix_ms.cmp(&right.created_at_unix_ms))
                .then_with(|| left.project_id.as_str().cmp(right.project_id.as_str()))
        });
        Ok(summaries)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectListSummary {
    pub project_id: ProjectScopeId,
    pub display_name: String,
    pub created_at_unix_ms: i64,
}

impl DurableProjectMemory {
    pub fn project_id(&self) -> &ProjectScopeId {
        &self.project_id
    }

    pub fn display_name(&self) -> &ProjectScopeDisplayName {
        &self.display_name
    }

    pub fn project_scope(&self) -> ProjectScope {
        ProjectScope::new(self.project_id.clone(), self.display_name.clone())
    }

    pub fn records(&self) -> &[ProjectMemoryRecord] {
        &self.records
    }

    pub fn format_version(&self) -> u32 {
        self.format_version
    }

    pub fn snapshot_identity(&self) -> ProjectMemorySnapshotIdentity {
        compute_project_memory_snapshot_identity(
            &self.project_id,
            self.format_version,
            self.records.len(),
            &self.records,
        )
    }

    pub fn governance_events(&self) -> Vec<ReusableGovernanceEvent> {
        self.records
            .iter()
            .map(|record| record.event.clone())
            .collect()
    }

    pub fn append_promotion(
        &mut self,
        source_session_id: impl Into<String>,
        event: ReusableGovernanceEvent,
    ) -> Result<(), ProjectMemoryError> {
        self.ensure_writable()?;
        let source_session_id = source_session_id.into();
        if source_session_id.is_empty()
            || source_session_id.contains('/')
            || source_session_id.contains('\\')
        {
            return Err(ProjectMemoryError::MissingSourceSessionId);
        }
        if !matches!(event, ReusableGovernanceEvent::PromotionAccepted { .. }) {
            return Err(ProjectMemoryError::CanonicalMismatch(
                "project memory P1 accepts PromotionAccepted only".to_owned(),
            ));
        }
        let candidate_record = ProjectMemoryRecord {
            source_session_id: source_session_id.clone(),
            event: event.clone(),
        };
        // Additive bump: only the metadata format_version changes. Historical event
        // rows are never rewritten.
        let bump_to_v3 = record_has_explicit_allowed_effects(&candidate_record)
            && self.format_version < PROJECT_MEMORY_FORMAT_VERSION_V3;
        let bump_to_v2 = record_is_human_raised(&candidate_record)
            && self.format_version < PROJECT_MEMORY_FORMAT_VERSION_V2
            && !bump_to_v3;
        let writer_token = self
            .opened
            .writer_token
            .clone()
            .ok_or(ProjectMemoryError::WriterOwnershipHeld)?;
        let persisted = PersistedProjectMemoryEventV1 {
            source_session_id: source_session_id.clone(),
            event: persist_governance_event(&event),
        };
        let event_json = serde_json::to_string(&persisted)
            .map_err(|error| ProjectMemoryError::CanonicalMismatch(error.to_string()))?;
        let tx = self
            .opened
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|error| ProjectMemoryError::Sqlite(error.to_string()))?;
        verify_writer_token(&tx, self.project_id.as_str(), &writer_token)?;
        let current_head: usize =
            tx.query_row(
                "SELECT governance_head FROM command_tokens WHERE project_id = ?1",
                [self.project_id.as_str()],
                |row| row.get::<_, i64>(0),
            )
            .map_err(|error| ProjectMemoryError::Sqlite(error.to_string()))? as usize;
        if current_head != self.records.len() {
            return Err(ProjectMemoryError::CanonicalMismatch(
                "governance head mismatch".to_owned(),
            ));
        }
        tx.execute(
            "INSERT INTO governance_events (project_id, event_index, source_session_id, event_json) VALUES (?1, ?2, ?3, ?4)",
            params![
                self.project_id.as_str(),
                current_head as i64,
                source_session_id,
                event_json
            ],
        )
        .map_err(|error| ProjectMemoryError::Sqlite(error.to_string()))?;
        tx.execute(
            "UPDATE command_tokens SET governance_head = ?1 WHERE project_id = ?2",
            params![(current_head + 1) as i64, self.project_id.as_str()],
        )
        .map_err(|error| ProjectMemoryError::Sqlite(error.to_string()))?;
        if bump_to_v3 {
            tx.execute(
                "UPDATE project_metadata SET format_version = ?1 WHERE project_id = ?2",
                params![PROJECT_MEMORY_FORMAT_VERSION_V3, self.project_id.as_str()],
            )
            .map_err(|error| ProjectMemoryError::Sqlite(error.to_string()))?;
        } else if bump_to_v2 {
            tx.execute(
                "UPDATE project_metadata SET format_version = ?1 WHERE project_id = ?2",
                params![PROJECT_MEMORY_FORMAT_VERSION_V2, self.project_id.as_str()],
            )
            .map_err(|error| ProjectMemoryError::Sqlite(error.to_string()))?;
        }
        insert_authority_transition(&tx, self.project_id.as_str())?;
        tx.commit()
            .map_err(|error| ProjectMemoryError::Sqlite(error.to_string()))?;
        if bump_to_v3 {
            self.format_version = PROJECT_MEMORY_FORMAT_VERSION_V3;
        } else if bump_to_v2 {
            self.format_version = PROJECT_MEMORY_FORMAT_VERSION_V2;
        }
        self.records.push(ProjectMemoryRecord {
            source_session_id: persisted.source_session_id,
            event,
        });
        Ok(())
    }

    pub fn update_display_name(
        &mut self,
        display_name: impl Into<String>,
    ) -> Result<(), ProjectMemoryError> {
        self.ensure_writable()?;
        let display_name = ProjectScopeDisplayName::new(display_name.into())
            .map_err(|_| ProjectMemoryError::InvalidDisplayName)?;
        let writer_token = self
            .opened
            .writer_token
            .clone()
            .ok_or(ProjectMemoryError::WriterOwnershipHeld)?;
        let tx = self
            .opened
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|error| ProjectMemoryError::Sqlite(error.to_string()))?;
        verify_writer_token(&tx, self.project_id.as_str(), &writer_token)?;
        tx.execute(
            "UPDATE project_metadata SET display_name = ?1 WHERE project_id = ?2",
            params![display_name.as_str(), self.project_id.as_str()],
        )
        .map_err(|error| ProjectMemoryError::Sqlite(error.to_string()))?;
        tx.commit()
            .map_err(|error| ProjectMemoryError::Sqlite(error.to_string()))?;
        self.display_name = display_name;
        Ok(())
    }

    pub fn close(mut self) -> Result<(), ProjectMemoryError> {
        if self.opened.mode != ProjectMemoryOpenMode::Writable {
            return Ok(());
        }
        let Some(token) = self.opened.writer_token.take() else {
            return Ok(());
        };
        let tx = self
            .opened
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|error| ProjectMemoryError::Sqlite(error.to_string()))?;
        tx.execute(
            "UPDATE writer_ownership SET writer_token = NULL, holder_pid = NULL WHERE project_id = ?1 AND writer_token = ?2",
            params![self.project_id.as_str(), token],
        )
        .map_err(|error| ProjectMemoryError::Sqlite(error.to_string()))?;
        tx.commit()
            .map_err(|error| ProjectMemoryError::Sqlite(error.to_string()))?;
        Ok(())
    }

    fn ensure_writable(&self) -> Result<(), ProjectMemoryError> {
        if self.opened.mode != ProjectMemoryOpenMode::Writable {
            return Err(ProjectMemoryError::ProjectNotWritable);
        }
        Ok(())
    }
}

fn load_records(
    connection: &Connection,
    project_id: &str,
) -> Result<Vec<ProjectMemoryRecord>, ProjectMemoryError> {
    let mut stmt = connection
        .prepare(
            "SELECT source_session_id, event_json FROM governance_events WHERE project_id = ?1 ORDER BY event_index ASC",
        )
        .map_err(|error| ProjectMemoryError::Sqlite(error.to_string()))?;
    let rows = stmt
        .query_map([project_id], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })
        .map_err(|error| ProjectMemoryError::Sqlite(error.to_string()))?;
    let mut records = Vec::new();
    for row in rows {
        let (source_session_id, event_json) =
            row.map_err(|error| ProjectMemoryError::Sqlite(error.to_string()))?;
        let persisted: PersistedProjectMemoryEventV1 = serde_json::from_str(&event_json)
            .map_err(|error| ProjectMemoryError::CanonicalMismatch(error.to_string()))?;
        if persisted.source_session_id != source_session_id {
            return Err(ProjectMemoryError::CanonicalMismatch(
                "source_session_id column mismatch".to_owned(),
            ));
        }
        let event = restore_governance_event(&persisted.event)
            .map_err(|error| ProjectMemoryError::CanonicalMismatch(error.to_string()))?;
        records.push(ProjectMemoryRecord {
            source_session_id,
            event,
        });
    }
    Ok(records)
}

fn initialize_schema(connection: &Connection) -> Result<(), ProjectMemoryError> {
    connection
        .execute_batch(
            "
            CREATE TABLE project_metadata (
              project_id TEXT PRIMARY KEY,
              format_version INTEGER NOT NULL,
              display_name TEXT NOT NULL,
              created_at_unix_ms INTEGER NOT NULL
            );
            CREATE TABLE governance_events (
              project_id TEXT NOT NULL,
              event_index INTEGER NOT NULL,
              source_session_id TEXT NOT NULL,
              event_json TEXT NOT NULL,
              PRIMARY KEY (project_id, event_index)
            );
            CREATE TABLE command_tokens (
              project_id TEXT PRIMARY KEY,
              governance_head INTEGER NOT NULL
            );
            CREATE TABLE authority_transitions (
              project_id TEXT NOT NULL,
              generation INTEGER NOT NULL,
              acknowledgement_status TEXT NOT NULL,
              PRIMARY KEY (project_id, generation)
            );
            CREATE TABLE writer_ownership (
              project_id TEXT PRIMARY KEY,
              writer_token TEXT,
              holder_pid INTEGER,
              lease_expires_at_unix_ms INTEGER NOT NULL
            );
            ",
        )
        .map_err(|error| ProjectMemoryError::Sqlite(error.to_string()))?;
    Ok(())
}

fn configure_connection(connection: &Connection) -> Result<(), ProjectMemoryError> {
    connection
        .pragma_update(None, "foreign_keys", "ON")
        .map_err(|error| ProjectMemoryError::Sqlite(error.to_string()))?;
    connection
        .pragma_update(None, "journal_mode", "WAL")
        .map_err(|error| ProjectMemoryError::Sqlite(error.to_string()))?;
    connection
        .pragma_update(None, "synchronous", "FULL")
        .map_err(|error| ProjectMemoryError::Sqlite(error.to_string()))?;
    Ok(())
}

fn verify_writer_token(
    tx: &rusqlite::Transaction<'_>,
    project_id: &str,
    expected_token: &str,
) -> Result<(), ProjectMemoryError> {
    let db_token: Option<String> = tx
        .query_row(
            "SELECT writer_token FROM writer_ownership WHERE project_id = ?1",
            [project_id],
            |row| row.get(0),
        )
        .map_err(|error| ProjectMemoryError::Sqlite(error.to_string()))?;
    match db_token.as_deref() {
        Some(token) if token == expected_token => Ok(()),
        _ => Err(ProjectMemoryError::WriterOwnershipHeld),
    }
}

fn insert_authority_transition(
    tx: &rusqlite::Transaction<'_>,
    project_id: &str,
) -> Result<(), ProjectMemoryError> {
    let next: i64 = tx
        .query_row(
            "SELECT COALESCE(MAX(generation), 0) + 1 FROM authority_transitions WHERE project_id = ?1",
            [project_id],
            |row| row.get(0),
        )
        .map_err(|error| ProjectMemoryError::Sqlite(error.to_string()))?;
    tx.execute(
        "INSERT INTO authority_transitions (project_id, generation, acknowledgement_status) VALUES (?1, ?2, 'committed')",
        params![project_id, next],
    )
    .map_err(|error| ProjectMemoryError::Sqlite(error.to_string()))?;
    Ok(())
}

fn validate_project_id(project_id: &str) -> Result<(), ProjectMemoryError> {
    if project_id.is_empty() || project_id.contains('/') || project_id.contains('\\') {
        return Err(ProjectMemoryError::InvalidProjectId);
    }
    Ok(())
}

fn unix_ms() -> Result<i64, ProjectMemoryError> {
    Ok(SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| ProjectMemoryError::Io(error.to_string()))?
        .as_millis() as i64)
}
