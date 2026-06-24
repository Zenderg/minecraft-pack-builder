use std::collections::{BTreeMap, BTreeSet};
use std::fs::{self, File};
use std::io::{Cursor, Read};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};

use crate::{AssetError, CancellationToken};

const PRISM_REGISTRY_SCHEMA_VERSION: u32 = 4;

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
    pub model: Option<String>,
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
    pub face_texture_paths: FaceTexturePaths,
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
    if blocks.is_empty() {
        return Err(AssetError::NoParseableBlocks);
    }
    let texture_atlas = texture_atlas_metadata(&blocks);
    let report_path = request.diagnostics_dir.join(format!(
        "{}-registry.json",
        safe_path_segment(&request.identity_fingerprint)
    ));
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
    let json = serde_json::to_string_pretty(&report)
        .map_err(|error| AssetError::InvalidAssetIndex(error.to_string()))?;
    fs::write(&report_path, json)?;
    on_event(AssetIndexEvent::message(format!(
        "Prism block registry written: {}",
        report_path.display()
    )));
    Ok(report)
}

#[derive(Debug, Clone)]
struct RuntimeStackOutcome {
    status: String,
    message: Option<String>,
    items: BTreeMap<String, u32>,
}

impl RuntimeStackOutcome {
    fn ready(items: BTreeMap<String, u32>) -> Self {
        Self {
            status: "ready".to_string(),
            message: None,
            items,
        }
    }

    fn unavailable(message: impl Into<String>) -> Self {
        Self {
            status: "unavailable".to_string(),
            message: Some(message.into()),
            items: BTreeMap::new(),
        }
    }
}

