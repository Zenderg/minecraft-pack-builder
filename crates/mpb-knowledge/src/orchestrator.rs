use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use thiserror::Error;
use uuid::Uuid;

use crate::coverage::PARTIAL_EXTRACTION;
use crate::orchestrator_phases::{
    run_adapter_expansion_phase, run_drafting_phase, run_experiment_planning_phase,
    run_runtime_verification_phase,
};
use crate::release::{
    run_bundle_phase, run_patcher_integration_phase, run_product_validation_phase, ReleaseError,
};
use crate::reports::write_blocking_report_artifacts;
use crate::{
    evaluate_extraction_coverage, load_source_pack, persist_coverage_summary, run_preflight,
    validate_source_pack, ApprovalError, ApprovalGateError, ApprovalKind, ArtifactRef,
    BundleBuildError, CoverageBlocker, CoverageEvaluation, ExtractionDraft, KnowledgePackSource,
    KnowledgeRunPhase, KnowledgeRunStore, PhaseCheckpoint, PhaseCheckpointStatus, PreflightError,
    RunBlocker, RunBlockerInput, RunStateError, TargetError, TargetManager, WorkerRuntimeError,
};

const MISSING_LONG_RUN_APPROVAL: &str = "MISSING_LONG_RUN_APPROVAL";
const PHASE_NOT_IMPLEMENTED: &str = "PHASE_NOT_IMPLEMENTED";

