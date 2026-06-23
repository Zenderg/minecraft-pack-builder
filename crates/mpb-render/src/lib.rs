//! Render chunk and mesh preparation for scheme viewers.

use std::collections::{BTreeMap, BTreeSet};

use mpb_core::{Coordinate, Dimensions, Scheme, StageRef};
use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RenderOptions {
    pub selected_stage: StageRef,
    pub chunk_size: i32,
    pub include_future_translucent: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PreparedRenderScene {
    pub dimensions: Dimensions,
    pub selected_stage: StageRef,
    pub visible_block_count: usize,
    pub translucent_block_count: usize,
    pub total_face_count: usize,
    pub chunks: Vec<RenderChunk>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChunkCoordinate {
    pub x: i32,
    pub y: i32,
    pub z: i32,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RenderChunk {
    pub coordinate: ChunkCoordinate,
    pub block_count: usize,
    pub mesh: MeshBuffer,
    pub picking: Vec<PickingMetadata>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MeshBuffer {
    pub positions: Vec<f32>,
    pub normals: Vec<f32>,
    pub indices: Vec<u32>,
    pub block_ids: Vec<String>,
    pub alphas: Vec<f32>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PickingMetadata {
    pub face_index: u32,
    pub index_start: u32,
    pub coordinate: Coordinate,
    pub normal: [i32; 3],
    pub block_id: String,
    pub stage: StageRef,
}

#[derive(Debug, Clone)]
struct RenderBlock {
    coordinate: Coordinate,
    block_id: String,
    stage: StageRef,
    alpha: f32,
}

#[derive(Debug, Clone, Copy)]
struct Face {
    normal: [i32; 3],
    corners: [[f32; 3]; 4],
}

const FACES: [Face; 6] = [
    Face {
        normal: [1, 0, 0],
        corners: [
            [1.0, 0.0, 0.0],
            [1.0, 1.0, 0.0],
            [1.0, 1.0, 1.0],
            [1.0, 0.0, 1.0],
        ],
    },
    Face {
        normal: [-1, 0, 0],
        corners: [
            [0.0, 0.0, 1.0],
            [0.0, 1.0, 1.0],
            [0.0, 1.0, 0.0],
            [0.0, 0.0, 0.0],
        ],
    },
    Face {
        normal: [0, 1, 0],
        corners: [
            [0.0, 1.0, 1.0],
            [1.0, 1.0, 1.0],
            [1.0, 1.0, 0.0],
            [0.0, 1.0, 0.0],
        ],
    },
    Face {
        normal: [0, -1, 0],
        corners: [
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [1.0, 0.0, 1.0],
            [0.0, 0.0, 1.0],
        ],
    },
    Face {
        normal: [0, 0, 1],
        corners: [
            [1.0, 0.0, 1.0],
            [1.0, 1.0, 1.0],
            [0.0, 1.0, 1.0],
            [0.0, 0.0, 1.0],
        ],
    },
    Face {
        normal: [0, 0, -1],
        corners: [
            [0.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            [1.0, 1.0, 0.0],
            [1.0, 0.0, 0.0],
        ],
    },
];

pub fn prepare_render_chunks(scheme: &Scheme, options: RenderOptions) -> PreparedRenderScene {
    let chunk_size = options.chunk_size.max(1);
    let render_blocks = collect_render_blocks(scheme, options);
    let occupied_opaque = render_blocks
        .iter()
        .filter(|block| is_opaque(&block.block_id) && block.alpha >= 1.0)
        .map(|block| block.coordinate)
        .collect::<BTreeSet<_>>();
    let visible_block_count = render_blocks
        .iter()
        .filter(|block| block.alpha >= 1.0)
        .count();
    let translucent_block_count = render_blocks
        .iter()
        .filter(|block| block.alpha < 1.0)
        .count();

    let mut chunks: BTreeMap<ChunkCoordinate, Vec<RenderBlock>> = BTreeMap::new();
    for block in render_blocks {
        chunks
            .entry(chunk_coordinate(block.coordinate, chunk_size))
            .or_default()
            .push(block);
    }

    let chunks = chunks
        .into_iter()
        .map(|(coordinate, blocks)| build_chunk(coordinate, blocks, &occupied_opaque))
        .collect::<Vec<_>>();
    let total_face_count = chunks.iter().map(|chunk| chunk.picking.len()).sum();

    PreparedRenderScene {
        dimensions: scheme.dimensions(),
        selected_stage: options.selected_stage,
        visible_block_count,
        translucent_block_count,
        total_face_count,
        chunks,
    }
}

fn collect_render_blocks(scheme: &Scheme, options: RenderOptions) -> Vec<RenderBlock> {
    scheme
        .blocks()
        .filter_map(|(coordinate, block)| {
            if scheme.is_visible_at(block.stage, options.selected_stage) {
                return Some(RenderBlock {
                    coordinate: *coordinate,
                    block_id: block.block_id.clone(),
                    stage: block.stage,
                    alpha: 1.0,
                });
            }
            if options.include_future_translucent
                && scheme.is_future_stage(block.stage, options.selected_stage)
            {
                return Some(RenderBlock {
                    coordinate: *coordinate,
                    block_id: block.block_id.clone(),
                    stage: block.stage,
                    alpha: 0.28,
                });
            }
            None
        })
        .collect()
}

fn build_chunk(
    coordinate: ChunkCoordinate,
    blocks: Vec<RenderBlock>,
    occupied_opaque: &BTreeSet<Coordinate>,
) -> RenderChunk {
    let mut mesh = MeshBuffer {
        positions: Vec::new(),
        normals: Vec::new(),
        indices: Vec::new(),
        block_ids: Vec::new(),
        alphas: Vec::new(),
    };
    let mut picking = Vec::new();

    for block in &blocks {
        for face in FACES {
            let neighbor = Coordinate::new(
                block.coordinate.x + face.normal[0],
                block.coordinate.y + face.normal[1],
                block.coordinate.z + face.normal[2],
            );
            if is_opaque(&block.block_id)
                && block.alpha >= 1.0
                && occupied_opaque.contains(&neighbor)
            {
                continue;
            }

            let vertex_start = (mesh.positions.len() / 3) as u32;
            for corner in face.corners {
                mesh.positions.extend([
                    block.coordinate.x as f32 + corner[0],
                    block.coordinate.y as f32 + corner[1],
                    block.coordinate.z as f32 + corner[2],
                ]);
                mesh.normals.extend([
                    face.normal[0] as f32,
                    face.normal[1] as f32,
                    face.normal[2] as f32,
                ]);
                mesh.block_ids.push(block.block_id.clone());
                mesh.alphas.push(block.alpha);
            }

            let index_start = mesh.indices.len() as u32;
            mesh.indices.extend([
                vertex_start,
                vertex_start + 1,
                vertex_start + 2,
                vertex_start,
                vertex_start + 2,
                vertex_start + 3,
            ]);
            picking.push(PickingMetadata {
                face_index: picking.len() as u32,
                index_start,
                coordinate: block.coordinate,
                normal: face.normal,
                block_id: block.block_id.clone(),
                stage: block.stage,
            });
        }
    }

    RenderChunk {
        coordinate,
        block_count: blocks.len(),
        mesh,
        picking,
    }
}

fn chunk_coordinate(coordinate: Coordinate, chunk_size: i32) -> ChunkCoordinate {
    ChunkCoordinate {
        x: coordinate.x.div_euclid(chunk_size),
        y: coordinate.y.div_euclid(chunk_size),
        z: coordinate.z.div_euclid(chunk_size),
    }
}

fn is_opaque(block_id: &str) -> bool {
    !block_id.contains("glass")
}