fn runtime_stack_metadata(
    request: &PrismAssetIndexRequest,
    cancellation: &CancellationToken,
    on_event: &mut impl FnMut(AssetIndexEvent),
) -> RuntimeStackOutcome {
    let runtime_report_path = runtime_report_path(request);
    if runtime_report_path.is_file() {
        return match read_runtime_stack_report(&runtime_report_path) {
            Ok(items) => RuntimeStackOutcome::ready(items),
            Err(error) => RuntimeStackOutcome::unavailable(format!(
                "Cached runtime stack report could not be parsed: {error}"
            )),
        };
    }

    match run_runtime_stack_extractor(request, &runtime_report_path, cancellation, on_event) {
        Ok(items) => RuntimeStackOutcome::ready(items),
        Err(message) => RuntimeStackOutcome::unavailable(message),
    }
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

fn runtime_report_path(request: &PrismAssetIndexRequest) -> PathBuf {
    request.diagnostics_dir.join(format!(
        "{}-{}-runtime.json",
        safe_path_segment(&request.identity_fingerprint),
        safe_path_segment(&request.content_fingerprint)
    ))
}

fn read_runtime_stack_report(path: &Path) -> Result<BTreeMap<String, u32>, AssetError> {
    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct RuntimeReport {
        items: Vec<RuntimeItem>,
    }

    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct RuntimeItem {
        item_id: String,
        max_stack_size: u32,
    }

    let text = fs::read_to_string(path)?;
    let report: RuntimeReport = serde_json::from_str(&text)
        .map_err(|error| AssetError::InvalidAssetIndex(error.to_string()))?;
    Ok(report
        .items
        .into_iter()
        .map(|item| (item.item_id, item.max_stack_size))
        .collect())
}

fn run_runtime_stack_extractor(
    request: &PrismAssetIndexRequest,
    runtime_report_path: &Path,
    cancellation: &CancellationToken,
    on_event: &mut impl FnMut(AssetIndexEvent),
) -> Result<BTreeMap<String, u32>, String> {
    ensure_not_cancelled(cancellation).map_err(|error| error.to_string())?;
    let loader = RuntimeLoader::from_request(request)?;
    match loader {
        RuntimeLoader::NeoForge => run_forge_like_runtime_stack_extractor(
            request,
            runtime_report_path,
            cancellation,
            on_event,
            ForgeLikeRuntime::NeoForge,
        ),
        RuntimeLoader::Forge => run_forge_like_runtime_stack_extractor(
            request,
            runtime_report_path,
            cancellation,
            on_event,
            ForgeLikeRuntime::Forge,
        ),
        RuntimeLoader::Fabric => {
            run_fabric_runtime_stack_extractor(request, runtime_report_path, cancellation, on_event)
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RuntimeLoader {
    NeoForge,
    Forge,
    Fabric,
}

impl RuntimeLoader {
    fn from_request(request: &PrismAssetIndexRequest) -> Result<Self, String> {
        let loader = request.loader.as_deref().ok_or_else(|| {
            "Runtime stack extraction requires a supported loader: Forge, NeoForge, or Fabric."
                .to_string()
        })?;
        let normalized = loader.to_ascii_lowercase();
        if normalized.contains("neoforge") {
            return Ok(Self::NeoForge);
        }
        if normalized.contains("forge") {
            return Ok(Self::Forge);
        }
        if normalized.contains("fabric") {
            return Ok(Self::Fabric);
        }
        Err(format!(
            "Runtime stack extraction is not available for {loader} instances yet."
        ))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ForgeLikeRuntime {
    NeoForge,
    Forge,
}

impl ForgeLikeRuntime {
    fn display_name(self) -> &'static str {
        match self {
            Self::NeoForge => "NeoForge",
            Self::Forge => "Forge",
        }
    }

    fn extractor_jar_bytes(self) -> Vec<u8> {
        match self {
            Self::NeoForge => neoforge_runtime_extractor_jar_bytes(),
            Self::Forge => forge_runtime_extractor_jar_bytes(),
        }
    }
}

fn run_forge_like_runtime_stack_extractor(
    request: &PrismAssetIndexRequest,
    runtime_report_path: &Path,
    cancellation: &CancellationToken,
    on_event: &mut impl FnMut(AssetIndexEvent),
    runtime: ForgeLikeRuntime,
) -> Result<BTreeMap<String, u32>, String> {
    let minecraft_version = request.minecraft_version.as_deref().ok_or_else(|| {
        "Runtime stack extraction requires a detected Minecraft version.".to_string()
    })?;
    let prism_root = prism_root_from_instance_path(&request.instance_path).ok_or_else(|| {
        "Runtime stack extraction requires a Prism instances/<name> path.".to_string()
    })?;
    let libraries_dir = prism_root.join("libraries");
    let assets_dir = prism_root.join("assets");
    let forge_runtime = match runtime {
        ForgeLikeRuntime::NeoForge => {
            ForgeRuntimeLaunch::neoforge(&libraries_dir, minecraft_version)?
        }
        ForgeLikeRuntime::Forge => {
            ForgeRuntimeLaunch::forge(&prism_root, &libraries_dir, minecraft_version)?
        }
    };
    let mojmap_path = forge_runtime.mojmap_path(&libraries_dir, minecraft_version);
    if !mojmap_path.is_file() {
        return Err(format!(
            "Runtime stack extraction needs Prism/{} generated artifacts that are not present yet. Launch this instance once in PrismLauncher so {} prepares {}, then let Minecraft Pack Builder sync again.",
            runtime.display_name(),
            runtime.display_name(),
            mojmap_path.display()
        ));
    }

    on_event(AssetIndexEvent::message(format!(
        "Running {} runtime stack extractor in an app-owned temporary instance...",
        runtime.display_name()
    )));
    let runtime_dir = prepare_runtime_work_dir(request)?;
    let runtime_libraries = runtime_dir.join("libraries");
    copy_dir_recursive(&libraries_dir, &runtime_libraries).map_err(|error| error.to_string())?;

    let (game_dir, mods_dir) = prepare_runtime_game_dir(request, &runtime_dir)?;
    fs::write(
        mods_dir.join("mpb-runtime-extractor.jar"),
        runtime.extractor_jar_bytes(),
    )
    .map_err(|error| error.to_string())?;

    let wrapper = runtime_libraries.join(
        "io/github/zekerzhayard/ForgeWrapper/prism-2025-12-07/ForgeWrapper-prism-2025-12-07.jar",
    );
    let installer = forge_runtime.installer_path(&runtime_libraries);
    let minecraft_jar = runtime_libraries.join(format!(
        "com/mojang/minecraft/{minecraft_version}/minecraft-{minecraft_version}-client.jar"
    ));
    for required in [&wrapper, &installer, &minecraft_jar] {
        if !required.is_file() {
            return Err(format!(
                "Runtime stack extraction is missing required Prism library: {}",
                required.display()
            ));
        }
    }
    if let Some(parent) = runtime_report_path.parent() {
        fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }

    let child = Command::new("java")
        .current_dir(&game_dir)
        .env("LANG", "en_US.UTF-8")
        .env("LC_ALL", "en_US.UTF-8")
        .arg(format!(
            "-Dforgewrapper.librariesDir={}",
            runtime_libraries.display()
        ))
        .arg(format!("-Dforgewrapper.installer={}", installer.display()))
        .arg(format!(
            "-Dforgewrapper.minecraft={}",
            minecraft_jar.display()
        ))
        .arg(format!(
            "-Dmpb.runtimeOutput={}",
            runtime_report_path.display()
        ))
        .arg("-cp")
        .arg(&wrapper)
        .arg("io.github.zekerzhayard.forgewrapper.installer.Main")
        .args(["--username", "MPB", "--version", "MPB", "--gameDir"])
        .arg(&game_dir)
        .args(["--assetsDir"])
        .arg(&assets_dir)
        .args([
            "--assetIndex",
            "17",
            "--uuid",
            "00000000-0000-0000-0000-000000000000",
            "--accessToken",
            "0",
            "--userType",
            "msa",
            "--versionType",
            "release",
        ])
        .args(forge_runtime.fml_args(minecraft_version))
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| format!("Could not start Java runtime extractor: {error}"))?;

    wait_for_runtime_stack_report(child, runtime_report_path, cancellation)
}

#[derive(Debug, Clone)]
struct ForgeRuntimeLaunch {
    loader_version: String,
    installer_version: String,
    installer_group_path: &'static str,
    installer_artifact: &'static str,
    generated_mappings_version: String,
    extra_fml_args: Vec<String>,
}

impl ForgeRuntimeLaunch {
    fn neoforge(libraries_dir: &Path, minecraft_version: &str) -> Result<Self, String> {
        let loader_version = find_neoforge_version(libraries_dir).ok_or_else(|| {
            "NeoForge libraries were not found in the Prism Launcher Root.".to_string()
        })?;
        let neoform_version =
            find_neoform_version(libraries_dir, minecraft_version).ok_or_else(|| {
                "NeoForm metadata was not found in the Prism Launcher Root.".to_string()
            })?;
        Ok(Self {
            loader_version: loader_version.clone(),
            installer_version: loader_version,
            installer_group_path: "net/neoforged/neoforge",
            installer_artifact: "neoforge",
            generated_mappings_version: neoform_version.clone(),
            extra_fml_args: vec![
                "--fml.neoForgeVersion".to_string(),
                "${loaderVersion}".to_string(),
                "--fml.fmlVersion".to_string(),
                "4.0.42".to_string(),
                "--fml.mcVersion".to_string(),
                "${minecraftVersion}".to_string(),
                "--fml.neoFormVersion".to_string(),
                neoform_version,
                "--launchTarget".to_string(),
                "forgeserver".to_string(),
                "nogui".to_string(),
            ],
        })
    }

    fn forge(
        prism_root: &Path,
        libraries_dir: &Path,
        minecraft_version: &str,
    ) -> Result<Self, String> {
        let loader_version =
            find_forge_version(libraries_dir, minecraft_version).ok_or_else(|| {
                "Forge libraries were not found in the Prism Launcher Root.".to_string()
            })?;
        let mcp_version = find_forge_mcp_version(prism_root, &loader_version).ok_or_else(|| {
            "Forge MCP metadata was not found in the Prism Launcher Root.".to_string()
        })?;
        let installer_version = format!("{minecraft_version}-{loader_version}");
        Ok(Self {
            loader_version,
            installer_version,
            installer_group_path: "net/minecraftforge/forge",
            installer_artifact: "forge",
            generated_mappings_version: mcp_version.clone(),
            extra_fml_args: vec![
                "--fml.forgeVersion".to_string(),
                "${loaderVersion}".to_string(),
                "--fml.mcVersion".to_string(),
                "${minecraftVersion}".to_string(),
                "--fml.forgeGroup".to_string(),
                "net.minecraftforge".to_string(),
                "--fml.mcpVersion".to_string(),
                mcp_version,
                "--launchTarget".to_string(),
                "forgeserver".to_string(),
                "nogui".to_string(),
            ],
        })
    }

    fn mojmap_path(&self, libraries_dir: &Path, minecraft_version: &str) -> PathBuf {
        libraries_dir.join(format!(
            "net/minecraft/client/{minecraft_version}-{}/client-{minecraft_version}-{}-mappings.txt",
            self.generated_mappings_version, self.generated_mappings_version
        ))
    }

    fn installer_path(&self, runtime_libraries: &Path) -> PathBuf {
        runtime_libraries.join(format!(
            "{}/{}/{}-{}-installer.jar",
            self.installer_group_path,
            self.installer_version,
            self.installer_artifact,
            self.installer_version
        ))
    }

    fn fml_args(&self, minecraft_version: &str) -> Vec<String> {
        self.extra_fml_args
            .iter()
            .map(|arg| {
                arg.replace("${loaderVersion}", &self.loader_version)
                    .replace("${minecraftVersion}", minecraft_version)
            })
            .collect()
    }
}

fn find_neoforge_version(libraries_dir: &Path) -> Option<String> {
    let root = libraries_dir.join("net/neoforged/neoforge");
    fs::read_dir(root)
        .ok()?
        .filter_map(Result::ok)
        .find(|entry| {
            let version = entry.file_name().to_string_lossy().to_string();
            entry
                .path()
                .join(format!("neoforge-{version}-installer.jar"))
                .is_file()
        })
        .map(|entry| entry.file_name().to_string_lossy().to_string())
}

fn find_neoform_version(libraries_dir: &Path, minecraft_version: &str) -> Option<String> {
    let root = libraries_dir.join("net/neoforged/neoform");
    fs::read_dir(root)
        .ok()?
        .filter_map(Result::ok)
        .filter_map(|entry| entry.file_name().to_str().map(ToString::to_string))
        .find_map(|version| {
            let prefix = format!("{minecraft_version}-");
            version.strip_prefix(&prefix).map(ToString::to_string)
        })
}

fn find_forge_version(libraries_dir: &Path, minecraft_version: &str) -> Option<String> {
    let root = libraries_dir.join("net/minecraftforge/forge");
    let prefix = format!("{minecraft_version}-");
    fs::read_dir(root)
        .ok()?
        .filter_map(Result::ok)
        .filter_map(|entry| entry.file_name().to_str().map(ToString::to_string))
        .find_map(|version| {
            let forge_version = version.strip_prefix(&prefix)?.to_string();
            let installer = libraries_dir.join(format!(
                "net/minecraftforge/forge/{version}/forge-{version}-installer.jar"
            ));
            installer.is_file().then_some(forge_version)
        })
}

fn find_forge_mcp_version(prism_root: &Path, forge_version: &str) -> Option<String> {
    let meta_path = prism_root
        .join("meta")
        .join("net.minecraftforge")
        .join(format!("{forge_version}.json"));
    let text = fs::read_to_string(meta_path).ok()?;
    let metadata = serde_json::from_str::<serde_json::Value>(&text).ok()?;
    let arguments = metadata.get("minecraftArguments")?.as_str()?;
    argument_value(arguments, "--fml.mcpVersion").map(ToString::to_string)
}

fn argument_value<'a>(arguments: &'a str, name: &str) -> Option<&'a str> {
    let mut parts = arguments.split_whitespace();
    while let Some(part) = parts.next() {
        if part == name {
            return parts.next();
        }
    }
    None
}

fn run_fabric_runtime_stack_extractor(
    request: &PrismAssetIndexRequest,
    runtime_report_path: &Path,
    cancellation: &CancellationToken,
    on_event: &mut impl FnMut(AssetIndexEvent),
) -> Result<BTreeMap<String, u32>, String> {
    let minecraft_version = request.minecraft_version.as_deref().ok_or_else(|| {
        "Fabric runtime stack extraction requires a detected Minecraft version.".to_string()
    })?;
    let prism_root = prism_root_from_instance_path(&request.instance_path).ok_or_else(|| {
        "Fabric runtime stack extraction requires a Prism instances/<name> path.".to_string()
    })?;
    let libraries_dir = prism_root.join("libraries");
    let fabric_loader_version = find_fabric_loader_version(&libraries_dir).ok_or_else(|| {
        "Fabric runtime stack extraction could not find Fabric Loader libraries in the Prism Launcher Root.".to_string()
    })?;
    let server_jar = find_minecraft_server_jar(&libraries_dir, minecraft_version).ok_or_else(|| {
        format!(
            "Fabric runtime stack extraction needs a local Minecraft server jar for {minecraft_version}. PrismLauncher usually keeps only the client jar, so stack sizes stay unavailable until that server jar exists locally."
        )
    })?;

    on_event(AssetIndexEvent::message(
        "Running Fabric runtime stack extractor in an app-owned temporary instance...",
    ));
    let runtime_dir = prepare_runtime_work_dir(request)?;
    let runtime_libraries = runtime_dir.join("libraries");
    copy_dir_recursive(&libraries_dir, &runtime_libraries).map_err(|error| error.to_string())?;
    let (game_dir, mods_dir) = prepare_runtime_game_dir(request, &runtime_dir)?;
    fs::write(
        mods_dir.join("mpb-runtime-extractor.jar"),
        fabric_runtime_extractor_jar_bytes(),
    )
    .map_err(|error| error.to_string())?;
    let runtime_server_jar = runtime_libraries.join(
        server_jar
            .strip_prefix(&libraries_dir)
            .map_err(|error| error.to_string())?,
    );
    let runtime_loader_jar = runtime_libraries.join(format!(
        "net/fabricmc/fabric-loader/{fabric_loader_version}/fabric-loader-{fabric_loader_version}.jar"
    ));
    for required in [&runtime_server_jar, &runtime_loader_jar] {
        if !required.is_file() {
            return Err(format!(
                "Fabric runtime stack extraction is missing required Prism library: {}",
                required.display()
            ));
        }
    }
    if let Some(parent) = runtime_report_path.parent() {
        fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }

    let mut classpath = collect_jars(&runtime_libraries).map_err(|error| error.to_string())?;
    classpath.push(runtime_server_jar);
    let classpath = std::env::join_paths(classpath)
        .map_err(|error| format!("Could not build Fabric runtime classpath: {error}"))?;
    let child = Command::new("java")
        .current_dir(&game_dir)
        .env("LANG", "en_US.UTF-8")
        .env("LC_ALL", "en_US.UTF-8")
        .arg(format!(
            "-Dmpb.runtimeOutput={}",
            runtime_report_path.display()
        ))
        .arg("-cp")
        .arg(classpath)
        .arg("net.fabricmc.loader.impl.launch.knot.KnotServer")
        .arg("nogui")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| format!("Could not start Fabric runtime extractor: {error}"))?;

    wait_for_runtime_stack_report(child, runtime_report_path, cancellation)
}

fn find_fabric_loader_version(libraries_dir: &Path) -> Option<String> {
    let root = libraries_dir.join("net/fabricmc/fabric-loader");
    fs::read_dir(root)
        .ok()?
        .filter_map(Result::ok)
        .find(|entry| {
            let version = entry.file_name().to_string_lossy().to_string();
            entry
                .path()
                .join(format!("fabric-loader-{version}.jar"))
                .is_file()
        })
        .map(|entry| entry.file_name().to_string_lossy().to_string())
}

fn find_minecraft_server_jar(libraries_dir: &Path, minecraft_version: &str) -> Option<PathBuf> {
    [
        libraries_dir.join(format!(
            "com/mojang/minecraft/{minecraft_version}/minecraft-{minecraft_version}-server.jar"
        )),
        libraries_dir.join(format!(
            "net/minecraft/server/{minecraft_version}/server-{minecraft_version}.jar"
        )),
    ]
    .into_iter()
    .find(|path| path.is_file())
}

fn prepare_runtime_work_dir(request: &PrismAssetIndexRequest) -> Result<PathBuf, String> {
    let runtime_dir = request.diagnostics_dir.join(format!(
        "{}-{}-runtime-work",
        safe_path_segment(&request.identity_fingerprint),
        safe_path_segment(&request.content_fingerprint)
    ));
    if runtime_dir.exists() {
        fs::remove_dir_all(&runtime_dir).map_err(|error| error.to_string())?;
    }
    fs::create_dir_all(&runtime_dir).map_err(|error| error.to_string())?;
    Ok(runtime_dir)
}

fn prepare_runtime_game_dir(
    request: &PrismAssetIndexRequest,
    runtime_dir: &Path,
) -> Result<(PathBuf, PathBuf), String> {
    let game_dir = runtime_dir.join("game");
    let mods_dir = game_dir.join("mods");
    fs::create_dir_all(&mods_dir).map_err(|error| error.to_string())?;
    copy_mod_archives(&request.minecraft_dir.join("mods"), &mods_dir)
        .map_err(|error| error.to_string())?;
    fs::write(game_dir.join("eula.txt"), "eula=true\n").map_err(|error| error.to_string())?;
    Ok((game_dir, mods_dir))
}

fn collect_jars(root: &Path) -> std::io::Result<Vec<PathBuf>> {
    let mut jars = Vec::new();
    collect_jars_recursive(root, &mut jars)?;
    jars.sort();
    Ok(jars)
}

fn collect_jars_recursive(root: &Path, jars: &mut Vec<PathBuf>) -> std::io::Result<()> {
    if !root.is_dir() {
        return Ok(());
    }
    for entry in fs::read_dir(root)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            collect_jars_recursive(&path, jars)?;
        } else if extension_is(&path, "jar") {
            jars.push(path);
        }
    }
    Ok(())
}

