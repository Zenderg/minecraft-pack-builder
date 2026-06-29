use std::fs;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use mpb_knowledge::KnowledgeRunStore;
use serde_json::json;

#[test]
fn fingerprint_command_prints_exact_fingerprint_document_summary() {
    let instance = unique_temp_dir("mpb-knowledge-cli-fingerprint");
    fs::create_dir_all(instance.join("minecraft/mods")).expect("create fixture dirs");
    fs::write(instance.join("instance.cfg"), "name=Fixture Pack\n").expect("write instance cfg");
    fs::write(
        instance.join("mmc-pack.json"),
        r#"{"components":[{"uid":"net.minecraft","version":"1.21.1"},{"uid":"net.neoforged","version":"21.1.233"}]}"#,
    )
    .expect("write mmc pack");
    fs::write(instance.join("minecraft/mods/example.jar"), b"fixture mod")
        .expect("write fixture mod");

    let output = Command::new(env!("CARGO_BIN_EXE_mpb-knowledge"))
        .arg("fingerprint")
        .arg(&instance)
        .arg("builder-test")
        .arg("lab-test")
        .arg("schema-test")
        .output()
        .expect("run fingerprint command");

    assert!(
        output.status.success(),
        "fingerprint command failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).expect("fingerprint output is utf-8");
    let json: serde_json::Value = serde_json::from_str(&stdout).expect("fingerprint output json");
    assert!(json["fingerprint"]
        .as_str()
        .is_some_and(|value| !value.is_empty()));
    assert_eq!(json["document"]["modpackIdentity"], "Fixture Pack");
    assert_eq!(json["document"]["minecraftVersion"], "1.21.1");
    assert_eq!(json["document"]["loader"], "NeoForge");
    assert!(json["document"]["inputs"]
        .as_array()
        .expect("fingerprint inputs")
        .iter()
        .any(|input| input["path"] == "mods/example.jar"));

    fs::remove_dir_all(instance).expect("remove fixture dir");
}

#[test]
fn release_attach_commands_persist_validated_pipeline_inputs() {
    let root = unique_temp_dir("mpb-knowledge-cli-attach");
    let artifact_root = root.join("knowledge");
    let source_dir = root.join("source");
    let run_id = "run-cli-attach";
    let fingerprint = "fixture-fingerprint";
    write_minimal_source_pack(&source_dir, fingerprint);

    let store = KnowledgeRunStore::open(&artifact_root, run_id).expect("open run store");
    store
        .record_run(
            Some(fingerprint),
            json!({"createdBy": "cli attach test", "instancePath": "fixture"}),
        )
        .expect("record run");
    drop(store);

    let source_output = Command::new(env!("CARGO_BIN_EXE_mpb-knowledge"))
        .arg("release")
        .arg("attach-source")
        .arg(run_id)
        .arg(&source_dir)
        .arg("--artifact-root")
        .arg(&artifact_root)
        .output()
        .expect("run attach-source command");
    assert!(
        source_output.status.success(),
        "attach-source failed: {}",
        String::from_utf8_lossy(&source_output.stderr)
    );

    let model_path = root.join("fixture-worker-model.gguf");
    fs::write(&model_path, b"fixture model bytes").expect("write model");
    let model_output = Command::new(env!("CARGO_BIN_EXE_mpb-knowledge"))
        .arg("release")
        .arg("attach-worker-model")
        .arg(run_id)
        .arg(&model_path)
        .arg("--identity")
        .arg("fixture-local-model")
        .arg("--hardware-fit")
        .arg("Fits")
        .arg("--artifact-root")
        .arg(&artifact_root)
        .output()
        .expect("run attach-worker-model command");
    assert!(
        model_output.status.success(),
        "attach-worker-model failed: {}",
        String::from_utf8_lossy(&model_output.stderr)
    );

    let store = KnowledgeRunStore::open(&artifact_root, run_id).expect("reopen run store");
    let draft = store
        .latest_artifact_ref("extraction-draft")
        .expect("query draft")
        .expect("draft artifact");
    assert_eq!(draft.target_fingerprint.as_deref(), Some(fingerprint));
    assert!(std::path::Path::new(&draft.path).is_file());
    assert!(store
        .latest_artifact_ref("knowledge-source-dir")
        .expect("query source dir")
        .is_some());
    let model = store
        .latest_artifact_ref("worker-model")
        .expect("query worker model")
        .expect("worker model artifact");
    assert_eq!(model.target_fingerprint.as_deref(), Some(fingerprint));
    assert_eq!(model.detail["identity"], "fixture-local-model");
    assert_eq!(model.detail["hardwareFit"], "Fits");

    let runtime_evidence_path = root.join("cloned-runtime-validation-evidence.json");
    fs::write(
        &runtime_evidence_path,
        serde_json::to_vec_pretty(&json!({
            "status": "passed",
            "label": "real cloned Prism runtime",
            "detail": "operator launched the disposable clone and verified Minecraft reached the MPB runtime",
            "artifactPaths": ["knowledge/prism-clones/run-cli-attach/instance"]
        }))
        .expect("runtime evidence json"),
    )
    .expect("write runtime evidence");
    let runtime_output = Command::new(env!("CARGO_BIN_EXE_mpb-knowledge"))
        .arg("release")
        .arg("attach-runtime-evidence")
        .arg(run_id)
        .arg(&runtime_evidence_path)
        .arg("--artifact-root")
        .arg(&artifact_root)
        .output()
        .expect("run attach-runtime-evidence command");
    assert!(
        runtime_output.status.success(),
        "attach-runtime-evidence failed: {}",
        String::from_utf8_lossy(&runtime_output.stderr)
    );
    let store = KnowledgeRunStore::open(&artifact_root, run_id).expect("reopen run store");
    let runtime_evidence = store
        .latest_artifact_ref("cloned-runtime-validation-evidence")
        .expect("query runtime evidence")
        .expect("runtime evidence artifact");
    assert_eq!(
        runtime_evidence.target_fingerprint.as_deref(),
        Some(fingerprint)
    );
    assert_eq!(runtime_evidence.detail["status"], "passed");
    assert!(store
        .events()
        .expect("events")
        .iter()
        .any(|event| event.event_kind == "release.runtime_evidence_attached"));

    fs::remove_dir_all(root).expect("remove fixture root");
}

fn unique_temp_dir(label: &str) -> std::path::PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock after unix epoch")
        .as_nanos();
    let path = std::env::temp_dir().join(format!("{label}-{nanos}"));
    fs::create_dir_all(&path).expect("create temp dir");
    path
}

