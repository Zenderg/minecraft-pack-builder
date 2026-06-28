use std::fs;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

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

fn unique_temp_dir(label: &str) -> std::path::PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock after unix epoch")
        .as_nanos();
    let path = std::env::temp_dir().join(format!("{label}-{nanos}"));
    fs::create_dir_all(&path).expect("create temp dir");
    path
}
