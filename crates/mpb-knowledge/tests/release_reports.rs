use std::fs;

use mpb_knowledge::{
    prepare_github_release_publication, require_github_release_publication_approval,
    write_blocking_report_artifacts, write_release_report_artifacts, ApprovalKind, BlockingReport,
    KnowledgeRunPhase, KnowledgeRunStore, PhaseCheckpointStatus, ReleaseReport, RunBlockerInput,
};
use serde_json::json;

const FINGERPRINT: &str = "release-fingerprint-001";

#[test]
fn blocking_reports_contain_every_required_field() {
    let temp = tempfile::tempdir().expect("tempdir");
    let artifact_root = temp.path().join("knowledge");
    let store = seed_release_store(&artifact_root, "run-blocking-report");
    let evidence_path = store.run_dir().join("coverage/accepted-evidence.json");
    fs::create_dir_all(evidence_path.parent().expect("parent")).expect("coverage dir");
    fs::write(&evidence_path, "{}").expect("evidence");
    store
        .record_artifact_ref(
            "accepted-evidence",
            &evidence_path,
            Some(FINGERPRINT),
            json!({"evidenceIds": ["evidence:static:gearbox"]}),
        )
        .expect("record evidence artifact");
    let blocker = store
        .record_blocker(RunBlockerInput {
            code: "UNSUPPORTED_SOURCE_KIND".to_string(),
            phase: Some(KnowledgeRunPhase::Validation),
            target_fingerprint: Some(FINGERPRINT.to_string()),
            message: "A KubeJS script source has no deterministic extractor.".to_string(),
            detail: json!({
                "affectedCoverageObligations": ["obligation:script:kubejs"],
                "acceptedEvidence": ["evidence:static:gearbox"],
                "missingCapabilityOrApproval": "KubeJS extractor support",
                "proposedAction": {
                    "kind": "adapter",
                    "summary": "Implement a deterministic KubeJS extractor adapter"
                },
                "localArtifactPaths": [evidence_path],
            }),
        })
        .expect("record blocker");

    let paths =
        write_blocking_report_artifacts(&store, &blocker).expect("write blocking report artifacts");
    let report: BlockingReport =
        serde_json::from_slice(&fs::read(&paths.json_path).expect("json bytes"))
            .expect("blocking report json");

    assert_eq!(report.run_id, "run-blocking-report");
    assert_eq!(
        report.target_instance.as_deref(),
        Some("/packs/All of Create - Aeronautics")
    );
    assert_eq!(report.fingerprint.as_deref(), Some(FINGERPRINT));
    assert_eq!(report.failed_phase, Some(KnowledgeRunPhase::Validation));
    assert_eq!(report.exact_blocker.code, "UNSUPPORTED_SOURCE_KIND");
    assert_eq!(
        report.affected_coverage_obligations,
        vec!["obligation:script:kubejs"]
    );
    assert_eq!(report.accepted_evidence, vec!["evidence:static:gearbox"]);
    assert_eq!(
        report.missing_capability_or_approval.as_deref(),
        Some("KubeJS extractor support")
    );
    assert_eq!(report.proposed_action["kind"], "adapter");
    assert!(report
        .resume_command
        .contains("mpb-knowledge release resume run-blocking-report"));
    assert!(report
        .local_artifact_paths
        .iter()
        .any(|path| path.ends_with("accepted-evidence.json")));
    assert!(fs::read_to_string(paths.markdown_path)
        .expect("markdown")
        .contains("UNSUPPORTED_SOURCE_KIND"));
}

