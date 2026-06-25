use std::collections::{BTreeMap, BTreeSet};

use mpb_core::{
    BlockPlacement, BlockRegistry, Coordinate, Dimensions, MaterialLine, Scheme, SchemeError,
    SchemeOperation, StageRef,
};

#[test]
fn new_schemes_are_sparse_and_compute_bounds_from_blocks() {
    let registry = BlockRegistry::synthetic_fixture();
    let mut scheme = Scheme::new("Sparse Factory");

    assert_eq!(scheme.bounds(), None);
    assert_eq!(scheme.computed_dimensions(), None);

    scheme
        .apply(
            &registry,
            SchemeOperation::Place(BlockPlacement::new(
                Coordinate::new(4, 2, 7),
                "minecraft:stone_bricks",
                [("cracked", "false")],
                StageRef::Unassigned,
            )),
        )
        .expect("place positive coordinate");

    let bounds = scheme.bounds().expect("bounds");
    assert_eq!(bounds.min, Coordinate::new(4, 2, 7));
    assert_eq!(bounds.max, Coordinate::new(4, 2, 7));
    assert_eq!(
        scheme.computed_dimensions(),
        Some(Dimensions::new(5, 3, 8).expect("computed dimensions"))
    );
}

#[test]
fn sparse_schemes_reject_negative_coordinates_without_fixed_size_limits() {
    let registry = BlockRegistry::synthetic_fixture();
    let mut scheme = Scheme::new("Sparse Bounds");

    scheme
        .apply(
            &registry,
            SchemeOperation::Place(BlockPlacement::new(
                Coordinate::new(512, 0, 512),
                "minecraft:stone_bricks",
                [("cracked", "false")],
                StageRef::Unassigned,
            )),
        )
        .expect("large positive coordinates are valid");

    let result = scheme.apply(
        &registry,
        SchemeOperation::Place(BlockPlacement::new(
            Coordinate::new(-1, 0, 0),
            "minecraft:stone_bricks",
            [("cracked", "false")],
            StageRef::Unassigned,
        )),
    );

    assert!(matches!(
        result,
        Err(SchemeError::NegativeCoordinate {
            coordinate: Coordinate { x: -1, y: 0, z: 0 }
        })
    ));
}

#[test]
fn incomplete_stages_fall_back_to_single_build_stage() {
    let registry = BlockRegistry::synthetic_fixture();
    let mut scheme = Scheme::new("Incomplete Stages");
    let foundation = scheme.add_stage("Foundation").expect("stage");
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
        .expect("staged block");
    scheme
        .apply(
            &registry,
            SchemeOperation::Place(BlockPlacement::new(
                Coordinate::new(1, 0, 0),
                "create:andesite_casing",
                [],
                StageRef::Unassigned,
            )),
        )
        .expect("unassigned block");

    let plan = scheme.stage_plan();

    assert!(!plan.complete);
    assert_eq!(plan.effective_stage_count, 1);
    assert_eq!(plan.message.as_deref(), Some("Stages incomplete"));
}

fn demo_scheme() -> (BlockRegistry, Scheme, u32, u32) {
    let registry = BlockRegistry::synthetic_fixture();
    let mut scheme = Scheme::new("Domain Demo");
    let foundation = scheme.add_stage("Foundation").expect("foundation stage");
    let machinery = scheme.add_stage("Machinery").expect("machinery stage");
    (registry, scheme, foundation, machinery)
}

#[test]
fn scheme_exposes_user_facing_name_for_export_metadata() {
    let scheme = Scheme::new("Exportable Factory");

    assert_eq!(scheme.name(), "Exportable Factory");
}

#[test]
fn operations_generate_materials_for_all_blocks_including_unassigned() {
    let (registry, mut scheme, foundation, machinery) = demo_scheme();

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
        .expect("place foundation");
    scheme
        .apply(
            &registry,
            SchemeOperation::Place(BlockPlacement::new(
                Coordinate::new(1, 0, 0),
                "create:andesite_casing",
                [],
                StageRef::Unassigned,
            )),
        )
        .expect("place unassigned");
    scheme
        .apply(
            &registry,
            SchemeOperation::BulkSet {
                selection: Coordinate::new(0, 1, 0).to_selection(Coordinate::new(1, 1, 0)),
                block: BlockPlacement::new(
                    Coordinate::new(0, 1, 0),
                    "minecraft:glass",
                    [("color", "clear")],
                    StageRef::Stage(machinery),
                )
                .block,
            },
        )
        .expect("bulk set glass");

    assert_eq!(
        scheme.materials(),
        vec![
            MaterialLine::new("create:andesite_casing", 1),
            MaterialLine::new("minecraft:glass", 2),
            MaterialLine::new("minecraft:stone_bricks", 1),
        ]
    );
    assert_eq!(scheme.block_count(), 4);
}

