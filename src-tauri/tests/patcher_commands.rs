use std::fs;
use std::path::Path;

use app_tauri_lib::{
    patch_prism_instance_path, patcher_instances_for_root, remove_patch_for_instance_path,
};
use tempfile::tempdir;

#[test]
fn patcher_instances_include_patch_statuses_for_root() {
    let temp = tempdir().expect("temp dir");
    write_instance(
        temp.path(),
        "Fabric Pack",
        r#"{
          "components": [
            { "uid": "net.minecraft", "version": "1.20.1" },
            { "uid": "net.fabricmc.fabric-loader", "version": "0.16.9" }
          ]
        }"#,
    );

    let instances = patcher_instances_for_root(temp.path()).expect("instances");

    assert_eq!(instances.len(), 1);
    assert_eq!(instances[0].display_name, "Fabric Pack");
    assert_eq!(instances[0].patch_status, "notPatched");
    assert_eq!(instances[0].knowledge_status, "unavailable");
    assert!(instances[0]
        .knowledge_reason
        .as_deref()
        .unwrap_or_default()
        .contains("No first-party curated knowledge bundle matches"));
    assert_eq!(instances[0].loader.as_deref(), Some("Fabric"));
    assert_eq!(instances[0].minecraft_version.as_deref(), Some("1.20.1"));
}

#[test]
fn patch_command_applies_and_remove_command_unpatches_instance_path() {
    let temp = tempdir().expect("temp dir");
    let instance_path = write_instance(
        temp.path(),
        "Forge Pack",
        r#"{
          "components": [
            { "uid": "net.minecraft", "version": "1.20.1" },
            { "uid": "net.minecraftforge", "version": "47.4.20" }
          ]
        }"#,
    );

    let applied = patch_prism_instance_path(&instance_path, "apply").expect("apply");
    assert_eq!(applied.status, "patched");
    assert!(instance_path.join("mpb/patch-manifest.json").is_file());

    let removed = remove_patch_for_instance_path(&instance_path, false).expect("remove");
    assert_eq!(removed.status, "notPatched");
    assert!(!instance_path.join("mpb/patch-manifest.json").exists());
    assert!(instance_path.join("mpb/schemes").is_dir());
}

#[test]
fn patcher_summary_marks_live_mpb_runtime_as_instance_running() {
    let temp = tempdir().expect("temp dir");
    let instance_path = write_instance(
        temp.path(),
        "Fabric Pack",
        r#"{
          "components": [
            { "uid": "net.minecraft", "version": "1.20.1" },
            { "uid": "net.fabricmc.fabric-loader", "version": "0.16.9" }
          ]
        }"#,
    );
    fs::create_dir_all(instance_path.join("mpb")).expect("mpb dir");
    fs::write(
        instance_path.join("mpb/runtime.pid"),
        std::process::id().to_string(),
    )
    .expect("pid");

    let instances = patcher_instances_for_root(temp.path()).expect("instances");

    assert_eq!(instances[0].patch_status, "instanceRunning");
    assert!(instances[0]
        .patch_reason
        .as_deref()
        .unwrap_or_default()
        .contains("appears to be running"));
}

fn write_instance(root: &Path, folder: &str, mmc_pack: &str) -> std::path::PathBuf {
    let instance_dir = root.join("instances").join(folder);
    fs::create_dir_all(instance_dir.join(".minecraft/mods")).expect("minecraft dir");
    fs::write(
        instance_dir.join("instance.cfg"),
        format!("name={folder}\n"),
    )
    .expect("cfg");
    fs::write(instance_dir.join("mmc-pack.json"), mmc_pack).expect("pack");
    instance_dir
}
