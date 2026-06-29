use std::fs;
use std::process::Command;

use mpb_knowledge::{
    CleanupPolicy, KnowledgeRunPhase, KnowledgeRunStore, LaunchProbeResult, TargetManager,
};
use serde_json::json;

#[test]
fn target_manager_clone_preserves_original_files_and_records_artifacts() {
    let temp = tempfile::tempdir().expect("tempdir");
    let artifact_root = temp.path().join("knowledge");
    let instance = temp.path().join("instance");
    write_prism_fixture(&instance);
    let original_snapshot = fixture_snapshot(&instance);

    let manager = TargetManager::new(&artifact_root);
    let original = manager
        .inspect_original(&instance)
        .expect("inspect original target");
    let clone = manager
        .create_disposable_clone("run-target", &instance)
        .expect("create disposable clone");

    assert_eq!(fixture_snapshot(&instance), original_snapshot);
    assert_eq!(fixture_snapshot(&clone.clone_path), original_snapshot);
    assert_eq!(clone.source_path, instance);
    assert_eq!(clone.fingerprint_before, original.fingerprint.fingerprint);
    assert_eq!(clone.fingerprint_after, original.fingerprint.fingerprint);

    let store = KnowledgeRunStore::open(&artifact_root, "run-target").expect("open run store");
    let artifact_refs = store.artifact_refs().expect("load artifact refs");
    assert!(artifact_refs.iter().any(|artifact| {
        artifact.artifact_kind == "target-original"
            && artifact.path == instance.display().to_string()
            && artifact.target_fingerprint.as_deref()
                == Some(original.fingerprint.fingerprint.as_str())
    }));
    assert!(artifact_refs.iter().any(|artifact| {
        artifact.artifact_kind == "target-clone"
            && artifact.path == clone.clone_path.display().to_string()
            && artifact.detail["cleanupPolicy"] == json!("DeleteAfterReport")
    }));
}

#[test]
fn target_manager_patch_hooks_and_cleanup_are_confined_to_disposable_clone() {
    let temp = tempfile::tempdir().expect("tempdir");
    let artifact_root = temp.path().join("knowledge");
    let instance = temp.path().join("instance");
    write_prism_fixture(&instance);
    let original_snapshot = fixture_snapshot(&instance);

    let manager = TargetManager::new(&artifact_root);
    let clone = manager
        .create_disposable_clone("run-patch", &instance)
        .expect("create disposable clone");
    manager
        .install_clone_instrumentation(
            "run-patch",
            &clone.clone_path,
            "mpb/lab/runtime-instrumentation.json",
            br#"{"enabled":true}"#,
        )
        .expect("install instrumentation in clone");
    manager
        .set_cleanup_policy("run-patch", CleanupPolicy::DeleteOnSuccess, Some(&clone))
        .expect("persist cleanup policy");
    manager
        .cleanup_clone(
            "run-patch",
            &clone.clone_path,
            CleanupPolicy::DeleteOnSuccess,
            true,
        )
        .expect("cleanup clone");

    assert_eq!(fixture_snapshot(&instance), original_snapshot);
    assert!(!clone.clone_path.exists());
    assert!(!instance
        .join("mpb/lab/runtime-instrumentation.json")
        .exists());

    let store = KnowledgeRunStore::open(&artifact_root, "run-patch").expect("open run store");
    let events = store.events().expect("load events");
    assert!(events
        .iter()
        .any(|event| event.event_kind == "target.instrumentation_installed"));
    assert!(events
        .iter()
        .any(|event| event.event_kind == "target.cleanup_completed"));
}