#[test]
fn invalid_bulk_operations_are_atomic() {
    let (registry, mut scheme, foundation, _) = demo_scheme();
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
        .expect("place initial block");
    let before = scheme.materials();

    let result = scheme.apply(
        &registry,
        SchemeOperation::BulkSet {
            selection: Coordinate::new(0, 0, 0).to_selection(Coordinate::new(2, 0, 0)),
            block: BlockPlacement::new(
                Coordinate::new(0, 0, 0),
                "minecraft:missing_block",
                [],
                StageRef::Unassigned,
            )
            .block,
        },
    );

    assert!(result.is_err());
    assert_eq!(scheme.materials(), before);
    assert_eq!(scheme.block_count(), 1);
}

#[test]
fn coordinates_are_valid_when_non_negative() {
    let registry = BlockRegistry::synthetic_fixture();
    let mut scheme = Scheme::new("Bounds");

    for x in -1..=2 {
        for y in -1..=2 {
            for z in -1..=2 {
                let coord = Coordinate::new(x, y, z);
                let result = scheme.apply(
                    &registry,
                    SchemeOperation::Place(BlockPlacement::new(
                        coord,
                        "minecraft:stone_bricks",
                        [("cracked", "false")],
                        StageRef::Unassigned,
                    )),
                );
                assert_eq!(
                    result.is_ok(),
                    x >= 0 && y >= 0 && z >= 0,
                    "unexpected coordinate validation for {coord:?}"
                );
            }
        }
    }
}

#[test]
fn construction_stage_visibility_is_cumulative() {
    let (registry, mut scheme, foundation, machinery) = demo_scheme();
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
        .expect("place foundation");
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
        .expect("place machinery");
    scheme
        .apply(
            &registry,
            SchemeOperation::Place(BlockPlacement::new(
                Coordinate::new(1, 1, 0),
                "create:andesite_casing",
                [],
                StageRef::Unassigned,
            )),
        )
        .expect("place unassigned");

    assert_eq!(scheme.visible_blocks(StageRef::Stage(foundation)).len(), 1);
    assert_eq!(scheme.visible_blocks(StageRef::Stage(machinery)).len(), 2);
    assert_eq!(scheme.visible_blocks(StageRef::Unassigned).len(), 1);
}

#[test]
fn registry_built_from_blockstate_definitions_validates_directional_states() {
    let registry = BlockRegistry::from_block_state_definitions([(
        "minecraft:furnace".to_string(),
        BTreeMap::from([(
            "facing".to_string(),
            BTreeSet::from([
                "north".to_string(),
                "south".to_string(),
                "east".to_string(),
                "west".to_string(),
            ]),
        )]),
    )]);
    let mut scheme = Scheme::new("Directional");

    scheme
        .apply(
            &registry,
            SchemeOperation::Place(BlockPlacement::new(
                Coordinate::new(0, 0, 0),
                "minecraft:furnace",
                [("facing", "north")],
                StageRef::Unassigned,
            )),
        )
        .expect("valid furnace facing");

    let missing = scheme
        .apply(
            &registry,
            SchemeOperation::Place(BlockPlacement::new(
                Coordinate::new(1, 0, 0),
                "minecraft:furnace",
                [],
                StageRef::Unassigned,
            )),
        )
        .expect_err("missing facing is rejected");
    assert!(matches!(missing, SchemeError::MissingBlockState { .. }));

    let invalid = scheme
        .apply(
            &registry,
            SchemeOperation::Place(BlockPlacement::new(
                Coordinate::new(1, 0, 0),
                "minecraft:furnace",
                [("facing", "up")],
                StageRef::Unassigned,
            )),
        )
        .expect_err("invalid facing is rejected");
    assert!(matches!(invalid, SchemeError::InvalidBlockState { .. }));

    let unknown = scheme
        .apply(
            &registry,
            SchemeOperation::Place(BlockPlacement::new(
                Coordinate::new(1, 0, 0),
                "minecraft:furnace",
                [("facing", "south"), ("lit", "false")],
                StageRef::Unassigned,
            )),
        )
        .expect_err("unknown state is rejected");
    assert!(matches!(unknown, SchemeError::UnknownBlockState { .. }));
    assert_eq!(scheme.block_count(), 1);
}
