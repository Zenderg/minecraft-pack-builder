use mpb_knowledge::{
    build_experiment_plan, record_experiment_attempt, summarize_experiment_suite,
    CoverageEvidenceRequirement, CoverageObligation, CoverageObligationKind, ExperimentAttempt,
    ExperimentAttemptStatus, ExperimentRetryPolicy, KnowledgeRunStore, LabExperimentOperation,
    LabExperimentStatus, LabObservation, LabObservedState,
};

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
