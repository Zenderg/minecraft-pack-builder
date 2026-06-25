use std::fs;
use std::path::Path;

use mpb_assets::{
    apply_mpb_patch, evaluate_mpb_patch, remove_mpb_patch, validate_prism_root, MpbPatchAction,
    MpbPatchStatus,
};
use tempfile::tempdir;

#[test]
fn evaluates_supported_and_unsupported_prism_instances_without_guessing() {
    let temp = tempdir().expect("temp dir");
    let root = temp.path();
    write_instance(
        root,
        "Fabric 120",
        r#"{
          "components": [
            { "uid": "net.minecraft", "version": "1.20.1" },
            { "uid": "net.fabricmc.fabric-loader", "version": "0.16.9" }
          ]
        }"#,
    );
    write_instance(
        root,
        "Old Forge",
        r#"{
          "components": [
            { "uid": "net.minecraft", "version": "1.19.4" },
            { "uid": "net.minecraftforge", "version": "45.0.1" }
          ]
        }"#,
    );
    write_instance(
        root,
        "Future Fabric",
        r#"{
          "components": [
            { "uid": "net.minecraft", "version": "26.2" },
            { "uid": "net.fabricmc.fabric-loader", "version": "0.19.3" }
          ]
        }"#,
    );
    write_instance(
        root,
        "Vanilla",
        r#"{
          "components": [
            { "uid": "net.minecraft", "version": "1.21.1" }
          ]
        }"#,
    );

    let validation = validate_prism_root(root).expect("root");
    let fabric = validation
        .instances
        .iter()
        .find(|instance| instance.display_name == "Fabric 120")
        .expect("fabric instance");
    let old = validation
        .instances
        .iter()
        .find(|instance| instance.display_name == "Old Forge")
        .expect("old instance");
    let vanilla = validation
        .instances
        .iter()
        .find(|instance| instance.display_name == "Vanilla")
        .expect("vanilla instance");
    let future_fabric = validation
        .instances
        .iter()
        .find(|instance| instance.display_name == "Future Fabric")
        .expect("future fabric instance");

    assert_eq!(
        evaluate_mpb_patch(fabric).status,
        MpbPatchStatus::NotPatched
    );
    assert_eq!(evaluate_mpb_patch(old).status, MpbPatchStatus::Unsupported);
    assert!(evaluate_mpb_patch(old)
        .reason
        .expect("reason")
        .contains("Minecraft 1.20 or newer"));
    assert_eq!(
        evaluate_mpb_patch(vanilla).status,
        MpbPatchStatus::Unsupported
    );
    assert!(evaluate_mpb_patch(vanilla)
        .reason
        .expect("reason")
        .contains("Fabric, Forge, or NeoForge"));
    assert_eq!(
        evaluate_mpb_patch(future_fabric).status,
        MpbPatchStatus::Unsupported
    );
    assert!(evaluate_mpb_patch(future_fabric)
        .reason
        .expect("reason")
        .contains("No bundled MPB artifact"));
}

#[test]
fn applies_repairs_and_removes_managed_patch_files() {
    let temp = tempdir().expect("temp dir");
    let root = temp.path();
    write_instance(
        root,
        "NeoForge Pack",
        r#"{
          "components": [
            { "uid": "net.minecraft", "version": "1.21.1" },
            { "uid": "net.neoforged", "version": "21.1.233" }
          ]
        }"#,
    );
    let instance = validate_prism_root(root).expect("root").instances[0].clone();

    let applied = apply_mpb_patch(&instance, MpbPatchAction::Apply).expect("apply");

    assert_eq!(applied.status, MpbPatchStatus::Patched);
    assert!(instance
        .minecraft_dir
        .join("mods/mpb-minecraft-mod.jar")
        .is_file());
    let installed_mod =
        fs::read(instance.minecraft_dir.join("mods/mpb-minecraft-mod.jar")).expect("mod jar");
    assert!(installed_mod.starts_with(b"PK"));
    assert_bytes_contain(&installed_mod, b"META-INF/neoforge.mods.toml");
    assert_bytes_contain(&installed_mod, b"com/mpb/runtime/MpbMcpHttpServer.class");
    assert!(instance
        .instance_path
        .join("mpb/patch-manifest.json")
        .is_file());
    assert_eq!(
        evaluate_mpb_patch(&instance).status,
        MpbPatchStatus::Patched
    );

    fs::write(
        instance.minecraft_dir.join("mods/mpb-minecraft-mod.jar"),
        b"changed",
    )
    .expect("damage managed file");
    assert_eq!(
        evaluate_mpb_patch(&instance).status,
        MpbPatchStatus::NeedsRepair
    );

    let repaired = apply_mpb_patch(&instance, MpbPatchAction::Repair).expect("repair");
    assert_eq!(repaired.status, MpbPatchStatus::Patched);
    assert_eq!(
        evaluate_mpb_patch(&instance).status,
        MpbPatchStatus::Patched
    );

    let removed = remove_mpb_patch(&instance, false).expect("remove");
    assert_eq!(removed.status, MpbPatchStatus::NotPatched);
    assert!(!instance
        .minecraft_dir
        .join("mods/mpb-minecraft-mod.jar")
        .exists());
    assert!(!instance
        .instance_path
        .join("mpb/patch-manifest.json")
        .exists());
    assert!(instance.instance_path.join("mpb/schemes").is_dir());
}

