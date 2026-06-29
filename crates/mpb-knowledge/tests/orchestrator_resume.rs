use std::cell::RefCell;
use std::fs;
use std::path::Path;
use std::process::Command;

use mpb_knowledge::{
    KnowledgePhaseRunner, KnowledgeReleaseOrchestrator, KnowledgeRunPhase, KnowledgeRunStore,
    OrchestratorError, PhaseCheckpointStatus, PhaseRunContext, PhaseRunStatus,
};
use serde_json::json;

#[test]
fn orchestrator_resume_continues_after_interrupted_checkpoint_without_repeating_completed_phases() {
    let temp = tempfile::tempdir().expect("tempdir");
    let artifact_root = temp.path().join("knowledge");

    assert_resume_advances_from_checkpoint(
        &artifact_root,
        "run-after-preflight",
        KnowledgeRunPhase::Preflight,
        KnowledgeRunPhase::Approvals,
    );
    assert_resume_advances_from_checkpoint(
        &artifact_root,
        "run-after-clone",
        KnowledgeRunPhase::Clone,
        KnowledgeRunPhase::Extraction,
    );
    assert_resume_advances_from_checkpoint(
        &artifact_root,
        "run-after-runtime",
        KnowledgeRunPhase::RuntimeVerification,
        KnowledgeRunPhase::Validation,
    );
}

