use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::{
    EvidenceKind, EvidenceSummary, KnowledgeValidationError, ValidationCode, ValidationFailure,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LabObservation {
    pub id: String,
    pub experiment_id: String,
    pub fingerprint: String,
    pub status: LabExperimentStatus,
    pub operations: Vec<LabExperimentOperation>,
    pub before: Vec<LabObservedState>,
    pub after: Vec<LabObservedState>,
    pub observed_entity_ids: Vec<String>,
    pub summary: String,
    pub limits: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LabExperimentStatus {
    Accepted,
    Rejected,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "operation")]
pub enum LabExperimentOperation {
    PrepareLabArea { radius: u32 },
    ResetLabArea,
    PlaceStructure { structure_id: String },
    SetBlockState { block_id: String, state: String },
    UseItemOnBlock { item_id: String, block_id: String },
    RunTicks { ticks: u32 },
    InspectState { target_id: String },
    CompareSnapshots,
    RecordObservation,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LabObservedState {
    pub target_id: String,
    pub state_type: String,
    pub value: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LabBatchReport {
    pub suite_id: String,
    pub fingerprint: String,
    pub observations: Vec<LabObservation>,
    pub failed_experiment_ids: Vec<String>,
    pub uncovered_entity_ids: Vec<String>,
    pub unresolved_mechanic_ids: Vec<String>,
    pub stale_fingerprint: bool,
    pub placeholder_artifact_ids: Vec<String>,
    pub invalid_bundle: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LabBatchReportSummary {
    pub experiment_count: usize,
    pub accepted_observation_count: usize,
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
#[error("lab observation cannot be converted: {message}")]
pub struct LabObservationError {
    code: ValidationCode,
    message: String,
}

impl LabObservationError {
    fn new(code: ValidationCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }

    pub fn code(&self) -> ValidationCode {
        self.code
    }
}

pub fn convert_lab_observation_to_evidence(
    observation: &LabObservation,
    linked_claim_ids: &[String],
    expected_fingerprint: &str,
) -> Result<EvidenceSummary, LabObservationError> {
    if linked_claim_ids.is_empty() {
        return Err(LabObservationError::new(
            ValidationCode::BehavioralClaimWithoutRuntimeEvidence,
            format!(
                "lab observation {} must be linked to at least one claim",
                observation.id
            ),
        ));
    }
    if observation.fingerprint != expected_fingerprint {
        return Err(LabObservationError::new(
            ValidationCode::FingerprintMismatch,
            format!(
                "lab observation {} fingerprint {} does not match {}",
                observation.id, observation.fingerprint, expected_fingerprint
            ),
        ));
    }
    if observation.status != LabExperimentStatus::Accepted {
        return Err(LabObservationError::new(
            ValidationCode::BehavioralClaimWithoutRuntimeEvidence,
            format!("lab observation {} was not accepted", observation.id),
        ));
    }
    if observation.summary.trim().is_empty() || observation.observed_entity_ids.is_empty() {
        return Err(LabObservationError::new(
            ValidationCode::UnresolvedPlaceholders,
            format!("lab observation {} is incomplete", observation.id),
        ));
    }

    Ok(EvidenceSummary {
        id: observation.id.clone(),
        kind: EvidenceKind::RuntimeObservation,
        summary: compact_summary(observation, linked_claim_ids),
        fingerprint: observation.fingerprint.clone(),
        accepted: true,
    })
}

pub fn validate_lab_batch_report(
    report: &LabBatchReport,
) -> Result<LabBatchReportSummary, KnowledgeValidationError> {
    let mut failures = Vec::new();

    if !report.failed_experiment_ids.is_empty() {
        failures.push(ValidationFailure {
            code: ValidationCode::BehavioralClaimWithoutRuntimeEvidence,
            message: format!(
                "lab suite {} has failed experiments: {}",
                report.suite_id,
                report.failed_experiment_ids.join(", ")
            ),
        });
    }
    if !report.uncovered_entity_ids.is_empty() {
        failures.push(ValidationFailure {
            code: ValidationCode::UncoveredEntities,
            message: format!(
                "lab suite {} has uncovered entities: {}",
                report.suite_id,
                report.uncovered_entity_ids.join(", ")
            ),
        });
    }
    if !report.unresolved_mechanic_ids.is_empty() {
        failures.push(ValidationFailure {
            code: ValidationCode::IncompleteOverlays,
            message: format!(
                "lab suite {} has unresolved mechanics: {}",
                report.suite_id,
                report.unresolved_mechanic_ids.join(", ")
            ),
        });
    }
    if report.stale_fingerprint
        || report
            .observations
            .iter()
            .any(|observation| observation.fingerprint != report.fingerprint)
    {
        failures.push(ValidationFailure {
            code: ValidationCode::FingerprintMismatch,
            message: format!("lab suite {} has stale fingerprint data", report.suite_id),
        });
    }
    if !report.placeholder_artifact_ids.is_empty()
        || report
            .observations
            .iter()
            .any(|observation| contains_placeholder(&observation.summary))
    {
        failures.push(ValidationFailure {
            code: ValidationCode::UnresolvedPlaceholders,
            message: format!("lab suite {} has placeholder artifacts", report.suite_id),
        });
    }
    if report.invalid_bundle {
        failures.push(ValidationFailure {
            code: ValidationCode::RuntimeBundleQueryGaps,
            message: format!("lab suite {} produced an invalid bundle", report.suite_id),
        });
    }

    if failures.is_empty() {
        Ok(LabBatchReportSummary {
            experiment_count: report.observations.len(),
            accepted_observation_count: report
                .observations
                .iter()
                .filter(|observation| observation.status == LabExperimentStatus::Accepted)
                .count(),
        })
    } else {
        Err(KnowledgeValidationError::new(failures))
    }
}

fn compact_summary(observation: &LabObservation, linked_claim_ids: &[String]) -> String {
    let entities = observation.observed_entity_ids.join(", ");
    let limits = if observation.limits.is_empty() {
        "no additional limits recorded".to_string()
    } else {
        observation.limits.join("; ")
    };
    format!(
        "{}. experiment={}; claims={}; observed={}; limits={}",
        observation.summary,
        observation.experiment_id,
        linked_claim_ids.join(", "),
        entities,
        limits
    )
}

fn contains_placeholder(value: &str) -> bool {
    let lower = value.to_lowercase();
    ["unknown", "todo", "stub", "inferred_only", "${"]
        .iter()
        .any(|marker| lower.contains(marker))
        || value.contains("<<<<<<<")
        || value.contains("=======")
        || value.contains(">>>>>>>")
}
