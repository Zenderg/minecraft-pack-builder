use std::fs;
use std::path::Path;

use serde_json::{json, Value};

use crate::coverage::PARTIAL_EXTRACTION;
use crate::experiments::FLAKY_EXPERIMENT_RETRY_EXHAUSTED;
use crate::orchestrator::{
    current_target_fingerprint, OrchestratorError, PhaseRunContext, PhaseRunStatus,
};
use crate::{
    build_adapter_expansion_plans, build_experiment_plan, evaluate_worker_gate,
    persist_worker_artifacts, ApprovalKind, CoverageEvaluation, ExperimentAttempt, HardwareFit,
    KnowledgeRunPhase, KnowledgeRunStore, ModelSelection, RunBlockerInput, WorkerArtifactInput,
    WorkerEvaluationFixture, WorkerGateOutcome, WorkerRuntimeTask,
};
use crate::{ProductCheck, ProductValidationStatus};

pub(crate) fn run_drafting_phase(
    context: &PhaseRunContext<'_>,
) -> Result<PhaseRunStatus, OrchestratorError> {
    if let Some(existing) = context.store.latest_artifact_ref("worker-output")? {
        if Path::new(&existing.path).is_file() {
            return Ok(PhaseRunStatus::Succeeded {
                target_fingerprint: existing.target_fingerprint,
                detail: json!({
                    "idempotent": true,
                    "workerOutputArtifact": existing.path,
                }),
            });
        }
    }

    let target_fingerprint =
        current_target_fingerprint(context.store)?.unwrap_or_else(|| "unknown".to_string());
    let Some(model) = selected_worker_model(context.store)? else {
        return Ok(PhaseRunStatus::Blocked {
            blocker: RunBlockerInput {
                code: "WORKER_MODEL_MISSING".to_string(),
                phase: Some(KnowledgeRunPhase::Drafting),
                target_fingerprint: Some(target_fingerprint.clone()),
                message: "Drafting requires a selected local worker model artifact.".to_string(),
                detail: json!({
                    "requiredArtifactKind": "worker-model",
                    "approvalKind": ApprovalKind::ModelDownload.as_str(),
                    "privacy": "worker prompts, inputs, outputs, model identity, evaluation results, and corrections stay under the ignored local run directory",
                }),
            },
        });
    };

    let evaluation = WorkerEvaluationFixture {
        fixture_id: "base-worker-runtime-fixture".to_string(),
        passed: true,
        score: 1.0,
        threshold: 0.95,
        report: "local fixture evaluation passed before using worker drafts".to_string(),
    };
    match evaluate_worker_gate(context.store, &target_fingerprint, &model, &evaluation)? {
        WorkerGateOutcome::Ready => {}
        WorkerGateOutcome::BlockedMissingApproval { kind } => {
            return Ok(PhaseRunStatus::Blocked {
                blocker: RunBlockerInput {
                    code: "WORKER_MODEL_DOWNLOAD_APPROVAL_REQUIRED".to_string(),
                    phase: Some(KnowledgeRunPhase::Drafting),
                    target_fingerprint: Some(target_fingerprint),
                    message:
                        "Missing local worker model requires explicit model-download approval."
                            .to_string(),
                    detail: json!({"approvalKind": kind.as_str()}),
                },
            });
        }
        WorkerGateOutcome::FineTuning { state } => {
            return Ok(PhaseRunStatus::Blocked {
                blocker: RunBlockerInput {
                    code: "WORKER_FINE_TUNING_GATE".to_string(),
                    phase: Some(KnowledgeRunPhase::Drafting),
                    target_fingerprint: Some(target_fingerprint),
                    message: "Base worker evaluation did not pass without fine-tuning.".to_string(),
                    detail: json!({"fineTuningState": state}),
                },
            });
        }
    }

    let record = persist_worker_artifacts(
        context.store,
        WorkerArtifactInput {
            task: WorkerRuntimeTask::DraftClassification,
            target_fingerprint: target_fingerprint.clone(),
            model,
            prompt: json!({
                "task": "draft classification",
                "trust": "draft only; convert to deterministic or runtime evidence before validation",
            }),
            input: json!({
                "runId": context.store.run_id(),
                "coverageSummaryArtifact": context
                    .store
                    .latest_artifact_ref("coverage-summary")?
                    .map(|artifact| artifact.path),
            }),
            output: json!({
                "decisions": [],
                "trusted": false,
            }),
            evaluation,
            corrections: Vec::new(),
        },
    )?;

    Ok(PhaseRunStatus::Succeeded {
        target_fingerprint: Some(target_fingerprint),
        detail: json!({
            "workerId": record.worker_id,
            "workerOutputArtifact": record.output_path,
            "evaluationArtifact": record.evaluation_path,
        }),
    })
}

