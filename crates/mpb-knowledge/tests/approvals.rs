use mpb_knowledge::{ApprovalKind, KnowledgeRunStore};
use serde_json::json;

#[test]
fn approvals_every_autonomous_gate_is_blocked_without_explicit_approval() {
    let temp = tempfile::tempdir().expect("tempdir");
    let store = KnowledgeRunStore::open(temp.path(), "run-approvals").expect("open run store");

    for kind in [
        ApprovalKind::LongRun,
        ApprovalKind::KeepAwake,
        ApprovalKind::ModelDownload,
        ApprovalKind::FineTuning,
        ApprovalKind::ProjectCodeChange,
        ApprovalKind::GitHubReleasePublication,
    ] {
        let error = store
            .require_approval(kind, Some("fingerprint-a"))
            .expect_err("missing approval should block");
        assert_eq!(error.kind, kind);
    }
}

#[test]
fn approvals_are_fingerprint_aware_and_newer_denials_revoke_without_overwriting_history() {
    let temp = tempfile::tempdir().expect("tempdir");
    let store = KnowledgeRunStore::open(temp.path(), "run-approval-history").expect("open store");

    store
        .record_approval(
            ApprovalKind::LongRun,
            Some("fingerprint-a"),
            true,
            "operator approved overnight run",
            json!({"ticket": "local"}),
        )
        .expect("record approval");
    store
        .require_approval(ApprovalKind::LongRun, Some("fingerprint-a"))
        .expect("approval should allow matching fingerprint");
    assert!(store
        .require_approval(ApprovalKind::LongRun, Some("fingerprint-b"))
        .is_err());

    store
        .record_approval(
            ApprovalKind::LongRun,
            Some("fingerprint-a"),
            false,
            "operator revoked after pack changed",
            json!({"revoked": true}),
        )
        .expect("record denial");

    assert!(store
        .require_approval(ApprovalKind::LongRun, Some("fingerprint-a"))
        .is_err());
    assert_eq!(
        store
            .approval_history(ApprovalKind::LongRun, Some("fingerprint-a"))
            .expect("load approval history")
            .len(),
        2
    );
}

#[test]
fn approvals_cli_rejects_unknown_kinds_and_persists_known_approval_events() {
    let temp = tempfile::tempdir().expect("tempdir");
    let artifact_root = temp.path().join("knowledge");

    let unknown = std::process::Command::new(env!("CARGO_BIN_EXE_mpb-knowledge"))
        .arg("approve")
        .arg("run-cli")
        .arg("LaunchMissiles")
        .arg("--artifact-root")
        .arg(&artifact_root)
        .arg("--reason")
        .arg("not a real approval")
        .output()
        .expect("run unknown approve command");
    assert!(!unknown.status.success());
    assert!(String::from_utf8_lossy(&unknown.stderr).contains("unknown approval kind"));

    let known = std::process::Command::new(env!("CARGO_BIN_EXE_mpb-knowledge"))
        .arg("approve")
        .arg("run-cli")
        .arg("LongRun")
        .arg("--artifact-root")
        .arg(&artifact_root)
        .arg("--target-fingerprint")
        .arg("fingerprint-a")
        .arg("--reason")
        .arg("operator approved local long run")
        .output()
        .expect("run approve command");
    assert!(
        known.status.success(),
        "approve command failed: {}",
        String::from_utf8_lossy(&known.stderr)
    );

    let store = KnowledgeRunStore::open(&artifact_root, "run-cli").expect("open store");
    store
        .require_approval(ApprovalKind::LongRun, Some("fingerprint-a"))
        .expect("cli approval should satisfy gate");
}