fn wait_for_runtime_stack_report(
    mut child: std::process::Child,
    runtime_report_path: &Path,
    cancellation: &CancellationToken,
) -> Result<BTreeMap<String, u32>, String> {
    let deadline = Instant::now() + Duration::from_secs(180);
    loop {
        if let Err(error) = ensure_not_cancelled(cancellation) {
            let _ = child.kill();
            let _ = child.wait();
            return Err(error.to_string());
        }
        if let Some(status) = child
            .try_wait()
            .map_err(|error| format!("Could not poll runtime extractor: {error}"))?
        {
            let output = child
                .wait_with_output()
                .map_err(|error| format!("Could not collect runtime extractor output: {error}"))?;
            if !status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                let stdout = String::from_utf8_lossy(&output.stdout);
                return Err(format!(
                    "Runtime stack extraction exited with {status}. {}{}",
                    stdout.lines().last().unwrap_or(""),
                    stderr.lines().last().unwrap_or("")
                ));
            }
            return read_runtime_stack_report(runtime_report_path)
                .map_err(|error| format!("Runtime stack report could not be read: {error}"));
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            return Err("Runtime stack extraction timed out after 180 seconds.".to_string());
        }
        thread::sleep(Duration::from_millis(400));
    }
}

fn copy_mod_archives(source: &Path, destination: &Path) -> std::io::Result<()> {
    if !source.is_dir() {
        return Ok(());
    }
    for entry in fs::read_dir(source)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            copy_mod_archives(&path, destination)?;
        } else if extension_is(&path, "jar") || extension_is(&path, "zip") {
            fs::copy(&path, destination.join(entry.file_name()))?;
        }
    }
    Ok(())
}