fn write_minimal_source_pack(source_dir: &std::path::Path, fingerprint: &str) {
    fs::create_dir_all(source_dir).expect("create source dir");
    fs::write(
        source_dir.join("manifest.json"),
        serde_json::to_vec_pretty(&json!({
            "packId": "fixture-pack",
            "packVersion": "2026.06.29",
            "schemaVersion": "mpb-knowledge-v1",
            "modpackId": "fixture-pack",
            "modpackVersion": "1.0.0",
            "minecraftVersion": "1.21.1",
            "loader": "NeoForge",
            "loaderVersion": "21.1.233",
            "targetFingerprint": fingerprint,
            "computedFingerprint": fingerprint,
            "builderVersion": "mpb-knowledge-test",
            "labVersion": "mpb-lab-test"
        }))
        .expect("manifest json"),
    )
    .expect("write manifest");
    fs::write(
        source_dir.join("entities.jsonl"),
        serde_json::to_string(&json!({
            "id": "fixture:item",
            "kind": "item",
            "localizedNames": {"en_us": "Fixture Item"},
            "tags": ["namespace:fixture"],
            "useCases": ["validation fixture"],
            "interfaces": ["inventory_item"],
            "mechanics": ["static metadata"],
            "covered": true
        }))
        .expect("entity json")
            + "\n",
    )
    .expect("write entities");
    fs::write(
        source_dir.join("evidence.jsonl"),
        serde_json::to_string(&json!({
            "id": "det-fixture",
            "kind": "deterministic_source",
            "summary": "Fixture source record.",
            "fingerprint": fingerprint,
            "accepted": true
        }))
        .expect("evidence json")
            + "\n",
    )
    .expect("write evidence");
    fs::write(
        source_dir.join("claims.jsonl"),
        serde_json::to_string(&json!({
            "id": "claim-fixture",
            "entityId": "fixture:item",
            "kind": "static",
            "statement": "Fixture item is present in deterministic source records.",
            "evidenceIds": ["det-fixture"],
            "workerDecisionIds": []
        }))
        .expect("claim json")
            + "\n",
    )
    .expect("write claims");
    for file in [
        "recipes.jsonl",
        "relationships.jsonl",
        "overlays.jsonl",
        "worker-decisions.jsonl",
    ] {
        fs::write(source_dir.join(file), "").expect("write empty jsonl");
    }
}
