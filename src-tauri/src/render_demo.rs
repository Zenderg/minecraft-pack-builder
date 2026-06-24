use mpb_core::{
    BlockPlacement, BlockRegistry, Coordinate, Dimensions, Scheme, SchemeOperation, StageRef,
};
use mpb_render::{prepare_render_chunks, RenderOptions};
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RenderStageDto {
    pub id: u32,
    pub name: String,
    pub order: u32,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RenderBlockDto {
    pub coordinate: [i32; 3],
    pub block_id: String,
    pub stage_id: Option<u32>,
    pub color: String,
    pub alpha: Option<f32>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RenderChunkSummaryDto {
    pub coordinate: [i32; 3],
    pub block_count: usize,
    pub face_count: usize,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RenderSceneDto {
    pub scheme_id: i64,
    pub scheme_name: String,
    pub dimensions: [i32; 3],
    pub stages: Vec<RenderStageDto>,
    pub blocks: Vec<RenderBlockDto>,
    pub chunks: Vec<RenderChunkSummaryDto>,
    pub large_scheme_threshold: usize,
}

pub fn demo_render_scene(scheme_id: i64) -> RenderSceneDto {
    let (scheme, foundation, machinery) = build_phase_7_demo_scheme();
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
    let mut chunks = stage_two
        .chunks
        .into_iter()
        .chain(unassigned.chunks)
        .map(|chunk| RenderChunkSummaryDto {
            coordinate: [chunk.coordinate.x, chunk.coordinate.y, chunk.coordinate.z],
            block_count: chunk.block_count,
            face_count: chunk.picking.len(),
        })
        .collect::<Vec<_>>();
    chunks.sort_by_key(|chunk| chunk.coordinate);

    RenderSceneDto {
        scheme_id,
        scheme_name: "Starter Factory".to_string(),
        dimensions: [8, 5, 8],
        stages: vec![
            RenderStageDto {
                id: foundation,
                name: "Stage 1".to_string(),
                order: 1,
            },
            RenderStageDto {
                id: machinery,
                name: "Stage 2".to_string(),
                order: 2,
            },
        ],
        blocks: scheme
            .blocks()
            .map(|(coordinate, block)| RenderBlockDto {
                coordinate: [coordinate.x, coordinate.y, coordinate.z],
                block_id: block.block_id.clone(),
                stage_id: match block.stage {
                    StageRef::Stage(id) => Some(id),
                    StageRef::Unassigned => None,
                },
                color: block_color(&block.block_id).to_string(),
                alpha: block_alpha(&block.block_id),
            })
            .collect(),
        chunks,
        large_scheme_threshold: 4096,
    }
}

pub fn demo_export_scheme(_scheme_id: i64) -> Scheme {
    build_phase_7_demo_scheme().0
}

fn build_phase_7_demo_scheme() -> (Scheme, u32, u32) {
    let registry = BlockRegistry::synthetic_fixture();
    let mut scheme = Scheme::new(
        "Starter Factory",
        Dimensions::new(8, 5, 8).expect("valid render demo dimensions"),
    );
    let foundation = scheme.add_stage("Stage 1").expect("stage 1");
    let machinery = scheme.add_stage("Stage 2").expect("stage 2");

    for (coordinate, block_id, states, stage) in [
        (
            Coordinate::new(0, 0, 0),
            "minecraft:stone_bricks",
            vec![("cracked", "false")],
            StageRef::Stage(foundation),
        ),
        (
            Coordinate::new(1, 0, 0),
            "minecraft:stone_bricks",
            vec![("cracked", "false")],
            StageRef::Stage(foundation),
        ),
        (
            Coordinate::new(2, 0, 0),
            "minecraft:stone_bricks",
            vec![("cracked", "false")],
            StageRef::Stage(foundation),
        ),
        (
            Coordinate::new(1, 1, 0),
            "thermal:machine_frame",
            vec![("tier", "basic")],
            StageRef::Stage(machinery),
        ),
        (
            Coordinate::new(2, 1, 0),
            "thermal:machine_frame",
            vec![("tier", "basic")],
            StageRef::Stage(machinery),
        ),
        (
            Coordinate::new(3, 0, 0),
            "create:andesite_casing",
            vec![],
            StageRef::Stage(machinery),
        ),
        (
            Coordinate::new(3, 1, 0),
            "minecraft:glass",
            vec![("color", "clear")],
            StageRef::Stage(machinery),
        ),
        (
            Coordinate::new(4, 0, 1),
            "create:andesite_casing",
            vec![],
            StageRef::Unassigned,
        ),
        (
            Coordinate::new(4, 1, 1),
            "minecraft:glass",
            vec![("color", "clear")],
            StageRef::Unassigned,
        ),
    ] {
        scheme
            .apply(
                &registry,
                SchemeOperation::Place(BlockPlacement::new(coordinate, block_id, states, stage)),
            )
            .expect("render demo operation should be valid");
    }

    (scheme, foundation, machinery)
}

fn block_color(block_id: &str) -> &'static str {
    match block_id {
        "minecraft:stone_bricks" => "#9aa39e",
        "thermal:machine_frame" => "#d3a44e",
        "create:andesite_casing" => "#6bb48f",
        "minecraft:glass" => "#9bd8ff",
        _ => "#93a19c",
    }
}

fn block_alpha(block_id: &str) -> Option<f32> {
    if block_id.contains("glass") {
        Some(0.58)
    } else {
        None
    }
}
