use std::fs;

use mpb_core::{BlockPlacement, BlockRegistry, Coordinate, Scheme, SchemeOperation, StageRef};
use mpb_storage::{InstanceMpbLayout, InstanceSchemeRepository};
use tempfile::tempdir;

#[test]
fn prepares_instance_local_mpb_layout_without_sqlite() {
    let temp = tempdir().expect("temp dir");

    let layout = InstanceMpbLayout::prepare(temp.path()).expect("layout");

    assert_eq!(layout.mpb_dir, temp.path().join("mpb"));
    assert!(layout.config_path.is_file());
    assert!(layout.schemes_dir.is_dir());
    assert!(layout.cache_dir.is_dir());
    assert!(layout
        .patch_manifest_path
        .ends_with("mpb/patch-manifest.json"));
    assert!(!layout.mpb_dir.join("library.sqlite3").exists());
}

#[test]
fn saves_and_loads_sparse_scheme_files_atomically() {
    let temp = tempdir().expect("temp dir");
    let registry = BlockRegistry::synthetic_fixture();
    let mut scheme = Scheme::new("Factory Line");
    let stage = scheme.add_stage("Frame").expect("stage");
    scheme
        .apply(
            &registry,
            SchemeOperation::Place(BlockPlacement::new(
                Coordinate::new(4, 0, 2),
                "minecraft:stone_bricks",
                [("cracked", "false")],
                StageRef::Stage(stage),
            )),
        )
        .expect("place block");

    let repository = InstanceSchemeRepository::open(temp.path()).expect("repository");
    let saved = repository
        .save_scheme("factory-line", &scheme)
        .expect("save scheme");

    assert_eq!(saved.scheme_id, "factory-line");
    assert_eq!(
        saved.path,
        temp.path().join("mpb/schemes/factory-line.mpb.json")
    );
    assert!(!temp
        .path()
        .join("mpb/schemes/factory-line.mpb.json.tmp")
        .exists());

    let json = fs::read_to_string(&saved.path).expect("scheme json");
    let document: serde_json::Value = serde_json::from_str(&json).expect("scheme document json");
    assert_eq!(document["schemaVersion"], 1);
    assert_eq!(document["schemeId"], "factory-line");
    assert!(document["palette"].is_array());
    assert!(document["blocks"].is_array());

    let loaded = repository.load_scheme("factory-line").expect("load scheme");
    assert_eq!(loaded.scheme_id, "factory-line");
    assert_eq!(loaded.scheme.name(), "Factory Line");
    assert_eq!(loaded.scheme.block_count(), 1);
    assert_eq!(
        loaded.scheme.computed_dimensions().expect("dimensions").x,
        5
    );
}

#[test]
fn rejects_scheme_ids_that_would_escape_the_instance_root() {
    let temp = tempdir().expect("temp dir");
    let repository = InstanceSchemeRepository::open(temp.path()).expect("repository");
    let scheme = Scheme::new("Unsafe");

    let error = repository
        .save_scheme("../escape", &scheme)
        .expect_err("unsafe scheme id rejected");

    assert!(error.to_string().contains("scheme id"));
}
