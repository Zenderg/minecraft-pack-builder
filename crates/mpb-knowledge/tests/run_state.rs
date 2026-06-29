use std::fs;

use mpb_knowledge::{KnowledgeRunPhase, KnowledgeRunStore, PhaseCheckpointStatus, RunBlockerInput};
use serde_json::json;

#[test]
fn run_state_migrations_are_idempotent_for_existing_run_database() {
    let temp = tempfile::tempdir().expect("tempdir");
    let run_id = "run-idempotent";
    let store = KnowledgeRunStore::open(temp.path(), run_id).expect("open run store");
    store
        .record_run(Some("fingerprint-a"), json!({"packId": "fixture"}))
        .expect("record run");
    drop(store);

    let reopened = KnowledgeRunStore::open(temp.path(), run_id).expect("reopen run store");
    let run = reopened.run().expect("load run").expect("run exists");

    assert_eq!(run.run_id, run_id);
    assert_eq!(run.target_fingerprint.as_deref(), Some("fingerprint-a"));
    assert_eq!(run.detail["packId"], "fixture");
}

#[test]
fn run_state_phase_checkpoints_resume_from_latest_successful_phase() {
    let temp = tempfile::tempdir().expect("tempdir");
    let store = KnowledgeRunStore::open(temp.path(), "run-checkpoints").expect("open run store");
    store
        .record_phase_checkpoint(
            KnowledgeRunPhase::Intake,
            PhaseCheckpointStatus::Succeeded,
            Some("fingerprint-a"),
            json!({"instancePath": "/packs/a"}),
        )
        .expect("record intake checkpoint");
    store
        .record_phase_checkpoint(
            KnowledgeRunPhase::Preflight,
            PhaseCheckpointStatus::Started,
            Some("fingerprint-a"),
            json!({"started": true}),
        )
        .expect("record preflight start");

    let latest = store
        .latest_successful_checkpoint()
        .expect("load latest checkpoint")
        .expect("successful checkpoint exists");

    assert_eq!(latest.phase, KnowledgeRunPhase::Intake);
    assert_eq!(latest.status, PhaseCheckpointStatus::Succeeded);
    assert_eq!(latest.target_fingerprint.as_deref(), Some("fingerprint-a"));
}

#[test]
fn run_state_events_are_append_only_in_sqlite_and_jsonl_order() {
    let temp = tempfile::tempdir().expect("tempdir");
    let store = KnowledgeRunStore::open(temp.path(), "run-events").expect("open run store");
    store
        .append_event(
            "phase.started",
            Some("fingerprint-a"),
            json!({"phase": "Intake"}),
        )
        .expect("append first event");
    store
        .append_event(
            "phase.succeeded",
            Some("fingerprint-a"),
            json!({"phase": "Intake"}),
        )
        .expect("append second event");

    let events = store.events().expect("load events");
    assert_eq!(events.len(), 2);
    assert_eq!(events[0].sequence + 1, events[1].sequence);
    assert_eq!(events[0].event_kind, "phase.started");
    assert_eq!(events[1].event_kind, "phase.succeeded");

    let jsonl = fs::read_to_string(store.event_log_path()).expect("read event log");
    let lines: Vec<_> = jsonl.lines().collect();
    assert_eq!(lines.len(), 2);
    assert!(lines[0].contains("\"phase.started\""));
    assert!(lines[1].contains("\"phase.succeeded\""));
}

#[test]
fn run_state_blockers_survive_closing_and_reopening_database() {
    let temp = tempfile::tempdir().expect("tempdir");
    let run_id = "run-blockers";
    let store = KnowledgeRunStore::open(temp.path(), run_id).expect("open run store");
    store
        .record_blocker(RunBlockerInput {
            code: "MISSING_LONG_RUN_APPROVAL".to_string(),
            phase: Some(KnowledgeRunPhase::Approvals),
            target_fingerprint: Some("fingerprint-a".to_string()),
            message: "Long-running release work requires explicit approval.".to_string(),
            detail: json!({"approvalKind": "LongRun"}),
        })
        .expect("record blocker");
    drop(store);

    let reopened = KnowledgeRunStore::open(temp.path(), run_id).expect("reopen run store");
    let blockers = reopened.blockers().expect("load blockers");

    assert_eq!(blockers.len(), 1);
    assert_eq!(blockers[0].code, "MISSING_LONG_RUN_APPROVAL");
    assert_eq!(blockers[0].phase, Some(KnowledgeRunPhase::Approvals));
    assert_eq!(
        blockers[0].target_fingerprint.as_deref(),
        Some("fingerprint-a")
    );
}
