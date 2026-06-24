use std::collections::BTreeMap;

use mpb_core::{
    BlockPlacement, BlockRegistry, Coordinate, Dimensions, Scheme, SchemeOperation, Selection,
    StageRef,
};
use serde::Serialize;

#[derive(Debug, Clone)]
pub struct AgentWorkspace {
    pub(crate) registry: BlockRegistry,
    pub(crate) modpacks: Vec<AgentModpack>,
    pub(crate) schemes: BTreeMap<i64, AgentScheme>,
    pub(crate) next_scheme_id: i64,
    pub(crate) current_selection: Option<Selection>,
}

impl AgentWorkspace {
    pub fn demo() -> Self {
        let registry = BlockRegistry::synthetic_fixture();
        let mut scheme = Scheme::new(
            "Starter Factory",
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
                .expect("valid demo placement");
        }

        let mut schemes = BTreeMap::new();
        schemes.insert(
            10,
            AgentScheme {
                id: 10,
                modpack_id: 1,
                name: "Starter Factory".to_string(),
                scheme,
            },
        );

        Self {
            registry,
            modpacks: vec![AgentModpack {
                id: 1,
                local_name: "AOC - 1.0.0".to_string(),
                source_url: Some("https://www.curseforge.com/minecraft/modpacks/aoc".to_string()),
                version_name: "1.0.0".to_string(),
                minecraft_version: Some("1.20.1".to_string()),
                loader: Some("Forge".to_string()),
            }],
            schemes,
            next_scheme_id: 11,
            current_selection: None,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AgentModpack {
    pub(crate) id: i64,
    pub(crate) local_name: String,
    pub(crate) source_url: Option<String>,
    pub(crate) version_name: String,
    pub(crate) minecraft_version: Option<String>,
    pub(crate) loader: Option<String>,
}

#[derive(Debug, Clone)]
pub(crate) struct AgentScheme {
    pub(crate) id: i64,
    pub(crate) modpack_id: i64,
    pub(crate) name: String,
    pub(crate) scheme: Scheme,
}