#[test]
fn live_mpb_runtime_pid_blocks_patch_mutations() {
    let temp = tempdir().expect("temp dir");
    let root = temp.path();
    write_instance(
        root,
        "Fabric Pack",
        r#"{
          "components": [
            { "uid": "net.minecraft", "version": "1.20.1" },
            { "uid": "net.fabricmc.fabric-loader", "version": "0.16.9" }
          ]
        }"#,
    );
    let instance = validate_prism_root(root).expect("root").instances[0].clone();
    fs::create_dir_all(instance.instance_path.join("mpb")).expect("mpb dir");
    fs::write(
        instance.instance_path.join("mpb/runtime.pid"),
        std::process::id().to_string(),
    )
    .expect("pid");

    let status = evaluate_mpb_patch(&instance);

    assert_eq!(status.status, MpbPatchStatus::InstanceRunning);
    assert!(status
        .reason
        .expect("reason")
        .contains("appears to be running"));
    assert!(apply_mpb_patch(&instance, MpbPatchAction::Apply).is_err());
    assert!(remove_mpb_patch(&instance, false).is_err());
}

#[test]
fn installs_loader_specific_real_jar_artifacts() {
    let temp = tempdir().expect("temp dir");
    let root = temp.path();
    write_instance(
        root,
        "Fabric Pack",
        r#"{
          "components": [
            { "uid": "net.minecraft", "version": "1.20.1" },
            { "uid": "net.fabricmc.fabric-loader", "version": "0.16.9" }
          ]
        }"#,
    );
    write_instance(
        root,
        "Forge Pack",
        r#"{
          "components": [
            { "uid": "net.minecraft", "version": "1.20.1" },
            { "uid": "net.minecraftforge", "version": "47.4.20" }
          ]
        }"#,
    );
    write_instance(
        root,
        "NeoForge Pack",
        r#"{
          "components": [
            { "uid": "net.minecraft", "version": "1.21.1" },
            { "uid": "net.neoforged", "version": "21.1.233" }
          ]
        }"#,
    );
    let validation = validate_prism_root(root).expect("root");

    for (instance_name, metadata) in [
        ("Fabric Pack", "fabric.mod.json"),
        ("Forge Pack", "META-INF/mods.toml"),
        ("NeoForge Pack", "META-INF/neoforge.mods.toml"),
    ] {
        let instance = validation
            .instances
            .iter()
            .find(|instance| instance.display_name == instance_name)
            .expect("instance");
        apply_mpb_patch(instance, MpbPatchAction::Apply).expect("apply");
        let bytes = fs::read(instance.minecraft_dir.join("mods/mpb-minecraft-mod.jar"))
            .expect("installed jar");
        assert!(bytes.starts_with(b"PK"));
        assert_bytes_contain(&bytes, metadata.as_bytes());
        assert_bytes_contain(&bytes, b"com/mpb/runtime/MpbClientRuntime.class");
    }
}

#[test]
fn unmanaged_mod_conflict_blocks_overwriting_user_file() {
    let temp = tempdir().expect("temp dir");
    let root = temp.path();
    write_instance(
        root,
        "Forge Pack",
        r#"{
          "components": [
            { "uid": "net.minecraft", "version": "1.20.1" },
            { "uid": "net.minecraftforge", "version": "47.4.20" }
          ]
        }"#,
    );
    let instance = validate_prism_root(root).expect("root").instances[0].clone();
    fs::create_dir_all(instance.minecraft_dir.join("mods")).expect("mods");
    fs::write(
        instance.minecraft_dir.join("mods/mpb-minecraft-mod.jar"),
        b"user-owned incompatible mod",
    )
    .expect("mod");

    let status = evaluate_mpb_patch(&instance);

    assert_eq!(status.status, MpbPatchStatus::Conflict);
    assert!(status.reason.expect("reason").contains("not managed"));
    assert!(apply_mpb_patch(&instance, MpbPatchAction::Apply).is_err());
}

fn assert_bytes_contain(haystack: &[u8], needle: &[u8]) {
    assert!(
        haystack
            .windows(needle.len())
            .any(|window| window == needle),
        "expected installed artifact to contain {}",
        String::from_utf8_lossy(needle)
    );
}

fn write_instance(root: &Path, folder: &str, mmc_pack: &str) {
    let instance_dir = root.join("instances").join(folder);
    fs::create_dir_all(instance_dir.join(".minecraft/mods")).expect("minecraft dir");
    fs::write(
        instance_dir.join("instance.cfg"),
        format!("name={folder}\n"),
    )
    .expect("cfg");
    fs::write(instance_dir.join("mmc-pack.json"), mmc_pack).expect("pack");
}