#[test]
fn release_reports_contain_every_required_field_and_unsigned_warnings() {
    let temp = tempfile::tempdir().expect("tempdir");
    let artifact_root = temp.path().join("knowledge");
    let store = seed_release_store(&artifact_root, "run-release-report");
    seed_release_artifacts(&store);

    let paths = write_release_report_artifacts(&store, None).expect("write release report");
    let report: ReleaseReport =
        serde_json::from_slice(&fs::read(&paths.json_path).expect("json bytes"))
            .expect("release report json");

    assert_eq!(report.run_id, "run-release-report");
    assert_eq!(
        report.target_pack_identity["packId"],
        "all-of-create-aeronautics"
    );
    assert_eq!(report.exact_fingerprint, FINGERPRINT);
    assert_eq!(report.coverage_summary["totalObligations"], 2);
    assert_eq!(report.evidence_summary_by_kind["static"], 1);
    assert_eq!(
        report.model_candidates[0]["candidateLabel"],
        "local-qwen-fixture"
    );
    assert_eq!(report.approvals[0].kind, ApprovalKind::LongRun);
    assert_eq!(report.worker_evaluations[0]["passed"], true);
    assert_eq!(report.fine_tuning_decisions[0]["state"], "NotUsed");
    assert_eq!(report.experiment_summary["accepted"], 1);
    assert_eq!(report.retry_statistics["retryableFailures"], 0);
    assert!(report.generated_source_paths[0].ends_with("source"));
    assert!(report.generated_bundle_paths[0].ends_with("knowledge-index.json"));
    assert_eq!(report.checksums["runtimeBundle"], "abc123");
    assert_eq!(report.compressed_size_bytes, Some(512));
    assert_eq!(report.patcher_validation["status"], "passed");
    assert_eq!(report.cloned_runtime_validation["status"], "passed");
    assert_eq!(report.mcp_query_validation["status"], "passed");
    assert!(report.desktop_artifact_list[0].ends_with("Minecraft Pack Builder.app"));
    assert!(report.github_release_url.is_none());
    assert!(report
        .unsigned_app_warnings
        .iter()
        .any(|warning| warning.platform == "macOS"));
    assert!(report
        .unsigned_app_warnings
        .iter()
        .any(|warning| warning.platform == "Windows"));
    assert!(report
        .unsigned_app_warnings
        .iter()
        .any(|warning| warning.platform == "Linux"));
    assert!(fs::read_to_string(paths.markdown_path)
        .expect("markdown")
        .contains("Unsigned App Warnings"));
}

#[test]
fn github_publication_is_blocked_without_approval() {
    let temp = tempfile::tempdir().expect("tempdir");
    let artifact_root = temp.path().join("knowledge");
    let store = seed_release_store(&artifact_root, "run-github-gate");
    seed_release_artifacts(&store);

    let preparation = prepare_github_release_publication(&store, "knowledge-aoca-001")
        .expect("preparation should not publish or require approval");
    assert!(!preparation.publication_approved);
    assert!(preparation
        .missing_approval
        .as_deref()
        .expect("missing approval reason")
        .contains("approval required"));

    let error = require_github_release_publication_approval(&store)
        .expect_err("publication should require approval");

    assert_eq!(error.kind, ApprovalKind::GitHubReleasePublication);
}

#[test]
fn github_publication_preparation_writes_local_command_after_approval() {
    let temp = tempfile::tempdir().expect("tempdir");
    let artifact_root = temp.path().join("knowledge");
    let store = seed_release_store(&artifact_root, "run-github-approved");
    seed_release_artifacts(&store);
    store
        .record_approval(
            ApprovalKind::GitHubReleasePublication,
            Some(FINGERPRINT),
            true,
            "operator approved publishing prepared GitHub release artifacts",
            json!({ "ticket": "local" }),
        )
        .expect("record approval");

    let preparation = prepare_github_release_publication(&store, "knowledge-aoca-001")
        .expect("prepare publication");

    assert_eq!(preparation.tag, "knowledge-aoca-001");
    assert!(preparation.publication_approved);
    assert!(preparation.missing_approval.is_none());
    assert!(preparation
        .gh_workflow_command
        .contains("gh workflow run release.yml"));
    assert!(preparation
        .release_notes_path
        .ends_with("github-release-notes.md"));
}

