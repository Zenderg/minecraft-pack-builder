use mpb_core::{Scheme, StageRef};
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

pub fn render_scene_from_scheme(scheme_id: i64, scheme: &Scheme) -> RenderSceneDto {
    let selected_stage = scheme
        .stages()
        .last()
        .map(|stage| StageRef::Stage(stage.id))
        .unwrap_or(StageRef::Unassigned);
    let selected = prepare_render_chunks(
        scheme,
        RenderOptions {
            selected_stage,
            chunk_size: 4,
            include_future_translucent: false,
        },
    );
    let unassigned = prepare_render_chunks(
        scheme,
        RenderOptions {
            selected_stage: StageRef::Unassigned,
            chunk_size: 4,
            include_future_translucent: false,
        },
    );
    let mut chunks = selected
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
    chunks.dedup_by_key(|chunk| chunk.coordinate);

    let dimensions = scheme.dimensions();
    RenderSceneDto {
        scheme_id,
        scheme_name: scheme.name().to_string(),
        dimensions: [dimensions.x, dimensions.y, dimensions.z],
        stages: scheme
            .stages()
            .iter()
            .map(|stage| RenderStageDto {
                id: stage.id,
                name: stage.name.clone(),
                order: stage.order,
            })
            .collect(),
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

fn block_color(block_id: &str) -> &'static str {
    const PALETTE: [&str; 12] = [
        "#9aa39e", "#d3a44e", "#6bb48f", "#9bd8ff", "#c07f5b", "#7f94c0", "#9f7fc0", "#c0b77f",
        "#7fc0b7", "#c07f95", "#86b96f", "#b9a06f",
    ];
    let hash = block_id.bytes().fold(0usize, |accumulator, byte| {
        accumulator.wrapping_mul(31).wrapping_add(byte as usize)
    });
    PALETTE[hash % PALETTE.len()]
}

fn block_alpha(block_id: &str) -> Option<f32> {
    if block_id.contains("glass") {
        Some(0.58)
    } else {
        None
    }
}
