use std::path::PathBuf;

use serde::Serialize;

use crate::blockstate::BlockstateModelCondition;

use super::{
    BakedRenderAssetSample, BlockAssetSample, BlockModelVariantSample, BlockStatePropertySample,
    FaceTexturePaths, PrismAssetIndexReport, TextureAtlasMetadata,
};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PrismAssetIndexRegistryFile<'a> {
    schema_version: u32,
    status: &'a str,
    static_status: &'a str,
    runtime_status: &'a str,
    runtime_message: Option<&'a str>,
    instance_id: &'a str,
    identity_fingerprint: &'a str,
    content_fingerprint: &'a str,
    minecraft_version: Option<&'a str>,
    loader: Option<&'a str>,
    archive_count: usize,
    block_count: usize,
    asset_count: usize,
    report_path: &'a PathBuf,
    blocks: Vec<RegistryBlockFile<'a>>,
    texture_atlas: &'a TextureAtlasMetadata,
    warnings: &'a [String],
}

impl<'a> From<&'a PrismAssetIndexReport> for PrismAssetIndexRegistryFile<'a> {
    fn from(report: &'a PrismAssetIndexReport) -> Self {
        Self {
            schema_version: report.schema_version,
            status: &report.status,
            static_status: &report.static_status,
            runtime_status: &report.runtime_status,
            runtime_message: report.runtime_message.as_deref(),
            instance_id: &report.instance_id,
            identity_fingerprint: &report.identity_fingerprint,
            content_fingerprint: &report.content_fingerprint,
            minecraft_version: report.minecraft_version.as_deref(),
            loader: report.loader.as_deref(),
            archive_count: report.archive_count,
            block_count: report.block_count,
            asset_count: report.asset_count,
            report_path: &report.report_path,
            blocks: report.blocks.iter().map(RegistryBlockFile::from).collect(),
            texture_atlas: &report.texture_atlas,
            warnings: &report.warnings,
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct RegistryBlockFile<'a> {
    identifier: &'a str,
    item_id: Option<&'a str>,
    max_stack_size: Option<u32>,
    display_name: &'a str,
    namespace: &'a str,
    allowed_states: &'a [BlockStatePropertySample],
    model: Option<&'a str>,
    texture_path: Option<&'a PathBuf>,
    face_texture_paths: Option<&'a FaceTexturePaths>,
    model_variants_are_multipart: bool,
    model_variants: Vec<RegistryBlockModelVariantFile<'a>>,
    render_assets: &'a [BakedRenderAssetSample],
}

impl<'a> From<&'a BlockAssetSample> for RegistryBlockFile<'a> {
    fn from(block: &'a BlockAssetSample) -> Self {
        Self {
            identifier: &block.identifier,
            item_id: block.item_id.as_deref(),
            max_stack_size: block.max_stack_size,
            display_name: &block.display_name,
            namespace: &block.namespace,
            allowed_states: &block.allowed_states,
            model: block.model.as_deref(),
            texture_path: block.texture_path.as_ref(),
            face_texture_paths: block.face_texture_paths.as_ref(),
            model_variants_are_multipart: block.model_variants_are_multipart,
            model_variants: block
                .model_variants
                .iter()
                .map(RegistryBlockModelVariantFile::from)
                .collect(),
            render_assets: &block.render_assets,
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct RegistryBlockModelVariantFile<'a> {
    condition: Option<&'a BlockstateModelCondition>,
    model: Option<&'a str>,
    x: Option<f32>,
    y: Option<f32>,
    uv_lock: bool,
    texture_path: Option<&'a PathBuf>,
    face_texture_paths: Option<&'a FaceTexturePaths>,
}

impl<'a> From<&'a BlockModelVariantSample> for RegistryBlockModelVariantFile<'a> {
    fn from(variant: &'a BlockModelVariantSample) -> Self {
        Self {
            condition: variant.condition.as_ref(),
            model: variant.model.as_deref(),
            x: variant.x,
            y: variant.y,
            uv_lock: variant.uv_lock,
            texture_path: variant.texture_path.as_ref(),
            face_texture_paths: variant.face_texture_paths.as_ref(),
        }
    }
}
