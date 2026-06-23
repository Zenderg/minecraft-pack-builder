use std::collections::BTreeMap;
use std::io::{Cursor, Read, Write};
use std::path::Path;

use mpb_assets::{
    build_modpack_asset_index, build_modpack_asset_index_with_events, AssetError, AssetImportEvent,
    CancellationToken, CurseForgeGateway, CurseForgeProject, CurseForgeRelease,
    ModpackAssetImportRequest,
};
use tempfile::tempdir;
use zip::write::SimpleFileOptions;

#[test]
fn builds_block_registry_and_diagnostics_from_manifest_mod_jars() {
    let temp = tempdir().expect("temp dir");
    let archive_path = temp.path().join("synthetic-pack.zip");
    write_zip(
        &archive_path,
        &[
            (
                "manifest.json",
                r#"{
                  "minecraft": {
                    "version": "1.20.1",
                    "modLoaders": [{ "id": "forge-47.2.0", "primary": true }]
                  },
                  "files": [{ "projectID": 500, "fileID": 900, "required": true }]
                }"#,
            ),
            (
                "overrides/resourcepacks/local/assets/local/lang/en_us.json",
                r#"{ "block.local.preview_block": "Local Preview Block" }"#,
            ),
        ],
    );

    let mut mod_files = BTreeMap::new();
    mod_files.insert(
        (500, 900),
        zip_bytes(&[
            (
                "assets/thermal/lang/en_us.json",
                r#"{ "block.thermal.machine_frame": "Machine Frame" }"#,
            ),
            (
                "assets/thermal/blockstates/machine_frame.json",
                r#"{ "variants": { "": { "model": "thermal:block/machine_frame" } } }"#,
            ),
            (
                "assets/thermal/models/block/machine_frame.json",
                r#"{ "textures": { "all": "thermal:block/machine_frame" } }"#,
            ),
            (
                "assets/thermal/textures/block/machine_frame.png",
                "fake-png",
            ),
        ]),
    );

    let report = build_modpack_asset_index(
        &SyntheticGateway { mod_files },
        "test-key",
        ModpackAssetImportRequest {
            archive_path: archive_path.clone(),
            cache_dir: temp.path().join("cache"),
            diagnostics_dir: temp.path().join("diagnostics"),
            source_slug: "synthetic-pack".to_string(),
            release_name: "Synthetic 1.0.0".to_string(),
            minecraft_version: Some("1.20.1".to_string()),
            loader: Some("Forge".to_string()),
        },
    )
    .expect("asset index");

    assert_eq!(report.status, "imported");
    assert_eq!(report.selected_release, "Synthetic 1.0.0");
    assert_eq!(report.minecraft_version.as_deref(), Some("1.20.1"));
    assert_eq!(report.loader.as_deref(), Some("Forge"));
    assert_eq!(report.mod_file_count, 1);
    assert_eq!(report.block_count, 1);
    assert_eq!(report.asset_count, 5);
    assert!(report.warnings.is_empty());
    assert_eq!(report.texture_atlas.textures.len(), 1);
    assert_eq!(report.blocks[0].identifier, "thermal:machine_frame");
    assert_eq!(report.blocks[0].display_name, "Machine Frame");
    assert!(report.blocks[0]
        .texture_path
        .as_ref()
        .is_some_and(|path| path.ends_with("assets/thermal/textures/block/machine_frame.png")));
    assert!(report.cache_location.ends_with("cache"));
    assert!(report
        .report_path
        .ends_with("diagnostics/synthetic-pack-assets.json"));
    assert!(report.report_path.exists());
}