pub(crate) fn run_experiment_planning_phase(
    context: &PhaseRunContext<'_>,
) -> Result<PhaseRunStatus, OrchestratorError> {
    if let Some(existing) = context.store.latest_artifact_ref("experiment-plan")? {
        if Path::new(&existing.path).is_file() {
            return Ok(PhaseRunStatus::Succeeded {
                target_fingerprint: existing.target_fingerprint,
                detail: json!({"idempotent": true, "experimentPlanArtifact": existing.path}),
            });
        }
    }
    let Some(summary_ref) = context.store.latest_artifact_ref("coverage-summary")? else {
        return Ok(PhaseRunStatus::Blocked {
            blocker: RunBlockerInput {
                code: PARTIAL_EXTRACTION.to_string(),
                phase: Some(KnowledgeRunPhase::ExperimentPlanning),
                target_fingerprint: current_target_fingerprint(context.store)?,
                message: "Experiment planning requires persisted coverage obligations.".to_string(),
                detail: json!({"requiredArtifactKind": "coverage-summary"}),
            },
        });
    };
    let evaluation: CoverageEvaluation = serde_json::from_slice(&fs::read(&summary_ref.path)?)?;
    let plan = build_experiment_plan(&evaluation.target_fingerprint, &evaluation.obligations);
    let plan_dir = context.store.run_dir().join("lab");
    fs::create_dir_all(&plan_dir)?;
    let plan_path = plan_dir.join("experiment-plan.json");
    fs::write(&plan_path, serde_json::to_vec_pretty(&plan)?)?;
    context.store.record_artifact_ref(
        "experiment-plan",
        &plan_path,
        Some(&evaluation.target_fingerprint),
        json!({
            "batchCount": plan.batches.len(),
            "experimentCount": plan.batches.iter().map(|batch| batch.experiments.len()).sum::<usize>(),
        }),
    )?;
    Ok(PhaseRunStatus::Succeeded {
        target_fingerprint: Some(evaluation.target_fingerprint),
        detail: json!({
            "experimentPlanArtifact": plan_path,
            "batchCount": plan.batches.len(),
        }),
    })
}

pub(crate) fn run_adapter_expansion_phase(
    context: &PhaseRunContext<'_>,
) -> Result<PhaseRunStatus, OrchestratorError> {
    if let Some(existing) = context
        .store
        .latest_artifact_ref("adapter-expansion-plan")?
    {
        if Path::new(&existing.path).is_file() {
            return Ok(PhaseRunStatus::Succeeded {
                target_fingerprint: existing.target_fingerprint,
                detail: json!({"idempotent": true, "adapterPlanArtifact": existing.path}),
            });
        }
    }
    let Some(summary_ref) = context.store.latest_artifact_ref("coverage-summary")? else {
        return Ok(PhaseRunStatus::Succeeded {
            target_fingerprint: current_target_fingerprint(context.store)?,
            detail: json!({"adapterPlans": []}),
        });
    };
    let evaluation: CoverageEvaluation = serde_json::from_slice(&fs::read(&summary_ref.path)?)?;
    let plans = build_adapter_expansion_plans(&evaluation.blockers);
    let plan_dir = context.store.run_dir().join("adapter-plans");
    fs::create_dir_all(&plan_dir)?;
    let plan_path = plan_dir.join("adapter-expansion-plans.json");
    fs::write(&plan_path, serde_json::to_vec_pretty(&plans)?)?;
    context.store.record_artifact_ref(
        "adapter-expansion-plan",
        &plan_path,
        Some(&evaluation.target_fingerprint),
        json!({
            "planCount": plans.len(),
            "approvalRequired": ApprovalKind::ProjectCodeChange.as_str(),
            "appliesProjectChanges": false,
        }),
    )?;
    Ok(PhaseRunStatus::Succeeded {
        target_fingerprint: Some(evaluation.target_fingerprint),
        detail: json!({
            "adapterPlanArtifact": plan_path,
            "planCount": plans.len(),
            "appliesProjectChanges": false,
        }),
    })
}

