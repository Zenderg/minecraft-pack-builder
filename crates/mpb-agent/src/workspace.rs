use std::collections::BTreeMap;

use mpb_core::{
    BlockPlacement, BlockRegistry, Coordinate, Dimensions, Scheme, SchemeOperation, Selection,
    StageRef,
};
use serde::Serialize;

#[derive(Debug, Clone)]
pub struct AgentWorkspace {
    pub(crate) registry: BlockRegistry,
    pub(crate) instances: Vec<AgentInstance>,
    pub(crate) schemes: BTreeMap<i64, AgentScheme>,
    pub(crate) next_scheme_id: i64,
    pub(crate) current_selection: Option<Selection>,
}

impl AgentWorkspace {
    pub fn test_fixture() -> Self {
        let registry = BlockRegistry::synthetic_fixture();
        let mut scheme = Scheme::new(
            "Protocol Fixture Scheme",
            Dimensions::new(8, 5, 8).expect("valid dimensions"),
        );
        let foundation = scheme.add_stage("Stage 1").expect("stage 1");
        let machinery = scheme.add_stage("Stage 2").expect("stage 2");
        for placement in [
            BlockPlacement::new(
                Coordinate::new(0, 0, 0),
                "minecraft:stone_bricks",
                [("cracked", "false")],
                StageRef::Stage(foundation),
            ),
            BlockPlacement::new(
                Coordinate::new(1, 0, 0),
                "minecraft:stone_bricks",
                [("cracked", "false")],
                StageRef::Stage(foundation),
            ),
            BlockPlacement::new(
                Coordinate::new(1, 1, 0),
                "thermal:machine_frame",
                [("tier", "basic")],
                StageRef::Stage(machinery),
            ),
            BlockPlacement::new(
                Coordinate::new(2, 1, 0),
                "minecraft:glass",
                [("color", "clear")],
                StageRef::Stage(machinery),
            ),
            BlockPlacement::new(
                Coordinate::new(3, 0, 0),
                "create:andesite_casing",
                [],
                StageRef::Unassigned,
            ),
        ] {
            scheme
                .apply(&registry, SchemeOperation::Place(placement))
                .expect("valid fixture placement");
        }

        let mut schemes = BTreeMap::new();
        schemes.insert(
            10,
            AgentScheme {
                id: 10,
                instance_id: 1,
                name: "Protocol Fixture Scheme".to_string(),
                scheme,
            },
        );

        Self {
            registry,
            instances: vec![AgentInstance {
                id: 1,
                instance_id: "protocol-fixture-pack".to_string(),
                display_name: "Protocol Fixture Pack".to_string(),
                instance_path: None,
                minecraft_version: Some("1.20.1".to_string()),
                loader: Some("Forge".to_string()),
                loader_version: Some("47.4.0".to_string()),
                status: "ready".to_string(),
            }],
            schemes,
            next_scheme_id: 11,
            current_selection: None,
        }
    }

    pub fn empty() -> Self {
        Self {
            registry: BlockRegistry::from_block_ids(Vec::<String>::new()),
            instances: Vec::new(),
            schemes: BTreeMap::new(),
            next_scheme_id: 1,
            current_selection: None,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AgentInstance {
    pub(crate) id: i64,
    pub(crate) instance_id: String,
    pub(crate) display_name: String,
    pub(crate) instance_path: Option<String>,
    pub(crate) minecraft_version: Option<String>,
    pub(crate) loader: Option<String>,
    pub(crate) loader_version: Option<String>,
    pub(crate) status: String,
}

#[derive(Debug, Clone)]
pub(crate) struct AgentScheme {
    pub(crate) id: i64,
    pub(crate) instance_id: i64,
    pub(crate) name: String,
    pub(crate) scheme: Scheme,
}
