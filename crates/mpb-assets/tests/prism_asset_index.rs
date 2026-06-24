use std::io::Write;
use std::path::Path;

use mpb_assets::{
    build_prism_asset_index, build_prism_asset_index_with_events, AssetError, AssetIndexEvent,
    CancellationToken, PrismAssetIndexMetadata, PrismAssetIndexRequest,
};
use tempfile::tempdir;
use zip::write::SimpleFileOptions;

#[test]
fn builds_registry_report_from_prism_mod_jars() {
    let temp = tempdir().expect("temp dir");
    let minecraft_dir = temp.path().join("PrismLauncher/instances/aoc/.minecraft");
    let mods_dir = minecraft_dir.join("mods");
    std::fs::create_dir_all(&mods_dir).expect("mods dir");
    write_zip(
        &mods_dir.join("thermal.jar"),
        &[
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
        ],
    );

    let report =
        build_prism_asset_index(request(temp.path(), &minecraft_dir)).expect("Prism asset index");

    assert_eq!(report.status, "ready");
    assert_eq!(report.schema_version, 6);
    assert_eq!(report.static_status, "ready");
    assert_eq!(report.runtime_status, "unavailable");
    assert_eq!(report.instance_id, "aoc");
    assert_eq!(report.archive_count, 1);
    assert_eq!(report.block_count, 1);
    assert_eq!(report.asset_count, 4);
    assert_eq!(report.blocks[0].identifier, "thermal:machine_frame");
    assert_eq!(
        report.blocks[0].item_id.as_deref(),
        Some("thermal:machine_frame")
    );
    assert_eq!(report.blocks[0].max_stack_size, None);
    assert_eq!(report.blocks[0].display_name, "Machine Frame");
    let texture_path = report.blocks[0]
        .texture_path
        .as_ref()
        .expect("cached texture path");
    assert_eq!(
        std::fs::read_to_string(texture_path).expect("cached texture"),
        "fake-png"
    );
    assert!(texture_path
        .ends_with("fingerprint-aoc-content-aoc-textures/thermal/block/machine_frame.png"));
    assert!(report
        .report_path
        .ends_with("diagnostics/fingerprint-aoc-registry.json"));
    assert!(report.report_path.exists());
    let registry_json = std::fs::read_to_string(&report.report_path).expect("registry report");
    assert!(registry_json.contains("allowedStates"));
    assert!(registry_json.contains("modelVariants"));
    assert!(!registry_json.contains("modelElements"));
    let metadata_path = temp
        .path()
        .join("diagnostics/fingerprint-aoc-registry-meta.json");
    let metadata = serde_json::from_str::<PrismAssetIndexMetadata>(
        &std::fs::read_to_string(metadata_path).expect("registry metadata"),
    )
    .expect("parse registry metadata");
    assert_eq!(metadata.schema_version, report.schema_version);
    assert_eq!(metadata.runtime_status, report.runtime_status);
    assert_eq!(metadata.content_fingerprint, report.content_fingerprint);
    assert_eq!(metadata.block_count, report.block_count);
    assert_eq!(metadata.report_path, report.report_path);
}

#[test]
fn merges_cached_runtime_stack_sizes_by_item_id() {
    let temp = tempdir().expect("temp dir");
    let minecraft_dir = temp.path().join("PrismLauncher/instances/aoc/.minecraft");
    let mods_dir = minecraft_dir.join("mods");
    std::fs::create_dir_all(&mods_dir).expect("mods dir");
    write_zip(
        &mods_dir.join("thermal.jar"),
        &[
            (
                "assets/thermal/lang/en_us.json",
                r#"{ "block.thermal.machine_frame": "Machine Frame" }"#,
            ),
            (
                "assets/thermal/blockstates/machine_frame.json",
                r#"{ "variants": { "": { "model": "thermal:block/machine_frame" } } }"#,
            ),
        ],
    );
    let diagnostics_dir = temp.path().join("diagnostics");
    std::fs::create_dir_all(&diagnostics_dir).expect("diagnostics dir");
    std::fs::write(
        diagnostics_dir.join("fingerprint-aoc-content-aoc-runtime.json"),
        r#"{
            "status": "ready",
            "items": [
                { "itemId": "thermal:machine_frame", "maxStackSize": 16 }
            ]
        }"#,
    )
    .expect("write runtime report");

    let report =
        build_prism_asset_index(request(temp.path(), &minecraft_dir)).expect("Prism asset index");

    assert_eq!(report.runtime_status, "ready");
    assert_eq!(report.runtime_message, None);
    assert_eq!(report.blocks[0].max_stack_size, Some(16));
}

