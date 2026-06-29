use std::fs;

use serde::{Deserialize, Serialize};
use serde_json::json;
use thiserror::Error;

use crate::{
    CoverageEvidenceRequirement, CoverageObligation, KnowledgeRunStore, LabExperimentOperation,
    LabExperimentStatus, LabObservation, RunStateError,
};

pub const FLAKY_EXPERIMENT_RETRY_EXHAUSTED: &str = "FLAKY_EXPERIMENT_RETRY_EXHAUSTED";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExperimentPlan {
    pub fingerprint: String,
    pub batches: Vec<ExperimentBatch>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExperimentBatch {
    pub id: String,
    pub experiments: Vec<ExperimentSpec>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExperimentSpec {
    pub id: String,
    pub obligation_ids: Vec<String>,
    pub deterministic_setup: Vec<String>,
    pub bounded_ticks: u32,
    pub operations: Vec<LabExperimentOperation>,
    pub retry_policy: ExperimentRetryPolicy,
    pub required_observation_adapters: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExperimentRetryPolicy {
    pub max_attempts: u32,
    pub retry_on_statuses: Vec<ExperimentAttemptStatus>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExperimentAttemptStatus {
    Accepted,
    Rejected,
    Failed,
}

impl ExperimentAttemptStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            ExperimentAttemptStatus::Accepted => "accepted",
            ExperimentAttemptStatus::Rejected => "rejected",
            ExperimentAttemptStatus::Failed => "failed",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExperimentAttempt {
    pub experiment_id: String,
    pub attempt: u32,
    pub status: ExperimentAttemptStatus,
    pub observation: Option<LabObservation>,
    pub raw_artifact_path: Option<String>,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExperimentAttemptRecord {
    pub path: String,
    pub attempt: ExperimentAttempt,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExperimentSuiteSummary {
    pub experiment_id: String,
    pub accepted_attempts: usize,
    pub rejected_attempts: usize,
    pub failed_attempts: usize,
    pub flake_counter: usize,
    pub release_blocker: Option<ExperimentReleaseBlocker>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExperimentReleaseBlocker {
    pub code: String,
    pub affected_obligation_ids: Vec<String>,
    pub raw_artifact_paths: Vec<String>,
}

#[derive(Debug, Error)]
pub enum ExperimentError {
    #[error("file operation failed: {0}")]
    Io(#[from] std::io::Error),
    #[error("json operation failed: {0}")]
    Json(#[from] serde_json::Error),
    #[error("run state operation failed: {0}")]
    RunState(#[from] RunStateError),
}

pub fn build_experiment_plan(
    fingerprint: &str,
    obligations: &[CoverageObligation],
) -> ExperimentPlan {
    let experiments = obligations
        .iter()
        .filter(|obligation| {
            !obligation.covered
                && obligation.evidence_requirement == CoverageEvidenceRequirement::Runtime
        })
        .map(|obligation| ExperimentSpec {
            id: format!("exp-{}", sanitize_id(&obligation.id)),
            obligation_ids: vec![obligation.id.clone()],
            deterministic_setup: vec![
                "create isolated lab area in disposable Prism clone".to_string(),
                format!("load subject {}", obligation.subject_id),
            ],
            bounded_ticks: 400,
            operations: vec![
                LabExperimentOperation::PrepareLabArea { radius: 5 },
                LabExperimentOperation::InspectState {
                    target_id: obligation.subject_id.clone(),
                },
                LabExperimentOperation::RunTicks { ticks: 400 },
                LabExperimentOperation::CompareSnapshots,
                LabExperimentOperation::RecordObservation,
            ],
            retry_policy: ExperimentRetryPolicy {
                max_attempts: 3,
                retry_on_statuses: vec![
                    ExperimentAttemptStatus::Rejected,
                    ExperimentAttemptStatus::Failed,
                ],
            },
            required_observation_adapters: vec!["generic_state_diff".to_string()],
        })
        .collect::<Vec<_>>();

    ExperimentPlan {
        fingerprint: fingerprint.to_string(),
        batches: if experiments.is_empty() {
            Vec::new()
        } else {
            vec![ExperimentBatch {
                id: "runtime-obligation-batch-001".to_string(),
                experiments,
            }]
        },
    }
}

pub fn record_experiment_attempt(
    store: &KnowledgeRunStore,
    attempt: ExperimentAttempt,
) -> Result<ExperimentAttemptRecord, ExperimentError> {
    let lab_dir = store.run_dir().join("lab");
    fs::create_dir_all(&lab_dir)?;
    let path = lab_dir.join(format!(
        "{}-attempt-{:03}.json",
        sanitize_id(&attempt.experiment_id),
        attempt.attempt
    ));
    fs::write(&path, serde_json::to_vec_pretty(&attempt)?)?;
    let target_fingerprint = attempt
        .observation
        .as_ref()
        .map(|observation| observation.fingerprint.as_str());
    store.record_artifact_ref(
        "experiment-attempt",
        &path,
        target_fingerprint,
        json!({
            "experimentId": attempt.experiment_id,
            "attempt": attempt.attempt,
            "status": attempt.status.as_str(),
            "rawArtifactPath": attempt.raw_artifact_path,
        }),
    )?;
    store.append_event(
        "experiment.attempt.recorded",
        target_fingerprint,
        json!({
            "experimentId": attempt.experiment_id,
            "attempt": attempt.attempt,
            "status": attempt.status.as_str(),
            "acceptedObservation": attempt
                .observation
                .as_ref()
                .is_some_and(|observation| observation.status == LabExperimentStatus::Accepted),
        }),
    )?;

    Ok(ExperimentAttemptRecord {
        path: path.display().to_string(),
        attempt,
    })
}

pub fn summarize_experiment_suite(
    experiment_id: &str,
    obligation_ids: &[String],
    policy: &ExperimentRetryPolicy,
    attempts: &[ExperimentAttempt],
) -> ExperimentSuiteSummary {
    let accepted_attempts = attempts
        .iter()
        .filter(|attempt| attempt.status == ExperimentAttemptStatus::Accepted)
        .count();
    let rejected_attempts = attempts
        .iter()
        .filter(|attempt| attempt.status == ExperimentAttemptStatus::Rejected)
        .count();
    let failed_attempts = attempts
        .iter()
        .filter(|attempt| attempt.status == ExperimentAttemptStatus::Failed)
        .count();
    let flake_counter = rejected_attempts + failed_attempts;
    let exhausted = accepted_attempts == 0
        && attempts.len() as u32 >= policy.max_attempts
        && attempts
            .iter()
            .all(|attempt| policy.retry_on_statuses.contains(&attempt.status));

    ExperimentSuiteSummary {
        experiment_id: experiment_id.to_string(),
        accepted_attempts,
        rejected_attempts,
        failed_attempts,
        flake_counter,
        release_blocker: exhausted.then(|| ExperimentReleaseBlocker {
            code: FLAKY_EXPERIMENT_RETRY_EXHAUSTED.to_string(),
            affected_obligation_ids: obligation_ids.to_vec(),
            raw_artifact_paths: attempts
                .iter()
                .filter_map(|attempt| attempt.raw_artifact_path.clone())
                .collect(),
        }),
    }
}

fn sanitize_id(value: &str) -> String {
    value
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '-' {
                ch
            } else {
                '-'
            }
        })
        .collect()
}