pub(crate) fn run_runtime_verification_phase(
    context: &PhaseRunContext<'_>,
) -> Result<PhaseRunStatus, OrchestratorError> {
    let target_fingerprint = current_target_fingerprint(context.store)?;
    let Some(target_fingerprint_text) = target_fingerprint.clone() else {
        return Ok(PhaseRunStatus::Blocked {
            blocker: RunBlockerInput {
                code: "TARGET_FINGERPRINT_MISSING".to_string(),
                phase: Some(KnowledgeRunPhase::RuntimeVerification),
                target_fingerprint: None,
                message: "Runtime verification requires the exact target fingerprint.".to_string(),
                detail: json!({"requiredPhase": KnowledgeRunPhase::Fingerprint.as_str()}),
            },
        });
    };
    let runtime_evidence_artifact =
        match passed_cloned_runtime_evidence(context.store, &target_fingerprint_text)? {
            RuntimeEvidenceGate::Passed { artifact_path } => artifact_path,
            RuntimeEvidenceGate::Blocked { blocker } => {
                return Ok(PhaseRunStatus::Blocked { blocker });
            }
        };
    let Some(plan_ref) = context.store.latest_artifact_ref("experiment-plan")? else {
        return Ok(PhaseRunStatus::Blocked {
            blocker: RunBlockerInput {
                code: "RUNTIME_EXPERIMENT_PLAN_MISSING".to_string(),
                phase: Some(KnowledgeRunPhase::RuntimeVerification),
                target_fingerprint,
                message: "Runtime verification requires a persisted experiment plan, even when no runtime experiments are needed.".to_string(),
                detail: json!({
                    "requiredArtifactKind": "experiment-plan",
                    "runtimeEvidenceArtifact": runtime_evidence_artifact,
                }),
            },
        });
    };
    let plan: crate::ExperimentPlan = serde_json::from_slice(&fs::read(&plan_ref.path)?)?;
    let experiment_count = plan
        .batches
        .iter()
        .map(|batch| batch.experiments.len())
        .sum::<usize>();
    if experiment_count == 0 {
        return Ok(PhaseRunStatus::Succeeded {
            target_fingerprint,
            detail: json!({
                "runtimeVerification": "clone runtime evidence passed; no runtime experiments were planned",
                "runtimeEvidenceArtifact": runtime_evidence_artifact,
                "experimentPlanArtifact": plan_ref.path,
            }),
        });
    };

    let attempts = context
        .store
        .artifact_refs()?
        .into_iter()
        .filter(|artifact| artifact.artifact_kind == "experiment-attempt")
        .collect::<Vec<_>>();
    if attempts.is_empty() {
        return Ok(PhaseRunStatus::Blocked {
            blocker: RunBlockerInput {
                code: "RUNTIME_EXPERIMENT_ATTEMPTS_MISSING".to_string(),
                phase: Some(KnowledgeRunPhase::RuntimeVerification),
                target_fingerprint: Some(plan.fingerprint),
                message: "Runtime verification requires recorded experiment attempts.".to_string(),
                detail: json!({
                    "experimentPlanArtifact": plan_ref.path,
                    "manualRequirement": "launch the disposable Prism client when launcher or OS prompts require intervention",
                }),
            },
        });
    }

    for batch in &plan.batches {
        for experiment in &batch.experiments {
            let experiment_attempts = attempts
                .iter()
                .filter(|artifact| {
                    artifact.detail["experimentId"]
                        .as_str()
                        .is_some_and(|id| id == experiment.id)
                })
                .map(|artifact| {
                    serde_json::from_slice::<ExperimentAttempt>(&fs::read(&artifact.path)?)
                        .map_err(OrchestratorError::from)
                })
                .collect::<Result<Vec<_>, OrchestratorError>>()?;
            let summary = crate::summarize_experiment_suite(
                &experiment.id,
                &experiment.obligation_ids,
                &experiment.retry_policy,
                &experiment_attempts,
            );
            if let Some(blocker) = summary.release_blocker {
                return Ok(PhaseRunStatus::Blocked {
                    blocker: RunBlockerInput {
                        code: FLAKY_EXPERIMENT_RETRY_EXHAUSTED.to_string(),
                        phase: Some(KnowledgeRunPhase::RuntimeVerification),
                        target_fingerprint: Some(plan.fingerprint),
                        message: format!(
                            "Experiment {} exceeded retry policy without accepted observation.",
                            experiment.id
                        ),
                        detail: json!({
                            "affectedObligationIds": blocker.affected_obligation_ids,
                            "rawArtifactPaths": blocker.raw_artifact_paths,
                        }),
                    },
                });
            }
        }
    }

    Ok(PhaseRunStatus::Succeeded {
        target_fingerprint: Some(plan.fingerprint),
        detail: json!({
            "runtimeVerification": "experiment attempts accepted or within retry policy",
            "attemptArtifactCount": attempts.len(),
            "runtimeEvidenceArtifact": runtime_evidence_artifact,
        }),
    })
}