#[test]
fn reports_forge_runtime_prerequisites_without_blocking_static_index() {
    let temp = tempdir().expect("temp dir");
    let minecraft_dir = temp.path().join("PrismLauncher/instances/aoc/.minecraft");
    write_minimal_block_mod(&minecraft_dir);

    let report =
        build_prism_asset_index(request(temp.path(), &minecraft_dir)).expect("Prism asset index");

    assert_eq!(report.static_status, "ready");
    assert_eq!(report.runtime_status, "unavailable");
    assert!(report
        .runtime_message
        .as_deref()
        .is_some_and(|message| message.contains("Forge libraries were not found")));
}

#[test]
fn reports_fabric_runtime_prerequisites_without_blocking_static_index() {
    let temp = tempdir().expect("temp dir");
    let minecraft_dir = temp
        .path()
        .join("PrismLauncher/instances/fabric/.minecraft");
    write_minimal_block_mod(&minecraft_dir);
    let mut request = request(temp.path(), &minecraft_dir);
    request.loader = Some("Fabric".to_string());

    let report = build_prism_asset_index(request).expect("Prism asset index");

    assert_eq!(report.static_status, "ready");
    assert_eq!(report.runtime_status, "unavailable");
    assert!(report
        .runtime_message
        .as_deref()
        .is_some_and(|message| message.contains("Fabric runtime stack extraction")));
}

#[test]
fn includes_vanilla_client_assets_from_prism_libraries() {
    let temp = tempdir().expect("temp dir");
    let minecraft_dir = temp.path().join("PrismLauncher/instances/aoc/minecraft");
    let vanilla_jar = temp
        .path()
        .join("PrismLauncher/libraries/com/mojang/minecraft/1.20.1/minecraft-1.20.1-client.jar");
    std::fs::create_dir_all(vanilla_jar.parent().expect("vanilla jar parent"))
        .expect("vanilla dir");
    write_zip(
        &vanilla_jar,
        &[
            (
                "assets/minecraft/lang/en_us.json",
                r#"{ "block.minecraft.stone": "Stone" }"#,
            ),
            (
                "assets/minecraft/blockstates/stone.json",
                r#"{ "variants": { "": { "model": "minecraft:block/stone" } } }"#,
            ),
            (
                "assets/minecraft/models/block/stone.json",
                r#"{ "textures": { "all": "minecraft:block/stone" } }"#,
            ),
            ("assets/minecraft/textures/block/stone.png", "fake-png"),
        ],
    );

    let mut request = request(temp.path(), &minecraft_dir);
    request.minecraft_version = Some("1.20.1".to_string());

    let report = build_prism_asset_index(request).expect("Prism asset index");

    let stone = report
        .blocks
        .iter()
        .find(|block| block.identifier == "minecraft:stone")
        .expect("vanilla stone block");
    assert_eq!(stone.display_name, "Stone");
    let texture_path = stone.texture_path.as_ref().expect("cached vanilla texture");
    assert_eq!(
        std::fs::read_to_string(texture_path).expect("cached texture"),
        "fake-png"
    );
    assert!(
        texture_path.ends_with("fingerprint-aoc-content-aoc-textures/minecraft/block/stone.png")
    );
}