#[test]
fn release_report_cli_writes_json_and_markdown_paths() {
    let temp = tempfile::tempdir().expect("tempdir");
    let artifact_root = temp.path().join("knowledge");
    let store = seed_release_store(&artifact_root, "run-report-cli");
    seed_release_artifacts(&store);
    drop(store);

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_mpb-knowledge"))
        .arg("release")
        .arg("report")
        .arg("run-report-cli")
        .arg("--artifact-root")
        .arg(&artifact_root)
        .output()
        .expect("run release report cli");

    assert!(
        output.status.success(),
        "release report failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let paths: serde_json::Value = serde_json::from_slice(&output.stdout).expect("cli path json");
    assert!(paths["jsonPath"]
        .as_str()
        .expect("json path")
        .ends_with("release-report.json"));
    assert!(paths["markdownPath"]
        .as_str()
        .expect("markdown path")
        .ends_with("release-report.md"));
}

fn seed_release_store(artifact_root: &std::path::Path, run_id: &str) -> KnowledgeRunStore {
    let store = KnowledgeRunStore::open(artifact_root, run_id).expect("open store");
    store
        .record_run(
            Some(FINGERPRINT),
            json!({
                "createdBy": "release report test",
                "packId": "all-of-create-aeronautics",
                "instancePath": "/packs/All of Create - Aeronautics",
            }),
        )
        .expect("record run");
    store
        .record_phase_checkpoint(
            KnowledgeRunPhase::Intake,
            PhaseCheckpointStatus::Succeeded,
            Some(FINGERPRINT),
            json!({
                "packId": "all-of-create-aeronautics",
                "instancePath": "/packs/All of Create - Aeronautics",
            }),
        )
        .expect("intake checkpoint");
    store
}

fn seed_release_artifacts(store: &KnowledgeRunStore) {
    let source_dir = store.run_dir().join("source");
    let bundle_path = store.run_dir().join("bundle/knowledge-index.json");
    let desktop_path = store.run_dir().join("desktop/Minecraft Pack Builder.app");
    fs::create_dir_all(&source_dir).expect("source dir");
    fs::create_dir_all(bundle_path.parent().expect("bundle parent")).expect("bundle dir");
    fs::create_dir_all(&desktop_path).expect("desktop artifact");
    fs::write(&bundle_path, "{}").expect("bundle");

    store
        .record_artifact_ref(
            "coverage-summary",
            store.run_dir().join("coverage/summary.json"),
            Some(FINGERPRINT),
            json!({
                "totalObligations": 2,
                "coveredObligations": 2,
                "openBlockers": 0,
            }),
        )
        .expect("coverage artifact");
    store
        .record_artifact_ref(
            "evidence-summary",
            store.run_dir().join("coverage/evidence-summary.json"),
            Some(FINGERPRINT),
            json!({"static": 1, "runtime": 1}),
        )
        .expect("evidence artifact");
    store
        .record_artifact_ref(
            "model-candidate",
            store.run_dir().join("workers/model.json"),
            Some(FINGERPRINT),
            json!({"candidateLabel": "local-qwen-fixture", "hardwareFit": "sufficient"}),
        )
        .expect("model artifact");
    store
        .record_artifact_ref(
            "worker-evaluation",
            store.run_dir().join("workers/eval.json"),
            Some(FINGERPRINT),
            json!({"task": "claim extraction", "passed": true}),
        )
        .expect("worker eval");
    store
        .record_artifact_ref(
            "fine-tuning-decision",
            store.run_dir().join("workers/fine-tuning.json"),
            Some(FINGERPRINT),
            json!({"state": "NotUsed", "reason": "base evaluation passed"}),
        )
        .expect("fine tuning");
    store
        .record_artifact_ref(
            "experiment-summary",
            store.run_dir().join("lab/summary.json"),
            Some(FINGERPRINT),
            json!({"planned": 1, "accepted": 1, "rejected": 0}),
        )
        .expect("experiment summary");
    store
        .record_artifact_ref(
            "retry-statistics",
            store.run_dir().join("lab/retries.json"),
            Some(FINGERPRINT),
            json!({"retryableFailures": 0, "exhausted": 0}),
        )
        .expect("retry stats");
    store
        .record_artifact_ref(
            "knowledge-source-dir",
            &source_dir,
            Some(FINGERPRINT),
            json!({"packId": "all-of-create-aeronautics"}),
        )
        .expect("source dir");
    store
        .record_artifact_ref(
            "runtime-bundle",
            &bundle_path,
            Some(FINGERPRINT),
            json!({"checksum": "abc123", "compressedSizeBytes": 512}),
        )
        .expect("runtime bundle");
    store
        .record_artifact_ref(
            "product-validation-report",
            store
                .run_dir()
                .join("reports/product-validation-report.json"),
            Some(FINGERPRINT),
            json!({
                "patcherValidation": {"status": "passed"},
                "clonedRuntimeValidation": {"status": "passed"},
                "mcpQueryValidation": {"status": "passed"},
            }),
        )
        .expect("product validation");
    store
        .record_artifact_ref(
            "desktop-artifact",
            &desktop_path,
            Some(FINGERPRINT),
            json!({"platform": "macOS"}),
        )
        .expect("desktop artifact");
    store
        .record_approval(
            ApprovalKind::LongRun,
            None,
            true,
            "operator approved local long run",
            json!({ "source": "fixture" }),
        )
        .expect("approval");
}