#[test]
fn release_start_cli_persists_intake_preflight_and_blocking_report_until_long_run_approval() {
    let temp = tempfile::tempdir().expect("tempdir");
    let instance = temp.path().join("instance");
    let artifact_root = temp.path().join("knowledge");
    write_prism_fixture(&instance);

    let output = Command::new(env!("CARGO_BIN_EXE_mpb-knowledge"))
        .arg("release")
        .arg("start")
        .arg(&instance)
        .arg("--pack-id")
        .arg("fixture-pack")
        .arg("--artifact-root")
        .arg(&artifact_root)
        .output()
        .expect("run release start command");

    assert!(
        output.status.success(),
        "release start failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let started: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("release start stdout json");
    assert_eq!(started["status"], "Blocked");
    assert_eq!(started["phase"], "Approvals");
    let run_id = started["runId"].as_str().expect("run id");
    let blocking_report_path = started["blockingReportPath"]
        .as_str()
        .expect("blocking report path");
    assert!(Path::new(blocking_report_path).is_file());

    let store = KnowledgeRunStore::open(&artifact_root, run_id).expect("open run store");
    let events = store.events().expect("load events");
    assert_eq!(checkpoint_count(&events, KnowledgeRunPhase::Intake), 1);
    assert_eq!(checkpoint_count(&events, KnowledgeRunPhase::Preflight), 1);
    assert!(events
        .iter()
        .any(|event| event.event_kind == "phase.started" && event.detail["phase"] == "Intake"));
    assert!(events
        .iter()
        .any(|event| event.event_kind == "phase.succeeded" && event.detail["phase"] == "Intake"));
    assert!(events
        .iter()
        .any(|event| event.event_kind == "blocker.recorded"
            && event.detail["code"] == "MISSING_LONG_RUN_APPROVAL"));

    let status_output = Command::new(env!("CARGO_BIN_EXE_mpb-knowledge"))
        .arg("release")
        .arg("status")
        .arg(run_id)
        .arg("--artifact-root")
        .arg(&artifact_root)
        .output()
        .expect("run release status command");
    assert!(
        status_output.status.success(),
        "release status failed: {}",
        String::from_utf8_lossy(&status_output.stderr)
    );
    let status: serde_json::Value =
        serde_json::from_slice(&status_output.stdout).expect("release status stdout json");
    assert_eq!(status["latestSuccessfulPhase"], "Preflight");
    assert_eq!(status["nextPhase"], "Approvals");
    assert_eq!(status["blockers"][0]["code"], "MISSING_LONG_RUN_APPROVAL");
    assert!(status["nextCommand"]
        .as_str()
        .expect("next command")
        .contains("approve"));
}

#[test]
fn release_resume_does_not_recreate_existing_clone_artifact() {
    let temp = tempfile::tempdir().expect("tempdir");
    let instance = temp.path().join("instance");
    let artifact_root = temp.path().join("knowledge");
    write_prism_fixture(&instance);

    let start_output = Command::new(env!("CARGO_BIN_EXE_mpb-knowledge"))
        .arg("release")
        .arg("start")
        .arg(&instance)
        .arg("--pack-id")
        .arg("fixture-pack")
        .arg("--artifact-root")
        .arg(&artifact_root)
        .output()
        .expect("run release start command");
    assert!(
        start_output.status.success(),
        "release start failed: {}",
        String::from_utf8_lossy(&start_output.stderr)
    );
    let started: serde_json::Value =
        serde_json::from_slice(&start_output.stdout).expect("release start stdout json");
    let run_id = started["runId"].as_str().expect("run id");

    let approve_output = Command::new(env!("CARGO_BIN_EXE_mpb-knowledge"))
        .arg("approve")
        .arg(run_id)
        .arg("LongRun")
        .arg("--reason")
        .arg("integration test approves local long-running work")
        .arg("--artifact-root")
        .arg(&artifact_root)
        .output()
        .expect("approve long run");
    assert!(
        approve_output.status.success(),
        "approve failed: {}",
        String::from_utf8_lossy(&approve_output.stderr)
    );
    let approved_status = status_cli(run_id, &artifact_root);
    assert_eq!(approved_status["nextPhase"], "Approvals");
    assert!(approved_status["nextCommand"]
        .as_str()
        .expect("next command after approval")
        .contains("release resume"));

    for expected_phase in [
        KnowledgeRunPhase::Approvals,
        KnowledgeRunPhase::Fingerprint,
        KnowledgeRunPhase::Clone,
    ] {
        let resume = resume_cli(run_id, &artifact_root);
        assert_eq!(resume["phase"], expected_phase.as_str());
        assert_eq!(resume["status"], "PhaseSucceeded");
    }

    let store = KnowledgeRunStore::open(&artifact_root, run_id).expect("open run store");
    assert_eq!(target_clone_artifacts(&store), 1);
    let clone_checkpoint_events_before = checkpoint_count(
        &store.events().expect("load events"),
        KnowledgeRunPhase::Clone,
    );

    let extraction_resume = resume_cli(run_id, &artifact_root);
    assert_eq!(extraction_resume["phase"], "Extraction");
    assert_eq!(extraction_resume["status"], "Blocked");

    let reopened = KnowledgeRunStore::open(&artifact_root, run_id).expect("reopen run store");
    assert_eq!(target_clone_artifacts(&reopened), 1);
    assert_eq!(
        checkpoint_count(
            &reopened.events().expect("load events"),
            KnowledgeRunPhase::Clone
        ),
        clone_checkpoint_events_before
    );
}

#[test]
fn orchestrator_records_failed_checkpoint_and_event_when_phase_runner_errors() {
    let temp = tempfile::tempdir().expect("tempdir");
    let artifact_root = temp.path().join("knowledge");
    let run_id = "run-error";
    seed_successful_phases(
        &artifact_root,
        run_id,
        "fingerprint-a",
        &[KnowledgeRunPhase::Intake],
    );

    let orchestrator = KnowledgeReleaseOrchestrator::with_phase_runner(
        &artifact_root,
        ErrorRunner {
            message: "synthetic preflight failure",
        },
    );
    let error = orchestrator
        .run_next_required_phase(run_id)
        .expect_err("phase runner error should be returned");
    assert!(error.to_string().contains("synthetic preflight failure"));

    let store = KnowledgeRunStore::open(&artifact_root, run_id).expect("open run store");
    let events = store.events().expect("load events");
    assert!(events.iter().any(|event| event.event_kind == "phase.failed"
        && event.detail["phase"] == "Preflight"
        && event.detail["error"]
            .as_str()
            .expect("error text")
            .contains("synthetic preflight failure")));
    assert!(store
        .phase_checkpoints()
        .expect("checkpoints")
        .iter()
        .any(
            |checkpoint| checkpoint.phase == KnowledgeRunPhase::Preflight
                && checkpoint.status == PhaseCheckpointStatus::Failed
                && checkpoint.detail["error"]
                    .as_str()
                    .expect("checkpoint error")
                    .contains("synthetic preflight failure")
        ));
}

#[test]
fn fingerprint_resume_reuses_durable_artifact_before_touching_original_instance() {
    let temp = tempfile::tempdir().expect("tempdir");
    let artifact_root = temp.path().join("knowledge");
    let run_id = "run-fingerprint-idempotent";
    seed_successful_phases(
        &artifact_root,
        run_id,
        "fingerprint-a",
        &[
            KnowledgeRunPhase::Intake,
            KnowledgeRunPhase::Preflight,
            KnowledgeRunPhase::Approvals,
        ],
    );
    let store = KnowledgeRunStore::open(&artifact_root, run_id).expect("open run store");
    let original_artifact_path = temp.path().join("missing-original-instance");
    store
        .record_artifact_ref(
            "target-original",
            &original_artifact_path,
            Some("fingerprint-a"),
            json!({"readOnly": true, "seeded": true}),
        )
        .expect("record durable original artifact");
    drop(store);

    let orchestrator = KnowledgeReleaseOrchestrator::new(&artifact_root);
    let outcome = orchestrator
        .run_next_required_phase(run_id)
        .expect("resume fingerprint from durable artifact");
    assert_eq!(outcome.phase, Some(KnowledgeRunPhase::Fingerprint));
    assert_eq!(outcome.status.as_str(), "PhaseSucceeded");

    let reopened = KnowledgeRunStore::open(&artifact_root, run_id).expect("reopen run store");
    assert_eq!(
        reopened
            .artifact_refs()
            .expect("artifact refs")
            .into_iter()
            .filter(|artifact| artifact.artifact_kind == "target-original")
            .count(),
        1
    );
}

#[test]
fn clone_phase_blocks_when_source_instance_changed_after_fingerprint() {
    let temp = tempfile::tempdir().expect("tempdir");
    let instance = temp.path().join("instance");
    let artifact_root = temp.path().join("knowledge");
    write_prism_fixture(&instance);

    let start_output = Command::new(env!("CARGO_BIN_EXE_mpb-knowledge"))
        .arg("release")
        .arg("start")
        .arg(&instance)
        .arg("--pack-id")
        .arg("fixture-pack")
        .arg("--artifact-root")
        .arg(&artifact_root)
        .output()
        .expect("run release start command");
    assert!(start_output.status.success());
    let started: serde_json::Value =
        serde_json::from_slice(&start_output.stdout).expect("release start stdout json");
    let run_id = started["runId"].as_str().expect("run id");
    let approve_output = Command::new(env!("CARGO_BIN_EXE_mpb-knowledge"))
        .arg("approve")
        .arg(run_id)
        .arg("LongRun")
        .arg("--reason")
        .arg("integration test approves local long-running work")
        .arg("--artifact-root")
        .arg(&artifact_root)
        .output()
        .expect("approve long run");
    assert!(approve_output.status.success());

    assert_eq!(resume_cli(run_id, &artifact_root)["phase"], "Approvals");
    assert_eq!(resume_cli(run_id, &artifact_root)["phase"], "Fingerprint");
    fs::write(
        instance.join("minecraft/config/changed.toml"),
        b"changed=true",
    )
    .expect("mutate original after fingerprint");

    let clone_outcome = resume_cli(run_id, &artifact_root);
    assert_eq!(clone_outcome["phase"], "Clone");
    assert_eq!(clone_outcome["status"], "Blocked");

    let store = KnowledgeRunStore::open(&artifact_root, run_id).expect("open run store");
    let blockers = store.blockers().expect("load blockers");
    assert!(blockers
        .iter()
        .any(|blocker| blocker.code == "TARGET_FINGERPRINT_CHANGED"));
    assert_eq!(target_clone_artifacts(&store), 0);
}

#[test]
fn preflight_resume_rebuilds_missing_report_artifact_instead_of_using_metadata_as_report() {
    let temp = tempfile::tempdir().expect("tempdir");
    let instance = temp.path().join("instance");
    let artifact_root = temp.path().join("knowledge");
    let run_id = "run-preflight-rebuild";
    write_prism_fixture(&instance);

    let store = KnowledgeRunStore::open(&artifact_root, run_id).expect("open run store");
    store
        .record_run(None, json!({"createdBy": "preflight rebuild test"}))
        .expect("record run");
    store
        .record_phase_checkpoint(
            KnowledgeRunPhase::Intake,
            PhaseCheckpointStatus::Succeeded,
            None,
            json!({"instancePath": instance}),
        )
        .expect("record intake");
    let missing_report = store.run_dir().join("preflight-report.json");
    store
        .record_artifact_ref(
            "preflight-report",
            &missing_report,
            None,
            json!({"format": "json"}),
        )
        .expect("record missing preflight artifact");
    drop(store);

    let orchestrator = KnowledgeReleaseOrchestrator::new(&artifact_root);
    let outcome = orchestrator
        .run_next_required_phase(run_id)
        .expect("rerun preflight");
    assert_eq!(outcome.phase, Some(KnowledgeRunPhase::Preflight));
    assert_eq!(outcome.status.as_str(), "PhaseSucceeded");

    let report_json: serde_json::Value =
        serde_json::from_slice(&fs::read(&missing_report).expect("rebuilt report"))
            .expect("report json");
    assert_eq!(report_json["prismInstance"]["readable"], true);
}

fn assert_resume_advances_from_checkpoint(
    artifact_root: &Path,
    run_id: &str,
    interrupted_after: KnowledgeRunPhase,
    expected_next: KnowledgeRunPhase,
) {
    let store = KnowledgeRunStore::open(artifact_root, run_id).expect("open store");
    store
        .record_run(
            Some("fingerprint-a"),
            json!({"createdBy": "orchestrator resume test"}),
        )
        .expect("record run");
    for phase in KnowledgeRunPhase::ALL {
        store
            .record_phase_checkpoint(
                phase,
                PhaseCheckpointStatus::Succeeded,
                Some("fingerprint-a"),
                json!({"seeded": true, "phase": phase.as_str()}),
            )
            .expect("seed checkpoint");
        if phase == interrupted_after {
            break;
        }
    }
    let completed_phase_events_before =
        checkpoint_count(&store.events().expect("events before"), interrupted_after);
    drop(store);

    let runner = RecordingRunner::default();
    let orchestrator = KnowledgeReleaseOrchestrator::with_phase_runner(artifact_root, runner);
    let outcome = orchestrator
        .run_next_required_phase(run_id)
        .expect("resume next phase");

    assert_eq!(outcome.phase, Some(expected_next));
    assert_eq!(outcome.status.as_str(), "PhaseSucceeded");
    assert_eq!(outcome.next_phase, phase_after(expected_next));

    let reopened = KnowledgeRunStore::open(artifact_root, run_id).expect("reopen store");
    let events = reopened.events().expect("events after");
    assert_eq!(
        checkpoint_count(&events, interrupted_after),
        completed_phase_events_before
    );
    assert_eq!(checkpoint_count(&events, expected_next), 1);
    assert_eq!(
        events
            .iter()
            .filter(|event| event.event_kind == "orchestrator.resume")
            .count(),
        1
    );
}

struct ErrorRunner {
    message: &'static str,
}

impl KnowledgePhaseRunner for ErrorRunner {
    fn run_phase(
        &self,
        _context: &PhaseRunContext<'_>,
        _phase: KnowledgeRunPhase,
    ) -> Result<PhaseRunStatus, OrchestratorError> {
        Err(OrchestratorError::Io(std::io::Error::new(
            std::io::ErrorKind::Other,
            self.message,
        )))
    }
}

#[derive(Default)]
struct RecordingRunner {
    phases: RefCell<Vec<KnowledgeRunPhase>>,
}

impl KnowledgePhaseRunner for RecordingRunner {
    fn run_phase(
        &self,
        _context: &PhaseRunContext<'_>,
        phase: KnowledgeRunPhase,
    ) -> Result<PhaseRunStatus, OrchestratorError> {
        self.phases.borrow_mut().push(phase);
        Ok(PhaseRunStatus::Succeeded {
            target_fingerprint: Some("fingerprint-a".to_string()),
            detail: json!({"runner": "recording", "phase": phase.as_str()}),
        })
    }
}

fn resume_cli(run_id: &str, artifact_root: &Path) -> serde_json::Value {
    let output = Command::new(env!("CARGO_BIN_EXE_mpb-knowledge"))
        .arg("release")
        .arg("resume")
        .arg(run_id)
        .arg("--artifact-root")
        .arg(artifact_root)
        .output()
        .expect("run release resume command");
    assert!(
        output.status.success(),
        "release resume failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).expect("release resume stdout json")
}

fn status_cli(run_id: &str, artifact_root: &Path) -> serde_json::Value {
    let output = Command::new(env!("CARGO_BIN_EXE_mpb-knowledge"))
        .arg("release")
        .arg("status")
        .arg(run_id)
        .arg("--artifact-root")
        .arg(artifact_root)
        .output()
        .expect("run release status command");
    assert!(
        output.status.success(),
        "release status failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).expect("release status stdout json")
}

fn checkpoint_count(events: &[mpb_knowledge::EventRecord], phase: KnowledgeRunPhase) -> usize {
    events
        .iter()
        .filter(|event| {
            event.event_kind == "phase.checkpoint"
                && event.detail["phase"] == phase.as_str()
                && event.detail["status"] == "Succeeded"
        })
        .count()
}

fn target_clone_artifacts(store: &KnowledgeRunStore) -> usize {
    store
        .artifact_refs()
        .expect("artifact refs")
        .into_iter()
        .filter(|artifact| artifact.artifact_kind == "target-clone")
        .count()
}

fn phase_after(phase: KnowledgeRunPhase) -> Option<KnowledgeRunPhase> {
    let index = KnowledgeRunPhase::ALL
        .iter()
        .position(|candidate| *candidate == phase)
        .expect("phase in order");
    KnowledgeRunPhase::ALL.get(index + 1).copied()
}

fn seed_successful_phases(
    artifact_root: &Path,
    run_id: &str,
    target_fingerprint: &str,
    phases: &[KnowledgeRunPhase],
) {
    let store = KnowledgeRunStore::open(artifact_root, run_id).expect("open seed store");
    store
        .record_run(
            Some(target_fingerprint),
            json!({"createdBy": "orchestrator resume test"}),
        )
        .expect("record run");
    for phase in phases {
        store
            .record_phase_checkpoint(
                *phase,
                PhaseCheckpointStatus::Succeeded,
                Some(target_fingerprint),
                json!({
                    "seeded": true,
                    "phase": phase.as_str(),
                    "instancePath": artifact_root.join("missing-instance"),
                }),
            )
            .expect("seed phase checkpoint");
    }
}

fn write_prism_fixture(instance: &Path) {
    fs::create_dir_all(instance.join("minecraft/mods")).expect("create fixture dirs");
    fs::create_dir_all(instance.join("minecraft/config")).expect("create config dir");
    fs::write(
        instance.join("instance.cfg"),
        "name=Fixture Pack\nManagedPackVersionName=1.0.0\n",
    )
    .expect("write instance cfg");
    fs::write(
        instance.join("mmc-pack.json"),
        r#"{"components":[{"uid":"net.minecraft","version":"1.21.1"},{"uid":"net.neoforged","version":"21.1.1"}]}"#,
    )
    .expect("write mmc pack");
    fs::write(instance.join("minecraft/mods/example.jar"), b"fixture mod").expect("write mod file");
    fs::write(
        instance.join("minecraft/config/example.toml"),
        b"enabled=true",
    )
    .expect("write config file");
}