#[test]
fn resolves_model_parent_face_textures_for_directional_blocks() {
    let temp = tempdir().expect("temp dir");
    let minecraft_dir = temp.path().join("PrismLauncher/instances/aoc/.minecraft");
    let mods_dir = minecraft_dir.join("mods");
    std::fs::create_dir_all(&mods_dir).expect("mods dir");
    write_zip(
        &mods_dir.join("vanilla-like.jar"),
        &[
            (
                "assets/minecraft/blockstates/furnace.json",
                r#"{ "variants": { "": { "model": "minecraft:block/furnace" } } }"#,
            ),
            (
                "assets/minecraft/models/block/orientable.json",
                r##"{
                    "textures": { "particle": "#side" },
                    "elements": [
                        {
                            "from": [0, 0, 0],
                            "to": [16, 16, 16],
                            "faces": {
                                "down": { "texture": "#top" },
                                "up": { "texture": "#top" },
                                "north": { "texture": "#front" },
                                "south": { "texture": "#side" },
                                "west": { "texture": "#side" },
                                "east": { "texture": "#side" }
                            }
                        }
                    ]
                }"##,
            ),
            (
                "assets/minecraft/models/block/furnace.json",
                r##"{
                    "parent": "minecraft:block/orientable",
                    "textures": {
                        "top": "minecraft:block/furnace_top",
                        "front": "minecraft:block/furnace_front",
                        "side": "minecraft:block/furnace_side"
                    }
                }"##,
            ),
            ("assets/minecraft/textures/block/furnace_top.png", "top"),
            ("assets/minecraft/textures/block/furnace_front.png", "front"),
            ("assets/minecraft/textures/block/furnace_side.png", "side"),
        ],
    );

    let report =
        build_prism_asset_index(request(temp.path(), &minecraft_dir)).expect("Prism asset index");
    let furnace = report
        .blocks
        .iter()
        .find(|block| block.identifier == "minecraft:furnace")
        .expect("furnace block");
    let face_textures = furnace.face_texture_paths.as_ref().expect("face textures");

    assert_eq!(
        std::fs::read_to_string(face_textures.north.as_ref().expect("front texture"))
            .expect("front texture"),
        "front"
    );
    assert_eq!(
        std::fs::read_to_string(face_textures.up.as_ref().expect("top texture"))
            .expect("top texture"),
        "top"
    );
    assert_eq!(
        std::fs::read_to_string(face_textures.east.as_ref().expect("side texture"))
            .expect("side texture"),
        "side"
    );
    assert_ne!(face_textures.north, face_textures.up);
    assert_ne!(face_textures.north, face_textures.east);
}

#[test]
fn resolves_model_parent_elements_for_non_full_cube_blocks() {
    let temp = tempdir().expect("temp dir");
    let minecraft_dir = temp.path().join("PrismLauncher/instances/aoc/.minecraft");
    let mods_dir = minecraft_dir.join("mods");
    std::fs::create_dir_all(&mods_dir).expect("mods dir");
    write_zip(
        &mods_dir.join("torch-like.jar"),
        &[
            (
                "assets/minecraft/blockstates/torch.json",
                r#"{ "variants": { "": { "model": "minecraft:block/torch" } } }"#,
            ),
            (
                "assets/minecraft/models/block/template_torch.json",
                r##"{
                    "textures": { "particle": "#torch" },
                    "elements": [
                        {
                            "from": [7, 0, 7],
                            "to": [9, 10, 9],
                            "faces": {
                                "down": { "texture": "#torch" },
                                "up": { "texture": "#torch" }
                            }
                        },
                        {
                            "from": [7, 0, 0],
                            "to": [9, 16, 16],
                            "rotation": {
                                "origin": [8, 8, 8],
                                "axis": "y",
                                "angle": 45,
                                "rescale": false
                            },
                            "faces": {
                                "west": { "texture": "#torch", "uv": [7, 0, 9, 16] },
                                "east": { "texture": "#torch", "uv": [7, 0, 9, 16] }
                            }
                        }
                    ]
                }"##,
            ),
            (
                "assets/minecraft/models/block/torch.json",
                r##"{
                    "parent": "minecraft:block/template_torch",
                    "textures": { "torch": "minecraft:block/torch" }
                }"##,
            ),
            ("assets/minecraft/textures/block/torch.png", "torch"),
        ],
    );

    let report =
        build_prism_asset_index(request(temp.path(), &minecraft_dir)).expect("Prism asset index");
    let torch = report
        .blocks
        .iter()
        .find(|block| block.identifier == "minecraft:torch")
        .expect("torch block");

    assert_eq!(torch.model_elements.len(), 2);
    assert_eq!(torch.model_elements[0].from, [7.0, 0.0, 7.0]);
    assert_eq!(torch.model_elements[0].to, [9.0, 10.0, 9.0]);
    assert_eq!(torch.model_elements[1].from, [7.0, 0.0, 0.0]);
    assert_eq!(torch.model_elements[1].to, [9.0, 16.0, 16.0]);
    assert_eq!(
        torch.model_elements[1].rotation.as_ref().map(|rotation| (
            rotation.origin,
            rotation.axis.as_str(),
            rotation.angle,
            rotation.rescale,
        )),
        Some(([8.0, 8.0, 8.0], "y", 45.0, false)),
    );
    assert_eq!(
        torch.model_elements[1].face_uvs.east,
        Some([7.0, 0.0, 9.0, 16.0])
    );
    assert!(torch.model_elements[0]
        .face_texture_paths
        .up
        .as_ref()
        .is_some_and(|path| std::fs::read_to_string(path).expect("torch texture") == "torch"));
    assert!(torch.model_elements[1]
        .face_texture_paths
        .east
        .as_ref()
        .is_some_and(|path| std::fs::read_to_string(path).expect("torch texture") == "torch"));
}

