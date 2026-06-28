use std::collections::BTreeMap;
use std::fs;
use std::hash::Hasher;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum FingerprintError {
    #[error("file operation failed: {0}")]
    Io(#[from] std::io::Error),
    #[error("metadata could not be parsed: {0}")]
    Parse(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TargetFingerprint {
    pub fingerprint: String,
    pub document: FingerprintDocument,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FingerprintDocument {
    pub modpack_identity: Option<String>,
    pub modpack_version: Option<String>,
    pub minecraft_version: Option<String>,
    pub loader: Option<String>,
    pub loader_version: Option<String>,
    pub builder_version: String,
    pub lab_tooling_version: String,
    pub knowledge_schema_version: String,
    pub inputs: Vec<FingerprintInput>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FingerprintInput {
    pub role: String,
    pub path: String,
    pub byte_len: u64,
    pub checksum: String,
}

#[derive(Debug, serde::Deserialize)]
struct MmcPack {
    #[serde(default)]
    components: Vec<MmcComponent>,
}

#[derive(Debug, serde::Deserialize)]
struct MmcComponent {
    uid: String,
    version: Option<String>,
}

pub fn compute_target_fingerprint(
    instance_path: impl AsRef<Path>,
    builder_version: &str,
    lab_version: &str,
    schema_version: &str,
) -> Result<TargetFingerprint, FingerprintError> {
    let document =
        collect_fingerprint_document(instance_path, builder_version, lab_version, schema_version)?;
    let canonical = serde_json::to_string(&document)
        .map_err(|error| FingerprintError::Parse(error.to_string()))?;
    Ok(TargetFingerprint {
        fingerprint: stable_checksum(canonical.as_bytes()),
        document,
    })
}

pub fn collect_fingerprint_document(
    instance_path: impl AsRef<Path>,
    builder_version: &str,
    lab_version: &str,
    schema_version: &str,
) -> Result<FingerprintDocument, FingerprintError> {
    let instance_path = instance_path.as_ref();
    let cfg = parse_instance_cfg(&read_optional(instance_path.join("instance.cfg"))?);
    let pack = read_mmc_pack(instance_path.join("mmc-pack.json"))?;
    let (minecraft_version, loader, loader_version) = component_summary(&pack.components);
    let minecraft_dir = minecraft_dir_for_instance(instance_path);
    let mut inputs = Vec::new();

    collect_optional_file(instance_path, "metadata", "instance.cfg", &mut inputs)?;
    collect_optional_file(instance_path, "metadata", "mmc-pack.json", &mut inputs)?;
    for (role, relative) in [
        ("mods", "mods"),
        ("config", "config"),
        ("datapacks", "datapacks"),
        ("kubejs", "kubejs"),
        ("scripts", "scripts"),
        ("resourcepacks", "resourcepacks"),
        ("resourcepacks", "texturepacks"),
        ("resourcepacks", "shaderpacks"),
        ("datapacks", "datapack"),
    ] {
        collect_dir(&minecraft_dir, role, relative, &mut inputs)?;
    }
    inputs.sort_by(|left, right| {
        left.path
            .cmp(&right.path)
            .then_with(|| left.role.cmp(&right.role))
    });

    Ok(FingerprintDocument {
        modpack_identity: cfg.get("name").cloned(),
        modpack_version: cfg
            .get("ManagedPackVersionName")
            .or_else(|| cfg.get("managedPackVersionName"))
            .cloned(),
        minecraft_version,
        loader,
        loader_version,
        builder_version: builder_version.to_string(),
        lab_tooling_version: lab_version.to_string(),
        knowledge_schema_version: schema_version.to_string(),
        inputs,
    })
}

fn read_optional(path: PathBuf) -> Result<Option<String>, FingerprintError> {
    if path.is_file() {
        Ok(Some(fs::read_to_string(path)?))
    } else {
        Ok(None)
    }
}

fn read_mmc_pack(path: PathBuf) -> Result<MmcPack, FingerprintError> {
    if !path.is_file() {
        return Ok(MmcPack {
            components: Vec::new(),
        });
    }
    let json = fs::read_to_string(path)?;
    serde_json::from_str(&json).map_err(|error| FingerprintError::Parse(error.to_string()))
}

fn minecraft_dir_for_instance(instance_path: &Path) -> PathBuf {
    let prism_dir = instance_path.join("minecraft");
    if prism_dir.is_dir() {
        prism_dir
    } else {
        instance_path.join(".minecraft")
    }
}

fn parse_instance_cfg(content: &Option<String>) -> BTreeMap<String, String> {
    content
        .as_deref()
        .unwrap_or_default()
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
            "net.minecraft" => minecraft_version = component.version.clone(),
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

fn collect_optional_file(
    base: &Path,
    role: &str,
    relative: &str,
    inputs: &mut Vec<FingerprintInput>,
) -> Result<(), FingerprintError> {
    let path = base.join(relative);
    if path.is_file() {
        inputs.push(file_input(role, Path::new(relative), &path)?);
    }
    Ok(())
}

fn collect_dir(
    minecraft_dir: &Path,
    role: &str,
    relative: &str,
    inputs: &mut Vec<FingerprintInput>,
) -> Result<(), FingerprintError> {
    let dir = minecraft_dir.join(relative);
    if !dir.is_dir() {
        return Ok(());
    }
    let mut stack = vec![dir.clone()];
    while let Some(current) = stack.pop() {
        let mut entries = fs::read_dir(&current)?.collect::<Result<Vec<_>, _>>()?;
        entries.sort_by_key(|entry| entry.path());
        for entry in entries {
            let path = entry.path();
            if entry.file_type()?.is_dir() {
                stack.push(path);
            } else {
                let relative_path = path.strip_prefix(minecraft_dir).unwrap_or(&path);
                inputs.push(file_input(role, relative_path, &path)?);
            }
        }
    }
    Ok(())
}

fn file_input(
    role: &str,
    relative: &Path,
    path: &Path,
) -> Result<FingerprintInput, FingerprintError> {
    let bytes = fs::read(path)?;
    Ok(FingerprintInput {
        role: role.to_string(),
        path: relative.to_string_lossy().replace('\\', "/"),
        byte_len: bytes.len() as u64,
        checksum: stable_checksum(&bytes),
    })
}

pub(crate) fn stable_checksum(bytes: &[u8]) -> String {
    let mut hasher = Fnv1a64::default();
    hasher.write(bytes);
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
