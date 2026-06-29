use std::fs;

use mpb_knowledge::{
    evaluate_worker_gate, persist_worker_artifacts, ApprovalKind, CoverageEvaluation,
    FineTuningPhaseState, HardwareFit, KnowledgeRunPhase, KnowledgeRunStore, ModelSelection,
    ObligationCoverageSummary, WorkerArtifactInput, WorkerEvaluationFixture, WorkerGateOutcome,
    WorkerOutputEnvelope, WorkerRuntimeTask, WorkerTaskKind,
};
use serde_json::json;

#[test]
fn worker_artifacts_are_persisted_under_run_workers_with_database_refs() {
    let temp = tempfile::tempdir().expect("tempdir");
    let artifact_root = temp.path().join("knowledge");
    let store = KnowledgeRunStore::open(&artifact_root, "run-worker").expect("open store");
    store
        .record_run(
            Some("fingerprint-worker"),
            json!({"createdBy": "worker runtime test"}),
        )
        .expect("record run");

    let record = persist_worker_artifacts(
        &store,
        WorkerArtifactInput {
            task: WorkerRuntimeTask::DraftClassification,
            target_fingerprint: "fingerprint-worker".to_string(),
            model: ModelSelection {
                identity: "local-qwen".to_string(),
                file_path: "knowledge/model-cache/qwen.gguf".to_string(),
                checksum: "sha256:abc123".to_string(),
                hardware_fit: HardwareFit::Fits,
            },
            prompt: json!({"system": "classify local draft records"}),
            input: json!({"records": ["create:depot"]}),
            output: json!({"classification": "mechanic"}),
            evaluation: WorkerEvaluationFixture {
                fixture_id: "fixture-classifier".to_string(),
                passed: true,
                score: 1.0,
                threshold: 0.95,
                report: "fixture matched expected schema".to_string(),
            },
            corrections: vec!["normalize mechanic ids before validation".to_string()],
        },
    )
    .expect("persist worker artifacts");

    assert!(record
        .prompt_path
        .ends_with("workers/worker-0001/prompt.json"));
    assert!(fs::metadata(&record.prompt_path)
        .expect("prompt file")
        .is_file());
    assert!(fs::metadata(&record.input_path)
        .expect("input file")
        .is_file());
    assert!(fs::metadata(&record.output_path)
        .expect("output file")
        .is_file());
    assert!(fs::metadata(&record.evaluation_path)
        .expect("evaluation file")
        .is_file());
    assert_eq!(record.envelope.model, "local-qwen");
    assert_eq!(record.envelope.model_checksum, "sha256:abc123");
    assert_eq!(
        store
            .artifact_refs()
            .expect("artifact refs")
            .into_iter()
            .filter(|artifact| artifact.artifact_kind.starts_with("worker-"))
            .count(),
        6
    );
    assert!(store
        .events()
        .expect("events")
        .iter()
        .any(|event| event.event_kind == "worker.artifacts.persisted"));
}

#[test]
fn worker_gate_requires_model_download_approval_before_missing_model_can_be_used() {
    let temp = tempfile::tempdir().expect("tempdir");
    let store = KnowledgeRunStore::open(temp.path(), "run-worker-download").expect("open store");
    let outcome = evaluate_worker_gate(
        &store,
        "fingerprint-worker",
        &ModelSelection {
            identity: "local-qwen".to_string(),
            file_path: temp.path().join("missing-model.gguf").display().to_string(),
            checksum: "sha256:missing".to_string(),
            hardware_fit: HardwareFit::Fits,
        },
        &WorkerEvaluationFixture {
            fixture_id: "fixture".to_string(),
            passed: true,
            score: 1.0,
            threshold: 0.9,
            report: "passed".to_string(),
        },
    )
    .expect("evaluate worker gate");

    assert_eq!(
        outcome,
        WorkerGateOutcome::BlockedMissingApproval {
            kind: ApprovalKind::ModelDownload,
        }
    );
}

