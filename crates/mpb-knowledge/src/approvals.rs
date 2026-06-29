use std::fmt;
use std::str::FromStr;

use rusqlite::{params, OptionalExtension};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use thiserror::Error;
use time::{format_description::well_known::Rfc3339, OffsetDateTime};

use crate::{KnowledgeRunStore, RunStateError};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ApprovalKind {
    LongRun,
    KeepAwake,
    ModelDownload,
    FineTuning,
    ProjectCodeChange,
    GitHubReleasePublication,
}

impl ApprovalKind {
    pub const ALL: [ApprovalKind; 6] = [
        ApprovalKind::LongRun,
        ApprovalKind::KeepAwake,
        ApprovalKind::ModelDownload,
        ApprovalKind::FineTuning,
        ApprovalKind::ProjectCodeChange,
        ApprovalKind::GitHubReleasePublication,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            ApprovalKind::LongRun => "LongRun",
            ApprovalKind::KeepAwake => "KeepAwake",
            ApprovalKind::ModelDownload => "ModelDownload",
            ApprovalKind::FineTuning => "FineTuning",
            ApprovalKind::ProjectCodeChange => "ProjectCodeChange",
            ApprovalKind::GitHubReleasePublication => "GitHubReleasePublication",
        }
    }
}

impl fmt::Display for ApprovalKind {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl FromStr for ApprovalKind {
    type Err = ApprovalError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        ApprovalKind::ALL
            .into_iter()
            .find(|kind| kind.as_str() == value)
            .ok_or_else(|| ApprovalError::UnknownKind(value.to_string()))
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ApprovalRecord {
    pub id: i64,
    pub run_id: String,
    pub kind: ApprovalKind,
    pub target_fingerprint: Option<String>,
    pub approved: bool,
    pub reason: String,
    pub created_at: String,
    pub detail: Value,
}

#[derive(Debug, Clone, PartialEq, Error)]
#[error("approval required for {kind}")]
pub struct ApprovalGateError {
    pub kind: ApprovalKind,
    pub target_fingerprint: Option<String>,
    pub latest_reason: Option<String>,
}

#[derive(Debug, Error)]
pub enum ApprovalError {
    #[error("unknown approval kind: {0}")]
    UnknownKind(String),
    #[error("sqlite operation failed: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("json operation failed: {0}")]
    Json(#[from] serde_json::Error),
    #[error("timestamp formatting failed: {0}")]
    TimeFormat(#[from] time::error::Format),
    #[error("run state operation failed: {0}")]
    RunState(#[from] RunStateError),
}

impl KnowledgeRunStore {
    pub fn record_approval(
        &self,
        kind: ApprovalKind,
        target_fingerprint: Option<&str>,
        approved: bool,
        reason: &str,
        detail: Value,
    ) -> Result<ApprovalRecord, ApprovalError> {
        let created_at = now_rfc3339()?;
        self.connection().execute(
            "INSERT INTO approvals
             (run_id, approval_kind, target_fingerprint, approved, reason, created_at, detail_json)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                self.run_id(),
                kind.as_str(),
                target_fingerprint,
                approved,
                reason,
                created_at,
                serde_json::to_string(&detail)?
            ],
        )?;
        let id = self.connection().last_insert_rowid();
        let record = ApprovalRecord {
            id,
            run_id: self.run_id().to_string(),
            kind,
            target_fingerprint: target_fingerprint.map(str::to_string),
            approved,
            reason: reason.to_string(),
            created_at,
            detail,
        };
        self.append_event(
            "approval.recorded",
            target_fingerprint,
            json!({
                "approvalId": id,
                "kind": kind.as_str(),
                "approved": approved,
                "reason": reason,
                "detail": record.detail,
            }),
        )?;
        Ok(record)
    }

    pub fn require_approval(
        &self,
        kind: ApprovalKind,
        target_fingerprint: Option<&str>,
    ) -> Result<(), ApprovalGateError> {
        let latest = self
            .latest_approval(kind, target_fingerprint)
            .map_err(|error| ApprovalGateError {
                kind,
                target_fingerprint: target_fingerprint.map(str::to_string),
                latest_reason: Some(error.to_string()),
            })?;
        match latest {
            Some(record) if record.approved => Ok(()),
            Some(record) => Err(ApprovalGateError {
                kind,
                target_fingerprint: target_fingerprint.map(str::to_string),
                latest_reason: Some(record.reason),
            }),
            None => Err(ApprovalGateError {
                kind,
                target_fingerprint: target_fingerprint.map(str::to_string),
                latest_reason: None,
            }),
        }
    }

    pub fn approval_history(
        &self,
        kind: ApprovalKind,
        target_fingerprint: Option<&str>,
    ) -> Result<Vec<ApprovalRecord>, ApprovalError> {
        let mut statement = self.connection().prepare(
            "SELECT id, run_id, approval_kind, target_fingerprint, approved, reason, created_at, detail_json
             FROM approvals
             WHERE run_id = ?1
               AND approval_kind = ?2
               AND ((?3 IS NULL AND target_fingerprint IS NULL) OR target_fingerprint = ?3)
             ORDER BY id ASC",
        )?;
        let rows = statement.query_map(
            params![self.run_id(), kind.as_str(), target_fingerprint],
            |row| {
                Ok(ApprovalRow {
                    id: row.get(0)?,
                    run_id: row.get(1)?,
                    kind: row.get(2)?,
                    target_fingerprint: row.get(3)?,
                    approved: row.get(4)?,
                    reason: row.get(5)?,
                    created_at: row.get(6)?,
                    detail_json: row.get(7)?,
                })
            },
        )?;
        rows.map(|row| approval_from_row(row?)).collect()
    }

    fn latest_approval(
        &self,
        kind: ApprovalKind,
        target_fingerprint: Option<&str>,
    ) -> Result<Option<ApprovalRecord>, ApprovalError> {
        self.connection()
            .query_row(
                "SELECT id, run_id, approval_kind, target_fingerprint, approved, reason, created_at, detail_json
                 FROM approvals
                 WHERE run_id = ?1
                   AND approval_kind = ?2
                   AND ((?3 IS NULL AND target_fingerprint IS NULL) OR target_fingerprint = ?3)
                 ORDER BY id DESC
                 LIMIT 1",
                params![self.run_id(), kind.as_str(), target_fingerprint],
                |row| {
                    Ok(ApprovalRow {
                        id: row.get(0)?,
                        run_id: row.get(1)?,
                        kind: row.get(2)?,
                        target_fingerprint: row.get(3)?,
                        approved: row.get(4)?,
                        reason: row.get(5)?,
                        created_at: row.get(6)?,
                        detail_json: row.get(7)?,
                    })
                },
            )
            .optional()?
            .map(approval_from_row)
            .transpose()
    }
}

struct ApprovalRow {
    id: i64,
    run_id: String,
    kind: String,
    target_fingerprint: Option<String>,
    approved: bool,
    reason: String,
    created_at: String,
    detail_json: String,
}

fn approval_from_row(row: ApprovalRow) -> Result<ApprovalRecord, ApprovalError> {
    Ok(ApprovalRecord {
        id: row.id,
        run_id: row.run_id,
        kind: ApprovalKind::from_str(&row.kind)?,
        target_fingerprint: row.target_fingerprint,
        approved: row.approved,
        reason: row.reason,
        created_at: row.created_at,
        detail: serde_json::from_str(&row.detail_json)?,
    })
}

fn now_rfc3339() -> Result<String, ApprovalError> {
    Ok(OffsetDateTime::now_utc().format(&Rfc3339)?)
}