#[test]
fn preserves_blockstate_variant_conditions_and_rotations() {
    let temp = tempdir().expect("temp dir");
    let minecraft_dir = temp.path().join("PrismLauncher/instances/aoc/.minecraft");
    let mods_dir = minecraft_dir.join("mods");
    std::fs::create_dir_all(&mods_dir).expect("mods dir");
    write_zip(
        &mods_dir.join("wall-torch.jar"),
        &[
            (
                "assets/minecraft/blockstates/wall_torch.json",
                r##"{
                    "variants": {
                        "facing=east": { "model": "minecraft:block/wall_torch", "y": 90, "uvlock": true },
                        "facing=south": { "model": "minecraft:block/wall_torch", "y": 180, "uvlock": true },
                        "facing=west": { "model": "minecraft:block/wall_torch", "y": 270, "uvlock": true },
                        "facing=north": { "model": "minecraft:block/wall_torch" }
                    }
                }"##,
            ),
            (
                "assets/minecraft/models/block/wall_torch.json",
                r##"{
                    "textures": { "torch": "minecraft:block/torch" },
                    "elements": [
                        {
                            "from": [7, 3, 0],
                            "to": [9, 13, 2],
                            "faces": {
                                "north": { "texture": "#torch", "uv": [7, 3, 9, 13] },
                                "south": { "texture": "#torch", "uv": [7, 3, 9, 13] }
                            }
                        }
                    ]
                }"##,
            ),
            ("assets/minecraft/textures/block/torch.png", "torch"),
        ],
    );

    let report =
        build_prism_asset_index(request(temp.path(), &minecraft_dir)).expect("Prism asset index");
    let wall_torch = report
        .blocks
        .iter()
        .find(|block| block.identifier == "minecraft:wall_torch")
        .expect("wall torch block");

    assert_eq!(wall_torch.model_variants.len(), 4);
    let east = wall_torch
        .model_variants
        .iter()
        .find(|variant| {
            variant.condition.as_ref().is_some_and(|condition| {
                condition.any_of.iter().any(|states| {
                    states
                        .get("facing")
                        .is_some_and(|values| values == &vec!["east".to_string()])
                })
            })
        })
        .expect("east-facing variant");
    assert_eq!(east.model.as_deref(), Some("minecraft:block/wall_torch"));
    assert_eq!(east.y, Some(90.0));
    assert!(east.uv_lock);
    assert_eq!(east.model_elements.len(), 1);
    assert_eq!(
        east.model_elements[0].face_uvs.north,
        Some([7.0, 3.0, 9.0, 13.0])
    );
    let facing = wall_torch
        .allowed_states
        .iter()
        .find(|state| state.name == "facing")
        .expect("facing state definition");
    assert_eq!(
        facing.values,
        vec![
            "east".to_string(),
            "north".to_string(),
            "south".to_string(),
            "west".to_string()
        ]
    );
}

