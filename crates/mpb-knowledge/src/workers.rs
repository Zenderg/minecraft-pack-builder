use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use thiserror::Error;

use crate::{ApprovalKind, HardwareFit, KnowledgeRunStore, RunStateError};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkerOutputEnvelope {
    pub id: String,
    pub task_kind: WorkerTaskKind,
    pub model: String,
    #[serde(default)]
    pub model_checksum: String,
    pub input_fingerprint: String,
    pub prompt_ref: String,
    pub output_ref: String,
    pub fine_tuning: FineTuningDecision,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkerTaskKind {
    DraftClassification,
    LocalDocumentationClaimExtraction,
    ConflictDetection,
    ExperimentProposal,
    LabLogSummarization,
    StructuredRepairSuggestion,
    Summarization,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "decision")]
pub enum FineTuningDecision {
    NoFineTuningUsed {
        reason: String,
    },
    FineTuningUsed {
        task: String,
        model: String,
        dataset: String,
        evaluation_threshold: String,
        evaluation_result: String,
    },
    FineTuningRequired {
        reason: String,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FineTuningPhaseState {
    NotUsed,
    ProposedBecauseBaseEvaluationFailed,
    ApprovedAndRun,
    RejectedByUser,
    BlockedByHardware,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkerRuntimeTask {
    DraftClassification,
    LocalDocumentationClaimExtraction,
    ConflictDetection,
    ExperimentProposal,
    LabLogSummarization,
    StructuredRepairSuggestion,
}

impl WorkerRuntimeTask {
    pub fn as_str(self) -> &'static str {
        match self {
            WorkerRuntimeTask::DraftClassification => "draft_classification",
            WorkerRuntimeTask::LocalDocumentationClaimExtraction => {
                "local_documentation_claim_extraction"
            }
            WorkerRuntimeTask::ConflictDetection => "conflict_detection",
            WorkerRuntimeTask::ExperimentProposal => "experiment_proposal",
            WorkerRuntimeTask::LabLogSummarization => "lab_log_summarization",
            WorkerRuntimeTask::StructuredRepairSuggestion => "structured_repair_suggestion",
        }
    }

    pub fn task_kind(self) -> WorkerTaskKind {
        match self {
            WorkerRuntimeTask::DraftClassification => WorkerTaskKind::DraftClassification,
            WorkerRuntimeTask::LocalDocumentationClaimExtraction => {
                WorkerTaskKind::LocalDocumentationClaimExtraction
            }
            WorkerRuntimeTask::ConflictDetection => WorkerTaskKind::ConflictDetection,
            WorkerRuntimeTask::ExperimentProposal => WorkerTaskKind::ExperimentProposal,
            WorkerRuntimeTask::LabLogSummarization => WorkerTaskKind::LabLogSummarization,
            WorkerRuntimeTask::StructuredRepairSuggestion => {
                WorkerTaskKind::StructuredRepairSuggestion
            }
        }
    }
}

pub trait WorkerRuntime {
    fn run_worker_task(
        &self,
        task: WorkerRuntimeTask,
        prompt: &Value,
        input: &Value,
    ) -> Result<Value, WorkerRuntimeError>;
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelSelection {
    pub identity: String,
    pub file_path: String,
    pub checksum: String,
    pub hardware_fit: HardwareFit,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkerEvaluationFixture {
    pub fixture_id: String,
    pub passed: bool,
    pub score: f64,
    pub threshold: f64,
    pub report: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkerArtifactInput {
    pub task: WorkerRuntimeTask,
    pub target_fingerprint: String,
    pub model: ModelSelection,
    pub prompt: Value,
    pub input: Value,
    pub output: Value,
    pub evaluation: WorkerEvaluationFixture,
    #[serde(default)]
    pub corrections: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkerArtifactRecord {
    pub worker_id: String,
    pub envelope: WorkerOutputEnvelope,
    pub prompt_path: String,
    pub input_path: String,
    pub output_path: String,
    pub model_identity_path: String,
    pub evaluation_path: String,
    pub corrections_path: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorkerGateOutcome {
    Ready,
    BlockedMissingApproval { kind: ApprovalKind },
    FineTuning { state: FineTuningPhaseState },
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
#[error("invalid worker output envelope: {message}")]
pub struct WorkerOutputEnvelopeError {
    message: String,
}

#[derive(Debug, Error)]
pub enum WorkerRuntimeError {
    #[error("file operation failed: {0}")]
    Io(#[from] std::io::Error),
    #[error("json operation failed: {0}")]
    Json(#[from] serde_json::Error),
    #[error("run state operation failed: {0}")]
    RunState(#[from] RunStateError),
    #[error("invalid worker output envelope: {0}")]
    Envelope(#[from] WorkerOutputEnvelopeError),
}

impl WorkerOutputEnvelope {
    pub fn new(
        id: impl Into<String>,
        task_kind: WorkerTaskKind,
        model: impl Into<String>,
        input_fingerprint: impl Into<String>,
        prompt_ref: impl Into<String>,
        output_ref: impl Into<String>,
        fine_tuning: FineTuningDecision,
    ) -> Result<Self, WorkerOutputEnvelopeError> {
        let envelope = Self {
            id: id.into(),
            task_kind,
            model: model.into(),
            model_checksum: "unrecorded".to_string(),
            input_fingerprint: input_fingerprint.into(),
            prompt_ref: prompt_ref.into(),
            output_ref: output_ref.into(),
            fine_tuning,
        };
        envelope.validate()?;
        Ok(envelope)
    }

    pub fn with_model_checksum(
        id: impl Into<String>,
        task_kind: WorkerTaskKind,
        model: impl Into<String>,
        model_checksum: impl Into<String>,
        input_fingerprint: impl Into<String>,
        prompt_ref: impl Into<String>,
        output_ref: impl Into<String>,
        fine_tuning: FineTuningPhaseState,
    ) -> Result<Self, WorkerOutputEnvelopeError> {
        let model = model.into();
        let fine_tuning = match fine_tuning {
            FineTuningPhaseState::NotUsed => FineTuningDecision::NoFineTuningUsed {
                reason: "base local evaluation passed or worker output remained draft-only"
                    .to_string(),
            },
            FineTuningPhaseState::ProposedBecauseBaseEvaluationFailed => {
                FineTuningDecision::FineTuningRequired {
                    reason: "base local fixture evaluation failed".to_string(),
                }
            }
            FineTuningPhaseState::ApprovedAndRun => FineTuningDecision::FineTuningUsed {
                task: task_kind.as_runtime_label().to_string(),
                model: model.clone(),
                dataset: "knowledge/model-datasets/local".to_string(),
                evaluation_threshold: "fixture threshold met".to_string(),
                evaluation_result: "approved local fine-tuning result recorded".to_string(),
            },
            FineTuningPhaseState::RejectedByUser => FineTuningDecision::FineTuningRequired {
                reason: "fine-tuning was rejected by user approval history".to_string(),
            },
            FineTuningPhaseState::BlockedByHardware => FineTuningDecision::FineTuningRequired {
                reason: "preflight hardware fit was insufficient for local fine-tuning".to_string(),
            },
        };
        let mut envelope = Self::new(
            id,
            task_kind,
            model,
            input_fingerprint,
            prompt_ref,
            output_ref,
            fine_tuning,
        )?;
        envelope.model_checksum = model_checksum.into();
        envelope.validate()?;
        Ok(envelope)
    }

    pub fn is_structured_transformation_candidate(&self) -> bool {
        self.model == "Qwen2.5-Coder-1.5B-Instruct"
            && matches!(
                self.task_kind,
                WorkerTaskKind::DraftClassification
                    | WorkerTaskKind::Summarization
                    | WorkerTaskKind::ConflictDetection
            )
    }

    pub fn is_reasoning_experiment_candidate(&self) -> bool {
        matches!(self.model.as_str(), "Qwen3-1.7B" | "Qwen3-4B")
            && self.task_kind == WorkerTaskKind::ExperimentProposal
    }

    pub fn is_source_of_truth(&self) -> bool {
        false
    }

    fn validate(&self) -> Result<(), WorkerOutputEnvelopeError> {
        for (field, value) in [
            ("id", self.id.as_str()),
            ("model", self.model.as_str()),
            ("model_checksum", self.model_checksum.as_str()),
            ("input_fingerprint", self.input_fingerprint.as_str()),
            ("prompt_ref", self.prompt_ref.as_str()),
            ("output_ref", self.output_ref.as_str()),
        ] {
            if value.trim().is_empty() {
                return Err(WorkerOutputEnvelopeError {
                    message: format!("{field} is required"),
                });
            }
        }

        validate_fine_tuning(&self.fine_tuning)
    }
}

impl WorkerTaskKind {
    fn as_runtime_label(self) -> &'static str {
        match self {
            WorkerTaskKind::DraftClassification => "draft_classification",
            WorkerTaskKind::LocalDocumentationClaimExtraction => {
                "local_documentation_claim_extraction"
            }
            WorkerTaskKind::ConflictDetection => "conflict_detection",
            WorkerTaskKind::ExperimentProposal => "experiment_proposal",
            WorkerTaskKind::LabLogSummarization => "lab_log_summarization",
            WorkerTaskKind::StructuredRepairSuggestion => "structured_repair_suggestion",
            WorkerTaskKind::Summarization => "summarization",
        }
    }
}

pub fn persist_worker_artifacts(
    store: &KnowledgeRunStore,
    input: WorkerArtifactInput,
) -> Result<WorkerArtifactRecord, WorkerRuntimeError> {
    let index = store
        .artifact_refs()?
        .into_iter()
        .filter(|artifact| artifact.artifact_kind == "worker-output")
        .count()
        + 1;
    let worker_id = format!("worker-{index:04}");
    let worker_dir = store.run_dir().join("workers").join(&worker_id);
    fs::create_dir_all(&worker_dir)?;

    let prompt_path = worker_dir.join("prompt.json");
    let input_path = worker_dir.join("input.json");
    let output_path = worker_dir.join("output.json");
    let model_identity_path = worker_dir.join("model.json");
    let evaluation_path = worker_dir.join("evaluation.json");
    let corrections_path = worker_dir.join("corrections.json");

    write_json(&prompt_path, &input.prompt)?;
    write_json(&input_path, &input.input)?;
    write_json(&output_path, &input.output)?;
    write_json(&model_identity_path, &input.model)?;
    write_json(&evaluation_path, &input.evaluation)?;
    write_json(&corrections_path, &input.corrections)?;

    let envelope = WorkerOutputEnvelope::with_model_checksum(
        &worker_id,
        input.task.task_kind(),
        &input.model.identity,
        &input.model.checksum,
        &input.target_fingerprint,
        prompt_path.display().to_string(),
        output_path.display().to_string(),
        FineTuningPhaseState::NotUsed,
    )?;

    let refs = [
        ("worker-prompt", &prompt_path),
        ("worker-input", &input_path),
        ("worker-output", &output_path),
        ("worker-model-identity", &model_identity_path),
        ("worker-evaluation", &evaluation_path),
        ("worker-corrections", &corrections_path),
    ];
    for (artifact_kind, path) in refs {
        store.record_artifact_ref(
            artifact_kind,
            path,
            Some(&input.target_fingerprint),
            json!({
                "workerId": worker_id,
                "task": input.task.as_str(),
                "model": input.model.identity,
                "modelChecksum": input.model.checksum,
            }),
        )?;
    }
    store.append_event(
        "worker.artifacts.persisted",
        Some(&input.target_fingerprint),
        json!({
            "workerId": worker_id,
            "task": input.task.as_str(),
            "evaluationPassed": input.evaluation.passed,
            "model": envelope.model,
            "modelChecksum": envelope.model_checksum,
        }),
    )?;

    Ok(WorkerArtifactRecord {
        worker_id,
        envelope,
        prompt_path: prompt_path.display().to_string(),
        input_path: input_path.display().to_string(),
        output_path: output_path.display().to_string(),
        model_identity_path: model_identity_path.display().to_string(),
        evaluation_path: evaluation_path.display().to_string(),
        corrections_path: corrections_path.display().to_string(),
    })
}

pub fn evaluate_worker_gate(
    store: &KnowledgeRunStore,
    target_fingerprint: &str,
    model: &ModelSelection,
    evaluation: &WorkerEvaluationFixture,
) -> Result<WorkerGateOutcome, WorkerRuntimeError> {
    if !Path::new(&model.file_path).is_file()
        && store
            .require_approval(ApprovalKind::ModelDownload, Some(target_fingerprint))
            .is_err()
    {
        return Ok(WorkerGateOutcome::BlockedMissingApproval {
            kind: ApprovalKind::ModelDownload,
        });
    }

    if evaluation.passed {
        return Ok(WorkerGateOutcome::Ready);
    }

    if model.hardware_fit == HardwareFit::Insufficient {
        return Ok(WorkerGateOutcome::FineTuning {
            state: FineTuningPhaseState::BlockedByHardware,
        });
    }

    if store
        .require_approval(ApprovalKind::FineTuning, Some(target_fingerprint))
        .is_err()
    {
        return Ok(WorkerGateOutcome::FineTuning {
            state: FineTuningPhaseState::ProposedBecauseBaseEvaluationFailed,
        });
    }

    Ok(WorkerGateOutcome::FineTuning {
        state: FineTuningPhaseState::ApprovedAndRun,
    })
}

fn write_json(path: &Path, value: &impl Serialize) -> Result<(), WorkerRuntimeError> {
    fs::write(path, serde_json::to_vec_pretty(value)?)?;
    Ok(())
}

fn validate_fine_tuning(decision: &FineTuningDecision) -> Result<(), WorkerOutputEnvelopeError> {
    match decision {
        FineTuningDecision::NoFineTuningUsed { reason }
        | FineTuningDecision::FineTuningRequired { reason } => require_text(reason, "reason"),
        FineTuningDecision::FineTuningUsed {
            task,
            model,
            dataset,
            evaluation_threshold,
            evaluation_result,
        } => {
            require_text(task, "task")?;
            require_text(model, "model")?;
            require_text(dataset, "dataset")?;
            require_text(evaluation_threshold, "evaluation_threshold")?;
            require_text(evaluation_result, "evaluation_result")
        }
    }
}

fn require_text(value: &str, field: &str) -> Result<(), WorkerOutputEnvelopeError> {
    if value.trim().is_empty() {
        Err(WorkerOutputEnvelopeError {
            message: format!("{field} is required"),
        })
    } else {
        Ok(())
    }
}