#[test]
fn target_manager_launch_manual_intervention_checkpoint_survives_reopen() {
    let temp = tempfile::tempdir().expect("tempdir");
    let artifact_root = temp.path().join("knowledge");
    let instance = temp.path().join("instance");
    let fake_launcher = temp.path().join("PrismLauncher");
    write_prism_fixture(&instance);
    fs::write(&fake_launcher, launcher_script()).expect("write fake launcher");
    make_executable(&fake_launcher);

    let manager = TargetManager::new(&artifact_root).with_launcher_command(vec![
        fake_launcher.display().to_string(),
        "--launch".to_string(),
    ]);
    manager
        .create_disposable_clone("run-launch", &instance)
        .expect("create disposable clone");

    let probe = manager
        .probe_launch("run-launch")
        .expect("record launch probe");
    assert_eq!(probe.result, LaunchProbeResult::ManualInterventionRequired);
    assert_eq!(probe.operating_system, std::env::consts::OS);
    assert!(probe
        .observed_status_text
        .as_deref()
        .expect("observed status")
        .contains("manual account prompt"));
    assert_eq!(
        probe.resume_command,
        "mpb-knowledge target probe-launch run-launch --artifact-root ".to_string()
            + &artifact_root.display().to_string()
    );

    let reopened = TargetManager::new(&artifact_root);
    let resumed = reopened
        .latest_launch_probe("run-launch")
        .expect("load latest probe")
        .expect("probe checkpoint exists");
    assert_eq!(
        resumed.result,
        LaunchProbeResult::ManualInterventionRequired
    );
    assert_eq!(resumed.phase, KnowledgeRunPhase::Clone);
    assert!(resumed
        .launcher_command_attempted
        .join(" ")
        .contains("--launch"));
}

#[test]
fn target_manager_cli_clone_and_probe_launch_persist_run_state() {
    let temp = tempfile::tempdir().expect("tempdir");
    let artifact_root = temp.path().join("knowledge");
    let instance = temp.path().join("instance");
    write_prism_fixture(&instance);

    let clone_output = Command::new(env!("CARGO_BIN_EXE_mpb-knowledge"))
        .arg("target")
        .arg("clone")
        .arg("run-cli-target")
        .arg(&instance)
        .arg("--artifact-root")
        .arg(&artifact_root)
        .output()
        .expect("run target clone command");
    assert!(
        clone_output.status.success(),
        "target clone failed: {}",
        String::from_utf8_lossy(&clone_output.stderr)
    );
    let clone_json: serde_json::Value =
        serde_json::from_slice(&clone_output.stdout).expect("clone stdout json");
    assert!(clone_json["clonePath"]
        .as_str()
        .expect("clone path")
        .contains("knowledge/prism-clones/run-cli-target/instance"));

    let probe_output = Command::new(env!("CARGO_BIN_EXE_mpb-knowledge"))
        .arg("target")
        .arg("probe-launch")
        .arg("run-cli-target")
        .arg("--artifact-root")
        .arg(&artifact_root)
        .output()
        .expect("run probe command");
    assert!(
        probe_output.status.success(),
        "probe launch failed: {}",
        String::from_utf8_lossy(&probe_output.stderr)
    );
    let probe_json: serde_json::Value =
        serde_json::from_slice(&probe_output.stdout).expect("probe stdout json");
    assert_eq!(probe_json["result"], "LauncherUnavailable");

    let store = KnowledgeRunStore::open(&artifact_root, "run-cli-target").expect("open store");
    let checkpoint = store
        .latest_successful_checkpoint()
        .expect("load checkpoint")
        .expect("checkpoint exists");
    assert_eq!(checkpoint.phase, KnowledgeRunPhase::Clone);
}

fn write_prism_fixture(instance: &std::path::Path) {
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

fn fixture_snapshot(instance: &std::path::Path) -> Vec<(String, Vec<u8>)> {
    let mut snapshot = Vec::new();
    collect_files(instance, instance, &mut snapshot);
    snapshot.sort_by(|left, right| left.0.cmp(&right.0));
    snapshot
}

fn collect_files(base: &std::path::Path, dir: &std::path::Path, out: &mut Vec<(String, Vec<u8>)>) {
    for entry in fs::read_dir(dir).expect("read fixture dir") {
        let entry = entry.expect("fixture entry");
        let path = entry.path();
        if path.is_dir() {
            collect_files(base, &path, out);
        } else {
            out.push((
                path.strip_prefix(base)
                    .expect("fixture relative path")
                    .to_string_lossy()
                    .replace('\\', "/"),
                fs::read(&path).expect("read fixture file"),
            ));
        }
    }
}

fn launcher_script() -> &'static [u8] {
    if cfg!(windows) {
        b"@echo manual account prompt\n"
    } else {
        b"#!/bin/sh\necho manual account prompt\n"
    }
}

#[cfg(unix)]
fn make_executable(path: &std::path::Path) {
    use std::os::unix::fs::PermissionsExt;

    let mut permissions = fs::metadata(path).expect("metadata").permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(path, permissions).expect("chmod");
}

#[cfg(not(unix))]
fn make_executable(_path: &std::path::Path) {}