#[test]
fn preserves_multipart_blockstate_conditions_as_additive_variants() {
    let temp = tempdir().expect("temp dir");
    let minecraft_dir = temp.path().join("PrismLauncher/instances/aoc/.minecraft");
    let mods_dir = minecraft_dir.join("mods");
    std::fs::create_dir_all(&mods_dir).expect("mods dir");
    write_zip(
        &mods_dir.join("fence.jar"),
        &[
            (
                "assets/minecraft/blockstates/oak_fence.json",
                r##"{
                    "multipart": [
                        { "apply": { "model": "minecraft:block/oak_fence_post" } },
                        { "when": { "north": "true" }, "apply": { "model": "minecraft:block/oak_fence_side", "y": 0, "uvlock": true } },
                        { "when": { "OR": [{ "east": "true" }, { "west": "true" }] }, "apply": { "model": "minecraft:block/oak_fence_side", "y": 90, "uvlock": true } }
                    ]
                }"##,
            ),
            (
                "assets/minecraft/models/block/oak_fence_post.json",
                r##"{ "textures": { "all": "minecraft:block/oak_planks" }, "elements": [
                    { "from": [6, 0, 6], "to": [10, 16, 10], "faces": { "north": { "texture": "#all" } } }
                ] }"##,
            ),
            (
                "assets/minecraft/models/block/oak_fence_side.json",
                r##"{ "textures": { "all": "minecraft:block/oak_planks" }, "elements": [
                    { "from": [7, 6, 0], "to": [9, 12, 8], "faces": { "north": { "texture": "#all" } } }
                ] }"##,
            ),
            ("assets/minecraft/textures/block/oak_planks.png", "planks"),
        ],
    );

    let report =
        build_prism_asset_index(request(temp.path(), &minecraft_dir)).expect("Prism asset index");
    let fence = report
        .blocks
        .iter()
        .find(|block| block.identifier == "minecraft:oak_fence")
        .expect("oak fence block");

    assert!(fence.model_variants_are_multipart);
    assert_eq!(fence.model_variants.len(), 3);
    assert!(fence.model_variants[0].condition.is_none());
    assert_eq!(fence.model_variants[2].y, Some(90.0));
    assert_eq!(
        fence.model_variants[2]
            .condition
            .as_ref()
            .expect("OR condition")
            .any_of
            .len(),
        2
    );
    assert_eq!(
        fence
            .allowed_states
            .iter()
            .map(|state| (state.name.as_str(), state.values.clone()))
            .collect::<Vec<_>>(),
        vec![
            ("east", vec!["false".to_string(), "true".to_string()]),
            ("north", vec!["false".to_string(), "true".to_string()]),
            ("west", vec!["false".to_string(), "true".to_string()])
        ]
    );
}

#[test]
fn scans_directory_resource_packs_next_to_mod_jars() {
    let temp = tempdir().expect("temp dir");
    let minecraft_dir = temp.path().join("PrismLauncher/instances/aoc/.minecraft");
    let resourcepack_assets =
        minecraft_dir.join("resourcepacks/local-pack/assets/local/blockstates");
    std::fs::create_dir_all(&resourcepack_assets).expect("resourcepack assets");
    std::fs::write(
        resourcepack_assets.join("preview_block.json"),
        r#"{ "variants": { "": { "model": "local:block/preview_block" } } }"#,
    )
    .expect("write blockstate");

    let report =
        build_prism_asset_index(request(temp.path(), &minecraft_dir)).expect("Prism asset index");

    assert_eq!(report.block_count, 1);
    assert_eq!(report.blocks[0].identifier, "local:preview_block");
}

#[test]
fn keeps_indexing_when_one_asset_json_is_malformed() {
    let temp = tempdir().expect("temp dir");
    let minecraft_dir = temp.path().join("PrismLauncher/instances/aoc/.minecraft");
    let mods_dir = minecraft_dir.join("mods");
    std::fs::create_dir_all(&mods_dir).expect("mods dir");
    write_zip(
        &mods_dir.join("create.jar"),
        &[
            (
                "assets/create/lang/en_us.json",
                r#"{ "block.create.andesite_casing": "Andesite Casing" }"#,
            ),
            (
                "assets/create/blockstates/andesite_casing.json",
                r#"{ "variants": { "": { "model": "create:block/andesite_casing" } } }"#,
            ),
            (
                "assets/create/models/block/broken_optional_model.json",
                "{\n    bad_key: true\n}",
            ),
        ],
    );

    let report = build_prism_asset_index(request(temp.path(), &minecraft_dir))
        .expect("asset index should tolerate malformed optional JSON");

    assert_eq!(report.block_count, 1);
    assert_eq!(report.blocks[0].identifier, "create:andesite_casing");
    assert_eq!(report.warnings.len(), 1);
    assert!(report.warnings[0].contains("broken_optional_model.json"));
}

