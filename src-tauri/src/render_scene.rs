use std::collections::BTreeMap;

use mpb_core::{Scheme, StageRef};
use mpb_render::{prepare_render_chunks, RenderOptions};
use serde::Serialize;
use serde_json::Value;

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
    pub texture_path: Option<String>,
    pub face_texture_paths: Option<FaceTexturePathsDto>,
    pub model_elements: Vec<ModelElementDto>,
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
pub struct RenderMaterialDto {
    pub block_id: String,
    pub display_name: String,
    pub count: u32,
    pub item_id: Option<String>,
    pub max_stack_size: Option<u32>,
    pub stack_count: Option<u32>,
    pub texture_path: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FaceTexturePathsDto {
    pub north: Option<String>,
    pub south: Option<String>,
    pub east: Option<String>,
    pub west: Option<String>,
    pub up: Option<String>,
    pub down: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelElementDto {
    pub from: [f32; 3],
    pub to: [f32; 3],
    pub face_texture_paths: FaceTexturePathsDto,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RenderSceneDto {
    pub scheme_id: i64,
    pub scheme_name: String,
    pub dimensions: [i32; 3],
    pub stages: Vec<RenderStageDto>,
    pub blocks: Vec<RenderBlockDto>,
    pub materials: Vec<RenderMaterialDto>,
    pub chunks: Vec<RenderChunkSummaryDto>,
    pub large_scheme_threshold: usize,
}

pub fn render_scene_from_scheme(scheme_id: i64, scheme: &Scheme) -> RenderSceneDto {
    render_scene_from_scheme_with_registry_report(scheme_id, scheme, None)
}

pub fn render_scene_from_scheme_with_registry_report(
    scheme_id: i64,
    scheme: &Scheme,
    registry_report: Option<&Value>,
) -> RenderSceneDto {
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
    let registry_blocks = registry_report
        .map(registry_block_metadata)
        .unwrap_or_default();

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
        materials: render_materials(scheme, &registry_blocks),
        blocks: scheme
            .blocks()
            .map(|(coordinate, block)| {
                let metadata = registry_blocks
                    .get(&block.block_id)
                    .cloned()
                    .unwrap_or_default();
                RenderBlockDto {
                    coordinate: [coordinate.x, coordinate.y, coordinate.z],
                    block_id: block.block_id.clone(),
                    stage_id: match block.stage {
                        StageRef::Stage(id) => Some(id),
                        StageRef::Unassigned => None,
                    },
                    color: block_color(&block.block_id).to_string(),
                    alpha: block_alpha(&block.block_id),
                    texture_path: metadata.texture_path.clone(),
                    face_texture_paths: metadata.face_texture_paths,
                    model_elements: metadata.model_elements,
                }
            })
            .collect(),
        chunks,
        large_scheme_threshold: 4096,
    }
}

#[derive(Debug, Clone, Default)]
struct RegistryBlockMetadata {
    display_name: Option<String>,
    item_id: Option<String>,
    max_stack_size: Option<u32>,
    texture_path: Option<String>,
    face_texture_paths: Option<FaceTexturePathsDto>,
    model_elements: Vec<ModelElementDto>,
}

fn render_materials(
    scheme: &Scheme,
    registry_blocks: &BTreeMap<String, RegistryBlockMetadata>,
) -> Vec<RenderMaterialDto> {
    scheme
        .materials()
        .into_iter()
        .map(|line| {
            let metadata = registry_blocks
                .get(&line.block_id)
                .cloned()
                .unwrap_or_default();
            let max_stack_size = metadata.max_stack_size;
            RenderMaterialDto {
                display_name: metadata
                    .display_name
                    .unwrap_or_else(|| line.block_id.clone()),
                item_id: metadata.item_id,
                max_stack_size,
                stack_count: max_stack_size
                    .filter(|size| *size > 0)
                    .map(|size| line.count.div_ceil(size)),
                texture_path: metadata.texture_path,
                block_id: line.block_id,
                count: line.count,
            }
        })
        .collect()
}

fn registry_block_metadata(report: &Value) -> BTreeMap<String, RegistryBlockMetadata> {
    report
        .get("blocks")
        .and_then(Value::as_array)
        .map(|blocks| {
            blocks
                .iter()
                .filter_map(|block| {
                    let identifier = block.get("identifier")?.as_str()?.to_string();
                    Some((
                        identifier,
                        RegistryBlockMetadata {
                            display_name: block
                                .get("displayName")
                                .and_then(Value::as_str)
                                .map(ToString::to_string),
                            item_id: block
                                .get("itemId")
                                .and_then(Value::as_str)
                                .map(ToString::to_string),
                            max_stack_size: block
                                .get("maxStackSize")
                                .and_then(Value::as_u64)
                                .and_then(|value| u32::try_from(value).ok()),
                            texture_path: block
                                .get("texturePath")
                                .and_then(Value::as_str)
                                .map(ToString::to_string),
                            face_texture_paths: block
                                .get("faceTexturePaths")
                                .map(registry_face_texture_paths),
                            model_elements: block
                                .get("modelElements")
                                .and_then(Value::as_array)
                                .map(|elements| {
                                    elements.iter().filter_map(registry_model_element).collect()
                                })
                                .unwrap_or_default(),
                        },
                    ))
                })
                .collect()
        })
        .unwrap_or_default()
}

fn registry_face_texture_paths(value: &Value) -> FaceTexturePathsDto {
    FaceTexturePathsDto {
        north: value
            .get("north")
            .and_then(Value::as_str)
            .map(ToString::to_string),
        south: value
            .get("south")
            .and_then(Value::as_str)
            .map(ToString::to_string),
        east: value
            .get("east")
            .and_then(Value::as_str)
            .map(ToString::to_string),
        west: value
            .get("west")
            .and_then(Value::as_str)
            .map(ToString::to_string),
        up: value
            .get("up")
            .and_then(Value::as_str)
            .map(ToString::to_string),
        down: value
            .get("down")
            .and_then(Value::as_str)
            .map(ToString::to_string),
    }
}

fn registry_model_element(value: &Value) -> Option<ModelElementDto> {
    Some(ModelElementDto {
        from: registry_model_vector(value.get("from")?)?,
        to: registry_model_vector(value.get("to")?)?,
        face_texture_paths: registry_face_texture_paths(value.get("faceTexturePaths")?),
    })
}

fn registry_model_vector(value: &Value) -> Option<[f32; 3]> {
    let array = value.as_array()?;
    let [x, y, z] = array.as_slice() else {
        return None;
    };
    Some([x.as_f64()? as f32, y.as_f64()? as f32, z.as_f64()? as f32])
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
