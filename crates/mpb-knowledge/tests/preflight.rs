use std::fs;
use std::process::Command;

use mpb_knowledge::{run_preflight, HardwareFit, KnowledgeRunPhase, KnowledgeRunStore};

#[test]
fn preflight_reports_environment_without_mutating_instance_or_model_cache() {
    let temp = tempfile::tempdir().expect("tempdir");
    let instance = temp.path().join("instance");
    let artifact_root = temp.path().join("knowledge");
    write_prism_fixture(&instance);
    let before = fixture_snapshot(&instance);

    let report = run_preflight(&instance, &artifact_root).expect("run preflight");

    assert_eq!(fixture_snapshot(&instance), before);
    assert!(!artifact_root.join("model-cache").exists());
    assert_eq!(report.cpu_architecture, std::env::consts::ARCH);
    assert_eq!(report.operating_system, std::env::consts::OS);
    assert!(report.prism_instance.readable);
    assert!(report.prism_instance.has_instance_cfg);
    assert!(report.prism_instance.has_mmc_pack);
    assert!(report.expected_clone_size_bytes >= "fixture mod".len() as u64);
    assert!(report
        .phase_duration_estimates
        .iter()
        .any(|estimate| estimate.phase == KnowledgeRunPhase::Preflight));
    assert!(report
        .model_needs
        .iter()
        .any(|need| need.hardware_fit != HardwareFit::Unknown || !need.reason.is_empty()));
}

#[test]
fn preflight_cli_prints_json_and_persists_checkpoint_when_run_id_is_supplied() {
    let temp = tempfile::tempdir().expect("tempdir");
    let instance = temp.path().join("instance");
    let artifact_root = temp.path().join("knowledge");
    write_prism_fixture(&instance);

    let output = Command::new(env!("CARGO_BIN_EXE_mpb-knowledge"))
        .arg("preflight")
        .arg(&instance)
        .arg("--artifact-root")
        .arg(&artifact_root)
        .arg("--run-id")
        .arg("run-preflight")
        .output()
        .expect("run preflight command");

    assert!(
        output.status.success(),
        "preflight command failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let json: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("preflight stdout json");
    assert_eq!(json["prismInstance"]["readable"], true);
    assert_eq!(json["artifactRoot"], artifact_root.display().to_string());

    let store = KnowledgeRunStore::open(&artifact_root, "run-preflight").expect("open run store");
    let checkpoint = store
        .latest_successful_checkpoint()
        .expect("load latest checkpoint")
        .expect("preflight checkpoint exists");
    assert_eq!(checkpoint.phase, KnowledgeRunPhase::Preflight);
}

fn write_prism_fixture(instance: &std::path::Path) {
    fs::create_dir_all(instance.join("minecraft/mods")).expect("create fixture dirs");
    fs::write(instance.join("instance.cfg"), "name=Fixture Pack\n").expect("write instance cfg");
    fs::write(
        instance.join("mmc-pack.json"),
        r#"{"components":[{"uid":"net.minecraft","version":"1.21.1"}]}"#,
    )
    .expect("write mmc pack");
    fs::write(instance.join("minecraft/mods/example.jar"), b"fixture mod").expect("write mod file");
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