fn copy_dir_recursive(source: &Path, destination: &Path) -> std::io::Result<()> {
    fs::create_dir_all(destination)?;
    for entry in fs::read_dir(source)? {
        let entry = entry?;
        let source_path = entry.path();
        let destination_path = destination.join(entry.file_name());
        if source_path.is_dir() {
            copy_dir_recursive(&source_path, &destination_path)?;
        } else {
            fs::copy(&source_path, destination_path)?;
        }
    }
    Ok(())
}

fn neoforge_runtime_extractor_jar_bytes() -> Vec<u8> {
    decode_hex(include_str!("runtime_extractor_jar.hex"))
}

fn forge_runtime_extractor_jar_bytes() -> Vec<u8> {
    decode_hex(include_str!("runtime_extractor_forge_jar.hex"))
}

fn fabric_runtime_extractor_jar_bytes() -> Vec<u8> {
    decode_hex(include_str!("runtime_extractor_fabric_jar.hex"))
}

fn decode_hex(value: &str) -> Vec<u8> {
    let digits = value
        .bytes()
        .filter(|byte| !byte.is_ascii_whitespace())
        .collect::<Vec<_>>();
    let mut bytes = Vec::with_capacity(digits.len() / 2);
    for pair in digits.chunks(2) {
        if pair.len() == 2 {
            bytes.push((hex_digit(pair[0]) << 4) | hex_digit(pair[1]));
        }
    }
    bytes
}