#[test]
fn keeps_indexing_when_one_optional_asset_json_is_malformed() {
    let temp = tempdir().expect("temp dir");
    let archive_path = temp.path().join("synthetic-pack.zip");
    write_zip(
        &archive_path,
        &[(
            "manifest.json",
            r#"{ "files": [{ "projectID": 500, "fileID": 900, "required": true }] }"#,
        )],
    );

    let mut mod_files = BTreeMap::new();
    mod_files.insert(
        (500, 900),
        zip_bytes(&[
            (
                "assets/create/lang/en_us.json",
                r#"{ "block.create.andesite_casing": "Andesite Casing" }"#,
            ),
            (
                "assets/create/blockstates/andesite_casing.json",
                r#"{ "variants": { "": { "model": "create:block/andesite_casing" } } }"#,
            ),
            (
                "assets/create/models/block/andesite_casing.json",
                r#"{ "textures": { "all": "create:block/andesite_casing" } }"#,
            ),
            (
                "assets/create/models/block/broken_optional_model.json",
                "{\n    bad_key: true\n}",
            ),
            (
                "assets/create/textures/block/andesite_casing.png",
                "fake-png",
            ),
        ]),
    );

    let report = build_modpack_asset_index(
        &SyntheticGateway { mod_files },
        "test-key",
        ModpackAssetImportRequest {
            archive_path,
            cache_dir: temp.path().join("cache"),
            diagnostics_dir: temp.path().join("diagnostics"),
            source_slug: "synthetic-pack".to_string(),
            release_name: "Synthetic 1.0.0".to_string(),
            minecraft_version: None,
            loader: None,
        },
    )
    .expect("asset index should tolerate malformed optional JSON");

    assert_eq!(report.block_count, 1);
    assert_eq!(report.blocks[0].identifier, "create:andesite_casing");
    assert_eq!(report.warnings.len(), 1);
    assert!(report.warnings[0].contains("broken_optional_model.json"));
    assert!(report.warnings[0].contains("key must be a string"));
}

#[test]
fn keeps_indexing_when_one_manifest_mod_file_cannot_be_downloaded() {
    let temp = tempdir().expect("temp dir");
    let archive_path = temp.path().join("synthetic-pack.zip");
    write_zip(
        &archive_path,
        &[(
            "manifest.json",
            r#"{ "files": [
              { "projectID": 500, "fileID": 900, "required": true },
              { "projectID": 501, "fileID": 901, "required": true }
            ] }"#,
        )],
    );

    let mut mod_files = BTreeMap::new();
    mod_files.insert(
        (500, 900),
        zip_bytes(&[
            (
                "assets/create/lang/en_us.json",
                r#"{ "block.create.andesite_casing": "Andesite Casing" }"#,
            ),
            (
                "assets/create/blockstates/andesite_casing.json",
                r#"{ "variants": { "": { "model": "create:block/andesite_casing" } } }"#,
            ),
        ]),
    );

    let report = build_modpack_asset_index(
        &SyntheticGateway { mod_files },
        "test-key",
        ModpackAssetImportRequest {
            archive_path,
            cache_dir: temp.path().join("cache"),
            diagnostics_dir: temp.path().join("diagnostics"),
            source_slug: "synthetic-pack".to_string(),
            release_name: "Synthetic 1.0.0".to_string(),
            minecraft_version: None,
            loader: None,
        },
    )
    .expect("asset index should tolerate one blocked mod file");

    assert_eq!(report.block_count, 1);
    assert_eq!(report.mod_file_count, 2);
    assert_eq!(report.warnings.len(), 1);
    assert!(report.warnings[0].contains("501"));
    assert!(report.warnings[0].contains("901"));
    assert!(report.warnings[0].contains("missing synthetic mod file"));
}

#[test]
fn emits_asset_import_events_for_long_running_parse_steps() {
    let temp = tempdir().expect("temp dir");
    let archive_path = temp.path().join("synthetic-pack.zip");
    write_zip(
        &archive_path,
        &[(
            "manifest.json",
            r#"{ "files": [{ "projectID": 500, "fileID": 900, "required": true }] }"#,
        )],
    );

    let mut mod_files = BTreeMap::new();
    mod_files.insert(
        (500, 900),
        zip_bytes(&[
            (
                "assets/create/lang/en_us.json",
                r#"{ "block.create.andesite_casing": "Andesite Casing" }"#,
            ),
            (
                "assets/create/blockstates/andesite_casing.json",
                r#"{ "variants": { "": { "model": "create:block/andesite_casing" } } }"#,
            ),
        ]),
    );

    let mut events: Vec<AssetImportEvent> = Vec::new();
    let report = build_modpack_asset_index_with_events(
        &SyntheticGateway { mod_files },
        "test-key",
        ModpackAssetImportRequest {
            archive_path,
            cache_dir: temp.path().join("cache"),
            diagnostics_dir: temp.path().join("diagnostics"),
            source_slug: "synthetic-pack".to_string(),
            release_name: "Synthetic 1.0.0".to_string(),
            minecraft_version: None,
            loader: None,
        },
        &CancellationToken::new(),
        |event| events.push(event),
    )
    .expect("asset index");

    assert_eq!(report.block_count, 1);
    assert!(events
        .iter()
        .any(|event| event.message.contains("Extracting modpack archive")));
    assert!(events
        .iter()
        .any(|event| event.message.contains("Manifest references 1 mod files")));
    assert!(events
        .iter()
        .any(|event| event.message.contains("Resolving mod file 1/1")));
    assert!(events
        .iter()
        .any(|event| event.message.contains("Writing asset diagnostics report")));
    assert!(events.iter().any(|event| {
        event.message.contains("Indexed mod file 1/1")
            && event
                .progress
                .as_ref()
                .is_some_and(|progress| progress.completed == 1 && progress.total == 1)
    }));
}

