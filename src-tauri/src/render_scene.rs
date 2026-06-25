use std::collections::BTreeMap;

use mpb_core::{Scheme, SchemeBlock, StageRef};
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
    pub render_fidelity: Option<String>,
    pub render_source: Option<String>,
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub north: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub south: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub east: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub west: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub up: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub down: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelElementDto {
    pub from: [f32; 3],
    pub to: [f32; 3],
    pub rotation: Option<ModelElementRotationDto>,
    pub model_rotation: Option<ModelRotationDto>,
    pub face_texture_paths: FaceTexturePathsDto,
    pub face_uvs: FaceUvsDto,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelElementRotationDto {
    pub origin: [f32; 3],
    pub axis: String,
    pub angle: f32,
    pub rescale: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelRotationDto {
    pub x: f32,
    pub y: f32,
    pub uv_lock: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FaceUvsDto {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub north: Option<[f32; 4]>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub south: Option<[f32; 4]>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub east: Option<[f32; 4]>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub west: Option<[f32; 4]>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub up: Option<[f32; 4]>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub down: Option<[f32; 4]>,
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
                let render_model = metadata.render_model_for_block(block);
                RenderBlockDto {
                    coordinate: [coordinate.x, coordinate.y, coordinate.z],
                    block_id: block.block_id.clone(),
                    stage_id: match block.stage {
                        StageRef::Stage(id) => Some(id),
                        StageRef::Unassigned => None,
                    },
                    color: block_color(&block.block_id).to_string(),
                    alpha: block_alpha(&block.block_id),
                    texture_path: render_model.texture_path,
                    face_texture_paths: render_model.face_texture_paths,
                    model_elements: render_model.model_elements,
                    render_fidelity: render_model.render_fidelity,
                    render_source: render_model.render_source,
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
    model_variants_are_multipart: bool,
    model_variants: Vec<RegistryModelVariant>,
    render_assets: Vec<RegistryRenderAsset>,
}

#[derive(Debug, Clone, Default)]
struct RegistryRenderModel {
    texture_path: Option<String>,
    face_texture_paths: Option<FaceTexturePathsDto>,
    model_elements: Vec<ModelElementDto>,
    render_fidelity: Option<String>,
    render_source: Option<String>,
}

#[derive(Debug, Clone)]
struct RegistryModelVariant {
    condition: Option<RegistryModelCondition>,
    model: Option<String>,
    x: f32,
    y: f32,
    uv_lock: bool,
    texture_path: Option<String>,
    face_texture_paths: Option<FaceTexturePathsDto>,
    model_elements: Vec<ModelElementDto>,
}

#[derive(Debug, Clone)]
struct RegistryRenderAsset {
    condition: Option<RegistryModelCondition>,
    fidelity: String,
    source: String,
    texture_path: Option<String>,
    face_texture_paths: Option<FaceTexturePathsDto>,
    model_elements: Vec<ModelElementDto>,
}

#[derive(Debug, Clone)]
struct RegistryModelCondition {
    any_of: Vec<BTreeMap<String, Vec<String>>>,
}

impl RegistryBlockMetadata {
    fn render_model_for_block(&self, block: &SchemeBlock) -> RegistryRenderModel {
        if let Some(render_model) =
            self.runtime_render_model_for_block(block, RuntimeRenderAssetPreference::Authoritative)
        {
            return render_model;
        }

        let static_render_model = self.static_render_model_for_block(block);
        if static_render_model.has_render_payload() || self.render_assets.is_empty() {
            return static_render_model;
        }

        self.runtime_render_model_for_block(block, RuntimeRenderAssetPreference::Any)
            .unwrap_or(static_render_model)
    }

    fn static_render_model_for_block(&self, block: &SchemeBlock) -> RegistryRenderModel {
        if self.model_variants.is_empty() {
            return RegistryRenderModel {
                texture_path: self.texture_path.clone(),
                face_texture_paths: self.face_texture_paths.clone(),
                model_elements: self.model_elements.clone(),
                render_fidelity: None,
                render_source: None,
            };
        }

        let matching = self
            .model_variants
            .iter()
            .filter(|variant| variant_matches(variant.condition.as_ref(), &block.states))
            .collect::<Vec<_>>();
        let selected = if self.model_variants_are_multipart {
            matching
        } else {
            matching.into_iter().take(1).collect()
        };
        if selected.is_empty() {
            return RegistryRenderModel {
                texture_path: self.texture_path.clone(),
                face_texture_paths: self.face_texture_paths.clone(),
                model_elements: self.model_elements.clone(),
                render_fidelity: None,
                render_source: None,
            };
        }

        let mut model_elements = Vec::new();
        for variant in &selected {
            let rotation = Some(ModelRotationDto {
                x: variant.x,
                y: variant.y,
                uv_lock: variant.uv_lock,
            });
            model_elements.extend(variant.model_elements.iter().cloned().map(|mut element| {
                element.model_rotation = rotation.clone();
                element
            }));
        }

        RegistryRenderModel {
            texture_path: selected
                .first()
                .and_then(|variant| variant.texture_path.clone())
                .or_else(|| self.texture_path.clone()),
            face_texture_paths: selected
                .first()
                .and_then(|variant| variant.face_texture_paths.clone())
                .or_else(|| self.face_texture_paths.clone()),
            model_elements,
            render_fidelity: Some("staticModel".to_string()),
            render_source: selected
                .first()
                .and_then(|variant| variant.model.clone())
                .or_else(|| self.texture_path.clone()),
        }
    }

    fn runtime_render_model_for_block(
        &self,
        block: &SchemeBlock,
        preference: RuntimeRenderAssetPreference,
    ) -> Option<RegistryRenderModel> {
        if self.render_assets.is_empty() {
            return None;
        }
        let matching = self
            .render_assets
            .iter()
            .filter(|asset| preference.accepts(&asset.fidelity))
            .filter(|asset| variant_matches(asset.condition.as_ref(), &block.states))
            .collect::<Vec<_>>();
        let selected = if self.model_variants_are_multipart {
            matching
        } else {
            matching.into_iter().take(1).collect()
        };
        if selected.is_empty() {
            return None;
        }

        let model_elements = selected
            .iter()
            .flat_map(|asset| asset.model_elements.iter().cloned())
            .collect::<Vec<_>>();
        if model_elements.is_empty() {
            return None;
        }

        Some(RegistryRenderModel {
            texture_path: selected
                .first()
                .and_then(|asset| asset.texture_path.clone())
                .or_else(|| self.texture_path.clone()),
            face_texture_paths: selected
                .first()
                .and_then(|asset| asset.face_texture_paths.clone())
                .or_else(|| self.face_texture_paths.clone()),
            model_elements,
            render_fidelity: selected.first().map(|asset| asset.fidelity.clone()),
            render_source: selected.first().map(|asset| asset.source.clone()),
        })
    }
}

impl RegistryRenderModel {
    fn has_render_payload(&self) -> bool {
        self.texture_path.is_some()
            || self.face_texture_paths.is_some()
            || !self.model_elements.is_empty()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RuntimeRenderAssetPreference {
    Authoritative,
    Any,
}

impl RuntimeRenderAssetPreference {
    fn accepts(self, fidelity: &str) -> bool {
        match self {
            Self::Any => true,
            Self::Authoritative => !matches!(
                fidelity,
                "approximation" | "unsupportedDynamic" | "unsupported"
            ),
        }
    }
}

fn variant_matches(
    condition: Option<&RegistryModelCondition>,
    states: &BTreeMap<String, String>,
) -> bool {
    let Some(condition) = condition else {
        return true;
    };
    condition.any_of.iter().any(|expected_states| {
        expected_states.iter().all(|(name, allowed_values)| {
            states
                .get(name)
                .is_some_and(|value| allowed_values.iter().any(|allowed| allowed == value))
        })
    })
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
                            model_variants_are_multipart: block
                                .get("modelVariantsAreMultipart")
                                .and_then(Value::as_bool)
                                .unwrap_or(false),
                            model_variants: block
                                .get("modelVariants")
                                .and_then(Value::as_array)
                                .map(|variants| {
                                    variants.iter().filter_map(registry_model_variant).collect()
                                })
                                .unwrap_or_default(),
                            render_assets: block
                                .get("renderAssets")
                                .and_then(Value::as_array)
                                .map(|assets| {
                                    assets.iter().filter_map(registry_render_asset).collect()
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
        rotation: value
            .get("rotation")
            .and_then(registry_model_element_rotation),
        model_rotation: value.get("modelRotation").and_then(registry_model_rotation),
        face_texture_paths: registry_face_texture_paths(value.get("faceTexturePaths")?),
        face_uvs: value
            .get("faceUvs")
            .map(registry_face_uvs)
            .unwrap_or_else(empty_face_uvs),
    })
}

fn registry_model_variant(value: &Value) -> Option<RegistryModelVariant> {
    Some(RegistryModelVariant {
        condition: value.get("condition").and_then(registry_model_condition),
        model: value
            .get("model")
            .and_then(Value::as_str)
            .map(ToString::to_string),
        x: value.get("x").and_then(Value::as_f64).unwrap_or(0.0) as f32,
        y: value.get("y").and_then(Value::as_f64).unwrap_or(0.0) as f32,
        uv_lock: value
            .get("uvLock")
            .and_then(Value::as_bool)
            .unwrap_or(false),
        texture_path: value
            .get("texturePath")
            .and_then(Value::as_str)
            .map(ToString::to_string),
        face_texture_paths: value
            .get("faceTexturePaths")
            .map(registry_face_texture_paths),
        model_elements: value
            .get("modelElements")
            .and_then(Value::as_array)
            .map(|elements| elements.iter().filter_map(registry_model_element).collect())
            .unwrap_or_default(),
    })
}

fn registry_render_asset(value: &Value) -> Option<RegistryRenderAsset> {
    let model_elements: Vec<ModelElementDto> = value
        .get("elements")
        .and_then(Value::as_array)
        .map(|elements| elements.iter().filter_map(registry_model_element).collect())
        .unwrap_or_default();
    let face_texture_paths = model_elements
        .iter()
        .find_map(|element| any_face_texture_paths(&element.face_texture_paths));
    let texture_path = face_texture_paths
        .as_ref()
        .and_then(first_face_texture_path);
    Some(RegistryRenderAsset {
        condition: value.get("condition").and_then(registry_model_condition),
        fidelity: value
            .get("fidelity")
            .and_then(Value::as_str)
            .unwrap_or("runtimeBaked")
            .to_string(),
        source: value
            .get("source")
            .and_then(Value::as_str)
            .unwrap_or("minecraft-runtime")
            .to_string(),
        texture_path,
        face_texture_paths,
        model_elements,
    })
}

fn any_face_texture_paths(paths: &FaceTexturePathsDto) -> Option<FaceTexturePathsDto> {
    [
        &paths.north,
        &paths.south,
        &paths.east,
        &paths.west,
        &paths.up,
        &paths.down,
    ]
    .iter()
    .any(|path| path.is_some())
    .then(|| paths.clone())
}

fn first_face_texture_path(paths: &FaceTexturePathsDto) -> Option<String> {
    [
        &paths.north,
        &paths.south,
        &paths.east,
        &paths.west,
        &paths.up,
        &paths.down,
    ]
    .into_iter()
    .find_map(Clone::clone)
}

fn registry_model_condition(value: &Value) -> Option<RegistryModelCondition> {
    let any_of = value
        .get("anyOf")?
        .as_array()?
        .iter()
        .filter_map(|condition| {
            let object = condition.as_object()?;
            Some(
                object
                    .iter()
                    .filter_map(|(name, values)| {
                        let allowed_values = values
                            .as_array()?
                            .iter()
                            .filter_map(Value::as_str)
                            .map(ToString::to_string)
                            .collect::<Vec<_>>();
                        (!allowed_values.is_empty()).then_some((name.clone(), allowed_values))
                    })
                    .collect::<BTreeMap<_, _>>(),
            )
        })
        .filter(|states| !states.is_empty())
        .collect::<Vec<_>>();
    (!any_of.is_empty()).then_some(RegistryModelCondition { any_of })
}

fn registry_model_rotation(value: &Value) -> Option<ModelRotationDto> {
    Some(ModelRotationDto {
        x: value.get("x").and_then(Value::as_f64).unwrap_or(0.0) as f32,
        y: value.get("y").and_then(Value::as_f64).unwrap_or(0.0) as f32,
        uv_lock: value
            .get("uvLock")
            .and_then(Value::as_bool)
            .unwrap_or(false),
    })
}

fn registry_face_uvs(value: &Value) -> FaceUvsDto {
    FaceUvsDto {
        north: value.get("north").and_then(registry_model_face_uv),
        south: value.get("south").and_then(registry_model_face_uv),
        east: value.get("east").and_then(registry_model_face_uv),
        west: value.get("west").and_then(registry_model_face_uv),
        up: value.get("up").and_then(registry_model_face_uv),
        down: value.get("down").and_then(registry_model_face_uv),
    }
}

fn empty_face_uvs() -> FaceUvsDto {
    FaceUvsDto {
        north: None,
        south: None,
        east: None,
        west: None,
        up: None,
        down: None,
    }
}

fn registry_model_face_uv(value: &Value) -> Option<[f32; 4]> {
    let array = value.as_array()?;
    let [u0, v0, u1, v1] = array.as_slice() else {
        return None;
    };
    Some([
        u0.as_f64()? as f32,
        v0.as_f64()? as f32,
        u1.as_f64()? as f32,
        v1.as_f64()? as f32,
    ])
}

fn registry_model_element_rotation(value: &Value) -> Option<ModelElementRotationDto> {
    let axis = value.get("axis")?.as_str()?;
    if !matches!(axis, "x" | "y" | "z") {
        return None;
    }
    Some(ModelElementRotationDto {
        origin: registry_model_vector(value.get("origin")?)?,
        axis: axis.to_string(),
        angle: value.get("angle")?.as_f64()? as f32,
        rescale: value
            .get("rescale")
            .and_then(Value::as_bool)
            .unwrap_or(false),
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
