use app_tauri_lib::{render_scene_from_scheme, render_scene_from_scheme_with_registry_report};
use mpb_core::{
    BlockPlacement, BlockRegistry, Coordinate, Dimensions, Scheme, SchemeOperation, StageRef,
};

#[test]
fn builds_render_scene_from_stored_domain_scheme() {
    let registry = BlockRegistry::synthetic_fixture();
    let mut scheme = Scheme::new(
        "Stored Scheme",
        Dimensions::new(4, 4, 4).expect("dimensions"),
    );
    scheme
        .apply(
            &registry,
            SchemeOperation::Place(BlockPlacement::new(
                Coordinate::new(0, 0, 0),
                "minecraft:stone_bricks",
                [("cracked", "false")],
                StageRef::Unassigned,
            )),
        )
        .expect("place block");
    let scene = render_scene_from_scheme(42, &scheme);

    assert_eq!(scene.scheme_id, 42);
    assert_eq!(scene.dimensions, [4, 4, 4]);
    assert_eq!(scene.blocks.len(), 1);
    assert_eq!(scene.chunks.len(), 1);
    assert!(scene.chunks.iter().all(|chunk| chunk.face_count > 0));
    assert!(scene.blocks.iter().any(|block| block.stage_id.is_none()));
}

#[test]
fn enriches_render_materials_from_registry_report_without_faking_stack_sizes() {
    let registry = BlockRegistry::from_block_ids([
        "minecraft:stone".to_string(),
        "minecraft:furnace".to_string(),
        "create:andesite_casing".to_string(),
    ]);
    let mut scheme = Scheme::new(
        "Stored Scheme",
        Dimensions::new(5, 5, 5).expect("dimensions"),
    );
    for x in 0..65 {
        scheme
            .apply(
                &registry,
                SchemeOperation::Place(BlockPlacement::new(
                    Coordinate::new(x % 5, (x / 5) % 5, x / 25),
                    "minecraft:stone",
                    [],
                    StageRef::Unassigned,
                )),
            )
            .expect("place stone");
    }
    scheme
        .apply(
            &registry,
            SchemeOperation::Place(BlockPlacement::new(
                Coordinate::new(4, 4, 4),
                "create:andesite_casing",
                [],
                StageRef::Unassigned,
            )),
        )
        .expect("place casing");
    scheme
        .apply(
            &registry,
            SchemeOperation::Place(BlockPlacement::new(
                Coordinate::new(3, 4, 4),
                "minecraft:furnace",
                [],
                StageRef::Unassigned,
            )),
        )
        .expect("place furnace");
    let report = serde_json::json!({
        "runtimeStatus": "ready",
        "blocks": [
            {
                "identifier": "minecraft:stone",
                "displayName": "Stone",
                "itemId": "minecraft:stone",
                "maxStackSize": 64,
                "texturePath": "/tmp/stone.png"
            },
            {
                "identifier": "minecraft:furnace",
                "displayName": "Furnace",
                "itemId": "minecraft:furnace",
                "maxStackSize": 64,
                "texturePath": "/tmp/furnace_front.png",
                "faceTexturePaths": {
                    "north": "/tmp/furnace_front.png",
                    "south": "/tmp/furnace_side.png",
                    "east": "/tmp/furnace_side.png",
                    "west": "/tmp/furnace_side.png",
                    "up": "/tmp/furnace_top.png",
                    "down": "/tmp/furnace_top.png"
                },
                "modelElements": [
                    {
                        "from": [7, 0, 7],
                        "to": [9, 10, 9],
                        "rotation": {
                            "origin": [8, 8, 8],
                            "axis": "y",
                            "angle": 45,
                            "rescale": false
                        },
                        "faceTexturePaths": {
                            "north": "/tmp/furnace_side.png",
                            "south": "/tmp/furnace_side.png",
                            "east": "/tmp/furnace_side.png",
                            "west": "/tmp/furnace_side.png",
                            "up": "/tmp/furnace_top.png",
                            "down": "/tmp/furnace_top.png"
                        },
                        "faceUvs": {
                            "east": [7, 0, 9, 16],
                            "west": [7, 0, 9, 16]
                        }
                    }
                ]
            },
            {
                "identifier": "create:andesite_casing",
                "displayName": "Andesite Casing",
                "itemId": "create:andesite_casing",
                "maxStackSize": null,
                "texturePath": null
            }
        ]
    });

    let scene = render_scene_from_scheme_with_registry_report(42, &scheme, Some(&report));

    let stone = scene
        .materials
        .iter()
        .find(|line| line.block_id == "minecraft:stone")
        .expect("stone material");
    assert_eq!(stone.display_name, "Stone");
    assert_eq!(stone.max_stack_size, Some(64));
    assert_eq!(stone.stack_count, Some(2));
    assert_eq!(stone.texture_path.as_deref(), Some("/tmp/stone.png"));
    assert!(scene
        .blocks
        .iter()
        .filter(|block| block.block_id == "minecraft:stone")
        .all(|block| block.texture_path.as_deref() == Some("/tmp/stone.png")));

    let furnace = scene
        .blocks
        .iter()
        .find(|block| block.block_id == "minecraft:furnace")
        .expect("furnace block");
    let face_textures = furnace
        .face_texture_paths
        .as_ref()
        .expect("furnace face textures");
    assert_eq!(
        face_textures.north.as_deref(),
        Some("/tmp/furnace_front.png")
    );
    assert_eq!(
        face_textures.south.as_deref(),
        Some("/tmp/furnace_side.png")
    );
    assert_eq!(face_textures.up.as_deref(), Some("/tmp/furnace_top.png"));
    assert_eq!(furnace.model_elements.len(), 1);
    assert_eq!(furnace.model_elements[0].from, [7.0, 0.0, 7.0]);
    assert_eq!(furnace.model_elements[0].to, [9.0, 10.0, 9.0]);
    let rotation = furnace.model_elements[0]
        .rotation
        .as_ref()
        .expect("model element rotation");
    assert_eq!(rotation.origin, [8.0, 8.0, 8.0]);
    assert_eq!(rotation.axis, "y");
    assert_eq!(rotation.angle, 45.0);
    assert!(!rotation.rescale);
    assert_eq!(
        furnace.model_elements[0].face_texture_paths.up.as_deref(),
        Some("/tmp/furnace_top.png")
    );
    assert_eq!(
        furnace.model_elements[0].face_uvs.east,
        Some([7.0, 0.0, 9.0, 16.0])
    );

    let casing = scene
        .materials
        .iter()
        .find(|line| line.block_id == "create:andesite_casing")
        .expect("casing material");
    assert_eq!(casing.display_name, "Andesite Casing");
    assert_eq!(casing.max_stack_size, None);
    assert_eq!(casing.stack_count, None);
}