#[test]
fn cancels_asset_import_before_starting_parse_work() {
    let temp = tempdir().expect("temp dir");
    let archive_path = temp.path().join("synthetic-pack.zip");
    write_zip(&archive_path, &[("manifest.json", r#"{ "files": [] }"#)]);
    let token = CancellationToken::new();
    token.cancel();

    let error = build_modpack_asset_index_with_events(
        &SyntheticGateway::default(),
        "test-key",
        ModpackAssetImportRequest {
            archive_path,
            cache_dir: temp.path().join("cache"),
            diagnostics_dir: temp.path().join("diagnostics"),
            source_slug: "synthetic-pack".to_string(),
            release_name: "Synthetic 1.0.0".to_string(),
            minecraft_version: None,
            loader: None,
        },
        &token,
        |_| {},
    )
    .expect_err("asset import should stop when already cancelled");

    assert!(matches!(error, AssetError::Cancelled));
}

#[test]
fn rejects_modpack_archives_that_cannot_be_indexed() {
    let temp = tempdir().expect("temp dir");
    let archive_path = temp.path().join("empty.zip");
    write_zip(&archive_path, &[("manifest.json", r#"{ "files": [] }"#)]);

    let error = build_modpack_asset_index(
        &SyntheticGateway::default(),
        "test-key",
        ModpackAssetImportRequest {
            archive_path,
            cache_dir: temp.path().join("cache"),
            diagnostics_dir: temp.path().join("diagnostics"),
            source_slug: "empty".to_string(),
            release_name: "Empty".to_string(),
            minecraft_version: None,
            loader: None,
        },
    )
    .expect_err("empty asset index should fail");

    assert!(matches!(error, AssetError::NoParseableBlocks));
}

#[derive(Default)]
struct SyntheticGateway {
    mod_files: BTreeMap<(u64, u64), Vec<u8>>,
}

impl CurseForgeGateway for SyntheticGateway {
    fn search_modpack_projects(
        &self,
        _api_key: &str,
        _query: &str,
    ) -> Result<Vec<CurseForgeProject>, AssetError> {
        Ok(Vec::new())
    }

    fn find_modpack_project(
        &self,
        _api_key: &str,
        _slug: &str,
    ) -> Result<Option<CurseForgeProject>, AssetError> {
        Ok(None)
    }

    fn list_project_files(
        &self,
        _api_key: &str,
        _project_id: u64,
    ) -> Result<Vec<CurseForgeRelease>, AssetError> {
        Ok(Vec::new())
    }

    fn open_download(
        &self,
        _api_key: &str,
        _release: &CurseForgeRelease,
    ) -> Result<Box<dyn Read>, AssetError> {
        Err(AssetError::Api("not used".to_string()))
    }

    fn open_mod_file_download(
        &self,
        _api_key: &str,
        project_id: u64,
        file_id: u64,
    ) -> Result<Box<dyn Read>, AssetError> {
        let bytes = self
            .mod_files
            .get(&(project_id, file_id))
            .cloned()
            .ok_or_else(|| AssetError::Api("missing synthetic mod file".to_string()))?;
        Ok(Box::new(Cursor::new(bytes)))
    }
}

fn write_zip(path: &Path, files: &[(&str, &str)]) {
    let bytes = zip_bytes(files);
    std::fs::write(path, bytes).expect("write zip");
}

fn zip_bytes(files: &[(&str, &str)]) -> Vec<u8> {
    let cursor = Cursor::new(Vec::new());
    let mut writer = zip::ZipWriter::new(cursor);
    let options = SimpleFileOptions::default();
    for (path, contents) in files {
        writer.start_file(*path, options).expect("start zip file");
        writer
            .write_all(contents.as_bytes())
            .expect("write zip file");
    }
    writer.finish().expect("finish zip").into_inner()
}
