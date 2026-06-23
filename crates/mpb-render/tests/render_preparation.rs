use mpb_core::{
    BlockPlacement, BlockRegistry, Coordinate, Dimensions, Scheme, SchemeOperation, StageRef,
};
use mpb_render::{prepare_render_chunks, ChunkCoordinate, RenderOptions};

fn staged_scheme() -> (BlockRegistry, Scheme, u32, u32) {
    let registry = BlockRegistry::synthetic_fixture();
    let mut scheme = Scheme::new("Render Demo", Dimensions::new(8, 4, 8).expect("dimensions"));
    let foundation = scheme.add_stage("Foundation").expect("foundation stage");
    let machinery = scheme.add_stage("Machinery").expect("machinery stage");
    (registry, scheme, foundation, machinery)
}

#[test]
fn visible_stage_rendering_is_cumulative_and_keeps_unassigned_separate() {
    let (registry, mut scheme, foundation, machinery) = staged_scheme();
    for coordinate in [Coordinate::new(0, 0, 0), Coordinate::new(1, 0, 0)] {
        scheme
            .apply(
                &registry,
                SchemeOperation::Place(BlockPlacement::new(
                    coordinate,
                    "minecraft:stone_bricks",
                    [("cracked", "false")],
                    StageRef::Stage(foundation),
                )),
            )
            .expect("place foundation block");
    }
    scheme
        .apply(
            &registry,
            SchemeOperation::Place(BlockPlacement::new(
                Coordinate::new(0, 1, 0),
                "thermal:machine_frame",
                [("tier", "basic")],
                StageRef::Stage(machinery),
            )),
        )
        .expect("place machinery block");
    scheme
        .apply(
            &registry,
            SchemeOperation::Place(BlockPlacement::new(
                Coordinate::new(7, 0, 7),
                "create:andesite_casing",
                [],
                StageRef::Unassigned,
            )),
        )
        .expect("place unassigned block");

    let stage_one = prepare_render_chunks(
        &scheme,
        RenderOptions {
            selected_stage: StageRef::Stage(foundation),
            chunk_size: 4,
            include_future_translucent: false,
        },
    );
    let stage_two = prepare_render_chunks(
        &scheme,
        RenderOptions {
            selected_stage: StageRef::Stage(machinery),
            chunk_size: 4,
            include_future_translucent: false,
        },
    );
    let unassigned = prepare_render_chunks(
        &scheme,
        RenderOptions {
            selected_stage: StageRef::Unassigned,
            chunk_size: 4,
            include_future_translucent: false,
        },
    );

    assert_eq!(stage_one.visible_block_count, 2);
    assert_eq!(stage_two.visible_block_count, 3);
    assert_eq!(unassigned.visible_block_count, 1);
    assert_eq!(stage_two.chunks.len(), 1);
    assert_eq!(
        unassigned.chunks[0].coordinate,
        ChunkCoordinate { x: 1, y: 0, z: 1 }
    );
}

#[test]
fn mesh_generation_skips_internal_opaque_faces_and_records_picking_metadata() {
    let (registry, mut scheme, foundation, _) = staged_scheme();
    for coordinate in [Coordinate::new(0, 0, 0), Coordinate::new(1, 0, 0)] {
        scheme
            .apply(
                &registry,
                SchemeOperation::Place(BlockPlacement::new(
                    coordinate,
                    "minecraft:stone_bricks",
                    [("cracked", "false")],
                    StageRef::Stage(foundation),
                )),
            )
            .expect("place adjacent block");
    }

    let prepared = prepare_render_chunks(
        &scheme,
        RenderOptions {
            selected_stage: StageRef::Stage(foundation),
            chunk_size: 16,
            include_future_translucent: false,
        },
    );

    assert_eq!(prepared.visible_block_count, 2);
    assert_eq!(prepared.total_face_count, 10);
    assert_eq!(prepared.chunks.len(), 1);
    assert_eq!(prepared.chunks[0].mesh.indices.len(), 60);
    assert_eq!(prepared.chunks[0].mesh.positions.len(), 120);
    assert_eq!(prepared.chunks[0].picking.len(), 10);
    assert!(prepared.chunks[0]
        .picking
        .iter()
        .any(|metadata| metadata.coordinate == Coordinate::new(1, 0, 0)));
}

#[test]
fn future_stage_translucency_keeps_later_blocks_in_the_buffers() {
    let (registry, mut scheme, foundation, machinery) = staged_scheme();
    scheme
        .apply(
            &registry,
            SchemeOperation::Place(BlockPlacement::new(
                Coordinate::new(0, 0, 0),
                "minecraft:stone_bricks",
                [("cracked", "false")],
                StageRef::Stage(foundation),
            )),
        )
        .expect("place foundation block");
    scheme
        .apply(
            &registry,
            SchemeOperation::Place(BlockPlacement::new(
                Coordinate::new(0, 1, 0),
                "thermal:machine_frame",
                [("tier", "basic")],
                StageRef::Stage(machinery),
            )),
        )
        .expect("place future block");

    let prepared = prepare_render_chunks(
        &scheme,
        RenderOptions {
            selected_stage: StageRef::Stage(foundation),
            chunk_size: 16,
            include_future_translucent: true,
        },
    );

    assert_eq!(prepared.visible_block_count, 1);
    assert_eq!(prepared.translucent_block_count, 1);
    assert!(prepared.chunks[0]
        .mesh
        .alphas
        .iter()
        .any(|alpha| *alpha < 1.0));
}