fn hex_digit(value: u8) -> u8 {
    match value {
        b'0'..=b'9' => value - b'0',
        b'a'..=b'f' => value - b'a' + 10,
        b'A'..=b'F' => value - b'A' + 10,
        _ => 0,
    }
}

fn scan_asset_entries_in_dir(
    collector: &mut AssetCollector,
    root: &Path,
    cancellation: &CancellationToken,
) -> Result<(), AssetError> {
    ensure_not_cancelled(cancellation)?;
    let mut files = Vec::new();
    collect_files(root, &mut files, cancellation)?;
    for file in files {
        ensure_not_cancelled(cancellation)?;
        let Ok(relative) = file.strip_prefix(root) else {
            continue;
        };
        let bytes = match fs::read(&file) {
            Ok(bytes) => bytes,
            Err(error) => {
                collector
                    .warnings
                    .push(format!("Skipped asset {}: {error}", file.display()));
                continue;
            }
        };
        collector.scan_asset_entry(relative, Some(file.clone()), &bytes);
    }
    Ok(())
}

fn collect_archives(
    root: &Path,
    cancellation: &CancellationToken,
) -> Result<Vec<PathBuf>, AssetError> {
    ensure_not_cancelled(cancellation)?;
    let mut archives = Vec::new();
    collect_archives_into(root, &mut archives, cancellation)?;
    archives.sort();
    Ok(archives)
}

fn collect_archives_into(
    root: &Path,
    archives: &mut Vec<PathBuf>,
    cancellation: &CancellationToken,
) -> Result<(), AssetError> {
    ensure_not_cancelled(cancellation)?;
    if !root.exists() {
        return Ok(());
    }
    for entry in fs::read_dir(root)? {
        ensure_not_cancelled(cancellation)?;
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            collect_archives_into(&path, archives, cancellation)?;
        } else if extension_is(&path, "jar") || extension_is(&path, "zip") {
            archives.push(path);
        }
    }
    Ok(())
}

fn collect_files(
    root: &Path,
    files: &mut Vec<PathBuf>,
    cancellation: &CancellationToken,
) -> Result<(), AssetError> {
    ensure_not_cancelled(cancellation)?;
    if !root.exists() {
        return Ok(());
    }
    for entry in fs::read_dir(root)? {
        ensure_not_cancelled(cancellation)?;
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            collect_files(&path, files, cancellation)?;
        } else {
            files.push(path);
        }
    }
    Ok(())
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

struct AssetCollector {
    texture_cache_dir: PathBuf,
    languages: BTreeMap<String, String>,
    blockstates: BTreeMap<String, BlockstateAsset>,
    items: BTreeMap<String, ItemAsset>,
    models: BTreeMap<String, ModelAsset>,
    textures: BTreeMap<String, PathBuf>,
    asset_paths_seen: BTreeSet<PathBuf>,
    warnings: Vec<String>,
}

impl AssetCollector {
    fn new(texture_cache_dir: PathBuf) -> Self {
        Self {
            texture_cache_dir,
            languages: BTreeMap::new(),
            blockstates: BTreeMap::new(),
            items: BTreeMap::new(),
            models: BTreeMap::new(),
            textures: BTreeMap::new(),
            asset_paths_seen: BTreeSet::new(),
            warnings: Vec::new(),
        }
    }

    fn scan_archive(
        &mut self,
        archive_path: &Path,
        cancellation: &CancellationToken,
    ) -> Result<(), AssetError> {
        ensure_not_cancelled(cancellation)?;
        let file = File::open(archive_path)?;
        let mut archive =
            zip::ZipArchive::new(file).map_err(|error| AssetError::Zip(error.to_string()))?;
        for index in 0..archive.len() {
            ensure_not_cancelled(cancellation)?;
            let mut entry = archive
                .by_index(index)
                .map_err(|error| AssetError::Zip(error.to_string()))?;
            if entry.is_dir() {
                continue;
            }
            let Some(enclosed_name) = entry.enclosed_name() else {
                continue;
            };
            let enclosed_name = enclosed_name.to_path_buf();
            if parse_asset_path(&enclosed_name).is_none() {
                continue;
            }
            let mut bytes = Vec::new();
            entry.read_to_end(&mut bytes)?;
            let display_path = PathBuf::from(format!(
                "{}::{}",
                archive_path.display(),
                enclosed_name.to_string_lossy()
            ));
            self.scan_asset_entry(&enclosed_name, Some(display_path), &bytes);
        }
        Ok(())
    }