#[test]
fn selects_render_model_variant_from_block_states() {
    let registry = BlockRegistry::from_block_ids(["minecraft:wall_torch".to_string()]);
    let mut scheme = Scheme::new(
        "Torch Variant",
        Dimensions::new(3, 3, 3).expect("dimensions"),
    );
    scheme
        .apply(
            &registry,
            SchemeOperation::Place(BlockPlacement::new(
                Coordinate::new(1, 1, 1),
                "minecraft:wall_torch",
                [("facing", "east")],
                StageRef::Unassigned,
            )),
        )
        .expect("place wall torch");
    let report = serde_json::json!({
        "runtimeStatus": "ready",
        "blocks": [
            {
                "identifier": "minecraft:wall_torch",
                "displayName": "Wall Torch",
                "itemId": "minecraft:torch",
                "maxStackSize": 64,
                "modelVariants": [
                    {
                        "condition": { "anyOf": [{ "facing": ["north"] }] },
                        "model": "minecraft:block/wall_torch",
                        "y": 0,
                        "uvLock": true,
                        "modelElements": [
                            {
                                "from": [7, 3, 0],
                                "to": [9, 13, 2],
                                "faceTexturePaths": { "north": "/tmp/torch.png" },
                                "faceUvs": { "north": [7, 3, 9, 13] }
                            }
                        ]
                    },
                    {
                        "condition": { "anyOf": [{ "facing": ["east"] }] },
                        "model": "minecraft:block/wall_torch",
                        "y": 90,
                        "uvLock": true,
                        "modelElements": [
                            {
                                "from": [7, 3, 0],
                                "to": [9, 13, 2],
                                "faceTexturePaths": { "north": "/tmp/torch.png" },
                                "faceUvs": { "north": [7, 3, 9, 13] }
                            }
                        ]
                    }
                ]
            }
        ]
    });

    let scene = render_scene_from_scheme_with_registry_report(42, &scheme, Some(&report));
    let torch = scene
        .blocks
        .iter()
        .find(|block| block.block_id == "minecraft:wall_torch")
        .expect("wall torch render block");

    assert_eq!(torch.model_elements.len(), 1);
    assert_eq!(
        torch.model_elements[0]
            .model_rotation
            .as_ref()
            .map(|rotation| (rotation.x, rotation.y, rotation.uv_lock,)),
        Some((0.0, 90.0, true))
    );
    assert_eq!(
        torch.model_elements[0].face_uvs.north,
        Some([7.0, 3.0, 9.0, 13.0])
    );
}