#[test]
fn base_evaluation_failure_proposes_fine_tuning_without_running_it() {
    let temp = tempfile::tempdir().expect("tempdir");
    let model_path = temp.path().join("model.gguf");
    fs::write(&model_path, b"fixture model").expect("write model");
    let store = KnowledgeRunStore::open(temp.path(), "run-worker-eval").expect("open store");

    let outcome = evaluate_worker_gate(
        &store,
        "fingerprint-worker",
        &ModelSelection {
            identity: "local-qwen".to_string(),
            file_path: model_path.display().to_string(),
            checksum: "sha256:model".to_string(),
            hardware_fit: HardwareFit::Fits,
        },
        &WorkerEvaluationFixture {
            fixture_id: "fixture".to_string(),
            passed: false,
            score: 0.42,
            threshold: 0.9,
            report: "schema repair failed".to_string(),
        },
    )
    .expect("evaluate worker gate");

    assert_eq!(
        outcome,
        WorkerGateOutcome::FineTuning {
            state: FineTuningPhaseState::ProposedBecauseBaseEvaluationFailed,
        }
    );
    assert!(store
        .require_approval(ApprovalKind::FineTuning, Some("fingerprint-worker"))
        .is_err());
}

#[test]
fn fine_tuning_cannot_run_without_approval_and_hardware_fit() {
    let temp = tempfile::tempdir().expect("tempdir");
    let model_path = temp.path().join("model.gguf");
    fs::write(&model_path, b"fixture model").expect("write model");
    let store = KnowledgeRunStore::open(temp.path(), "run-worker-fine-tune").expect("open store");

    let missing_approval = evaluate_worker_gate(
        &store,
        "fingerprint-worker",
        &ModelSelection {
            identity: "local-qwen".to_string(),
            file_path: model_path.display().to_string(),
            checksum: "sha256:model".to_string(),
            hardware_fit: HardwareFit::Fits,
        },
        &WorkerEvaluationFixture {
            fixture_id: "fixture".to_string(),
            passed: false,
            score: 0.42,
            threshold: 0.9,
            report: "schema repair failed".to_string(),
        },
    )
    .expect("evaluate gate");
    assert_eq!(
        missing_approval,
        WorkerGateOutcome::FineTuning {
            state: FineTuningPhaseState::ProposedBecauseBaseEvaluationFailed,
        }
    );

    store
        .record_approval(
            ApprovalKind::FineTuning,
            Some("fingerprint-worker"),
            true,
            "operator approved local fine tuning",
            json!({}),
        )
        .expect("approve fine tuning");
    let blocked_by_hardware = evaluate_worker_gate(
        &store,
        "fingerprint-worker",
        &ModelSelection {
            identity: "local-qwen".to_string(),
            file_path: model_path.display().to_string(),
            checksum: "sha256:model".to_string(),
            hardware_fit: HardwareFit::Insufficient,
        },
        &WorkerEvaluationFixture {
            fixture_id: "fixture".to_string(),
            passed: false,
            score: 0.42,
            threshold: 0.9,
            report: "schema repair failed".to_string(),
        },
    )
    .expect("evaluate gate");

    assert_eq!(
        blocked_by_hardware,
        WorkerGateOutcome::FineTuning {
            state: FineTuningPhaseState::BlockedByHardware,
        }
    );
}

#[test]
fn worker_output_envelope_supports_all_runtime_tasks_and_untrusted_state() {
    let envelope = WorkerOutputEnvelope::with_model_checksum(
        "worker-repair-001",
        WorkerTaskKind::StructuredRepairSuggestion,
        "local-qwen",
        "sha256:model",
        "fingerprint-worker",
        "knowledge/runs/run-worker/workers/worker-0001/prompt.json",
        "knowledge/runs/run-worker/workers/worker-0001/output.json",
        FineTuningPhaseState::NotUsed,
    )
    .expect("schema repair envelope");

    assert_eq!(
        envelope.task_kind,
        WorkerTaskKind::StructuredRepairSuggestion
    );
    assert!(!envelope.is_source_of_truth());
}

