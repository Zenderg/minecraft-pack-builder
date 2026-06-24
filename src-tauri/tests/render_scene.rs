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
                        "faceTexturePaths": {
                            "north": "/tmp/furnace_side.png",
                            "south": "/tmp/furnace_side.png",
                            "east": "/tmp/furnace_side.png",
                            "west": "/tmp/furnace_side.png",
                            "up": "/tmp/furnace_top.png",
                            "down": "/tmp/furnace_top.png"
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
    assert_eq!(
        furnace.model_elements[0].face_texture_paths.up.as_deref(),
        Some("/tmp/furnace_top.png")
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