#[test]
fn combines_matching_multipart_render_model_variants() {
    let registry = BlockRegistry::from_block_ids(["minecraft:oak_fence".to_string()]);
    let mut scheme = Scheme::new(
        "Fence Variant",
        Dimensions::new(3, 3, 3).expect("dimensions"),
    );
    scheme
        .apply(
            &registry,
            SchemeOperation::Place(BlockPlacement::new(
                Coordinate::new(1, 1, 1),
                "minecraft:oak_fence",
                [
                    ("north", "true"),
                    ("east", "true"),
                    ("south", "false"),
                    ("west", "false"),
                ],
                StageRef::Unassigned,
            )),
        )
        .expect("place fence");
    let report = serde_json::json!({
        "runtimeStatus": "ready",
        "blocks": [
            {
                "identifier": "minecraft:oak_fence",
                "displayName": "Oak Fence",
                "modelVariantsAreMultipart": true,
                "modelVariants": [
                    {
                        "model": "minecraft:block/oak_fence_post",
                        "modelElements": [
                            {
                                "from": [6, 0, 6],
                                "to": [10, 16, 10],
                                "faceTexturePaths": { "north": "/tmp/planks.png" },
                                "faceUvs": {}
                            }
                        ]
                    },
                    {
                        "condition": { "anyOf": [{ "north": ["true"] }] },
                        "model": "minecraft:block/oak_fence_side",
                        "y": 0,
                        "uvLock": true,
                        "modelElements": [
                            {
                                "from": [7, 6, 0],
                                "to": [9, 12, 8],
                                "faceTexturePaths": { "north": "/tmp/planks.png" },
                                "faceUvs": {}
                            }
                        ]
                    },
                    {
                        "condition": { "anyOf": [{ "east": ["true"] }] },
                        "model": "minecraft:block/oak_fence_side",
                        "y": 90,
                        "uvLock": true,
                        "modelElements": [
                            {
                                "from": [7, 6, 0],
                                "to": [9, 12, 8],
                                "faceTexturePaths": { "north": "/tmp/planks.png" },
                                "faceUvs": {}
                            }
                        ]
                    }
                ]
            }
        ]
    });

    let scene = render_scene_from_scheme_with_registry_report(42, &scheme, Some(&report));
    let fence = scene
        .blocks
        .iter()
        .find(|block| block.block_id == "minecraft:oak_fence")
        .expect("fence render block");

    assert_eq!(fence.model_elements.len(), 3);
    assert_eq!(
        fence.model_elements[2]
            .model_rotation
            .as_ref()
            .map(|rotation| rotation.y),
        Some(90.0)
    );
}
