use std::collections::BTreeMap;
use std::fs;
use std::hash::Hasher;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

use serde::{Deserialize, Serialize};

use crate::AssetError;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PrismRootValidation {
    pub root_path: PathBuf,
    pub valid: bool,
    pub message: String,
    pub instance_count: usize,
    pub instances: Vec<PrismInstanceDescriptor>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum PrismInstanceStatus {
    Pending,
    Indexing,
    Ready,
    Failed,
    Missing,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PrismInstanceDescriptor {
    pub instance_id: String,
    pub display_name: String,
    pub instance_path: PathBuf,
    pub minecraft_dir: PathBuf,
    pub minecraft_version: Option<String>,
    pub loader: Option<String>,
    pub loader_version: Option<String>,
    pub identity_fingerprint: String,
    pub content_fingerprint: String,
    pub status: PrismInstanceStatus,
    pub status_message: Option<String>,
}

#[derive(Debug, Deserialize)]
struct MmcPack {
    #[serde(default)]
    components: Vec<MmcComponent>,
}

#[derive(Debug, Deserialize)]
struct MmcComponent {
    uid: String,
    version: Option<String>,
}

pub fn validate_prism_root(root: impl AsRef<Path>) -> Result<PrismRootValidation, AssetError> {
    let root_path = root.as_ref().to_path_buf();
    let instances_dir = root_path.join("instances");
    if !instances_dir.is_dir() {
        return Ok(PrismRootValidation {
            root_path,
            valid: false,
            message: "Select the PrismLauncher Launcher Root. In PrismLauncher, use Folders > Launcher Root, then select that folder here.".to_string(),
            instance_count: 0,
            instances: Vec::new(),
        });
    }

    let mut instances = Vec::new();
    for entry in fs::read_dir(&instances_dir)? {
        let entry = entry?;
        if !entry.file_type()?.is_dir() {
            continue;
        }
        if let Some(instance) = read_instance(entry.path())? {
            instances.push(instance);
        }
    }
    instances.sort_by(|left, right| {
        left.display_name
            .to_lowercase()
            .cmp(&right.display_name.to_lowercase())
            .then_with(|| left.instance_id.cmp(&right.instance_id))
    });

    let instance_count = instances.len();
    Ok(PrismRootValidation {
        root_path,
        valid: true,
        message: format!(
            "PrismLauncher Launcher Root is valid. {instance_count} instances detected."
        ),
        instance_count,
        instances,
    })
}

fn read_instance(instance_path: PathBuf) -> Result<Option<PrismInstanceDescriptor>, AssetError> {
    let cfg_path = instance_path.join("instance.cfg");
    let pack_path = instance_path.join("mmc-pack.json");
    if !cfg_path.is_file() && !pack_path.is_file() {
        return Ok(None);
    }

    let instance_id = instance_path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("instance")
        .to_string();
    let cfg = if cfg_path.is_file() {
        parse_instance_cfg(&fs::read_to_string(&cfg_path)?)
    } else {
        BTreeMap::new()
    };
    let pack = if pack_path.is_file() {
        let json = fs::read_to_string(&pack_path)?;
        serde_json::from_str::<MmcPack>(&json).map_err(|error| {
            AssetError::InvalidAssetIndex(format!(
                "Prism mmc-pack.json could not be parsed: {error}"
            ))
        })?
    } else {
        MmcPack {
            components: Vec::new(),
        }
    };
    let display_name = cfg
        .get("name")
        .filter(|value| !value.trim().is_empty())
        .cloned()
        .unwrap_or_else(|| instance_id.clone());
    let (minecraft_version, loader, loader_version) = component_summary(&pack.components);
    let minecraft_dir = minecraft_dir_for_instance(&instance_path);
    let identity_input = stable_identity_input(
        &display_name,
        &minecraft_version,
        &loader,
        &loader_version,
        &pack.components,
    );
    let content_input = stable_content_input(&identity_input, &instance_path)?;

    Ok(Some(PrismInstanceDescriptor {
        instance_id,
        display_name,
        instance_path,
        minecraft_dir,
        minecraft_version,
        loader,
        loader_version,
        identity_fingerprint: deterministic_fingerprint(&identity_input),
        content_fingerprint: deterministic_fingerprint(&content_input),
        status: PrismInstanceStatus::Pending,
        status_message: None,
    }))
}

fn minecraft_dir_for_instance(instance_path: &Path) -> PathBuf {
    let prism_dir = instance_path.join("minecraft");
    if prism_dir.is_dir() {
        prism_dir
    } else {
        instance_path.join(".minecraft")
    }
}

fn parse_instance_cfg(content: &str) -> BTreeMap<String, String> {
    content
        .lines()
        .filter_map(|line| {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                return None;
            }
            let (key, value) = line.split_once('=')?;
            Some((key.trim().to_string(), value.trim().to_string()))
        })
        .collect()
}

fn component_summary(
    components: &[MmcComponent],
) -> (Option<String>, Option<String>, Option<String>) {
    let mut minecraft_version = None;
    let mut loader = None;
    let mut loader_version = None;

    for component in components {
        match component.uid.as_str() {
            "net.minecraft" => {
                minecraft_version = component.version.clone();
            }
            uid if uid.contains("neoforged") => {
                loader = Some("NeoForge".to_string());
                loader_version = component.version.clone();
            }
            uid if uid.contains("minecraftforge") || uid.contains("forge") => {
                loader = Some("Forge".to_string());
                loader_version = component.version.clone();
            }
            uid if uid.contains("fabric") => {
                loader = Some("Fabric".to_string());
                loader_version = component.version.clone();
            }
            uid if uid.contains("quilt") => {
                loader = Some("Quilt".to_string());
                loader_version = component.version.clone();
            }
            _ => {}
        }
    }

    (minecraft_version, loader, loader_version)
}

fn stable_identity_input(
    display_name: &str,
    minecraft_version: &Option<String>,
    loader: &Option<String>,
    loader_version: &Option<String>,
    components: &[MmcComponent],
) -> String {
    let mut component_lines = components
        .iter()
        .map(|component| {
            format!(
                "{}={}",
                component.uid,
                component.version.clone().unwrap_or_default()
            )
        })
        .collect::<Vec<_>>();
    component_lines.sort();
    format!(
        "name={}\nmc={}\nloader={}\nloaderVersion={}\ncomponents=\n{}",
        display_name.trim(),
        minecraft_version.clone().unwrap_or_default(),
        loader.clone().unwrap_or_default(),
        loader_version.clone().unwrap_or_default(),
        component_lines.join("\n")
    )
}

fn stable_content_input(identity_input: &str, instance_path: &Path) -> Result<String, AssetError> {
    let mut lines = vec![identity_input.to_string()];
    let minecraft_dir = minecraft_dir_for_instance(instance_path);
    collect_file_fingerprints(&instance_path.join("instance.cfg"), &mut lines)?;
    collect_file_fingerprints(&instance_path.join("mmc-pack.json"), &mut lines)?;
    for (label, relative) in [
        ("mods", "mods"),
        ("config", "config"),
        ("datapacks", "datapacks"),
        ("kubejs", "kubejs"),
        ("scripts", "scripts"),
        ("resourcepacks", "resourcepacks"),
        ("resourcepacks", "texturepacks"),
        ("resourcepacks", "shaderpacks"),
    ] {
        collect_dir_fingerprints(&minecraft_dir.join(relative), label, &mut lines)?;
    }
    lines.sort();
    Ok(lines.join("\n"))
}

fn collect_dir_fingerprints(
    dir: &Path,
    label: &str,
    lines: &mut Vec<String>,
) -> Result<(), AssetError> {
    if !dir.is_dir() {
        return Ok(());
    }
    let mut stack = vec![dir.to_path_buf()];
    while let Some(current) = stack.pop() {
        for entry in fs::read_dir(&current)? {
            let entry = entry?;
            let path = entry.path();
            if entry.file_type()?.is_dir() {
                stack.push(path);
                continue;
            }
            let relative = path.strip_prefix(dir).unwrap_or(&path);
            lines.push(file_fingerprint_line(label, relative, &path)?);
        }
    }
    Ok(())
}

fn collect_file_fingerprints(path: &Path, lines: &mut Vec<String>) -> Result<(), AssetError> {
    if path.is_file() {
        let file_name = path
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or("file");
        lines.push(file_fingerprint_line(
            "metadata",
            Path::new(file_name),
            path,
        )?);
    }
    Ok(())
}

fn file_fingerprint_line(label: &str, relative: &Path, path: &Path) -> Result<String, AssetError> {
    let metadata = fs::metadata(path)?;
    let modified = metadata
        .modified()
        .ok()
        .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
        .map(|duration| duration.as_secs())
        .unwrap_or(0);
    Ok(format!(
        "{label}:{}:{}:{modified}",
        relative.to_string_lossy().replace('\\', "/"),
        metadata.len(),
    ))
}

fn deterministic_fingerprint(input: &str) -> String {
    let mut hasher = Fnv1a64::default();
    hasher.write(input.as_bytes());
    format!("{:016x}", hasher.finish())
}

struct Fnv1a64(u64);

impl Default for Fnv1a64 {
    fn default() -> Self {
        Self(0xcbf29ce484222325)
    }
}

impl Hasher for Fnv1a64 {
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