#[test]
fn emits_index_events_for_long_running_steps() {
    let temp = tempdir().expect("temp dir");
    let minecraft_dir = temp.path().join("PrismLauncher/instances/aoc/.minecraft");
    let mods_dir = minecraft_dir.join("mods");
    std::fs::create_dir_all(&mods_dir).expect("mods dir");
    write_zip(
        &mods_dir.join("create.jar"),
        &[(
            "assets/create/blockstates/andesite_casing.json",
            r#"{ "variants": { "": { "model": "create:block/andesite_casing" } } }"#,
        )],
    );

    let mut events: Vec<AssetIndexEvent> = Vec::new();
    let report = build_prism_asset_index_with_events(
        request(temp.path(), &minecraft_dir),
        &CancellationToken::new(),
        |event| events.push(event),
    )
    .expect("asset index");

    assert_eq!(report.block_count, 1);
    assert!(events
        .iter()
        .any(|event| event.message.contains("Scanning mod archive 1/1")));
    assert!(events.iter().any(|event| {
        event.message.contains("Indexed mod archive 1/1")
            && event
                .progress
                .as_ref()
                .is_some_and(|progress| progress.completed == 1 && progress.total == 1)
    }));
    assert!(events
        .iter()
        .any(|event| event.message.contains("Prism block registry written")));
}

#[test]
fn cancels_indexing_before_starting_scan_work() {
    let temp = tempdir().expect("temp dir");
    let minecraft_dir = temp.path().join("PrismLauncher/instances/aoc/.minecraft");
    let token = CancellationToken::new();
    token.cancel();

    let error =
        build_prism_asset_index_with_events(request(temp.path(), &minecraft_dir), &token, |_| {})
            .expect_err("asset index should stop when already cancelled");

    assert!(matches!(error, AssetError::Cancelled));
}

#[test]
fn rejects_instances_without_parseable_block_assets() {
    let temp = tempdir().expect("temp dir");
    let minecraft_dir = temp.path().join("PrismLauncher/instances/aoc/.minecraft");
    std::fs::create_dir_all(&minecraft_dir).expect("minecraft dir");

    let error = build_prism_asset_index(request(temp.path(), &minecraft_dir))
        .expect_err("empty asset index should fail");

    assert!(matches!(error, AssetError::NoParseableBlocks));
}

fn request(root: &Path, minecraft_dir: &Path) -> PrismAssetIndexRequest {
    PrismAssetIndexRequest {
        instance_id: "aoc".to_string(),
        identity_fingerprint: "fingerprint-aoc".to_string(),
        content_fingerprint: "content-aoc".to_string(),
        instance_path: root.join("PrismLauncher/instances/aoc"),
        minecraft_dir: minecraft_dir.to_path_buf(),
        diagnostics_dir: root.join("diagnostics"),
        minecraft_version: Some("1.20.1".to_string()),
        loader: Some("Forge".to_string()),
    }
}

fn write_zip(path: &Path, files: &[(&str, &str)]) {
    let cursor = std::io::Cursor::new(Vec::new());
    let mut writer = zip::ZipWriter::new(cursor);
    let options = SimpleFileOptions::default();
    for (path, contents) in files {
        writer.start_file(*path, options).expect("start zip file");
        writer
            .write_all(contents.as_bytes())
            .expect("write zip file");
    }
    let bytes = writer.finish().expect("finish zip").into_inner();
    std::fs::write(path, bytes).expect("write zip");
}

fn write_minimal_block_mod(minecraft_dir: &Path) {
    let mods_dir = minecraft_dir.join("mods");
    std::fs::create_dir_all(&mods_dir).expect("mods dir");
    write_zip(
        &mods_dir.join("minimal.jar"),
        &[(
            "assets/example/blockstates/runtime_probe.json",
            r#"{ "variants": { "": { "model": "example:block/runtime_probe" } } }"#,
        )],
    );
}
