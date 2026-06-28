use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkerOutputEnvelope {
    pub id: String,
    pub task_kind: WorkerTaskKind,
    pub model: String,
    pub input_fingerprint: String,
    pub prompt_ref: String,
    pub output_ref: String,
    pub fine_tuning: FineTuningDecision,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkerTaskKind {
    DraftClassification,
    Summarization,
    ConflictDetection,
    ExperimentProposal,
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

#[derive(Debug, Error, Clone, PartialEq, Eq)]
#[error("invalid worker output envelope: {message}")]
pub struct WorkerOutputEnvelopeError {
    message: String,
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
            input_fingerprint: input_fingerprint.into(),
            prompt_ref: prompt_ref.into(),
            output_ref: output_ref.into(),
            fine_tuning,
        };
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

        if !self.is_structured_transformation_candidate()
            && !self.is_reasoning_experiment_candidate()
        {
            return Err(WorkerOutputEnvelopeError {
                message: format!(
                    "model {} is not reserved for task {:?}",
                    self.model, self.task_kind
                ),
            });
        }

        validate_fine_tuning(&self.fine_tuning)
    }
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