#[derive(Debug, Error)]
pub enum OrchestratorError {
    #[error("run state operation failed: {0}")]
    RunState(#[from] RunStateError),
    #[error("preflight failed: {0}")]
    Preflight(#[from] PreflightError),
    #[error("target operation failed: {0}")]
    Target(#[from] TargetError),
    #[error("worker runtime failed: {0}")]
    WorkerRuntime(#[from] WorkerRuntimeError),
    #[error("release operation failed: {0}")]
    Release(#[from] ReleaseError),
    #[error("bundle operation failed: {0}")]
    Bundle(#[from] BundleBuildError),
    #[error("approval operation failed: {0}")]
    Approval(#[from] ApprovalError),
    #[error("approval gate failed: {0}")]
    ApprovalGate(#[from] ApprovalGateError),
    #[error("file operation failed: {0}")]
    Io(#[from] std::io::Error),
    #[error("json operation failed: {0}")]
    Json(#[from] serde_json::Error),
    #[error("run has no intake checkpoint with an instance path")]
    MissingIntakeInstancePath,
    #[error("run not found: {0}")]
    MissingRun(String),
}

#[derive(Debug, Clone)]
pub struct KnowledgeReleaseOrchestrator<R = DefaultPhaseRunner> {
    artifact_root: PathBuf,
    phase_runner: R,
}

impl KnowledgeReleaseOrchestrator<DefaultPhaseRunner> {
    pub fn new(artifact_root: impl AsRef<Path>) -> Self {
        Self {
            artifact_root: artifact_root.as_ref().to_path_buf(),
            phase_runner: DefaultPhaseRunner,
        }
    }
}

impl<R> KnowledgeReleaseOrchestrator<R>
where
    R: KnowledgePhaseRunner,
{
    pub fn with_phase_runner(artifact_root: impl AsRef<Path>, phase_runner: R) -> Self {
        Self {
            artifact_root: artifact_root.as_ref().to_path_buf(),
            phase_runner,
        }
    }

    pub fn start_release(
        &self,
        instance_path: impl AsRef<Path>,
        pack_id: &str,
    ) -> Result<OrchestratorOutcome, OrchestratorError> {
        let run_id = format!("run-{}", Uuid::new_v4());
        let instance_path = instance_path.as_ref();
        let store = KnowledgeRunStore::open(&self.artifact_root, &run_id)?;
        let detail = json!({
            "createdBy": "mpb-knowledge release start",
            "packId": pack_id,
            "instancePath": instance_path,
            "artifactRoot": self.artifact_root,
        });
        store.record_run(None, detail.clone())?;
        store.append_event("orchestrator.start", None, detail.clone())?;
        store.append_event(
            "phase.started",
            None,
            json!({"phase": KnowledgeRunPhase::Intake.as_str()}),
        )?;
        store.record_artifact_ref(
            "release-intake-instance",
            instance_path,
            None,
            json!({
                "packId": pack_id,
                "readOnly": true,
            }),
        )?;
        store.record_phase_checkpoint(
            KnowledgeRunPhase::Intake,
            PhaseCheckpointStatus::Succeeded,
            None,
            detail,
        )?;
        store.append_event(
            "phase.succeeded",
            None,
            json!({"phase": KnowledgeRunPhase::Intake.as_str()}),
        )?;
        drop(store);

        let preflight = self.run_next_required_phase(&run_id)?;
        if preflight.status != OrchestratorRunStatus::PhaseSucceeded {
            return Ok(preflight);
        }
        self.run_next_required_phase(&run_id)
    }

    pub fn run_next_required_phase(
        &self,
        run_id: &str,
    ) -> Result<OrchestratorOutcome, OrchestratorError> {
        let store = KnowledgeRunStore::open(&self.artifact_root, run_id)?;
        if store.run()?.is_none() {
            return Err(OrchestratorError::MissingRun(run_id.to_string()));
        }
        store.append_event(
            "orchestrator.resume",
            current_target_fingerprint(&store)?.as_deref(),
            json!({
                "latestSuccessfulPhase": latest_successful_phase(&store)?.map(|phase| phase.as_str()),
                "nextPhase": next_required_phase(&store)?.map(|phase| phase.as_str()),
            }),
        )?;
        let Some(phase) = next_required_phase(&store)? else {
            return Ok(OrchestratorOutcome::complete(run_id, None));
        };

        store.append_event("phase.started", None, json!({ "phase": phase.as_str() }))?;
        store.record_phase_checkpoint(
            phase,
            PhaseCheckpointStatus::Started,
            current_target_fingerprint(&store)?.as_deref(),
            json!({"phase": phase.as_str()}),
        )?;

        let context = PhaseRunContext {
            store: &store,
            artifact_root: &self.artifact_root,
        };
        match self.phase_runner.run_phase(&context, phase) {
            Ok(PhaseRunStatus::Succeeded {
                target_fingerprint,
                detail,
            }) => {
                if successful_checkpoint_for_phase(&store, phase)?.is_none() {
                    store.record_phase_checkpoint(
                        phase,
                        PhaseCheckpointStatus::Succeeded,
                        target_fingerprint.as_deref(),
                        detail,
                    )?;
                }
                store.append_event(
                    "phase.succeeded",
                    target_fingerprint.as_deref(),
                    json!({"phase": phase.as_str()}),
                )?;
                Ok(OrchestratorOutcome {
                    run_id: run_id.to_string(),
                    phase: Some(phase),
                    status: OrchestratorRunStatus::PhaseSucceeded,
                    next_phase: next_required_phase(&store)?,
                    blocking_report_path: None,
                })
            }
            Ok(PhaseRunStatus::Blocked { blocker }) => {
                let blocker = store.record_blocker(blocker)?;
                let report_path = write_blocking_report(&store, &blocker)?;
                store.record_artifact_ref(
                    "blocking-report",
                    &report_path,
                    blocker.target_fingerprint.as_deref(),
                    json!({
                        "format": "json",
                        "blockerId": blocker.id,
                        "phase": blocker.phase.map(|phase| phase.as_str()),
                    }),
                )?;
                store.record_phase_checkpoint(
                    phase,
                    PhaseCheckpointStatus::Failed,
                    blocker.target_fingerprint.as_deref(),
                    json!({
                        "blockerId": blocker.id,
                        "code": blocker.code,
                        "blockingReportPath": report_path,
                    }),
                )?;
                store.append_event(
                    "phase.failed",
                    blocker.target_fingerprint.as_deref(),
                    json!({
                        "phase": phase.as_str(),
                        "blockerId": blocker.id,
                        "code": blocker.code,
                    }),
                )?;
                Ok(OrchestratorOutcome {
                    run_id: run_id.to_string(),
                    phase: Some(phase),
                    status: OrchestratorRunStatus::Blocked,
                    next_phase: Some(phase),
                    blocking_report_path: Some(report_path),
                })
            }
            Err(error) => {
                let error_text = error.to_string();
                let target_fingerprint = current_target_fingerprint(&store).ok().flatten();
                let detail = json!({
                    "phase": phase.as_str(),
                    "error": error_text,
                });
                let _ = store.record_phase_checkpoint(
                    phase,
                    PhaseCheckpointStatus::Failed,
                    target_fingerprint.as_deref(),
                    detail.clone(),
                );
                let _ = store.append_event("phase.failed", target_fingerprint.as_deref(), detail);
                Err(error)
            }
        }
    }

    pub fn status(&self, run_id: &str) -> Result<ReleaseStatus, OrchestratorError> {
        let store = KnowledgeRunStore::open(&self.artifact_root, run_id)?;
        if store.run()?.is_none() {
            return Err(OrchestratorError::MissingRun(run_id.to_string()));
        }
        let latest_successful_phase = latest_successful_phase(&store)?;
        let next_phase = next_required_phase(&store)?;
        let blockers = store.blockers()?;
        let artifacts = store.artifact_refs()?;
        let target_fingerprint = current_target_fingerprint(&store)?;
        let approvals = ApprovalKind::ALL
            .into_iter()
            .map(|kind| {
                let history = store.approval_history(kind, target_fingerprint.as_deref())?;
                let latest = history.last();
                Ok(ApprovalStatus {
                    kind,
                    approved: latest.map(|record| record.approved).unwrap_or(false),
                    latest_reason: latest.map(|record| record.reason.clone()),
                    target_fingerprint: target_fingerprint.clone(),
                })
            })
            .collect::<Result<Vec<_>, ApprovalError>>()?;
        let next_command = next_command(run_id, &self.artifact_root, next_phase, &approvals);
        Ok(ReleaseStatus {
            run_id: run_id.to_string(),
            latest_successful_phase,
            next_phase,
            blockers,
            approvals,
            artifacts,
            next_command,
        })
    }
}

pub trait KnowledgePhaseRunner {
    fn run_phase(
        &self,
        context: &PhaseRunContext<'_>,
        phase: KnowledgeRunPhase,
    ) -> Result<PhaseRunStatus, OrchestratorError>;
}

#[derive(Debug, Clone, Copy, Default)]
pub struct DefaultPhaseRunner;

impl KnowledgePhaseRunner for DefaultPhaseRunner {
    fn run_phase(
        &self,
        context: &PhaseRunContext<'_>,
        phase: KnowledgeRunPhase,
    ) -> Result<PhaseRunStatus, OrchestratorError> {
        match phase {
            KnowledgeRunPhase::Intake => Ok(PhaseRunStatus::Succeeded {
                target_fingerprint: None,
                detail: json!({"source": "existing intake"}),
            }),
            KnowledgeRunPhase::Preflight => run_preflight_phase(context),
            KnowledgeRunPhase::Approvals => run_approvals_phase(context),
            KnowledgeRunPhase::Fingerprint => run_fingerprint_phase(context),
            KnowledgeRunPhase::Clone => run_clone_phase(context),
            KnowledgeRunPhase::Extraction => run_extraction_phase(context),
            KnowledgeRunPhase::Drafting => run_drafting_phase(context),
            KnowledgeRunPhase::ExperimentPlanning => run_experiment_planning_phase(context),
            KnowledgeRunPhase::AdapterExpansion => run_adapter_expansion_phase(context),
            KnowledgeRunPhase::RuntimeVerification => run_runtime_verification_phase(context),
            KnowledgeRunPhase::Validation => run_validation_phase(context),
            KnowledgeRunPhase::Bundle => run_bundle_phase(context),
            KnowledgeRunPhase::PatcherIntegration => run_patcher_integration_phase(context),
            KnowledgeRunPhase::ProductValidation => run_product_validation_phase(context),
            _ => Ok(PhaseRunStatus::Blocked {
                blocker: RunBlockerInput {
                    code: PHASE_NOT_IMPLEMENTED.to_string(),
                    phase: Some(phase),
                    target_fingerprint: current_target_fingerprint(context.store)?,
                    message: format!(
                        "{} is not implemented in the current knowledge release pipeline slice.",
                        phase.as_str()
                    ),
                    detail: json!({
                        "phase": phase.as_str(),
                        "plannedBy": "docs/superpowers/plans/2026-06-29-autonomous-knowledge-release-pipeline.md",
                        "resumeCommand": format!(
                            "mpb-knowledge release resume {} --artifact-root {}",
                            context.store.run_id(),
                            context.artifact_root.display()
                        ),
                    }),
                },
            }),
        }
    }
}

pub struct PhaseRunContext<'a> {
    pub store: &'a KnowledgeRunStore,
    pub artifact_root: &'a Path,
}

#[derive(Debug, Clone, PartialEq)]
pub enum PhaseRunStatus {
    Succeeded {
        target_fingerprint: Option<String>,
        detail: Value,
    },
    Blocked {
        blocker: RunBlockerInput,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OrchestratorRunStatus {
    PhaseSucceeded,
    Blocked,
    Complete,
}

impl OrchestratorRunStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            OrchestratorRunStatus::PhaseSucceeded => "PhaseSucceeded",
            OrchestratorRunStatus::Blocked => "Blocked",
            OrchestratorRunStatus::Complete => "Complete",
        }
    }
}

impl fmt::Display for OrchestratorRunStatus {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OrchestratorOutcome {
    pub run_id: String,
    pub phase: Option<KnowledgeRunPhase>,
    pub status: OrchestratorRunStatus,
    pub next_phase: Option<KnowledgeRunPhase>,
    pub blocking_report_path: Option<PathBuf>,
}

impl OrchestratorOutcome {
    fn complete(run_id: &str, phase: Option<KnowledgeRunPhase>) -> Self {
        Self {
            run_id: run_id.to_string(),
            phase,
            status: OrchestratorRunStatus::Complete,
            next_phase: None,
            blocking_report_path: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReleaseStatus {
    pub run_id: String,
    pub latest_successful_phase: Option<KnowledgeRunPhase>,
    pub next_phase: Option<KnowledgeRunPhase>,
    pub blockers: Vec<RunBlocker>,
    pub approvals: Vec<ApprovalStatus>,
    pub artifacts: Vec<ArtifactRef>,
    pub next_command: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApprovalStatus {
    pub kind: ApprovalKind,
    pub approved: bool,
    pub latest_reason: Option<String>,
    pub target_fingerprint: Option<String>,
}

fn run_preflight_phase(context: &PhaseRunContext<'_>) -> Result<PhaseRunStatus, OrchestratorError> {
    if let Some(existing) = context.store.latest_artifact_ref("preflight-report")? {
        if Path::new(&existing.path).is_file() {
            return Ok(PhaseRunStatus::Succeeded {
                target_fingerprint: existing.target_fingerprint,
                detail: serde_json::from_slice(&fs::read(&existing.path)?)?,
            });
        }
    }

    let instance_path = intake_instance_path(context.store)?;
    let report = run_preflight(&instance_path, context.artifact_root)?;
    let report_path = context.store.run_dir().join("preflight-report.json");
    fs::write(&report_path, serde_json::to_vec_pretty(&report)?)?;
    context.store.record_artifact_ref(
        "preflight-report",
        &report_path,
        None,
        json!({"format": "json"}),
    )?;
    Ok(PhaseRunStatus::Succeeded {
        target_fingerprint: None,
        detail: serde_json::to_value(report)?,
    })
}

fn run_approvals_phase(context: &PhaseRunContext<'_>) -> Result<PhaseRunStatus, OrchestratorError> {
    match context.store.require_approval(ApprovalKind::LongRun, None) {
        Ok(()) => Ok(PhaseRunStatus::Succeeded {
            target_fingerprint: current_target_fingerprint(context.store)?,
            detail: json!({
                "approvalKind": ApprovalKind::LongRun.as_str(),
                "approved": true,
            }),
        }),
        Err(error) => Ok(PhaseRunStatus::Blocked {
            blocker: RunBlockerInput {
                code: MISSING_LONG_RUN_APPROVAL.to_string(),
                phase: Some(KnowledgeRunPhase::Approvals),
                target_fingerprint: error.target_fingerprint.clone(),
                message: "Long-running release work requires explicit approval.".to_string(),
                detail: json!({
                    "approvalKind": ApprovalKind::LongRun.as_str(),
                    "latestReason": error.latest_reason,
                    "approvalCommand": format!(
                        "mpb-knowledge approve {} LongRun --artifact-root {} --reason <text>",
                        context.store.run_id(),
                        context.artifact_root.display()
                    ),
                }),
            },
        }),
    }
}

fn run_fingerprint_phase(
    context: &PhaseRunContext<'_>,
) -> Result<PhaseRunStatus, OrchestratorError> {
    if let Some(existing) = context.store.latest_artifact_ref("target-original")? {
        if let Some(fingerprint) = existing.target_fingerprint {
            return Ok(PhaseRunStatus::Succeeded {
                target_fingerprint: Some(fingerprint),
                detail: json!({
                    "idempotent": true,
                    "artifactId": existing.id,
                    "sourcePath": existing.path,
                    "detail": existing.detail,
                }),
            });
        }
    }

    let instance_path = intake_instance_path(context.store)?;
    let manager = TargetManager::new(context.artifact_root);
    let inspection = manager.inspect_original(&instance_path)?;
    let fingerprint = inspection.fingerprint.fingerprint.clone();
    context.store.record_run(
        Some(&fingerprint),
        json!({
            "createdBy": "mpb-knowledge release fingerprint",
            "instancePath": instance_path,
            "metadata": inspection.metadata,
        }),
    )?;
    context.store.record_artifact_ref(
        "target-original",
        &inspection.source_path,
        Some(&fingerprint),
        json!({
            "readOnly": true,
            "metadata": inspection.metadata,
        }),
    )?;
    Ok(PhaseRunStatus::Succeeded {
        target_fingerprint: Some(fingerprint),
        detail: serde_json::to_value(inspection.fingerprint)?,
    })
}

fn run_clone_phase(context: &PhaseRunContext<'_>) -> Result<PhaseRunStatus, OrchestratorError> {
    if let Some(existing) = context.store.latest_artifact_ref("target-clone")? {
        let clone_path = PathBuf::from(&existing.path);
        if clone_path.exists() {
            return Ok(PhaseRunStatus::Succeeded {
                target_fingerprint: existing.target_fingerprint,
                detail: json!({
                    "idempotent": true,
                    "clonePath": clone_path,
                    "artifactId": existing.id,
                }),
            });
        }
    }

    let instance_path = intake_instance_path(context.store)?;
    let manager = TargetManager::new(context.artifact_root);
    let inspection = manager.inspect_original(&instance_path)?;
    let current_fingerprint = inspection.fingerprint.fingerprint;
    if let Some(expected_fingerprint) = current_target_fingerprint(context.store)? {
        if expected_fingerprint != current_fingerprint {
            return Ok(PhaseRunStatus::Blocked {
                blocker: RunBlockerInput {
                    code: "TARGET_FINGERPRINT_CHANGED".to_string(),
                    phase: Some(KnowledgeRunPhase::Clone),
                    target_fingerprint: Some(expected_fingerprint.clone()),
                    message: "Target Prism instance changed after the Fingerprint phase."
                        .to_string(),
                    detail: json!({
                        "expectedFingerprint": expected_fingerprint,
                        "observedFingerprint": current_fingerprint,
                        "instancePath": instance_path,
                    }),
                },
            });
        }
    }
    let clone = manager.create_disposable_clone(context.store.run_id(), &instance_path)?;
    Ok(PhaseRunStatus::Succeeded {
        target_fingerprint: Some(clone.fingerprint_after.clone()),
        detail: serde_json::to_value(clone)?,
    })
}

fn run_extraction_phase(
    context: &PhaseRunContext<'_>,
) -> Result<PhaseRunStatus, OrchestratorError> {
    let Some(draft_ref) = context.store.latest_artifact_ref("extraction-draft")? else {
        return Ok(PhaseRunStatus::Blocked {
            blocker: RunBlockerInput {
                code: PARTIAL_EXTRACTION.to_string(),
                phase: Some(KnowledgeRunPhase::Extraction),
                target_fingerprint: current_target_fingerprint(context.store)?,
                message: "Extraction cannot continue without a persisted extraction draft."
                    .to_string(),
                detail: json!({
                    "requiredArtifactKind": "extraction-draft",
                    "resumeCommand": format!(
                        "mpb-knowledge release resume {} --artifact-root {}",
                        context.store.run_id(),
                        context.artifact_root.display()
                    ),
                }),
            },
        });
    };
    let target_fingerprint = current_target_fingerprint(context.store)?
        .or(draft_ref.target_fingerprint.clone())
        .unwrap_or_else(|| "unknown".to_string());
    let draft: ExtractionDraft = serde_json::from_slice(&fs::read(&draft_ref.path)?)?;
    let evaluation = evaluate_extraction_coverage(&draft, &target_fingerprint);
    let summary_artifact =
        persist_coverage_summary(context.store, KnowledgeRunPhase::Extraction, &evaluation)?;
    if let Some(blocker) = evaluation.blockers.first() {
        return Ok(PhaseRunStatus::Blocked {
            blocker: coverage_blocker_input(
                KnowledgeRunPhase::Extraction,
                Some(target_fingerprint),
                blocker,
                &evaluation,
                Some(&summary_artifact.path),
            ),
        });
    }
    Ok(PhaseRunStatus::Succeeded {
        target_fingerprint: Some(target_fingerprint),
        detail: json!({
            "coverageSummaryArtifact": summary_artifact.path,
            "summary": evaluation.summary,
        }),
    })
}

fn run_validation_phase(
    context: &PhaseRunContext<'_>,
) -> Result<PhaseRunStatus, OrchestratorError> {
    let Some(summary_ref) = context.store.latest_artifact_ref("coverage-summary")? else {
        return Ok(PhaseRunStatus::Blocked {
            blocker: RunBlockerInput {
                code: PARTIAL_EXTRACTION.to_string(),
                phase: Some(KnowledgeRunPhase::Validation),
                target_fingerprint: current_target_fingerprint(context.store)?,
                message: "Validation requires a persisted coverage summary from extraction."
                    .to_string(),
                detail: json!({"requiredArtifactKind": "coverage-summary"}),
            },
        });
    };
    let evaluation: CoverageEvaluation = serde_json::from_slice(&fs::read(&summary_ref.path)?)?;
    if let Some(blocker) = evaluation.blockers.first() {
        return Ok(PhaseRunStatus::Blocked {
            blocker: coverage_blocker_input(
                KnowledgeRunPhase::Validation,
                Some(evaluation.target_fingerprint.clone()),
                blocker,
                &evaluation,
                Some(&summary_ref.path),
            ),
        });
    }

    let Some((source_ref, pack)) = validation_source_pack(context.store)? else {
        return Ok(PhaseRunStatus::Blocked {
            blocker: RunBlockerInput {
                code: "VALIDATION_SOURCE_PACK_MISSING".to_string(),
                phase: Some(KnowledgeRunPhase::Validation),
                target_fingerprint: Some(evaluation.target_fingerprint.clone()),
                message: "Validation requires a persisted knowledge source artifact.".to_string(),
                detail: json!({
                    "acceptedArtifactKinds": ["knowledge-source-dir", "knowledge-source-pack"],
                    "coverageSummaryArtifact": summary_ref.path,
                }),
            },
        });
    };
    if let Err(error) = validate_source_pack(&pack) {
        let failure = error
            .failures()
            .first()
            .expect("validation error should contain at least one failure");
        return Ok(PhaseRunStatus::Blocked {
            blocker: RunBlockerInput {
                code: failure.code.as_str().to_ascii_uppercase(),
                phase: Some(KnowledgeRunPhase::Validation),
                target_fingerprint: Some(evaluation.target_fingerprint.clone()),
                message: failure.message.clone(),
                detail: json!({
                    "validationCode": failure.code.as_str(),
                    "sourcePackArtifact": source_ref.path,
                    "coverageSummaryArtifact": summary_ref.path,
                }),
            },
        });
    }

    let validation_artifact =
        persist_coverage_summary(context.store, KnowledgeRunPhase::Validation, &evaluation)?;
    Ok(PhaseRunStatus::Succeeded {
        target_fingerprint: Some(evaluation.target_fingerprint),
        detail: json!({
            "coverageSummaryArtifact": validation_artifact.path,
            "sourcePackValidation": "passed",
            "summary": evaluation.summary,
        }),
    })
}

fn validation_source_pack(
    store: &KnowledgeRunStore,
) -> Result<Option<(ArtifactRef, KnowledgePackSource)>, OrchestratorError> {
    if let Some(source_dir) = store.latest_artifact_ref("knowledge-source-dir")? {
        let path = PathBuf::from(&source_dir.path);
        if path.is_dir() {
            return Ok(Some((source_dir, load_source_pack(path)?)));
        }
    }
    let Some(source_pack) = store.latest_artifact_ref("knowledge-source-pack")? else {
        return Ok(None);
    };
    let pack: KnowledgePackSource = serde_json::from_slice(&fs::read(&source_pack.path)?)?;
    Ok(Some((source_pack, pack)))
}

fn coverage_blocker_input(
    phase: KnowledgeRunPhase,
    target_fingerprint: Option<String>,
    blocker: &CoverageBlocker,
    evaluation: &CoverageEvaluation,
    coverage_summary_artifact: Option<&str>,
) -> RunBlockerInput {
    RunBlockerInput {
        code: blocker.code.clone(),
        phase: Some(phase),
        target_fingerprint,
        message: blocker.message.clone(),
        detail: json!({
            "obligationId": blocker.obligation_id.clone(),
            "affectedEvidenceIds": blocker.affected_evidence_ids.clone(),
            "coverageSummaryArtifact": coverage_summary_artifact,
            "summary": evaluation.summary.clone(),
            "blockers": evaluation.blockers.clone(),
        }),
    }
}

fn write_blocking_report(
    store: &KnowledgeRunStore,
    blocker: &RunBlocker,
) -> Result<PathBuf, OrchestratorError> {
    let paths = write_blocking_report_artifacts(store, blocker)?;
    Ok(PathBuf::from(paths.json_path))
}

fn intake_instance_path(store: &KnowledgeRunStore) -> Result<PathBuf, OrchestratorError> {
    let checkpoint = successful_checkpoint_for_phase(store, KnowledgeRunPhase::Intake)?
        .ok_or(OrchestratorError::MissingIntakeInstancePath)?;
    checkpoint
        .detail
        .get("instancePath")
        .and_then(Value::as_str)
        .map(PathBuf::from)
        .ok_or(OrchestratorError::MissingIntakeInstancePath)
}

pub(crate) fn current_target_fingerprint(
    store: &KnowledgeRunStore,
) -> Result<Option<String>, OrchestratorError> {
    if let Some(run) = store.run()? {
        if run.target_fingerprint.is_some() {
            return Ok(run.target_fingerprint);
        }
    }
    Ok(
        successful_checkpoint_for_phase(store, KnowledgeRunPhase::Fingerprint)?
            .and_then(|checkpoint| checkpoint.target_fingerprint),
    )
}

fn latest_successful_phase(
    store: &KnowledgeRunStore,
) -> Result<Option<KnowledgeRunPhase>, OrchestratorError> {
    let checkpoints = store.phase_checkpoints()?;
    let mut latest = None;
    for phase in KnowledgeRunPhase::ALL {
        if checkpoints.iter().any(|checkpoint| {
            checkpoint.phase == phase && checkpoint.status == PhaseCheckpointStatus::Succeeded
        }) {
            latest = Some(phase);
        } else {
            break;
        }
    }
    Ok(latest)
}

fn next_required_phase(
    store: &KnowledgeRunStore,
) -> Result<Option<KnowledgeRunPhase>, OrchestratorError> {
    let checkpoints = store.phase_checkpoints()?;
    Ok(KnowledgeRunPhase::ALL.into_iter().find(|phase| {
        !checkpoints.iter().any(|checkpoint| {
            checkpoint.phase == *phase && checkpoint.status == PhaseCheckpointStatus::Succeeded
        })
    }))
}

fn successful_checkpoint_for_phase(
    store: &KnowledgeRunStore,
    phase: KnowledgeRunPhase,
) -> Result<Option<PhaseCheckpoint>, OrchestratorError> {
    Ok(store
        .phase_checkpoints()?
        .into_iter()
        .rev()
        .find(|checkpoint| {
            checkpoint.phase == phase && checkpoint.status == PhaseCheckpointStatus::Succeeded
        }))
}

fn next_command(
    run_id: &str,
    artifact_root: &Path,
    next_phase: Option<KnowledgeRunPhase>,
    approvals: &[ApprovalStatus],
) -> String {
    let needs_long_run = next_phase == Some(KnowledgeRunPhase::Approvals)
        && approvals
            .iter()
            .find(|approval| approval.kind == ApprovalKind::LongRun)
            .map(|approval| !approval.approved)
            .unwrap_or(true);
    if needs_long_run {
        return format!(
            "mpb-knowledge approve {run_id} LongRun --artifact-root {} --reason <text>",
            artifact_root.display()
        );
    }
    match next_phase {
        Some(_) => format!(
            "mpb-knowledge release resume {run_id} --artifact-root {}",
            artifact_root.display()
        ),
        None => "release pipeline complete".to_string(),
    }
}
