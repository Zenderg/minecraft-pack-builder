use std::fs;

use mpb_knowledge::{
    collect_fingerprint_document, compute_target_fingerprint, ExtractionDiagnosticSeverity,
    ExtractionDraft, ExtractionSourceKind,
};
use tempfile::tempdir;

#[test]
fn fingerprint_document_records_exact_sorted_inputs() {
    let temp = tempdir().expect("temp dir");
    let instance = temp.path().join("Instance");
    fs::create_dir_all(instance.join(".minecraft").join("mods")).expect("mods");
    fs::create_dir_all(instance.join(".minecraft").join("config")).expect("config");
    fs::create_dir_all(instance.join(".minecraft").join("datapacks")).expect("datapacks");
    fs::create_dir_all(
        instance
            .join(".minecraft")
            .join("kubejs")
            .join("server_scripts"),
    )
    .expect("kubejs");
    fs::create_dir_all(instance.join(".minecraft").join("scripts")).expect("scripts");
    fs::create_dir_all(instance.join(".minecraft").join("resourcepacks")).expect("resourcepacks");
    fs::write(instance.join("instance.cfg"), "name=Exact Pack\n").expect("cfg");
    fs::write(
        instance.join("mmc-pack.json"),
        r#"{"components":[{"uid":"net.minecraft","version":"1.21.1"},{"uid":"net.neoforged","version":"21.1.233"}]}"#,
    )
    .expect("pack");
    fs::write(
        instance.join(".minecraft").join("mods").join("create.jar"),
        b"mod",
    )
    .expect("mod");
    fs::write(
        instance
            .join(".minecraft")
            .join("config")
            .join("create.toml"),
        b"cfg",
    )
    .expect("config");
    fs::write(
        instance
            .join(".minecraft")
            .join("datapacks")
            .join("recipes.zip"),
        b"data",
    )
    .expect("data");
    fs::write(
        instance
            .join(".minecraft")
            .join("kubejs")
            .join("server_scripts")
            .join("recipes.js"),
        b"kubejs",
    )
    .expect("kubejs");
    fs::write(
        instance
            .join(".minecraft")
            .join("scripts")
            .join("recipes.zs"),
        b"crafttweaker",
    )
    .expect("scripts");
    fs::write(
        instance
            .join(".minecraft")
            .join("resourcepacks")
            .join("guide.zip"),
        b"resourcepack",
    )
    .expect("resourcepack");

    let document = collect_fingerprint_document(&instance, "builder-1", "lab-1", "schema-1")
        .expect("fingerprint document");

    assert_eq!(document.modpack_identity.as_deref(), Some("Exact Pack"));
    assert_eq!(document.minecraft_version.as_deref(), Some("1.21.1"));
    assert_eq!(document.loader.as_deref(), Some("NeoForge"));
    assert_eq!(document.loader_version.as_deref(), Some("21.1.233"));
    assert_eq!(document.builder_version, "builder-1");
    assert_eq!(document.lab_tooling_version, "lab-1");
    assert_eq!(document.knowledge_schema_version, "schema-1");
    assert!(document
        .inputs
        .iter()
        .any(|input| input.role == "mods" && input.path == "mods/create.jar"));
    assert!(document
        .inputs
        .iter()
        .any(|input| input.role == "config" && input.path == "config/create.toml"));
    assert!(document
        .inputs
        .iter()
        .any(|input| input.role == "datapacks" && input.path == "datapacks/recipes.zip"));
    assert!(document
        .inputs
        .iter()
        .any(|input| input.role == "kubejs" && input.path == "kubejs/server_scripts/recipes.js"));
    assert!(document
        .inputs
        .iter()
        .any(|input| input.role == "scripts" && input.path == "scripts/recipes.zs"));
    assert!(document
        .inputs
        .iter()
        .all(|input| !input.checksum.is_empty()));

    let paths = document
        .inputs
        .iter()
        .map(|input| input.path.clone())
        .collect::<Vec<_>>();
    let mut sorted = paths.clone();
    sorted.sort();
    assert_eq!(paths, sorted);
}

#[test]
fn target_fingerprint_changes_when_exact_inputs_change() {
    let temp = tempdir().expect("temp dir");
    let instance = temp.path().join("Instance");
    fs::create_dir_all(instance.join(".minecraft").join("config")).expect("config");
    fs::write(instance.join("instance.cfg"), "name=Exact Pack\n").expect("cfg");
    fs::write(
        instance.join(".minecraft").join("config").join("a.toml"),
        b"one",
    )
    .expect("config");

    let first = compute_target_fingerprint(&instance, "builder-1", "lab-1", "schema-1")
        .expect("fingerprint");
    fs::write(
        instance.join(".minecraft").join("config").join("a.toml"),
        b"two",
    )
    .expect("config");
    let changed = compute_target_fingerprint(&instance, "builder-1", "lab-1", "schema-1")
        .expect("changed fingerprint");

    assert_ne!(first.fingerprint, changed.fingerprint);
    assert_eq!(first.document.inputs[0].byte_len, 3);
    assert_eq!(changed.document.inputs[0].byte_len, 3);
}

#[test]
fn extractor_reports_unsupported_inputs_as_blocking_diagnostics() {
    let draft = ExtractionDraft::from_sources(vec![ExtractionSourceKind::Guidebook]);

    assert!(draft.records.is_empty());
    assert!(draft
        .diagnostics
        .iter()
        .any(|diagnostic| diagnostic.severity == ExtractionDiagnosticSeverity::Blocking));
}
