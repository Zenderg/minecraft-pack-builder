use std::fs;
use std::io::Read;
use std::path::Path;

use mpb_assets::{
    apply_mpb_patch, evaluate_mpb_patch, remove_mpb_patch, validate_prism_root, MpbKnowledgeStatus,
    MpbPatchAction, MpbPatchManifest, MpbPatchStatus,
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
fn installs_repairs_and_removes_matching_curated_knowledge_bundle() {
    let temp = tempdir().expect("temp dir");
    let root = temp.path();
    write_fixture_knowledge_instance(root);
    let instance = validate_prism_root(root).expect("root").instances[0].clone();

    let pending = evaluate_mpb_patch(&instance);
    assert_eq!(pending.status, MpbPatchStatus::NotPatched);
    assert_eq!(pending.knowledge.status, MpbKnowledgeStatus::Available);
    assert_eq!(
        pending.knowledge.pack_id.as_deref(),
        Some("fixture-minimal")
    );

    let applied = apply_mpb_patch(&instance, MpbPatchAction::Apply).expect("apply");

    assert_eq!(applied.status, MpbPatchStatus::Patched);
    let bundle_path = instance
        .instance_path
        .join("mpb/knowledge/fixture-minimal/knowledge-index.json");
    assert!(bundle_path.is_file());
    let manifest = read_patch_manifest(&instance.instance_path);
    assert_eq!(
        manifest.knowledge_pack_id.as_deref(),
        Some("fixture-minimal")
    );
    assert_eq!(
        manifest.knowledge_fingerprint.as_deref(),
        Some("bc986558e54f6a24")
    );
    assert_eq!(
        manifest.knowledge_schema_version.as_deref(),
        Some("mpb-knowledge-v1")
    );
    assert!(manifest.files.iter().any(|file| {
        file.path == "mpb/knowledge/fixture-minimal/knowledge-index.json"
            && file.owner == mpb_assets::MpbFileOwner::Managed
    }));
    let patched = evaluate_mpb_patch(&instance);
    assert_eq!(patched.status, MpbPatchStatus::Patched);
    assert_eq!(patched.knowledge.status, MpbKnowledgeStatus::Installed);

    fs::write(&bundle_path, b"damaged knowledge bundle").expect("damage knowledge");
    let damaged = evaluate_mpb_patch(&instance);
    assert_eq!(damaged.status, MpbPatchStatus::NeedsRepair);
    assert!(damaged
        .reason
        .expect("repair reason")
        .contains("Managed file changed"));

    apply_mpb_patch(&instance, MpbPatchAction::Repair).expect("repair");
    assert_eq!(
        evaluate_mpb_patch(&instance).status,
        MpbPatchStatus::Patched
    );

    let scheme_dir = instance.instance_path.join("mpb/schemes");
    fs::create_dir_all(&scheme_dir).expect("scheme dir");
    fs::write(scheme_dir.join("user-scheme.json"), b"{}").expect("scheme");
    remove_mpb_patch(&instance, false).expect("remove");
    assert!(!bundle_path.exists());
    assert!(scheme_dir.join("user-scheme.json").is_file());
}

#[test]
fn matching_knowledge_metadata_changes_require_update() {
    let temp = tempdir().expect("temp dir");
    let root = temp.path();
    write_fixture_knowledge_instance(root);
    let instance = validate_prism_root(root).expect("root").instances[0].clone();
    apply_mpb_patch(&instance, MpbPatchAction::Apply).expect("apply");

    let mut manifest = read_patch_manifest(&instance.instance_path);
    manifest.knowledge_fingerprint = Some("old-fingerprint".to_string());
    write_patch_manifest(&instance.instance_path, &manifest);

    let status = evaluate_mpb_patch(&instance);
    assert_eq!(status.status, MpbPatchStatus::NeedsUpdate);
    assert!(status
        .reason
        .expect("update reason")
        .contains("Curated MPB knowledge artifact has changed"));
}

#[test]
fn existing_unmanaged_knowledge_bundle_blocks_matching_install() {
    let temp = tempdir().expect("temp dir");
    let root = temp.path();
    write_fixture_knowledge_instance(root);
    let instance = validate_prism_root(root).expect("root").instances[0].clone();
    let unmanaged = instance
        .instance_path
        .join("mpb/knowledge/fixture-minimal/knowledge-index.json");
    fs::create_dir_all(unmanaged.parent().expect("knowledge parent")).expect("knowledge dir");
    fs::write(&unmanaged, b"user knowledge").expect("knowledge");

    let status = evaluate_mpb_patch(&instance);

    assert_eq!(status.status, MpbPatchStatus::Conflict);
    assert!(status.reason.expect("reason").contains("not managed"));
    assert!(apply_mpb_patch(&instance, MpbPatchAction::Apply).is_err());
}

#[test]
fn mismatched_knowledge_fingerprint_installs_base_mod_without_curated_pack() {
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

    let pending = evaluate_mpb_patch(&instance);
    assert_eq!(pending.status, MpbPatchStatus::NotPatched);
    assert_eq!(pending.knowledge.status, MpbKnowledgeStatus::Unavailable);
    assert!(pending
        .knowledge
        .reason
        .as_deref()
        .expect("knowledge reason")
        .contains("No first-party curated knowledge bundle matches"));

    apply_mpb_patch(&instance, MpbPatchAction::Apply).expect("apply");

    assert!(instance
        .minecraft_dir
        .join("mods/mpb-minecraft-mod.jar")
        .is_file());
    assert!(!instance.instance_path.join("mpb/knowledge").exists());
    let manifest = read_patch_manifest(&instance.instance_path);
    assert_eq!(manifest.knowledge_pack_id, None);
    assert!(!manifest.knowledge_compatibility.matched);
    assert!(manifest
        .knowledge_compatibility
        .reason
        .as_deref()
        .expect("compatibility reason")
        .contains("No first-party curated knowledge bundle matches"));
    assert_eq!(
        evaluate_mpb_patch(&instance).knowledge.status,
        MpbKnowledgeStatus::Unavailable
    );
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
        if instance_name == "NeoForge Pack" {
            let mods_toml = read_zip_file(&bytes, "META-INF/neoforge.mods.toml");
            assert!(
                mods_toml.contains("loaderVersion = \"[4,)\""),
                "NeoForge 1.21.1 uses javafml 4.x; got metadata:\n{mods_toml}"
            );
        }
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

fn read_zip_file(bytes: &[u8], path: &str) -> String {
    let central_directory = find_central_directory(bytes).expect("central directory");
    let mut cursor = central_directory;
    while bytes.get(cursor..cursor + 4) == Some(b"PK\x01\x02") {
        let method = read_u16(bytes, cursor + 10);
        let compressed_size = read_u32(bytes, cursor + 20) as usize;
        let name_len = read_u16(bytes, cursor + 28) as usize;
        let extra_len = read_u16(bytes, cursor + 30) as usize;
        let comment_len = read_u16(bytes, cursor + 32) as usize;
        let local_header = read_u32(bytes, cursor + 42) as usize;
        let name_start = cursor + 46;
        let name_end = name_start + name_len;
        let name = std::str::from_utf8(&bytes[name_start..name_end]).expect("zip name");
        if name == path {
            return read_zip_local_file(bytes, local_header, method, compressed_size);
        }
        cursor = name_end + extra_len + comment_len;
    }
    panic!("zip entry not found: {path}");
}

fn read_zip_local_file(
    bytes: &[u8],
    local_header: usize,
    method: u16,
    compressed_size: usize,
) -> String {
    assert_eq!(
        bytes.get(local_header..local_header + 4),
        Some(&b"PK\x03\x04"[..])
    );
    let name_len = read_u16(bytes, local_header + 26) as usize;
    let extra_len = read_u16(bytes, local_header + 28) as usize;
    let data_start = local_header + 30 + name_len + extra_len;
    let data_end = data_start + compressed_size;
    let data = &bytes[data_start..data_end];
    let raw = match method {
        0 => data.to_vec(),
        8 => {
            let mut decoder = flate2::read::DeflateDecoder::new(data);
            let mut decompressed = Vec::new();
            decoder
                .read_to_end(&mut decompressed)
                .expect("deflate entry");
            decompressed
        }
        other => panic!("unsupported zip compression method: {other}"),
    };
    String::from_utf8(raw).expect("utf-8 zip entry")
}

fn find_central_directory(bytes: &[u8]) -> Option<usize> {
    bytes
        .windows(4)
        .rposition(|window| window == b"PK\x05\x06")
        .map(|eocd| read_u32(bytes, eocd + 16) as usize)
}

fn read_u16(bytes: &[u8], offset: usize) -> u16 {
    u16::from_le_bytes(bytes[offset..offset + 2].try_into().expect("u16"))
}

fn read_u32(bytes: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes(bytes[offset..offset + 4].try_into().expect("u32"))
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

fn write_fixture_knowledge_instance(root: &Path) {
    let instance_dir = root.join("instances").join("Fixture Knowledge Pack");
    fs::create_dir_all(instance_dir.join(".minecraft/mods")).expect("minecraft dir");
    fs::write(
        instance_dir.join("instance.cfg"),
        "name=fixture-pack\nManagedPackVersionName=1.0.0\n",
    )
    .expect("cfg");
    fs::write(
        instance_dir.join("mmc-pack.json"),
        "{\"components\":[{\"uid\":\"net.minecraft\",\"version\":\"1.21.1\"},{\"uid\":\"net.neoforged\",\"version\":\"21.1.233\"}]}\n",
    )
    .expect("pack");
}

fn read_patch_manifest(instance_path: &Path) -> MpbPatchManifest {
    serde_json::from_str(
        &fs::read_to_string(instance_path.join("mpb/patch-manifest.json")).expect("manifest"),
    )
    .expect("manifest json")
}

fn write_patch_manifest(instance_path: &Path, manifest: &MpbPatchManifest) {
    fs::write(
        instance_path.join("mpb/patch-manifest.json"),
        serde_json::to_vec_pretty(manifest).expect("manifest json"),
    )
    .expect("write manifest");
}
