use mpb_core::{
    BlockPlacement, BlockRegistry, Coordinate, Dimensions, MaterialLine, Scheme, SchemeOperation,
    StageRef,
};

fn demo_scheme() -> (BlockRegistry, Scheme, u32, u32) {
    let registry = BlockRegistry::synthetic_fixture();
    let mut scheme = Scheme::new("Domain Demo", Dimensions::new(4, 3, 4).expect("dimensions"));
    let foundation = scheme.add_stage("Foundation").expect("foundation stage");
    let machinery = scheme.add_stage("Machinery").expect("machinery stage");
    (registry, scheme, foundation, machinery)
}

#[test]
fn scheme_exposes_user_facing_name_for_export_metadata() {
    let scheme = Scheme::new(
        "Exportable Factory",
        Dimensions::new(4, 3, 4).expect("dimensions"),
    );

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
fn coordinates_are_valid_only_inside_scheme_dimensions() {
    let registry = BlockRegistry::synthetic_fixture();
    let mut scheme = Scheme::new("Bounds", Dimensions::new(2, 2, 2).expect("dimensions"));

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
                    (0..2).contains(&x) && (0..2).contains(&y) && (0..2).contains(&z),
                    "unexpected coordinate validation for {coord:?}"
                );
            }
        }
    }
}

#[test]
fn resize_rejects_dimensions_that_would_drop_existing_blocks() {
    let (registry, mut scheme, foundation, _) = demo_scheme();
    scheme
        .apply(
            &registry,
            SchemeOperation::Place(BlockPlacement::new(
                Coordinate::new(3, 2, 3),
                "minecraft:stone_bricks",
                [("cracked", "false")],
                StageRef::Stage(foundation),
            )),
        )
        .expect("place edge block");

    let result = scheme.apply(
        &registry,
        SchemeOperation::Resize(Dimensions::new(3, 3, 4).expect("smaller dimensions")),
    );

    assert!(result.is_err());
    assert_eq!(
        scheme.dimensions(),
        Dimensions::new(4, 3, 4).expect("original dimensions")
    );
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
fn domain_demo_report_snapshots_structured_validation_errors() {
    let report = mpb_core::domain_demo_report();
    let json = serde_json::to_string_pretty(&report).expect("serialize report");

    assert_eq!(report.summary.block_count, 6);
    assert_eq!(report.summary.material_count, 4);
    assert!(report
        .rejected_actions
        .iter()
        .any(|action| action.code == "unknown_block"));
    assert!(report
        .rejected_actions
        .iter()
        .any(|action| action.code == "coordinate_out_of_bounds"));
    assert_eq!(
        json,
        r#"{
  "schemeName": "Domain Demo Scheme",
  "summary": {
    "dimensions": {
      "x": 4,
      "y": 3,
      "z": 4
    },
    "stageCount": 3,
    "blockCount": 6,
    "materialCount": 4
  },
  "stages": [
    {
      "id": 1,
      "name": "Foundation",
      "order": 1
    },
    {
      "id": 2,
      "name": "Machines",
      "order": 2
    },
    {
      "id": null,
      "name": "Unassigned",
      "order": null
    }
  ],
  "materials": [
    {
      "blockId": "create:andesite_casing",
      "count": 1
    },
    {
      "blockId": "minecraft:glass",
      "count": 2
    },
    {
      "blockId": "minecraft:stone_bricks",
      "count": 2
    },
    {
      "blockId": "thermal:machine_frame",
      "count": 1
    }
  ],
  "rejectedActions": [
    {
      "action": "place missing block",
      "code": "unknown_block",
      "message": "unknown block id minecraft:missing_block"
    },
    {
      "action": "bulk set out of bounds",
      "code": "coordinate_out_of_bounds",
      "message": "coordinate (4, 0, 0) is outside 4 x 3 x 4"
    }
  ]
}"#
    );
}