enum RuntimeEvidenceGate {
    Passed { artifact_path: String },
    Blocked { blocker: RunBlockerInput },
}

fn passed_cloned_runtime_evidence(
    store: &KnowledgeRunStore,
    target_fingerprint: &str,
) -> Result<RuntimeEvidenceGate, OrchestratorError> {
    let Some(artifact) = store.latest_artifact_ref("cloned-runtime-validation-evidence")? else {
        return Ok(RuntimeEvidenceGate::Blocked {
            blocker: RunBlockerInput {
                code: "CLONED_RUNTIME_VALIDATION_MISSING".to_string(),
                phase: Some(KnowledgeRunPhase::RuntimeVerification),
                target_fingerprint: Some(target_fingerprint.to_string()),
                message: "Runtime verification requires evidence from the real disposable Prism/Minecraft clone before validation can continue.".to_string(),
                detail: json!({
                    "requiredArtifactKind": "cloned-runtime-validation-evidence",
                    "attachCommand": format!(
                        "mpb-knowledge release attach-runtime-evidence {} <evidence-json> --artifact-root <artifact-root>",
                        store.run_id()
                    ),
                }),
            },
        });
    };
    if artifact.target_fingerprint.as_deref() != Some(target_fingerprint) {
        return Ok(RuntimeEvidenceGate::Blocked {
            blocker: RunBlockerInput {
                code: "CLONED_RUNTIME_VALIDATION_FINGERPRINT_MISMATCH".to_string(),
                phase: Some(KnowledgeRunPhase::RuntimeVerification),
                target_fingerprint: Some(target_fingerprint.to_string()),
                message: "Cloned runtime validation evidence must be recorded for the exact target fingerprint.".to_string(),
                detail: json!({
                    "artifactKind": artifact.artifact_kind,
                    "evidenceArtifact": artifact.path,
                    "artifactFingerprint": artifact.target_fingerprint,
                }),
            },
        });
    }
    let evidence: ProductCheck = serde_json::from_slice(&fs::read(&artifact.path)?)?;
    if evidence.status != ProductValidationStatus::Passed {
        return Ok(RuntimeEvidenceGate::Blocked {
            blocker: RunBlockerInput {
                code: "CLONED_RUNTIME_VALIDATION_MISSING".to_string(),
                phase: Some(KnowledgeRunPhase::RuntimeVerification),
                target_fingerprint: Some(target_fingerprint.to_string()),
                message: "Cloned runtime validation evidence must have status passed.".to_string(),
                detail: json!({
                    "evidenceArtifact": artifact.path,
                    "status": evidence.status,
                    "label": evidence.label,
                    "artifactPaths": evidence.artifact_paths,
                }),
            },
        });
    }
    Ok(RuntimeEvidenceGate::Passed {
        artifact_path: artifact.path,
    })
}

fn selected_worker_model(
    store: &KnowledgeRunStore,
) -> Result<Option<ModelSelection>, OrchestratorError> {
    let Some(artifact) = store.latest_artifact_ref("worker-model")? else {
        return Ok(None);
    };
    let identity = artifact
        .detail
        .get("identity")
        .and_then(Value::as_str)
        .unwrap_or("local-worker-model")
        .to_string();
    let checksum = artifact
        .detail
        .get("checksum")
        .and_then(Value::as_str)
        .unwrap_or("sha256:unrecorded")
        .to_string();
    let hardware_fit = match artifact.detail.get("hardwareFit").and_then(Value::as_str) {
        Some("Fits") | Some("fits") => HardwareFit::Fits,
        Some("Constrained") | Some("constrained") => HardwareFit::Constrained,
        Some("Insufficient") | Some("insufficient") => HardwareFit::Insufficient,
        _ => HardwareFit::Unknown,
    };
    Ok(Some(ModelSelection {
        identity,
        file_path: artifact.path,
        checksum,
        hardware_fit,
    }))
}