    fn scan_asset_entry(
        &mut self,
        relative_path: &Path,
        _source_path: Option<PathBuf>,
        bytes: &[u8],
    ) {
        let Some(asset_path) = parse_asset_path(relative_path) else {
            return;
        };
        self.asset_paths_seen.insert(relative_path.to_path_buf());
        let result = match asset_path.kind.as_str() {
            "lang" if extension_is(relative_path, "json") => self.read_language_bytes(bytes),
            "blockstates" if extension_is(relative_path, "json") => {
                self.read_blockstate_bytes(asset_path, bytes)
            }
            "models" if extension_is(relative_path, "json") => {
                self.read_model_bytes(asset_path, bytes)
            }
            "items" if extension_is(relative_path, "json") => {
                self.read_item_bytes(asset_path, bytes)
            }
            "textures" if extension_is(relative_path, "png") => {
                let id = format!(
                    "{}:{}",
                    asset_path.namespace,
                    without_extension(&asset_path.relative_asset_path)
                );
                match self.cache_texture(&asset_path, bytes) {
                    Ok(path) => {
                        self.textures.insert(id, path);
                        Ok(())
                    }
                    Err(error) => Err(error),
                }
            }
            _ => Ok(()),
        };
        if let Err(error) = result {
            self.warnings.push(format!(
                "Skipped asset {}: {error}",
                relative_path.display()
            ));
        }
    }

    fn cache_texture(&self, asset_path: &AssetPath, bytes: &[u8]) -> Result<PathBuf, AssetError> {
        let path = self
            .texture_cache_dir
            .join(&asset_path.namespace)
            .join(&asset_path.relative_asset_path);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&path, bytes)?;
        Ok(path)
    }

    fn read_language_bytes(&mut self, bytes: &[u8]) -> Result<(), AssetError> {
        let value = read_json_bytes(bytes)?;
        let Some(object) = value.as_object() else {
            return Ok(());
        };
        for (key, value) in object {
            if let Some(text) = value.as_str() {
                self.languages.insert(key.clone(), text.to_string());
            }
        }
        Ok(())
    }

    fn read_blockstate_bytes(
        &mut self,
        asset_path: AssetPath,
        bytes: &[u8],
    ) -> Result<(), AssetError> {
        let value = read_json_bytes(bytes)?;
        let identifier = format!(
            "{}:{}",
            asset_path.namespace,
            without_extension(&asset_path.relative_asset_path)
        );
        self.blockstates.insert(
            identifier.clone(),
            BlockstateAsset {
                identifier,
                namespace: asset_path.namespace,
                models: collect_string_fields(&value, "model"),
            },
        );
        Ok(())
    }

    fn read_model_bytes(&mut self, asset_path: AssetPath, bytes: &[u8]) -> Result<(), AssetError> {
        let value = read_json_bytes(bytes)?;
        let id = format!(
            "{}:{}",
            asset_path.namespace,
            without_extension(&asset_path.relative_asset_path)
        );
        self.models.insert(
            id,
            ModelAsset {
                parent: value
                    .get("parent")
                    .and_then(serde_json::Value::as_str)
                    .map(|parent| normalize_asset_reference(parent, &asset_path.namespace)),
                textures: collect_model_textures(&value),
                face_textures: collect_model_face_textures(&value),
                elements: collect_model_elements(&value),
            },
        );
        Ok(())
    }

    fn read_item_bytes(&mut self, asset_path: AssetPath, bytes: &[u8]) -> Result<(), AssetError> {
        let value = read_json_bytes(bytes)?;
        let id = format!(
            "{}:{}",
            asset_path.namespace,
            without_extension(&asset_path.relative_asset_path)
        );
        self.items.insert(
            id,
            ItemAsset {
                max_stack_size: explicit_max_stack_size(&value),
            },
        );
        Ok(())
    }

    fn block_samples(&self) -> Vec<BlockAssetSample> {
        self.blockstates
            .values()
            .map(|blockstate| {
                let item_id = Some(blockstate.identifier.clone());
                let max_stack_size = item_id
                    .as_ref()
                    .and_then(|id| self.items.get(id))
                    .and_then(|item| item.max_stack_size);
                let model = blockstate
                    .models
                    .first()
                    .map(|model| normalize_asset_reference(model, &blockstate.namespace));
                let resolved_model = model
                    .as_ref()
                    .and_then(|model_id| self.resolved_model_textures(model_id));
                let texture_path = resolved_model
                    .as_ref()
                    .and_then(|resolved| resolved.primary_texture_id())
                    .and_then(|texture_id| self.textures.get(&texture_id).cloned());
                let face_texture_paths = resolved_model
                    .as_ref()
                    .and_then(|resolved| resolved.face_paths(&self.textures));
                let model_elements = resolved_model
                    .as_ref()
                    .map(|resolved| resolved.element_samples(&self.textures))
                    .unwrap_or_default();
                let language_key = format!(
                    "block.{}.{}",
                    blockstate.namespace,
                    blockstate
                        .identifier
                        .split_once(':')
                        .map(|(_, path)| path.replace('/', "."))
                        .unwrap_or_else(|| blockstate.identifier.replace('/', "."))
                );
                BlockAssetSample {
                    identifier: blockstate.identifier.clone(),
                    item_id,
                    max_stack_size,
                    display_name: self
                        .languages
                        .get(&language_key)
                        .cloned()
                        .unwrap_or_else(|| blockstate.identifier.clone()),
                    namespace: blockstate.namespace.clone(),
                    model,
                    texture_path,
                    face_texture_paths,
                    model_elements,
                }
            })
            .collect()
    }

    fn resolved_model_textures(&self, model_id: &str) -> Option<ResolvedModelTextures> {
        let mut visiting = BTreeSet::new();
        self.resolved_model_textures_inner(model_id, &mut visiting)
    }

    fn resolved_model_textures_inner(
        &self,
        model_id: &str,
        visiting: &mut BTreeSet<String>,
    ) -> Option<ResolvedModelTextures> {
        if !visiting.insert(model_id.to_string()) {
            return None;
        }
        let model = self.models.get(model_id)?;
        let mut resolved = model
            .parent
            .as_deref()
            .and_then(|parent| self.resolved_model_textures_inner(parent, visiting))
            .unwrap_or_default();
        for (name, texture) in &model.textures {
            let namespace = model_id
                .split_once(':')
                .map(|(namespace, _)| namespace)
                .unwrap_or("minecraft");
            resolved.textures.insert(
                name.clone(),
                normalize_texture_reference(texture, namespace),
            );
        }
        for (face, texture) in &model.face_textures {
            resolved.face_textures.insert(*face, texture.clone());
        }
        if !model.elements.is_empty() {
            resolved.elements = model.elements.clone();
        }
        visiting.remove(model_id);
        Some(resolved)
    }
}

