use std::fs;
use std::path::Path;

use mpb_assets::{validate_prism_root, PrismInstanceStatus};
use tempfile::tempdir;

#[test]
fn validates_prism_root_and_counts_instances() {
    let temp = tempdir().expect("temp dir");
    let root = temp.path();
    write_instance(
        root,
        "All of Create",
        "name=All of Create\n",
        r#"{
          "components": [
            { "uid": "net.minecraft", "version": "1.21.1" },
            { "uid": "net.neoforged", "version": "21.1.233" }
          ]
        }"#,
    );
    write_instance(root, "Vanilla", "name=Vanilla\n", "");
    fs::create_dir_all(root.join("instances").join("Not An Instance")).expect("junk dir");

    let validation = validate_prism_root(root).expect("validate root");

    assert!(validation.valid);
    assert_eq!(validation.instance_count, 2);
    assert_eq!(validation.instances.len(), 2);
    assert_eq!(validation.instances[0].display_name, "All of Create");
    assert_eq!(
        validation.instances[0].minecraft_version.as_deref(),
        Some("1.21.1")
    );
    assert_eq!(validation.instances[0].loader.as_deref(), Some("NeoForge"));
    assert_eq!(
        validation.instances[0].loader_version.as_deref(),
        Some("21.1.233")
    );
    assert_eq!(validation.instances[0].status, PrismInstanceStatus::Pending);
}

#[test]
fn invalid_prism_root_explains_launcher_root_requirement() {
    let temp = tempdir().expect("temp dir");

    let validation = validate_prism_root(temp.path()).expect("validate root");

    assert!(!validation.valid);
    assert_eq!(validation.instance_count, 0);
    assert!(validation.message.contains("Folders > Launcher Root"));
}

#[test]
fn prefers_prism_minecraft_directory_over_dot_minecraft_when_present() {
    let temp = tempdir().expect("temp dir");
    let root = temp.path();
    let instance_dir = write_instance(root, "AOC", "name=AOC\n", "");
    fs::remove_dir_all(instance_dir.join(".minecraft")).expect("remove fallback dir");
    fs::create_dir_all(instance_dir.join("minecraft").join("mods")).expect("Prism game dir");

    let instance = validate_prism_root(root).expect("validate root").instances[0].clone();

    assert!(instance.minecraft_dir.ends_with("instances/AOC/minecraft"));
}

#[test]
fn identity_fingerprint_survives_folder_move_but_content_fingerprint_tracks_mod_changes() {
    let temp = tempdir().expect("temp dir");
    let root = temp.path();
    let instance_dir = write_instance(
        root,
        "Factory Pack",
        "name=Factory Pack\niconKey=gear\n",
        r#"{
          "components": [
            { "uid": "net.minecraft", "version": "1.20.1" },
            { "uid": "net.fabricmc.fabric-loader", "version": "0.16.9" }
          ]
        }"#,
    );
    let mods = instance_dir.join(".minecraft").join("mods");
    fs::create_dir_all(&mods).expect("mods dir");
    fs::write(mods.join("create.jar"), b"first").expect("mod file");

    let first = validate_prism_root(root).expect("validate root").instances[0].clone();
    let moved_root = temp.path().join("MovedPrism");
    fs::create_dir_all(moved_root.join("instances")).expect("moved instances");
    fs::rename(
        instance_dir,
        moved_root.join("instances").join("Renamed Folder"),
    )
    .expect("move instance");
    let moved = validate_prism_root(&moved_root)
        .expect("validate moved root")
        .instances[0]
        .clone();

    assert_eq!(moved.identity_fingerprint, first.identity_fingerprint);
    assert_eq!(moved.content_fingerprint, first.content_fingerprint);

    fs::write(
        moved_root
            .join("instances")
            .join("Renamed Folder")
            .join(".minecraft")
            .join("mods")
            .join("create.jar"),
        b"changed",
    )
    .expect("change mod file");
    let changed = validate_prism_root(&moved_root)
        .expect("validate changed root")
        .instances[0]
        .clone();

    assert_eq!(changed.identity_fingerprint, first.identity_fingerprint);
    assert_ne!(changed.content_fingerprint, first.content_fingerprint);
}

#[test]
fn content_fingerprint_tracks_pack_affecting_config_and_script_folders() {
    let temp = tempdir().expect("temp dir");
    let root = temp.path();
    let instance_dir = write_instance(root, "Scripted Pack", "name=Scripted Pack\n", "");
    let minecraft = instance_dir.join(".minecraft");
    fs::create_dir_all(minecraft.join("config")).expect("config");
    fs::create_dir_all(minecraft.join("datapacks")).expect("datapacks");
    fs::create_dir_all(minecraft.join("kubejs").join("server_scripts")).expect("kubejs");
    fs::create_dir_all(minecraft.join("scripts")).expect("scripts");
    fs::create_dir_all(minecraft.join("resourcepacks")).expect("resourcepacks");
    fs::write(minecraft.join("config").join("pack.toml"), b"first").expect("config");
    fs::write(minecraft.join("datapacks").join("data.zip"), b"data").expect("datapack");
    fs::write(
        minecraft
            .join("kubejs")
            .join("server_scripts")
            .join("recipes.js"),
        b"kube",
    )
    .expect("kubejs");
    fs::write(minecraft.join("scripts").join("recipes.zs"), b"script").expect("script");
    fs::write(minecraft.join("resourcepacks").join("guide.zip"), b"guide").expect("resourcepack");

    let first = validate_prism_root(root).expect("validate root").instances[0].clone();
    fs::write(minecraft.join("config").join("pack.toml"), b"changed").expect("config changed");
    let changed_config = validate_prism_root(root).expect("validate root").instances[0].clone();
    fs::write(
        minecraft
            .join("kubejs")
            .join("server_scripts")
            .join("recipes.js"),
        b"kube changed",
    )
    .expect("kube changed");
    let changed_script = validate_prism_root(root).expect("validate root").instances[0].clone();

    assert_ne!(
        changed_config.content_fingerprint,
        first.content_fingerprint
    );
    assert_ne!(
        changed_script.content_fingerprint,
        changed_config.content_fingerprint
    );
}

fn write_instance(root: &Path, folder: &str, cfg: &str, mmc_pack: &str) -> std::path::PathBuf {
    let instance_dir = root.join("instances").join(folder);
    fs::create_dir_all(instance_dir.join(".minecraft")).expect("minecraft dir");
    fs::write(instance_dir.join("instance.cfg"), cfg).expect("instance cfg");
    if !mmc_pack.is_empty() {
        fs::write(instance_dir.join("mmc-pack.json"), mmc_pack).expect("mmc pack");
    }
    instance_dir
}
