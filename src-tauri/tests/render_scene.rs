use app_tauri_lib::render_scene_from_scheme;
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