struct BlockstateAsset {
    identifier: String,
    namespace: String,
    models: Vec<String>,
}

struct ModelAsset {
    parent: Option<String>,
    textures: BTreeMap<String, String>,
    face_textures: BTreeMap<BlockFace, String>,
    elements: Vec<ModelElementAsset>,
}

#[derive(Debug, Clone)]
struct ModelElementAsset {
    from: [f32; 3],
    to: [f32; 3],
    face_textures: BTreeMap<BlockFace, String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum BlockFace {
    North,
    South,
    East,
    West,
    Up,
    Down,
}

#[derive(Debug, Clone, Default)]
struct ResolvedModelTextures {
    textures: BTreeMap<String, String>,
    face_textures: BTreeMap<BlockFace, String>,
    elements: Vec<ModelElementAsset>,
}

impl ResolvedModelTextures {
    fn primary_texture_id(&self) -> Option<String> {
        for face in [
            BlockFace::North,
            BlockFace::South,
            BlockFace::East,
            BlockFace::West,
            BlockFace::Up,
            BlockFace::Down,
        ] {
            if let Some(texture) = self.face_texture_id(face) {
                return Some(texture);
            }
        }
        self.textures
            .values()
            .find_map(|texture| self.resolve_texture_reference(texture))
    }

    fn face_paths(&self, textures: &BTreeMap<String, PathBuf>) -> Option<FaceTexturePaths> {
        let paths = FaceTexturePaths {
            north: self
                .face_texture_id(BlockFace::North)
                .and_then(|id| textures.get(&id).cloned()),
            south: self
                .face_texture_id(BlockFace::South)
                .and_then(|id| textures.get(&id).cloned()),
            east: self
                .face_texture_id(BlockFace::East)
                .and_then(|id| textures.get(&id).cloned()),
            west: self
                .face_texture_id(BlockFace::West)
                .and_then(|id| textures.get(&id).cloned()),
            up: self
                .face_texture_id(BlockFace::Up)
                .and_then(|id| textures.get(&id).cloned()),
            down: self
                .face_texture_id(BlockFace::Down)
                .and_then(|id| textures.get(&id).cloned()),
        };
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
        .then_some(paths)
    }

    fn face_texture_id(&self, face: BlockFace) -> Option<String> {
        self.face_textures
            .get(&face)
            .and_then(|texture| self.resolve_texture_reference(texture))
    }

    fn element_samples(&self, textures: &BTreeMap<String, PathBuf>) -> Vec<ModelElementSample> {
        self.elements
            .iter()
            .map(|element| {
                let face_texture_paths = FaceTexturePaths {
                    north: self.element_face_path(element, BlockFace::North, textures),
                    south: self.element_face_path(element, BlockFace::South, textures),
                    east: self.element_face_path(element, BlockFace::East, textures),
                    west: self.element_face_path(element, BlockFace::West, textures),
                    up: self.element_face_path(element, BlockFace::Up, textures),
                    down: self.element_face_path(element, BlockFace::Down, textures),
                };
                ModelElementSample {
                    from: element.from,
                    to: element.to,
                    face_texture_paths,
                }
            })
            .collect()
    }

    fn element_face_path(
        &self,
        element: &ModelElementAsset,
        face: BlockFace,
        textures: &BTreeMap<String, PathBuf>,
    ) -> Option<PathBuf> {
        element
            .face_textures
            .get(&face)
            .and_then(|texture| self.resolve_texture_reference(texture))
            .and_then(|id| textures.get(&id).cloned())
    }

