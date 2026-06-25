use std::fs;
use std::hash::Hasher;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use crate::{AssetError, PrismInstanceDescriptor};

const PATCH_SCHEMA_VERSION: u32 = 1;
const PATCHER_VERSION: &str = env!("CARGO_PKG_VERSION");
const MPB_MOD_FILE: &str = "mods/mpb-minecraft-mod.jar";
const MPB_RUNTIME_PID_FILE: &str = "mpb/runtime.pid";
const FABRIC_MOD_HEX: &str = include_str!("mpb_mod_fabric_jar.hex");
const FORGE_MOD_HEX: &str = include_str!("mpb_mod_forge_jar.hex");
const NEOFORGE_MOD_HEX: &str = include_str!("mpb_mod_neoforge_jar.hex");

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum MpbPatchStatus {
    NotPatched,
    Patched,
    NeedsUpdate,
    NeedsRepair,
    Conflict,
    Unsupported,
    InstanceRunning,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MpbPatchAction {
    Apply,
    Update,
    Repair,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MpbPatchEvaluation {
    pub status: MpbPatchStatus,
    pub reason: Option<String>,
    pub manifest_path: PathBuf,
    pub managed_files: Vec<PathBuf>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MpbPatchOperationResult {
    pub status: MpbPatchStatus,
    pub steps: Vec<MpbPatchProgressStep>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MpbPatchProgressStep {
    pub label: String,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MpbPatchManifest {
    pub schema_version: u32,
    pub patcher_version: String,
    pub mod_version: String,
    pub loader: String,
    pub minecraft_version: String,
    pub installed_at: String,
    pub files: Vec<MpbManagedFile>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MpbManagedFile {
    pub path: String,
    pub checksum: String,
    pub owner: MpbFileOwner,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum MpbFileOwner {
    Managed,
    Preexisting,
}

pub fn evaluate_mpb_patch(instance: &PrismInstanceDescriptor) -> MpbPatchEvaluation {
    let manifest_path = manifest_path(instance);
    if let Some(reason) = unsupported_reason(instance) {
        return MpbPatchEvaluation {
            status: MpbPatchStatus::Unsupported,
            reason: Some(reason),
            manifest_path,
            managed_files: Vec::new(),
        };
    }
    if let Some(reason) = running_instance_reason(instance) {
        return MpbPatchEvaluation {
            status: MpbPatchStatus::InstanceRunning,
            reason: Some(reason),
            manifest_path,
            managed_files: Vec::new(),
        };
    }
    if let Some(reason) = unmanaged_mod_conflict_reason(instance) {
        return MpbPatchEvaluation {
            status: MpbPatchStatus::Conflict,
            reason: Some(reason),
            manifest_path,
            managed_files: Vec::new(),
        };
    }
    let Some(manifest) = read_manifest(instance).ok() else {
        return MpbPatchEvaluation {
            status: MpbPatchStatus::NotPatched,
            reason: None,
            manifest_path,
            managed_files: Vec::new(),
        };
    };
    let managed_files = manifest
        .files
        .iter()
        .filter(|file| file.owner == MpbFileOwner::Managed)
        .map(|file| instance_root_path(instance, &file.path))
        .collect::<Vec<_>>();
    if manifest.schema_version != PATCH_SCHEMA_VERSION
        || manifest.patcher_version != PATCHER_VERSION
    {
        return MpbPatchEvaluation {
            status: MpbPatchStatus::NeedsUpdate,
            reason: Some("MPB patch manifest was created by another patcher version.".to_string()),
            manifest_path,
            managed_files,
        };
    }
    if let Ok(expected_bytes) = mod_artifact_bytes(instance) {
        let expected_checksum = checksum(&expected_bytes);
        let managed_mod_is_current = manifest.files.iter().any(|file| {
            file.owner == MpbFileOwner::Managed
                && file.path == MPB_MOD_FILE
                && file.checksum == expected_checksum
        });
        if !managed_mod_is_current {
            return MpbPatchEvaluation {
                status: MpbPatchStatus::NeedsUpdate,
                reason: Some("Bundled MPB mod artifact has changed.".to_string()),
                manifest_path,
                managed_files,
            };
        }
    }
    for file in &manifest.files {
        if file.owner != MpbFileOwner::Managed {
            continue;
        }
        let path = instance_root_path(instance, &file.path);
        let Ok(bytes) = fs::read(&path) else {
            return MpbPatchEvaluation {
                status: MpbPatchStatus::NeedsRepair,
                reason: Some(format!("Managed file is missing: {}", file.path)),
                manifest_path,
                managed_files,
            };
        };
        if checksum(&bytes) != file.checksum {
            return MpbPatchEvaluation {
                status: MpbPatchStatus::NeedsRepair,
                reason: Some(format!("Managed file changed: {}", file.path)),
                manifest_path,
                managed_files,
            };
        }
    }
    MpbPatchEvaluation {
        status: MpbPatchStatus::Patched,
        reason: None,
        manifest_path,
        managed_files,
    }
}

pub fn apply_mpb_patch(
    instance: &PrismInstanceDescriptor,
    _action: MpbPatchAction,
) -> Result<MpbPatchOperationResult, AssetError> {
    let evaluation = evaluate_mpb_patch(instance);
    if matches!(
        evaluation.status,
        MpbPatchStatus::Unsupported | MpbPatchStatus::Conflict | MpbPatchStatus::InstanceRunning
    ) {
        return Err(AssetError::Patch(evaluation.reason.unwrap_or_else(|| {
            "MPB patch cannot be applied to this instance.".to_string()
        })));
    }

    let mut steps = Vec::new();
    fs::create_dir_all(instance.instance_path.join("mpb/schemes"))?;
    fs::create_dir_all(instance.instance_path.join("mpb/cache"))?;
    fs::create_dir_all(instance.minecraft_dir.join("mods"))?;
    steps.push(done("Prepared instance mpb folders"));

    let mod_bytes = mod_artifact_bytes(instance)?;
    write_managed_file(instance, MPB_MOD_FILE, &mod_bytes)?;
    steps.push(done("Installed MPB Minecraft mod"));

    let manifest = MpbPatchManifest {
        schema_version: PATCH_SCHEMA_VERSION,
        patcher_version: PATCHER_VERSION.to_string(),
        mod_version: PATCHER_VERSION.to_string(),
        loader: instance.loader.clone().unwrap_or_default(),
        minecraft_version: instance.minecraft_version.clone().unwrap_or_default(),
        installed_at: timestamp_string(),
        files: vec![MpbManagedFile {
            path: MPB_MOD_FILE.to_string(),
            checksum: checksum(&mod_bytes),
            owner: MpbFileOwner::Managed,
        }],
    };
    write_manifest(instance, &manifest)?;
    steps.push(done("Wrote MPB patch manifest"));

    Ok(MpbPatchOperationResult {
        status: MpbPatchStatus::Patched,
        steps,
    })
}

pub fn remove_mpb_patch(
    instance: &PrismInstanceDescriptor,
    delete_schemes: bool,
) -> Result<MpbPatchOperationResult, AssetError> {
    if let Some(reason) = running_instance_reason(instance) {
        return Err(AssetError::Patch(reason));
    }

    let mut steps = Vec::new();
    if let Ok(manifest) = read_manifest(instance) {
        for file in manifest.files {
            if file.owner == MpbFileOwner::Managed {
                let path = instance_root_path(instance, &file.path);
                if path.is_file() {
                    fs::remove_file(path)?;
                }
            }
        }
        steps.push(done("Removed managed MPB files"));
    }

    let mpb_dir = instance.instance_path.join("mpb");
    remove_file_if_exists(&mpb_dir.join("config.json"))?;
    remove_dir_if_exists(&mpb_dir.join("cache"))?;
    remove_file_if_exists(&mpb_dir.join("patch-manifest.json"))?;
    if delete_schemes {
        remove_dir_if_exists(&mpb_dir.join("schemes"))?;
    } else {
        fs::create_dir_all(mpb_dir.join("schemes"))?;
    }
    steps.push(done("Removed MPB manifest, config, and cache"));

    Ok(MpbPatchOperationResult {
        status: MpbPatchStatus::NotPatched,
        steps,
    })
}

fn unsupported_reason(instance: &PrismInstanceDescriptor) -> Option<String> {
    let Some(loader) = instance.loader.as_deref() else {
        return Some("MPB supports Fabric, Forge, or NeoForge instances only.".to_string());
    };
    let normalized_loader = loader.to_ascii_lowercase();
    if !["fabric", "forge", "neoforge"]
        .iter()
        .any(|supported| normalized_loader == *supported)
    {
        return Some("MPB supports Fabric, Forge, or NeoForge instances only.".to_string());
    }
    let Some(version) = instance.minecraft_version.as_deref() else {
        return Some("Minecraft version is missing from PrismLauncher metadata.".to_string());
    };
    if !minecraft_version_is_1_20_or_newer(version) {
        return Some("MPB requires Minecraft 1.20 or newer.".to_string());
    }
    if !has_bundled_artifact(&normalized_loader, version) {
        return Some(format!(
            "No bundled MPB artifact is compatible with {loader} Minecraft {version}."
        ));
    }
    None
}

fn running_instance_reason(instance: &PrismInstanceDescriptor) -> Option<String> {
    let pid_path = instance_root_path(instance, MPB_RUNTIME_PID_FILE);
    let raw_pid = fs::read_to_string(&pid_path).ok()?;
    let pid = raw_pid.trim().parse::<u32>().ok()?;
    if process_is_running(pid) {
        Some(format!(
            "Instance {} appears to be running with MPB runtime pid {pid}. Close Minecraft before patching or unpatching.",
            instance.display_name
        ))
    } else {
        None
    }
}

fn unmanaged_mod_conflict_reason(instance: &PrismInstanceDescriptor) -> Option<String> {
    if manifest_path(instance).is_file() {
        return None;
    }
    let mod_path = instance_root_path(instance, MPB_MOD_FILE);
    if !mod_path.is_file() {
        return None;
    }
    let bytes = fs::read(&mod_path).ok()?;
    let expected = mod_artifact_bytes(instance).ok()?;
    if checksum(&bytes) == checksum(&expected) {
        None
    } else {
        Some(format!(
            "Existing mod {} is not managed by MPB and will not be overwritten.",
            mod_path.display()
        ))
    }
}

#[cfg(unix)]
fn process_is_running(pid: u32) -> bool {
    if pid == std::process::id() {
        return true;
    }
    unsafe extern "C" {
        fn kill(pid: i32, sig: i32) -> i32;
    }
    unsafe { kill(pid as i32, 0) == 0 }
}

#[cfg(windows)]
fn process_is_running(pid: u32) -> bool {
    if pid == std::process::id() {
        return true;
    }
    std::process::Command::new("cmd")
        .args([
            "/C",
            "tasklist",
            "/FI",
            &format!("PID eq {pid}"),
            "/FO",
            "CSV",
            "/NH",
        ])
        .output()
        .ok()
        .map(|output| {
            String::from_utf8_lossy(&output.stdout)
                .lines()
                .any(|line| line.contains(&format!("\",{pid},")))
        })
        .unwrap_or(false)
}

fn minecraft_version_is_1_20_or_newer(version: &str) -> bool {
    let mut parts = version.split('.').filter_map(|part| {
        part.chars()
            .take_while(|character| character.is_ascii_digit())
            .collect::<String>()
            .parse::<u32>()
            .ok()
    });
    let major = parts.next().unwrap_or(0);
    let minor = parts.next().unwrap_or(0);
    major > 1 || (major == 1 && minor >= 20)
}

fn has_bundled_artifact(loader: &str, minecraft_version: &str) -> bool {
    matches!(
        (loader, minecraft_version),
        ("fabric", "1.20.1") | ("forge", "1.20.1") | ("neoforge", "1.21.1")
    )
}

fn mod_artifact_bytes(instance: &PrismInstanceDescriptor) -> Result<Vec<u8>, AssetError> {
    let hex = match instance
        .loader
        .as_deref()
        .unwrap_or_default()
        .to_ascii_lowercase()
        .as_str()
    {
        "fabric" => FABRIC_MOD_HEX,
        "forge" => FORGE_MOD_HEX,
        "neoforge" => NEOFORGE_MOD_HEX,
        _ => {
            return Err(AssetError::Patch(
                "No bundled MPB artifact is compatible with this loader.".to_string(),
            ))
        }
    };
    decode_hex_artifact(hex)
}

fn decode_hex_artifact(hex: &str) -> Result<Vec<u8>, AssetError> {
    let digits = hex
        .bytes()
        .filter(|byte| !byte.is_ascii_whitespace())
        .collect::<Vec<_>>();
    if digits.len() % 2 != 0 {
        return Err(AssetError::Patch(
            "Bundled MPB artifact has invalid hexadecimal length.".to_string(),
        ));
    }
    let mut bytes = Vec::with_capacity(digits.len() / 2);
    for pair in digits.chunks_exact(2) {
        let high = hex_value(pair[0])?;
        let low = hex_value(pair[1])?;
        bytes.push((high << 4) | low);
    }
    if !bytes.starts_with(b"PK") {
        return Err(AssetError::Patch(
            "Bundled MPB artifact is not a JAR archive.".to_string(),
        ));
    }
    Ok(bytes)
}

fn hex_value(byte: u8) -> Result<u8, AssetError> {
    match byte {
        b'0'..=b'9' => Ok(byte - b'0'),
        b'a'..=b'f' => Ok(byte - b'a' + 10),
        b'A'..=b'F' => Ok(byte - b'A' + 10),
        _ => Err(AssetError::Patch(
            "Bundled MPB artifact contains invalid hexadecimal data.".to_string(),
        )),
    }
}

fn write_managed_file(
    instance: &PrismInstanceDescriptor,
    relative_path: &str,
    bytes: &[u8],
) -> Result<(), AssetError> {
    let path = instance_root_path(instance, relative_path);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, bytes)?;
    Ok(())
}

fn manifest_path(instance: &PrismInstanceDescriptor) -> PathBuf {
    instance.instance_path.join("mpb/patch-manifest.json")
}

fn read_manifest(instance: &PrismInstanceDescriptor) -> Result<MpbPatchManifest, AssetError> {
    let json = fs::read_to_string(manifest_path(instance))?;
    serde_json::from_str(&json).map_err(|error| AssetError::InvalidAssetIndex(error.to_string()))
}

fn write_manifest(
    instance: &PrismInstanceDescriptor,
    manifest: &MpbPatchManifest,
) -> Result<(), AssetError> {
    let path = manifest_path(instance);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let json = serde_json::to_vec_pretty(manifest)
        .map_err(|error| AssetError::InvalidAssetIndex(error.to_string()))?;
    fs::write(path, json)?;
    Ok(())
}

fn instance_root_path(instance: &PrismInstanceDescriptor, relative_path: &str) -> PathBuf {
    if let Some(path) = relative_path.strip_prefix("mods/") {
        instance.minecraft_dir.join("mods").join(path)
    } else {
        instance.instance_path.join(relative_path)
    }
}

fn remove_file_if_exists(path: &Path) -> Result<(), AssetError> {
    if path.is_file() {
        fs::remove_file(path)?;
    }
    Ok(())
}

fn remove_dir_if_exists(path: &Path) -> Result<(), AssetError> {
    if path.exists() {
        fs::remove_dir_all(path)?;
    }
    Ok(())
}

fn done(label: &str) -> MpbPatchProgressStep {
    MpbPatchProgressStep {
        label: label.to_string(),
        status: "done".to_string(),
    }
}

fn checksum(bytes: &[u8]) -> String {
    let mut hasher = PatchFnv1a64::default();
    hasher.write(bytes);
    format!("{:016x}", hasher.finish())
}

fn timestamp_string() -> String {
    let seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0);
    format!("unix:{seconds}")
}

struct PatchFnv1a64(u64);

impl Default for PatchFnv1a64 {
    fn default() -> Self {
        Self(0xcbf29ce484222325)
    }
}

impl Hasher for PatchFnv1a64 {
    fn finish(&self) -> u64 {
        self.0
    }

    fn write(&mut self, bytes: &[u8]) {
        for byte in bytes {
            self.0 ^= u64::from(*byte);
            self.0 = self.0.wrapping_mul(0x100000001b3);
        }
    }
}
