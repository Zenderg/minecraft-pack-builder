use std::fs;

use mpb_knowledge::{
    build_experiment_plan, record_experiment_attempt, summarize_experiment_suite,
    CoverageEvidenceRequirement, CoverageObligation, CoverageObligationKind, ExperimentAttempt,
    ExperimentAttemptStatus, ExperimentPlan, ExperimentRetryPolicy, KnowledgeReleaseOrchestrator,
    KnowledgeRunPhase, KnowledgeRunStore, LabExperimentOperation, LabExperimentStatus,
    LabObservation, LabObservedState, PhaseCheckpointStatus,
};
use serde_json::json;

#[test]
fn experiment_batches_are_derived_from_runtime_coverage_obligations() {
    let obligations = vec![
        CoverageObligation {
            id: "behavior_claim:pressing".to_string(),
            kind: CoverageObligationKind::BehaviorClaim,
            subject_id: "claim-pressing".to_string(),
            evidence_requirement: CoverageEvidenceRequirement::Runtime,
            evidence_ids: Vec::new(),
            covered: false,
        },
        CoverageObligation {
            id: "entity:create:depot".to_string(),
            kind: CoverageObligationKind::DiscoveredEntity,
            subject_id: "create:depot".to_string(),
            evidence_requirement: CoverageEvidenceRequirement::Deterministic,
            evidence_ids: Vec::new(),
            covered: false,
        },
    ];

    let plan = build_experiment_plan("fingerprint-exp", &obligations);

    assert_eq!(plan.fingerprint, "fingerprint-exp");
    assert_eq!(plan.batches.len(), 1);
    assert_eq!(plan.batches[0].experiments.len(), 1);
    let experiment = &plan.batches[0].experiments[0];
    assert_eq!(experiment.obligation_ids, vec!["behavior_claim:pressing"]);
    assert_eq!(experiment.retry_policy.max_attempts, 3);
    assert!(experiment
        .operations
        .iter()
        .any(|operation| matches!(operation, LabExperimentOperation::CompareSnapshots)));
    assert!(experiment
        .required_observation_adapters
        .contains(&"generic_state_diff".to_string()));
}

#[test]
fn experiment_attempts_and_retries_are_recorded_as_run_artifacts_and_events() {
    let temp = tempfile::tempdir().expect("tempdir");
    let store = KnowledgeRunStore::open(temp.path(), "run-experiments").expect("open store");

    let first = record_experiment_attempt(
        &store,
        ExperimentAttempt {
            experiment_id: "exp-pressing".to_string(),
            attempt: 1,
            status: ExperimentAttemptStatus::Rejected,
            observation: None,
            raw_artifact_path: Some("knowledge/runs/run-experiments/lab/raw-1.json".to_string()),
            message: "snapshot was incomplete".to_string(),
        },
    )
    .expect("record first attempt");
    let second = record_experiment_attempt(
        &store,
        ExperimentAttempt {
            experiment_id: "exp-pressing".to_string(),
            attempt: 2,
            status: ExperimentAttemptStatus::Accepted,
            observation: Some(accepted_observation()),
            raw_artifact_path: Some("knowledge/runs/run-experiments/lab/raw-2.json".to_string()),
            message: "accepted".to_string(),
        },
    )
    .expect("record accepted attempt");

    assert!(first.path.ends_with("lab/exp-pressing-attempt-001.json"));
    assert!(second.path.ends_with("lab/exp-pressing-attempt-002.json"));
    assert!(store
        .events()
        .expect("events")
        .iter()
        .any(|event| event.event_kind == "experiment.attempt.recorded"
            && event.detail["status"] == "accepted"));
}

#[test]
fn retry_exhaustion_blocks_release_with_affected_obligations_and_raw_artifacts() {
    let policy = ExperimentRetryPolicy {
        max_attempts: 2,
        retry_on_statuses: vec![
            ExperimentAttemptStatus::Rejected,
            ExperimentAttemptStatus::Failed,
        ],
    };
    let attempts = vec![
        ExperimentAttempt {
            experiment_id: "exp-flaky".to_string(),
            attempt: 1,
            status: ExperimentAttemptStatus::Rejected,
            observation: None,
            raw_artifact_path: Some("raw-1.json".to_string()),
            message: "no after snapshot".to_string(),
        },
        ExperimentAttempt {
            experiment_id: "exp-flaky".to_string(),
            attempt: 2,
            status: ExperimentAttemptStatus::Failed,
            observation: None,
            raw_artifact_path: Some("raw-2.json".to_string()),
            message: "client timeout".to_string(),
        },
    ];

    let summary = summarize_experiment_suite(
        "exp-flaky",
        &["behavior_claim:flaky".to_string()],
        &policy,
        &attempts,
    );

    assert!(summary.release_blocker.is_some());
    let blocker = summary.release_blocker.expect("release blocker");
    assert_eq!(blocker.code, "FLAKY_EXPERIMENT_RETRY_EXHAUSTED");
    assert_eq!(
        blocker.affected_obligation_ids,
        vec!["behavior_claim:flaky"]
    );
    assert_eq!(blocker.raw_artifact_paths, vec!["raw-1.json", "raw-2.json"]);
}

