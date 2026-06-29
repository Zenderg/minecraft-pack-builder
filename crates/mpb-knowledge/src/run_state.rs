use std::fmt;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::str::FromStr;

use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use thiserror::Error;
use time::{format_description::well_known::Rfc3339, OffsetDateTime};

#[derive(Debug, Error)]
pub enum RunStateError {
    #[error("file operation failed: {0}")]
    Io(#[from] std::io::Error),
    #[error("sqlite operation failed: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("json operation failed: {0}")]
    Json(#[from] serde_json::Error),
    #[error("timestamp formatting failed: {0}")]
    TimeFormat(#[from] time::error::Format),
    #[error("unknown knowledge run phase: {0}")]
    UnknownPhase(String),
    #[error("unknown phase checkpoint status: {0}")]
    UnknownCheckpointStatus(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum KnowledgeRunPhase {
    Intake,
    Preflight,
    Approvals,
    Fingerprint,
    Clone,
    Extraction,
    Drafting,
    ExperimentPlanning,
    AdapterExpansion,
    RuntimeVerification,
    Validation,
    Bundle,
    PatcherIntegration,
    ProductValidation,
    Release,
    Report,
}

impl KnowledgeRunPhase {
    pub const ALL: [KnowledgeRunPhase; 16] = [
        KnowledgeRunPhase::Intake,
        KnowledgeRunPhase::Preflight,
        KnowledgeRunPhase::Approvals,
        KnowledgeRunPhase::Fingerprint,
        KnowledgeRunPhase::Clone,
        KnowledgeRunPhase::Extraction,
        KnowledgeRunPhase::Drafting,
        KnowledgeRunPhase::ExperimentPlanning,
        KnowledgeRunPhase::AdapterExpansion,
        KnowledgeRunPhase::RuntimeVerification,
        KnowledgeRunPhase::Validation,
        KnowledgeRunPhase::Bundle,
        KnowledgeRunPhase::PatcherIntegration,
        KnowledgeRunPhase::ProductValidation,
        KnowledgeRunPhase::Release,
        KnowledgeRunPhase::Report,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            KnowledgeRunPhase::Intake => "Intake",
            KnowledgeRunPhase::Preflight => "Preflight",
            KnowledgeRunPhase::Approvals => "Approvals",
            KnowledgeRunPhase::Fingerprint => "Fingerprint",
            KnowledgeRunPhase::Clone => "Clone",
            KnowledgeRunPhase::Extraction => "Extraction",
            KnowledgeRunPhase::Drafting => "Drafting",
            KnowledgeRunPhase::ExperimentPlanning => "ExperimentPlanning",
            KnowledgeRunPhase::AdapterExpansion => "AdapterExpansion",
            KnowledgeRunPhase::RuntimeVerification => "RuntimeVerification",
            KnowledgeRunPhase::Validation => "Validation",
            KnowledgeRunPhase::Bundle => "Bundle",
            KnowledgeRunPhase::PatcherIntegration => "PatcherIntegration",
            KnowledgeRunPhase::ProductValidation => "ProductValidation",
            KnowledgeRunPhase::Release => "Release",
            KnowledgeRunPhase::Report => "Report",
        }
    }
}

impl fmt::Display for KnowledgeRunPhase {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl FromStr for KnowledgeRunPhase {
    type Err = RunStateError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        KnowledgeRunPhase::ALL
            .into_iter()
            .find(|phase| phase.as_str() == value)
            .ok_or_else(|| RunStateError::UnknownPhase(value.to_string()))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PhaseCheckpointStatus {
    Started,
    Succeeded,
    Failed,
}

impl PhaseCheckpointStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            PhaseCheckpointStatus::Started => "Started",
            PhaseCheckpointStatus::Succeeded => "Succeeded",
            PhaseCheckpointStatus::Failed => "Failed",
        }
    }
}

impl fmt::Display for PhaseCheckpointStatus {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl FromStr for PhaseCheckpointStatus {
    type Err = RunStateError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "Started" => Ok(PhaseCheckpointStatus::Started),
            "Succeeded" => Ok(PhaseCheckpointStatus::Succeeded),
            "Failed" => Ok(PhaseCheckpointStatus::Failed),
            _ => Err(RunStateError::UnknownCheckpointStatus(value.to_string())),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct KnowledgeRun {
    pub run_id: String,
    pub target_fingerprint: Option<String>,
    pub created_at: String,
    pub detail: Value,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PhaseCheckpoint {
    pub id: i64,
    pub run_id: String,
    pub phase: KnowledgeRunPhase,
    pub status: PhaseCheckpointStatus,
    pub target_fingerprint: Option<String>,
    pub created_at: String,
    pub detail: Value,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RunBlocker {
    pub id: i64,
    pub run_id: String,
    pub code: String,
    pub phase: Option<KnowledgeRunPhase>,
    pub target_fingerprint: Option<String>,
    pub message: String,
    pub created_at: String,
    pub detail: Value,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RunBlockerInput {
    pub code: String,
    pub phase: Option<KnowledgeRunPhase>,
    pub target_fingerprint: Option<String>,
    pub message: String,
    pub detail: Value,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ArtifactRef {
    pub id: i64,
    pub run_id: String,
    pub artifact_kind: String,
    pub path: String,
    pub target_fingerprint: Option<String>,
    pub created_at: String,
    pub detail: Value,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EventRecord {
    pub sequence: i64,
    pub run_id: String,
    pub event_kind: String,
    pub target_fingerprint: Option<String>,
    pub created_at: String,
    pub detail: Value,
}

pub struct KnowledgeRunStore {
    run_id: String,
    run_dir: PathBuf,
    event_log_path: PathBuf,
    conn: Connection,
}

impl KnowledgeRunStore {
    pub fn open(artifact_root: impl AsRef<Path>, run_id: &str) -> Result<Self, RunStateError> {
        let run_dir = artifact_root.as_ref().join("runs").join(run_id);
        fs::create_dir_all(&run_dir)?;
        let event_log_path = run_dir.join("events.jsonl");
        let conn = Connection::open(run_dir.join("run.sqlite3"))?;
        let store = KnowledgeRunStore {
            run_id: run_id.to_string(),
            run_dir,
            event_log_path,
            conn,
        };
        store.apply_migrations()?;
        OpenOptions::new()
            .create(true)
            .append(true)
            .open(&store.event_log_path)?;
        Ok(store)
    }

    pub fn run_id(&self) -> &str {
        &self.run_id
    }

    pub(crate) fn connection(&self) -> &Connection {
        &self.conn
    }

    pub fn run_dir(&self) -> &Path {
        &self.run_dir
    }

    pub fn event_log_path(&self) -> &Path {
        &self.event_log_path
    }

    pub fn record_run(
        &self,
        target_fingerprint: Option<&str>,
        detail: Value,
    ) -> Result<(), RunStateError> {
        let created_at = now_rfc3339()?;
        self.conn.execute(
            "INSERT INTO runs (run_id, target_fingerprint, created_at, detail_json)
             VALUES (?1, ?2, ?3, ?4)
             ON CONFLICT(run_id) DO UPDATE SET
                target_fingerprint = COALESCE(excluded.target_fingerprint, runs.target_fingerprint),
                detail_json = excluded.detail_json",
            params![
                self.run_id,
                target_fingerprint,
                created_at,
                serde_json::to_string(&detail)?
            ],
        )?;
        Ok(())
    }

    pub fn run(&self) -> Result<Option<KnowledgeRun>, RunStateError> {
        self.conn
            .query_row(
                "SELECT run_id, target_fingerprint, created_at, detail_json
                 FROM runs WHERE run_id = ?1",
                params![self.run_id],
                |row| {
                    let detail_json: String = row.get(3)?;
                    Ok((row.get(0)?, row.get(1)?, row.get(2)?, detail_json))
                },
            )
            .optional()?
            .map(
                |(run_id, target_fingerprint, created_at, detail_json): (
                    String,
                    Option<String>,
                    String,
                    String,
                )| {
                    Ok(KnowledgeRun {
                        run_id,
                        target_fingerprint,
                        created_at,
                        detail: serde_json::from_str(&detail_json)?,
                    })
                },
            )
            .transpose()
    }

    pub fn record_phase_checkpoint(
        &self,
        phase: KnowledgeRunPhase,
        status: PhaseCheckpointStatus,
        target_fingerprint: Option<&str>,
        detail: Value,
    ) -> Result<PhaseCheckpoint, RunStateError> {
        let created_at = now_rfc3339()?;
        self.conn.execute(
            "INSERT INTO phase_checkpoints
             (run_id, phase, status, target_fingerprint, created_at, detail_json)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                self.run_id,
                phase.as_str(),
                status.as_str(),
                target_fingerprint,
                created_at,
                serde_json::to_string(&detail)?
            ],
        )?;
        let id = self.conn.last_insert_rowid();
        self.append_event(
            "phase.checkpoint",
            target_fingerprint,
            json!({
                "phase": phase.as_str(),
                "status": status.as_str(),
                "checkpointId": id,
                "detail": detail,
            }),
        )?;
        Ok(PhaseCheckpoint {
            id,
            run_id: self.run_id.clone(),
            phase,
            status,
            target_fingerprint: target_fingerprint.map(str::to_string),
            created_at,
            detail,
        })
    }

    pub fn latest_successful_checkpoint(&self) -> Result<Option<PhaseCheckpoint>, RunStateError> {
        self.conn
            .query_row(
                "SELECT id, run_id, phase, status, target_fingerprint, created_at, detail_json
                 FROM phase_checkpoints
                 WHERE run_id = ?1 AND status = ?2
                ORDER BY id DESC
                LIMIT 1",
                params![self.run_id, PhaseCheckpointStatus::Succeeded.as_str()],
                |row| {
                    Ok(CheckpointRow {
                        id: row.get(0)?,
                        run_id: row.get(1)?,
                        phase: row.get(2)?,
                        status: row.get(3)?,
                        target_fingerprint: row.get(4)?,
                        created_at: row.get(5)?,
                        detail_json: row.get(6)?,
                    })
                },
            )
            .optional()?
            .map(checkpoint_from_row)
            .transpose()
    }

    pub fn record_blocker(&self, input: RunBlockerInput) -> Result<RunBlocker, RunStateError> {
        let created_at = now_rfc3339()?;
        self.conn.execute(
            "INSERT INTO blockers
             (run_id, code, phase, target_fingerprint, message, created_at, detail_json)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                self.run_id,
                input.code,
                input.phase.map(|phase| phase.as_str().to_string()),
                input.target_fingerprint,
                input.message,
                created_at,
                serde_json::to_string(&input.detail)?
            ],
        )?;
        let id = self.conn.last_insert_rowid();
        let blocker = self
            .blocker_by_id(id)?
            .expect("inserted blocker should be readable");
        self.append_event(
            "blocker.recorded",
            blocker.target_fingerprint.as_deref(),
            json!({
                "blockerId": blocker.id,
                "code": blocker.code,
                "phase": blocker.phase.map(|phase| phase.as_str()),
                "message": blocker.message,
                "detail": blocker.detail,
            }),
        )?;
        Ok(blocker)
    }

    pub fn blockers(&self) -> Result<Vec<RunBlocker>, RunStateError> {
        let mut statement = self.conn.prepare(
            "SELECT id, run_id, code, phase, target_fingerprint, message, created_at, detail_json
             FROM blockers
             WHERE run_id = ?1
             ORDER BY id ASC",
        )?;
        let rows = statement.query_map(params![self.run_id], |row| {
            Ok(BlockerRow {
                id: row.get(0)?,
                run_id: row.get(1)?,
                code: row.get(2)?,
                phase: row.get(3)?,
                target_fingerprint: row.get(4)?,
                message: row.get(5)?,
                created_at: row.get(6)?,
                detail_json: row.get(7)?,
            })
        })?;
        rows.map(|row| blocker_from_row(row?)).collect()
    }

    pub fn record_artifact_ref(
        &self,
        artifact_kind: &str,
        path: impl AsRef<Path>,
        target_fingerprint: Option<&str>,
        detail: Value,
    ) -> Result<ArtifactRef, RunStateError> {
        let created_at = now_rfc3339()?;
        let path = path.as_ref().display().to_string();
        self.conn.execute(
            "INSERT INTO artifact_refs
             (run_id, artifact_kind, path, target_fingerprint, created_at, detail_json)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                self.run_id,
                artifact_kind,
                path,
                target_fingerprint,
                created_at,
                serde_json::to_string(&detail)?
            ],
        )?;
        let id = self.conn.last_insert_rowid();
        Ok(ArtifactRef {
            id,
            run_id: self.run_id.clone(),
            artifact_kind: artifact_kind.to_string(),
            path,
            target_fingerprint: target_fingerprint.map(str::to_string),
            created_at,
            detail,
        })
    }

    pub fn append_event(
        &self,
        event_kind: &str,
        target_fingerprint: Option<&str>,
        detail: Value,
    ) -> Result<EventRecord, RunStateError> {
        let created_at = now_rfc3339()?;
        self.conn.execute(
            "INSERT INTO events (run_id, event_kind, target_fingerprint, created_at, detail_json)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                self.run_id,
                event_kind,
                target_fingerprint,
                created_at,
                serde_json::to_string(&detail)?
            ],
        )?;
        let sequence = self.conn.last_insert_rowid();
        let event = EventRecord {
            sequence,
            run_id: self.run_id.clone(),
            event_kind: event_kind.to_string(),
            target_fingerprint: target_fingerprint.map(str::to_string),
            created_at,
            detail,
        };
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.event_log_path)?;
        serde_json::to_writer(&mut file, &event)?;
        file.write_all(b"\n")?;
        file.flush()?;
        Ok(event)
    }

    pub fn events(&self) -> Result<Vec<EventRecord>, RunStateError> {
        let mut statement = self.conn.prepare(
            "SELECT sequence, run_id, event_kind, target_fingerprint, created_at, detail_json
             FROM events
             WHERE run_id = ?1
             ORDER BY sequence ASC",
        )?;
        let rows = statement.query_map(params![self.run_id], |row| {
            Ok(EventRow {
                sequence: row.get(0)?,
                run_id: row.get(1)?,
                event_kind: row.get(2)?,
                target_fingerprint: row.get(3)?,
                created_at: row.get(4)?,
                detail_json: row.get(5)?,
            })
        })?;
        rows.map(|row| event_from_row(row?)).collect()
    }

    fn apply_migrations(&self) -> Result<(), RunStateError> {
        self.conn.execute_batch(
            "
            PRAGMA foreign_keys = ON;

            CREATE TABLE IF NOT EXISTS runs (
                run_id TEXT PRIMARY KEY NOT NULL,
                target_fingerprint TEXT,
                created_at TEXT NOT NULL,
                detail_json TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS phase_checkpoints (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                run_id TEXT NOT NULL,
                phase TEXT NOT NULL,
                status TEXT NOT NULL,
                target_fingerprint TEXT,
                created_at TEXT NOT NULL,
                detail_json TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS approvals (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                run_id TEXT NOT NULL,
                approval_kind TEXT NOT NULL,
                target_fingerprint TEXT,
                approved INTEGER NOT NULL,
                reason TEXT NOT NULL,
                created_at TEXT NOT NULL,
                detail_json TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS blockers (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                run_id TEXT NOT NULL,
                code TEXT NOT NULL,
                phase TEXT,
                target_fingerprint TEXT,
                message TEXT NOT NULL,
                created_at TEXT NOT NULL,
                detail_json TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS artifact_refs (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                run_id TEXT NOT NULL,
                artifact_kind TEXT NOT NULL,
                path TEXT NOT NULL,
                target_fingerprint TEXT,
                created_at TEXT NOT NULL,
                detail_json TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS events (
                sequence INTEGER PRIMARY KEY AUTOINCREMENT,
                run_id TEXT NOT NULL,
                event_kind TEXT NOT NULL,
                target_fingerprint TEXT,
                created_at TEXT NOT NULL,
                detail_json TEXT NOT NULL
            );
            ",
        )?;
        Ok(())
    }

    fn blocker_by_id(&self, id: i64) -> Result<Option<RunBlocker>, RunStateError> {
        self.conn
            .query_row(
                "SELECT id, run_id, code, phase, target_fingerprint, message, created_at, detail_json
                 FROM blockers
                 WHERE id = ?1",
                params![id],
                |row| {
                    Ok(BlockerRow {
                        id: row.get(0)?,
                        run_id: row.get(1)?,
                        code: row.get(2)?,
                        phase: row.get(3)?,
                        target_fingerprint: row.get(4)?,
                        message: row.get(5)?,
                        created_at: row.get(6)?,
                        detail_json: row.get(7)?,
                    })
                },
            )
            .optional()?
            .map(blocker_from_row)
            .transpose()
    }
}

struct CheckpointRow {
    id: i64,
    run_id: String,
    phase: String,
    status: String,
    target_fingerprint: Option<String>,
    created_at: String,
    detail_json: String,
}

struct BlockerRow {
    id: i64,
    run_id: String,
    code: String,
    phase: Option<String>,
    target_fingerprint: Option<String>,
    message: String,
    created_at: String,
    detail_json: String,
}

struct EventRow {
    sequence: i64,
    run_id: String,
    event_kind: String,
    target_fingerprint: Option<String>,
    created_at: String,
    detail_json: String,
}

fn checkpoint_from_row(row: CheckpointRow) -> Result<PhaseCheckpoint, RunStateError> {
    Ok(PhaseCheckpoint {
        id: row.id,
        run_id: row.run_id,
        phase: KnowledgeRunPhase::from_str(&row.phase)?,
        status: PhaseCheckpointStatus::from_str(&row.status)?,
        target_fingerprint: row.target_fingerprint,
        created_at: row.created_at,
        detail: serde_json::from_str(&row.detail_json)?,
    })
}

fn blocker_from_row(row: BlockerRow) -> Result<RunBlocker, RunStateError> {
    Ok(RunBlocker {
        id: row.id,
        run_id: row.run_id,
        code: row.code,
        phase: row
            .phase
            .map(|value| KnowledgeRunPhase::from_str(&value))
            .transpose()?,
        target_fingerprint: row.target_fingerprint,
        message: row.message,
        created_at: row.created_at,
        detail: serde_json::from_str(&row.detail_json)?,
    })
}

fn event_from_row(row: EventRow) -> Result<EventRecord, RunStateError> {
    Ok(EventRecord {
        sequence: row.sequence,
        run_id: row.run_id,
        event_kind: row.event_kind,
        target_fingerprint: row.target_fingerprint,
        created_at: row.created_at,
        detail: serde_json::from_str(&row.detail_json)?,
    })
}

fn now_rfc3339() -> Result<String, RunStateError> {
    Ok(OffsetDateTime::now_utc().format(&Rfc3339)?)
}
