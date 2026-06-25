use std::collections::BTreeMap;
use std::fs::{self, File};
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::{AssetError, CancellationToken};

use crate::blockstate::BlockstateModelCondition;

mod registry_file;
mod runtime;
mod static_assets;
use registry_file::PrismAssetIndexRegistryFile;
use runtime::runtime_stack_metadata;
pub use runtime::BakedRenderAssetSample;
use static_assets::{
    collect_archives, scan_asset_entries_in_dir, texture_atlas_metadata, AssetCollector,
};

pub const PRISM_REGISTRY_SCHEMA_VERSION: u32 = 6;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrismAssetIndexRequest {
    pub instance_id: String,
    pub identity_fingerprint: String,
    pub content_fingerprint: String,
    pub instance_path: PathBuf,
    pub minecraft_dir: PathBuf,
    pub diagnostics_dir: PathBuf,
    pub minecraft_version: Option<String>,
    pub loader: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PrismAssetIndexReport {
    pub schema_version: u32,
    pub status: String,
    pub static_status: String,
    pub runtime_status: String,
    pub runtime_message: Option<String>,
    pub instance_id: String,
    pub identity_fingerprint: String,
    pub content_fingerprint: String,
    pub minecraft_version: Option<String>,
    pub loader: Option<String>,
    pub archive_count: usize,
    pub block_count: usize,
    pub asset_count: usize,
    pub report_path: PathBuf,
    pub blocks: Vec<BlockAssetSample>,
    pub texture_atlas: TextureAtlasMetadata,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PrismAssetIndexMetadata {
    pub schema_version: u32,
    pub status: String,
    pub static_status: String,
    pub runtime_status: String,
    pub runtime_message: Option<String>,
    pub instance_id: String,
    pub identity_fingerprint: String,
    pub content_fingerprint: String,
    pub minecraft_version: Option<String>,
    pub loader: Option<String>,
    pub archive_count: usize,
    pub block_count: usize,
    pub asset_count: usize,
    pub report_path: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssetIndexEvent {
    pub message: String,
    pub progress: Option<AssetIndexProgress>,
}

impl AssetIndexEvent {
    fn message(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            progress: None,
        }
    }

    fn progress(message: impl Into<String>, completed: usize, total: usize) -> Self {
        Self {
            message: message.into(),
            progress: Some(AssetIndexProgress {
                completed: completed as u64,
                total: total as u64,
            }),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssetIndexProgress {
    pub completed: u64,
    pub total: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BlockAssetSample {
    pub identifier: String,
    pub item_id: Option<String>,
    pub max_stack_size: Option<u32>,
    pub display_name: String,
    pub namespace: String,
    pub allowed_states: Vec<BlockStatePropertySample>,
    pub model: Option<String>,
    pub texture_path: Option<PathBuf>,
    pub face_texture_paths: Option<FaceTexturePaths>,
    pub model_elements: Vec<ModelElementSample>,
    pub model_variants_are_multipart: bool,
    pub model_variants: Vec<BlockModelVariantSample>,
    #[serde(default)]
    pub render_assets: Vec<BakedRenderAssetSample>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BlockStatePropertySample {
    pub name: String,
    pub values: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BlockModelVariantSample {
    pub condition: Option<BlockstateModelCondition>,
    pub model: Option<String>,
    pub x: Option<f32>,
    pub y: Option<f32>,
    pub uv_lock: bool,
    pub texture_path: Option<PathBuf>,
    pub face_texture_paths: Option<FaceTexturePaths>,
    pub model_elements: Vec<ModelElementSample>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FaceTexturePaths {
    pub north: Option<PathBuf>,
    pub south: Option<PathBuf>,
    pub east: Option<PathBuf>,
    pub west: Option<PathBuf>,
    pub up: Option<PathBuf>,
    pub down: Option<PathBuf>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelElementSample {
    pub from: [f32; 3],
    pub to: [f32; 3],
    #[serde(default)]
    pub rotation: Option<ModelElementRotationSample>,
    pub face_texture_paths: FaceTexturePaths,
    #[serde(default)]
    pub face_uvs: FaceUvs,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelElementRotationSample {
    pub origin: [f32; 3],
    pub axis: String,
    pub angle: f32,
    pub rescale: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FaceUvs {
    pub north: Option<[f32; 4]>,
    pub south: Option<[f32; 4]>,
    pub east: Option<[f32; 4]>,
    pub west: Option<[f32; 4]>,
    pub up: Option<[f32; 4]>,
    pub down: Option<[f32; 4]>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TextureAtlasMetadata {
    pub textures: Vec<TextureAtlasEntry>,
    pub tile_size: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TextureAtlasEntry {
    pub identifier: String,
    pub source_path: PathBuf,
    pub tile_index: usize,
}

pub fn build_prism_asset_index(
    request: PrismAssetIndexRequest,
) -> Result<PrismAssetIndexReport, AssetError> {
    build_prism_asset_index_with_events(request, &CancellationToken::new(), |_| {})
}

pub fn build_prism_asset_index_with_events(
    request: PrismAssetIndexRequest,
    cancellation: &CancellationToken,
    mut on_event: impl FnMut(AssetIndexEvent),
) -> Result<PrismAssetIndexReport, AssetError> {
    ensure_not_cancelled(cancellation)?;
    on_event(AssetIndexEvent::message("Preparing Prism asset index..."));
    fs::create_dir_all(&request.diagnostics_dir)?;

    let texture_cache_dir = request.diagnostics_dir.join(format!(
        "{}-{}-textures",
        safe_path_segment(&request.identity_fingerprint),
        safe_path_segment(&request.content_fingerprint)
    ));
    if texture_cache_dir.exists() {
        fs::remove_dir_all(&texture_cache_dir)?;
    }
    fs::create_dir_all(&texture_cache_dir)?;

    let mut collector = AssetCollector::new(texture_cache_dir);
    let mut archive_count = 0;

    if let Some(vanilla_assets) = vanilla_client_jar(&request) {
        if vanilla_assets.is_file() {
            archive_count += 1;
            on_event(AssetIndexEvent::message(format!(
                "Scanning vanilla Minecraft assets: {}",
                vanilla_assets.display()
            )));
            collector.scan_archive(&vanilla_assets, cancellation)?;
        }
    }

    let resourcepacks_dir = request.minecraft_dir.join("resourcepacks");
    if resourcepacks_dir.is_dir() {
        on_event(AssetIndexEvent::message("Scanning Prism resource packs..."));
        scan_asset_entries_in_dir(&mut collector, &resourcepacks_dir, cancellation)?;
        for archive in collect_archives(&resourcepacks_dir, cancellation)? {
            archive_count += 1;
            let message = format!("Scanning resource pack {}...", archive.display());
            on_event(AssetIndexEvent::message(message));
            collector.scan_archive(&archive, cancellation)?;
        }
    }

    let mods_dir = request.minecraft_dir.join("mods");
    let archives = collect_archives(&mods_dir, cancellation)?;
    let total = archives.len();
    for (index, archive) in archives.iter().enumerate() {
        ensure_not_cancelled(cancellation)?;
        archive_count += 1;
        let position = index + 1;
        on_event(AssetIndexEvent::message(format!(
            "Scanning mod archive {position}/{total}: {}",
            archive.display()
        )));
        match collector.scan_archive(archive, cancellation) {
            Ok(()) => on_event(AssetIndexEvent::progress(
                format!("Indexed mod archive {position}/{total}."),
                position,
                total,
            )),
            Err(error) => {
                let warning = format!("Skipped archive {}: {error}", archive.display());
                collector.warnings.push(warning.clone());
                on_event(AssetIndexEvent::progress(warning, position, total));
            }
        }
    }

    ensure_not_cancelled(cancellation)?;
    on_event(AssetIndexEvent::message(
        "Building Prism block registry from collected assets...",
    ));
    let runtime = runtime_stack_metadata(&request, cancellation, &mut on_event);
    let mut blocks = collector.block_samples();
    apply_runtime_stack_sizes(&mut blocks, &runtime.items);
    apply_runtime_render_assets(&mut blocks, &runtime.render_assets);
    if blocks.is_empty() {
        return Err(AssetError::NoParseableBlocks);
    }
    let texture_atlas = texture_atlas_metadata(&blocks);
    let report_path =
        prism_registry_report_path(&request.diagnostics_dir, &request.identity_fingerprint);
    let report = PrismAssetIndexReport {
        schema_version: PRISM_REGISTRY_SCHEMA_VERSION,
        status: "ready".to_string(),
        static_status: "ready".to_string(),
        runtime_status: runtime.status,
        runtime_message: runtime.message,
        instance_id: request.instance_id,
        identity_fingerprint: request.identity_fingerprint,
        content_fingerprint: request.content_fingerprint,
        minecraft_version: request.minecraft_version,
        loader: request.loader,
        archive_count,
        block_count: blocks.len(),
        asset_count: collector.asset_paths_seen.len(),
        report_path: report_path.clone(),
        blocks,
        texture_atlas,
        warnings: collector.warnings,
    };
    let registry_file = PrismAssetIndexRegistryFile::from(&report);
    write_json_file(&report_path, &registry_file)?;
    let metadata = PrismAssetIndexMetadata::from(&report);
    write_json_file(
        prism_registry_metadata_path(&request.diagnostics_dir, &report.identity_fingerprint),
        &metadata,
    )?;
    on_event(AssetIndexEvent::message(format!(
        "Prism block registry written: {}",
        report_path.display()
    )));
    Ok(report)
}

fn write_json_file<T: Serialize>(path: impl AsRef<Path>, value: &T) -> Result<(), AssetError> {
    let file = File::create(path)?;
    let mut writer = BufWriter::new(file);
    serde_json::to_writer(&mut writer, value)
        .map_err(|error| AssetError::InvalidAssetIndex(error.to_string()))?;
    writer.flush()?;
    Ok(())
}

impl From<&PrismAssetIndexReport> for PrismAssetIndexMetadata {
    fn from(report: &PrismAssetIndexReport) -> Self {
        Self {
            schema_version: report.schema_version,
            status: report.status.clone(),
            static_status: report.static_status.clone(),
            runtime_status: report.runtime_status.clone(),
            runtime_message: report.runtime_message.clone(),
            instance_id: report.instance_id.clone(),
            identity_fingerprint: report.identity_fingerprint.clone(),
            content_fingerprint: report.content_fingerprint.clone(),
            minecraft_version: report.minecraft_version.clone(),
            loader: report.loader.clone(),
            archive_count: report.archive_count,
            block_count: report.block_count,
            asset_count: report.asset_count,
            report_path: report.report_path.clone(),
        }
    }
}

pub fn prism_registry_report_path(diagnostics_dir: &Path, identity_fingerprint: &str) -> PathBuf {
    diagnostics_dir.join(format!(
        "{}-registry.json",
        safe_path_segment(identity_fingerprint)
    ))
}

pub fn prism_registry_metadata_path(diagnostics_dir: &Path, identity_fingerprint: &str) -> PathBuf {
    diagnostics_dir.join(format!(
        "{}-registry-meta.json",
        safe_path_segment(identity_fingerprint)
    ))
}

fn apply_runtime_stack_sizes(blocks: &mut [BlockAssetSample], items: &BTreeMap<String, u32>) {
    for block in blocks {
        if let Some(stack_size) = block
            .item_id
            .as_ref()
            .and_then(|item_id| items.get(item_id))
            .copied()
        {
            block.max_stack_size = Some(stack_size);
        }
    }
}

fn apply_runtime_render_assets(
    blocks: &mut [BlockAssetSample],
    render_assets: &BTreeMap<String, Vec<BakedRenderAssetSample>>,
) {
    for block in blocks {
        if let Some(assets) = render_assets.get(&block.identifier) {
            block.render_assets = assets.clone();
        }
    }
}

fn ensure_not_cancelled(cancellation: &CancellationToken) -> Result<(), AssetError> {
    if cancellation.is_cancelled() {
        return Err(AssetError::Cancelled);
    }
    Ok(())
}

fn vanilla_client_jar(request: &PrismAssetIndexRequest) -> Option<PathBuf> {
    let minecraft_version = request.minecraft_version.as_deref()?;
    let root = prism_root_from_instance_path(&request.instance_path)?;
    Some(root.join(format!(
        "libraries/com/mojang/minecraft/{minecraft_version}/minecraft-{minecraft_version}-client.jar"
    )))
}

fn prism_root_from_instance_path(instance_path: &Path) -> Option<PathBuf> {
    let instances_dir = instance_path.parent()?;
    if instances_dir.file_name()?.to_string_lossy() != "instances" {
        return None;
    }
    instances_dir.parent().map(Path::to_path_buf)
}

fn safe_path_segment(value: &str) -> String {
    let cleaned = value
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || matches!(character, '.' | '-' | '_') {
                character
            } else {
                '-'
            }
        })
        .collect::<String>();
    if cleaned.is_empty() {
        "prism-instance".to_string()
    } else {
        cleaned
    }
}
