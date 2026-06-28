use std::fs;

use mpb_knowledge::{
    build_runtime_bundle, load_source_pack, read_runtime_bundle, validate_source_dir,
    RuntimeBundleQuery,
};
use tempfile::tempdir;

#[test]
fn build_bundle_refuses_invalid_source() {
    let temp = tempdir().expect("temp dir");
    let source_dir = temp.path().join("source");
    let output_dir = temp.path().join("bundle");
    fs::create_dir_all(&source_dir).expect("source dir");
    fs::write(source_dir.join("manifest.json"), "{}").expect("manifest");

    let error = build_runtime_bundle(&source_dir, &output_dir).expect_err("invalid source");

    assert!(error.to_string().contains("missing_manifest_metadata"));
    assert!(!output_dir.join("knowledge-index.json").exists());
}

#[test]
fn fixture_source_validates_and_bundle_answers_read_only_queries() {
    let source_dir = format!(
        "{}/../../knowledge/packs/fixtures/minimal/source",
        env!("CARGO_MANIFEST_DIR")
    );
    let pack = load_source_pack(&source_dir).expect("load source pack");
    validate_source_dir(&source_dir).expect("validate source");
    let temp = tempdir().expect("temp dir");

    let bundle = build_runtime_bundle(&source_dir, temp.path()).expect("build bundle");
    let from_disk =
        read_runtime_bundle(temp.path().join("knowledge-index.json")).expect("read bundle");
    let query = RuntimeBundleQuery::new(&from_disk);

    assert_eq!(bundle.manifest.pack_id, pack.manifest.pack_id);
    assert_eq!(
        query.entity_by_id("minecraft:stone").unwrap().id,
        "minecraft:stone"
    );
    assert!(query
        .search_by_localized_name("Stone")
        .iter()
        .any(|entity| entity.id == "minecraft:stone"));
    assert!(query
        .search_by_tag("minecraft:mineable/pickaxe")
        .iter()
        .any(|entity| entity.id == "minecraft:cobblestone"));
    assert!(query
        .search_by_use_case("build")
        .iter()
        .any(|entity| entity.id == "minecraft:stone"));
    assert!(query
        .search_by_mechanic("mining")
        .iter()
        .any(|entity| entity.id == "minecraft:stone"));
    assert!(query
        .search_by_interface("solid_block")
        .iter()
        .any(|entity| entity.id == "minecraft:cobblestone"));
    assert_eq!(
        query
            .recipe_graph_for("minecraft:cobblestone")
            .unwrap()
            .recipes[0]
            .input_entity_ids,
        vec!["minecraft:stone"]
    );
    assert_eq!(query.mechanic_details("mining").unwrap().id, "mining");
    assert_eq!(
        query.evidence("ev-runtime-drop").unwrap().id,
        "ev-runtime-drop"
    );
    assert!(from_disk
        .manifest
        .validation_command
        .contains("validate-source"));
    assert!(!from_disk.manifest.validation_timestamp.is_empty());
    assert!(from_disk
        .checksums
        .iter()
        .any(|checksum| checksum.path == "knowledge-index.json"));
}