#[test]
fn orchestrator_drafting_phase_persists_worker_artifacts_and_checkpoint() {
    let temp = tempfile::tempdir().expect("tempdir");
    let artifact_root = temp.path().join("knowledge");
    let run_id = "run-drafting";
    let store = seed_successful_phases(
        &artifact_root,
        run_id,
        &[
            KnowledgeRunPhase::Intake,
            KnowledgeRunPhase::Preflight,
            KnowledgeRunPhase::Approvals,
            KnowledgeRunPhase::Fingerprint,
            KnowledgeRunPhase::Clone,
            KnowledgeRunPhase::Extraction,
        ],
    );
    let model_path = store.run_dir().join("fixture-worker-model.gguf");
    fs::write(&model_path, b"fixture model").expect("write model");
    store
        .record_artifact_ref(
            "worker-model",
            &model_path,
            Some("fingerprint-worker"),
            json!({
                "identity": "local-fixture-worker",
                "checksum": "sha256:fixture",
                "hardwareFit": "Fits"
            }),
        )
        .expect("record worker model");
    drop(store);

    let outcome = mpb_knowledge::KnowledgeReleaseOrchestrator::new(&artifact_root)
        .run_next_required_phase(run_id)
        .expect("run drafting");

    assert_eq!(outcome.phase, Some(KnowledgeRunPhase::Drafting));
    assert_eq!(outcome.status.as_str(), "PhaseSucceeded");
    let reopened = KnowledgeRunStore::open(&artifact_root, run_id).expect("open store");
    assert!(reopened
        .latest_artifact_ref("worker-output")
        .expect("worker output artifact")
        .is_some());
}

#[test]
fn orchestrator_drafting_phase_requires_worker_model_even_when_deterministic_coverage_is_complete()
{
    let temp = tempfile::tempdir().expect("tempdir");
    let artifact_root = temp.path().join("knowledge");
    let run_id = "run-drafting-skip";
    let store = seed_successful_phases(
        &artifact_root,
        run_id,
        &[
            KnowledgeRunPhase::Intake,
            KnowledgeRunPhase::Preflight,
            KnowledgeRunPhase::Approvals,
            KnowledgeRunPhase::Fingerprint,
            KnowledgeRunPhase::Clone,
            KnowledgeRunPhase::Extraction,
        ],
    );
    let coverage_path = store.run_dir().join("coverage/Extraction-summary.json");
    fs::create_dir_all(coverage_path.parent().expect("coverage parent")).expect("coverage dir");
    fs::write(
        &coverage_path,
        serde_json::to_vec_pretty(&CoverageEvaluation {
            target_fingerprint: "fingerprint-worker".to_string(),
            obligations: Vec::new(),
            blockers: Vec::new(),
            summary: ObligationCoverageSummary {
                total_obligations: 0,
                covered_obligations: 0,
                deterministic_obligations: 0,
                runtime_obligations: 0,
                blocker_count: 0,
            },
        })
        .expect("coverage json"),
    )
    .expect("write coverage");
    store
        .record_artifact_ref(
            "coverage-summary",
            &coverage_path,
            Some("fingerprint-worker"),
            json!({"blockerCount": 0, "totalObligations": 0, "coveredObligations": 0}),
        )
        .expect("record coverage");
    drop(store);

    let outcome = mpb_knowledge::KnowledgeReleaseOrchestrator::new(&artifact_root)
        .run_next_required_phase(run_id)
        .expect("run drafting");

    assert_eq!(outcome.phase, Some(KnowledgeRunPhase::Drafting));
    assert_eq!(outcome.status.as_str(), "Blocked");
    let reopened = KnowledgeRunStore::open(&artifact_root, run_id).expect("open store");
    assert!(reopened
        .blockers()
        .expect("blockers")
        .into_iter()
        .any(|blocker| blocker.code == "WORKER_MODEL_MISSING"));
}

fn seed_successful_phases(
    artifact_root: &std::path::Path,
    run_id: &str,
    phases: &[KnowledgeRunPhase],
) -> KnowledgeRunStore {
    let store = KnowledgeRunStore::open(artifact_root, run_id).expect("open store");
    store
        .record_run(
            Some("fingerprint-worker"),
            json!({"createdBy": "worker runtime test"}),
        )
        .expect("record run");
    for phase in phases {
        store
            .record_phase_checkpoint(
                *phase,
                mpb_knowledge::PhaseCheckpointStatus::Succeeded,
                Some("fingerprint-worker"),
                json!({
                    "seeded": true,
                    "phase": phase.as_str(),
                    "instancePath": artifact_root.join("missing-instance"),
                }),
            )
            .expect("record checkpoint");
    }
    store
}