#[test]
fn runtime_verification_blocks_without_real_clone_runtime_evidence_even_when_no_experiments_are_planned(
) {
    let temp = tempfile::tempdir().expect("tempdir");
    let artifact_root = temp.path().join("knowledge");
    let run_id = "run-runtime-evidence-missing";
    let store = seed_runtime_verification_run(&artifact_root, run_id, "fingerprint-runtime");
    write_zero_experiment_plan(&store, "fingerprint-runtime");
    drop(store);

    let outcome = KnowledgeReleaseOrchestrator::new(&artifact_root)
        .run_next_required_phase(run_id)
        .expect("run runtime verification");

    assert_eq!(outcome.phase, Some(KnowledgeRunPhase::RuntimeVerification));
    assert_eq!(outcome.status.as_str(), "Blocked");
    let reopened = KnowledgeRunStore::open(&artifact_root, run_id).expect("open store");
    assert!(reopened
        .blockers()
        .expect("blockers")
        .into_iter()
        .any(|blocker| blocker.code == "CLONED_RUNTIME_VALIDATION_MISSING"));
}

#[test]
fn runtime_verification_accepts_zero_experiment_plan_only_after_passed_clone_runtime_evidence() {
    let temp = tempfile::tempdir().expect("tempdir");
    let artifact_root = temp.path().join("knowledge");
    let run_id = "run-runtime-evidence-passed";
    let store = seed_runtime_verification_run(&artifact_root, run_id, "fingerprint-runtime");
    write_zero_experiment_plan(&store, "fingerprint-runtime");
    let evidence_path = store
        .run_dir()
        .join("cloned-runtime-validation-evidence.json");
    fs::write(
        &evidence_path,
        serde_json::to_vec_pretty(&json!({
            "status": "passed",
            "label": "real cloned Prism runtime",
            "detail": "operator launched the disposable clone and verified Minecraft reached the MPB runtime",
            "artifactPaths": ["knowledge/prism-clones/run-runtime-evidence-passed/instance"]
        }))
        .expect("evidence json"),
    )
    .expect("write runtime evidence");
    store
        .record_artifact_ref(
            "cloned-runtime-validation-evidence",
            &evidence_path,
            Some("fingerprint-runtime"),
            json!({"status": "passed"}),
        )
        .expect("record runtime evidence");
    drop(store);

    let outcome = KnowledgeReleaseOrchestrator::new(&artifact_root)
        .run_next_required_phase(run_id)
        .expect("run runtime verification");

    assert_eq!(outcome.phase, Some(KnowledgeRunPhase::RuntimeVerification));
    assert_eq!(outcome.status.as_str(), "PhaseSucceeded");
}

fn accepted_observation() -> LabObservation {
    LabObservation {
        id: "obs-pressing".to_string(),
        experiment_id: "exp-pressing".to_string(),
        fingerprint: "fingerprint-exp".to_string(),
        status: LabExperimentStatus::Accepted,
        operations: vec![LabExperimentOperation::PrepareLabArea { radius: 4 }],
        before: vec![LabObservedState {
            target_id: "create:depot".to_string(),
            state_type: "inventory".to_string(),
            value: "minecraft:iron_ingot".to_string(),
        }],
        after: vec![LabObservedState {
            target_id: "create:depot".to_string(),
            state_type: "inventory".to_string(),
            value: "create:iron_sheet".to_string(),
        }],
        observed_entity_ids: vec!["create:depot".to_string()],
        summary: "pressing produced iron sheet".to_string(),
        limits: Vec::new(),
        required_observation_adapters: Vec::new(),
    }
}

fn seed_runtime_verification_run(
    artifact_root: &std::path::Path,
    run_id: &str,
    target_fingerprint: &str,
) -> KnowledgeRunStore {
    let store = KnowledgeRunStore::open(artifact_root, run_id).expect("open store");
    store
        .record_run(
            Some(target_fingerprint),
            json!({"createdBy": "runtime verification test"}),
        )
        .expect("record run");
    for phase in [
        KnowledgeRunPhase::Intake,
        KnowledgeRunPhase::Preflight,
        KnowledgeRunPhase::Approvals,
        KnowledgeRunPhase::Fingerprint,
        KnowledgeRunPhase::Clone,
        KnowledgeRunPhase::Extraction,
        KnowledgeRunPhase::Drafting,
        KnowledgeRunPhase::ExperimentPlanning,
        KnowledgeRunPhase::AdapterExpansion,
    ] {
        store
            .record_phase_checkpoint(
                phase,
                PhaseCheckpointStatus::Succeeded,
                Some(target_fingerprint),
                json!({"seeded": true, "phase": phase.as_str()}),
            )
            .expect("record checkpoint");
    }
    store
}

fn write_zero_experiment_plan(store: &KnowledgeRunStore, target_fingerprint: &str) {
    let plan_dir = store.run_dir().join("lab");
    fs::create_dir_all(&plan_dir).expect("plan dir");
    let plan_path = plan_dir.join("experiment-plan.json");
    fs::write(
        &plan_path,
        serde_json::to_vec_pretty(&ExperimentPlan {
            fingerprint: target_fingerprint.to_string(),
            batches: Vec::new(),
        })
        .expect("plan json"),
    )
    .expect("write plan");
    store
        .record_artifact_ref(
            "experiment-plan",
            &plan_path,
            Some(target_fingerprint),
            json!({"batchCount": 0, "experimentCount": 0}),
        )
        .expect("record experiment plan");
}
