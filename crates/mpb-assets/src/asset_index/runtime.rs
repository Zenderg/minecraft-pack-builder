use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};

use crate::blockstate::BlockstateModelCondition;
use crate::{AssetError, CancellationToken};

use super::{AssetIndexEvent, ModelElementSample, PrismAssetIndexRequest};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BakedRenderAssetSample {
    pub fidelity: String,
    pub source: String,
    #[serde(default)]
    pub condition: Option<BlockstateModelCondition>,
    #[serde(default)]
    pub model: Option<String>,
    #[serde(default)]
    pub elements: Vec<ModelElementSample>,
}

#[derive(Debug, Clone)]
pub struct RuntimeReportData {
    pub items: BTreeMap<String, u32>,
    pub render_assets: BTreeMap<String, Vec<BakedRenderAssetSample>>,
}

pub fn read_runtime_report(path: &Path) -> Result<RuntimeReportData, AssetError> {
    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct RuntimeReport {
        #[serde(default)]
        items: Vec<RuntimeItem>,
        #[serde(default)]
        blocks: Vec<RuntimeBlock>,
    }

    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct RuntimeItem {
        item_id: String,
        max_stack_size: u32,
    }

    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct RuntimeBlock {
        identifier: String,
        #[serde(default)]
        render_assets: Vec<BakedRenderAssetSample>,
    }

    let text = fs::read_to_string(path)?;
    let report: RuntimeReport = serde_json::from_str(&text)
        .map_err(|error| AssetError::InvalidAssetIndex(error.to_string()))?;
    let items = report
        .items
        .into_iter()
        .map(|item| (item.item_id, item.max_stack_size))
        .collect();
    let render_assets = report
        .blocks
        .into_iter()
        .filter(|block| !block.render_assets.is_empty())
        .map(|block| (block.identifier, block.render_assets))
        .collect();
    Ok(RuntimeReportData {
        items,
        render_assets,
    })
}

#[derive(Debug, Clone)]
pub struct RuntimeStackOutcome {
    pub status: String,
    pub message: Option<String>,
    pub items: BTreeMap<String, u32>,
    pub render_assets: BTreeMap<String, Vec<BakedRenderAssetSample>>,
}

impl RuntimeStackOutcome {
    fn ready_with_render_assets(
        items: BTreeMap<String, u32>,
        render_assets: BTreeMap<String, Vec<BakedRenderAssetSample>>,
    ) -> Self {
        Self {
            status: "ready".to_string(),
            message: None,
            items,
            render_assets,
        }
    }

    fn unavailable(message: impl Into<String>) -> Self {
        Self {
            status: "unavailable".to_string(),
            message: Some(message.into()),
            items: BTreeMap::new(),
            render_assets: BTreeMap::new(),
        }
    }
}

pub fn runtime_stack_metadata(
    request: &PrismAssetIndexRequest,
    cancellation: &CancellationToken,
    on_event: &mut impl FnMut(AssetIndexEvent),
) -> RuntimeStackOutcome {
    let runtime_report_path = runtime_report_path(request);
    if runtime_report_path.is_file() {
        return match read_runtime_report(&runtime_report_path) {
            Ok(report) => {
                RuntimeStackOutcome::ready_with_render_assets(report.items, report.render_assets)
            }
            Err(error) => RuntimeStackOutcome::unavailable(format!(
                "Cached runtime stack report could not be parsed: {error}"
            )),
        };
    }

    match run_runtime_stack_extractor(request, &runtime_report_path, cancellation, on_event) {
        Ok(report) => {
            RuntimeStackOutcome::ready_with_render_assets(report.items, report.render_assets)
        }
        Err(message) => RuntimeStackOutcome::unavailable(message),
    }
}

fn runtime_report_path(request: &PrismAssetIndexRequest) -> PathBuf {
    request.diagnostics_dir.join(format!(
        "{}-{}-runtime.json",
        safe_path_segment(&request.identity_fingerprint),
        safe_path_segment(&request.content_fingerprint)
    ))
}

fn run_runtime_stack_extractor(
    request: &PrismAssetIndexRequest,
    runtime_report_path: &Path,
    cancellation: &CancellationToken,
    on_event: &mut impl FnMut(AssetIndexEvent),
) -> Result<RuntimeReportData, String> {
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
) -> Result<RuntimeReportData, String> {
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
) -> Result<RuntimeReportData, String> {
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
) -> Result<RuntimeReportData, String> {
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
            return read_runtime_report(runtime_report_path)
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
            fs::copy(source_path, destination_path)?;
        }
    }
    Ok(())
}

fn neoforge_runtime_extractor_jar_bytes() -> Vec<u8> {
    decode_hex(include_str!("../runtime_extractor_jar.hex"))
}

fn forge_runtime_extractor_jar_bytes() -> Vec<u8> {
    decode_hex(include_str!("../runtime_extractor_forge_jar.hex"))
}

fn fabric_runtime_extractor_jar_bytes() -> Vec<u8> {
    decode_hex(include_str!("../runtime_extractor_fabric_jar.hex"))
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

fn prism_root_from_instance_path(instance_path: &Path) -> Option<PathBuf> {
    let instances_dir = instance_path.parent()?;
    if instances_dir.file_name()?.to_string_lossy() != "instances" {
        return None;
    }
    instances_dir.parent().map(Path::to_path_buf)
}

fn ensure_not_cancelled(cancellation: &CancellationToken) -> Result<(), AssetError> {
    if cancellation.is_cancelled() {
        return Err(AssetError::Cancelled);
    }
    Ok(())
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