    fn resolve_texture_reference(&self, texture: &str) -> Option<String> {
        let mut current = texture;
        let mut seen = BTreeSet::new();
        loop {
            if let Some(variable) = current.strip_prefix('#') {
                if !seen.insert(variable.to_string()) {
                    return None;
                }
                current = self.textures.get(variable)?;
                continue;
            }
            return Some(current.to_string());
        }
    }
}

struct ItemAsset {
    max_stack_size: Option<u32>,
}

struct AssetPath {
    namespace: String,
    kind: String,
    relative_asset_path: String,
}

fn parse_asset_path(path: &Path) -> Option<AssetPath> {
    let components = path
        .components()
        .map(|component| component.as_os_str().to_string_lossy().to_string())
        .collect::<Vec<_>>();
    let assets_index = components
        .iter()
        .position(|component| component == "assets")?;
    let namespace = components.get(assets_index + 1)?.clone();
    let kind = components.get(assets_index + 2)?.clone();
    let relative_asset_path = components
        .get(assets_index + 3..)?
        .join("/")
        .trim()
        .to_string();
    if namespace.is_empty() || kind.is_empty() || relative_asset_path.is_empty() {
        return None;
    }
    Some(AssetPath {
        namespace,
        kind,
        relative_asset_path,
    })
}

fn read_json_bytes(bytes: &[u8]) -> Result<serde_json::Value, AssetError> {
    serde_json::from_reader(Cursor::new(bytes))
        .map_err(|error| AssetError::InvalidAssetIndex(error.to_string()))
}

fn collect_string_fields(value: &serde_json::Value, field: &str) -> Vec<String> {
    let mut values = Vec::new();
    collect_string_fields_into(value, field, &mut values);
    values
}

fn collect_string_fields_into(value: &serde_json::Value, field: &str, values: &mut Vec<String>) {
    match value {
        serde_json::Value::Object(object) => {
            for (key, value) in object {
                if key == field {
                    if let Some(text) = value.as_str() {
                        values.push(text.to_string());
                    }
                }
                collect_string_fields_into(value, field, values);
            }
        }
        serde_json::Value::Array(array) => {
            for item in array {
                collect_string_fields_into(item, field, values);
            }
        }
        _ => {}
    }
}

fn collect_model_textures(value: &serde_json::Value) -> BTreeMap<String, String> {
    value
        .get("textures")
        .and_then(|textures| textures.as_object())
        .map(|textures| {
            textures
                .iter()
                .filter_map(|(name, texture)| Some((name.clone(), texture.as_str()?.to_string())))
                .collect()
        })
        .unwrap_or_default()
}

fn collect_model_face_textures(value: &serde_json::Value) -> BTreeMap<BlockFace, String> {
    let mut faces = BTreeMap::new();
    let Some(elements) = value.get("elements").and_then(serde_json::Value::as_array) else {
        return faces;
    };
    for element in elements {
        let Some(face_object) = element.get("faces").and_then(serde_json::Value::as_object) else {
            continue;
        };
        for (face_name, face_value) in face_object {
            let Some(face) = parse_block_face(face_name) else {
                continue;
            };
            let Some(texture) = face_value
                .get("texture")
                .and_then(serde_json::Value::as_str)
            else {
                continue;
            };
            faces.entry(face).or_insert_with(|| texture.to_string());
        }
    }
    faces
}

fn collect_model_elements(value: &serde_json::Value) -> Vec<ModelElementAsset> {
    let Some(elements) = value.get("elements").and_then(serde_json::Value::as_array) else {
        return Vec::new();
    };
    elements
        .iter()
        .filter_map(|element| {
            let from = parse_model_vector(element.get("from")?)?;
            let to = parse_model_vector(element.get("to")?)?;
            if from == to {
                return None;
            }
            let face_object = element
                .get("faces")
                .and_then(serde_json::Value::as_object)?;
            let face_textures = face_object
                .iter()
                .filter_map(|(face_name, face_value)| {
                    let face = parse_block_face(face_name)?;
                    let texture = face_value.get("texture")?.as_str()?.to_string();
                    Some((face, texture))
                })
                .collect::<BTreeMap<_, _>>();
            if face_textures.is_empty() {
                return None;
            }
            Some(ModelElementAsset {
                from,
                to,
                face_textures,
            })
        })
        .collect()
}

fn parse_model_vector(value: &serde_json::Value) -> Option<[f32; 3]> {
    let array = value.as_array()?;
    let [x, y, z] = array.as_slice() else {
        return None;
    };
    Some([x.as_f64()? as f32, y.as_f64()? as f32, z.as_f64()? as f32])
}

fn parse_block_face(value: &str) -> Option<BlockFace> {
    match value {
        "north" => Some(BlockFace::North),
        "south" => Some(BlockFace::South),
        "east" => Some(BlockFace::East),
        "west" => Some(BlockFace::West),
        "up" => Some(BlockFace::Up),
        "down" => Some(BlockFace::Down),
        _ => None,
    }
}

fn explicit_max_stack_size(value: &serde_json::Value) -> Option<u32> {
    match value {
        serde_json::Value::Object(object) => {
            for key in ["minecraft:max_stack_size", "max_stack_size", "maxStackSize"] {
                if let Some(size) = object.get(key).and_then(serde_json::Value::as_u64) {
                    return u32::try_from(size).ok();
                }
            }
            object.values().find_map(explicit_max_stack_size)
        }
        serde_json::Value::Array(array) => array.iter().find_map(explicit_max_stack_size),
        _ => None,
    }
}

fn normalize_asset_reference(value: &str, fallback_namespace: &str) -> String {
    if value.contains(':') {
        value.to_string()
    } else {
        format!("{fallback_namespace}:{value}")
    }
}

fn normalize_texture_reference(value: &str, fallback_namespace: &str) -> String {
    if value.starts_with('#') {
        value.to_string()
    } else {
        normalize_asset_reference(value, fallback_namespace)
    }
}

fn texture_atlas_metadata(blocks: &[BlockAssetSample]) -> TextureAtlasMetadata {
    let mut seen = BTreeSet::new();
    let textures = blocks
        .iter()
        .flat_map(|block| {
            let mut paths = Vec::new();
            if let Some(path) = &block.texture_path {
                paths.push(path.clone());
            }
            if let Some(faces) = &block.face_texture_paths {
                for path in [
                    &faces.north,
                    &faces.south,
                    &faces.east,
                    &faces.west,
                    &faces.up,
                    &faces.down,
                ]
                .into_iter()
                .flatten()
                {
                    paths.push(path.clone());
                }
            }
            for element in &block.model_elements {
                for path in [
                    &element.face_texture_paths.north,
                    &element.face_texture_paths.south,
                    &element.face_texture_paths.east,
                    &element.face_texture_paths.west,
                    &element.face_texture_paths.up,
                    &element.face_texture_paths.down,
                ]
                .into_iter()
                .flatten()
                {
                    paths.push(path.clone());
                }
            }
            paths
                .into_iter()
                .filter_map(|path| {
                    let key = path.to_string_lossy().to_string();
                    seen.insert(key).then_some((block.identifier.clone(), path))
                })
                .collect::<Vec<_>>()
        })
        .enumerate()
        .map(
            |(tile_index, (identifier, source_path))| TextureAtlasEntry {
                identifier,
                source_path,
                tile_index,
            },
        )
        .collect();

    TextureAtlasMetadata {
        textures,
        tile_size: 16,
    }
}

fn without_extension(path: &str) -> String {
    Path::new(path)
        .with_extension("")
        .to_string_lossy()
        .replace('\\', "/")
}

fn extension_is(path: &Path, extension: &str) -> bool {
    path.extension()
        .and_then(|value| value.to_str())
        .is_some_and(|value| value.eq_ignore_ascii_case(extension))
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
