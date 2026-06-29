use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::KnowledgeRunPhase;

#[derive(Debug, Error)]
pub enum PreflightError {
    #[error("file operation failed: {0}")]
    Io(#[from] std::io::Error),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PreflightReport {
    pub artifact_root: String,
    pub cpu_architecture: String,
    pub operating_system: String,
    pub memory_estimate_bytes: Option<u64>,
    pub repository_disk_free: DiskFreeEstimate,
    pub prism_clone_disk_free: DiskFreeEstimate,
    pub tools: Vec<ToolAvailability>,
    pub prism_instance: PrismInstanceReadiness,
    pub expected_clone_size_bytes: u64,
    pub extraction_scale: ExtractionScaleEstimate,
    pub model_cache: ModelCacheStatus,
    pub keep_awake: KeepAwakeAvailability,
    pub phase_duration_estimates: Vec<PhaseDurationEstimate>,
    pub model_needs: Vec<ModelNeed>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiskFreeEstimate {
    pub path: String,
    pub free_bytes: Option<u64>,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolAvailability {
    pub name: String,
    pub command: String,
    pub available: bool,
    pub version: Option<String>,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PrismInstanceReadiness {
    pub path: String,
    pub readable: bool,
    pub has_instance_cfg: bool,
    pub has_mmc_pack: bool,
    pub has_minecraft_dir: bool,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExtractionScaleEstimate {
    pub file_count: u64,
    pub total_bytes: u64,
    pub mod_file_count: u64,
    pub config_file_count: u64,
    pub resource_file_count: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelCacheStatus {
    pub path: String,
    pub exists: bool,
    pub file_count: u64,
    pub total_bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KeepAwakeAvailability {
    pub available: bool,
    pub command: Option<String>,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PhaseDurationEstimate {
    pub phase: KnowledgeRunPhase,
    pub min_seconds: u64,
    pub max_seconds: u64,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelNeed {
    pub task: String,
    pub candidate_label: String,
    pub expected_size_bytes: u64,
    pub runtime_mode: RuntimeMode,
    pub hardware_fit: HardwareFit,
    pub reason: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RuntimeMode {
    LocalCpu,
    LocalGpuPreferred,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HardwareFit {
    Fits,
    Constrained,
    Insufficient,
    Unknown,
}

pub fn run_preflight(
    instance_path: impl AsRef<Path>,
    artifact_root: impl AsRef<Path>,
) -> Result<PreflightReport, PreflightError> {
    let instance_path = instance_path.as_ref();
    let artifact_root = artifact_root.as_ref();
    let scale = estimate_tree(instance_path)?;
    let memory_estimate_bytes = memory_estimate_bytes();
    Ok(PreflightReport {
        artifact_root: artifact_root.display().to_string(),
        cpu_architecture: std::env::consts::ARCH.to_string(),
        operating_system: std::env::consts::OS.to_string(),
        memory_estimate_bytes,
        repository_disk_free: disk_free_estimate(Path::new(".")),
        prism_clone_disk_free: disk_free_estimate(&artifact_root.join("prism-clones")),
        tools: tool_availability(),
        prism_instance: inspect_prism_instance(instance_path),
        expected_clone_size_bytes: scale.total_bytes,
        extraction_scale: scale,
        model_cache: inspect_model_cache(&artifact_root.join("model-cache"))?,
        keep_awake: inspect_keep_awake(),
        phase_duration_estimates: phase_duration_estimates(),
        model_needs: model_needs(memory_estimate_bytes),
    })
}

fn inspect_prism_instance(instance_path: &Path) -> PrismInstanceReadiness {
    let readable = fs::read_dir(instance_path).is_ok();
    let has_instance_cfg = instance_path.join("instance.cfg").is_file();
    let has_mmc_pack = instance_path.join("mmc-pack.json").is_file();
    let has_minecraft_dir =
        instance_path.join("minecraft").is_dir() || instance_path.join(".minecraft").is_dir();
    let reason = if readable {
        "Prism instance path is readable without mutation.".to_string()
    } else {
        "Prism instance path is not readable.".to_string()
    };
    PrismInstanceReadiness {
        path: instance_path.display().to_string(),
        readable,
        has_instance_cfg,
        has_mmc_pack,
        has_minecraft_dir,
        reason,
    }
}

fn inspect_model_cache(path: &Path) -> Result<ModelCacheStatus, PreflightError> {
    if !path.exists() {
        return Ok(ModelCacheStatus {
            path: path.display().to_string(),
            exists: false,
            file_count: 0,
            total_bytes: 0,
        });
    }
    let estimate = estimate_tree(path)?;
    Ok(ModelCacheStatus {
        path: path.display().to_string(),
        exists: true,
        file_count: estimate.file_count,
        total_bytes: estimate.total_bytes,
    })
}

fn estimate_tree(path: &Path) -> Result<ExtractionScaleEstimate, PreflightError> {
    let mut estimate = ExtractionScaleEstimate {
        file_count: 0,
        total_bytes: 0,
        mod_file_count: 0,
        config_file_count: 0,
        resource_file_count: 0,
    };
    if path.exists() {
        collect_tree_estimate(path, path, &mut estimate)?;
    }
    Ok(estimate)
}

fn collect_tree_estimate(
    root: &Path,
    path: &Path,
    estimate: &mut ExtractionScaleEstimate,
) -> Result<(), PreflightError> {
    for entry in fs::read_dir(path)? {
        let entry = entry?;
        let path = entry.path();
        let metadata = entry.metadata()?;
        if metadata.is_dir() {
            collect_tree_estimate(root, &path, estimate)?;
        } else if metadata.is_file() {
            estimate.file_count += 1;
            estimate.total_bytes += metadata.len();
            let relative = path.strip_prefix(root).unwrap_or(&path);
            let relative_text = relative.to_string_lossy().replace('\\', "/");
            if relative_text.contains("/mods/") || relative_text.starts_with("mods/") {
                estimate.mod_file_count += 1;
            } else if relative_text.contains("/config/") || relative_text.starts_with("config/") {
                estimate.config_file_count += 1;
            } else if relative_text.contains("/resourcepacks/")
                || relative_text.starts_with("resourcepacks/")
                || relative_text.contains("/kubejs/")
                || relative_text.starts_with("kubejs/")
                || relative_text.contains("/datapacks/")
                || relative_text.starts_with("datapacks/")
            {
                estimate.resource_file_count += 1;
            }
        }
    }
    Ok(())
}

fn tool_availability() -> Vec<ToolAvailability> {
    vec![
        command_version("Java", "java", &["-version"]),
        command_version("Gradle", "gradle", &["--version"]),
        command_version("Rust", "rustc", &["--version"]),
        command_version("Node", "node", &["--version"]),
        command_version("pnpm", "pnpm", &["--version"]),
        command_version("Tauri", "tauri", &["--version"]),
        command_version("GitHub CLI", "gh", &["--version"]),
    ]
}

fn command_version(name: &str, command: &str, args: &[&str]) -> ToolAvailability {
    match Command::new(command).args(args).output() {
        Ok(output) if output.status.success() => {
            let mut text = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if text.is_empty() {
                text = String::from_utf8_lossy(&output.stderr).trim().to_string();
            }
            ToolAvailability {
                name: name.to_string(),
                command: command.to_string(),
                available: true,
                version: first_line(&text),
                reason: "command is available".to_string(),
            }
        }
        Ok(output) => ToolAvailability {
            name: name.to_string(),
            command: command.to_string(),
            available: false,
            version: None,
            reason: format!("command exited with status {}", output.status),
        },
        Err(error) => ToolAvailability {
            name: name.to_string(),
            command: command.to_string(),
            available: false,
            version: None,
            reason: error.to_string(),
        },
    }
}

fn first_line(text: &str) -> Option<String> {
    text.lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .map(str::to_string)
}

fn disk_free_estimate(path: &Path) -> DiskFreeEstimate {
    let probe_path = existing_probe_path(path);
    let output = Command::new("df").arg("-Pk").arg(&probe_path).output();
    match output {
        Ok(output) if output.status.success() => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let free_bytes = stdout.lines().nth(1).and_then(|line| {
                line.split_whitespace()
                    .nth(3)
                    .and_then(|blocks| blocks.parse::<u64>().ok())
                    .map(|blocks| blocks * 1024)
            });
            DiskFreeEstimate {
                path: path.display().to_string(),
                free_bytes,
                reason: "estimated with df -Pk".to_string(),
            }
        }
        Ok(output) => DiskFreeEstimate {
            path: path.display().to_string(),
            free_bytes: None,
            reason: format!("df exited with status {}", output.status),
        },
        Err(error) => DiskFreeEstimate {
            path: path.display().to_string(),
            free_bytes: None,
            reason: error.to_string(),
        },
    }
}

fn existing_probe_path(path: &Path) -> PathBuf {
    let mut probe = path;
    while !probe.exists() {
        match probe.parent() {
            Some(parent) => probe = parent,
            None => return PathBuf::from("."),
        }
    }
    probe.to_path_buf()
}

fn memory_estimate_bytes() -> Option<u64> {
    if cfg!(target_os = "macos") {
        let output = Command::new("sysctl")
            .arg("-n")
            .arg("hw.memsize")
            .output()
            .ok()?;
        if !output.status.success() {
            return None;
        }
        return String::from_utf8_lossy(&output.stdout).trim().parse().ok();
    }
    if cfg!(target_os = "linux") {
        let meminfo = fs::read_to_string("/proc/meminfo").ok()?;
        let kb = meminfo.lines().find_map(|line| {
            let value = line.strip_prefix("MemTotal:")?;
            value.split_whitespace().next()?.parse::<u64>().ok()
        })?;
        return Some(kb * 1024);
    }
    None
}

fn inspect_keep_awake() -> KeepAwakeAvailability {
    let candidates: &[(&str, &str)] = if cfg!(target_os = "macos") {
        &[(
            "caffeinate",
            "macOS caffeinate is available for approved keep-awake runs",
        )]
    } else if cfg!(target_os = "linux") {
        &[(
            "systemd-inhibit",
            "systemd-inhibit is available for approved keep-awake runs",
        )]
    } else if cfg!(target_os = "windows") {
        &[(
            "powercfg",
            "powercfg is available for manual keep-awake checks",
        )]
    } else {
        &[]
    };
    for (command, reason) in candidates {
        if Command::new(command).arg("--help").output().is_ok() {
            return KeepAwakeAvailability {
                available: true,
                command: Some((*command).to_string()),
                reason: (*reason).to_string(),
            };
        }
    }
    KeepAwakeAvailability {
        available: false,
        command: None,
        reason: "no supported keep-awake command was found; preflight did not enable one"
            .to_string(),
    }
}

fn phase_duration_estimates() -> Vec<PhaseDurationEstimate> {
    KnowledgeRunPhase::ALL
        .into_iter()
        .map(|phase| {
            let (min_seconds, max_seconds) = match phase {
                KnowledgeRunPhase::Intake
                | KnowledgeRunPhase::Preflight
                | KnowledgeRunPhase::Approvals
                | KnowledgeRunPhase::Fingerprint
                | KnowledgeRunPhase::Report => (5, 60),
                KnowledgeRunPhase::Clone
                | KnowledgeRunPhase::Extraction
                | KnowledgeRunPhase::Validation
                | KnowledgeRunPhase::Bundle
                | KnowledgeRunPhase::PatcherIntegration
                | KnowledgeRunPhase::ProductValidation
                | KnowledgeRunPhase::Release => (60, 900),
                KnowledgeRunPhase::Drafting
                | KnowledgeRunPhase::ExperimentPlanning
                | KnowledgeRunPhase::AdapterExpansion
                | KnowledgeRunPhase::RuntimeVerification => (300, 7200),
            };
            PhaseDurationEstimate {
                phase,
                min_seconds,
                max_seconds,
                reason: "static local estimate; no long-running action was started".to_string(),
            }
        })
        .collect()
}

fn model_needs(memory_estimate_bytes: Option<u64>) -> Vec<ModelNeed> {
    let small_model = 2_500_000_000;
    let reasoning_model = 6_000_000_000;
    vec![
        ModelNeed {
            task: "draft classification and schema repair".to_string(),
            candidate_label: "small local instruct model".to_string(),
            expected_size_bytes: small_model,
            runtime_mode: RuntimeMode::LocalCpu,
            hardware_fit: hardware_fit(memory_estimate_bytes, small_model),
            reason: "planned only; preflight does not download or load models".to_string(),
        },
        ModelNeed {
            task: "experiment proposal and lab-log summarization".to_string(),
            candidate_label: "local reasoning model".to_string(),
            expected_size_bytes: reasoning_model,
            runtime_mode: RuntimeMode::LocalGpuPreferred,
            hardware_fit: hardware_fit(memory_estimate_bytes, reasoning_model),
            reason: "planned only; approval is required before model download or fine-tuning"
                .to_string(),
        },
    ]
}

fn hardware_fit(memory_estimate_bytes: Option<u64>, model_size_bytes: u64) -> HardwareFit {
    let Some(memory) = memory_estimate_bytes else {
        return HardwareFit::Unknown;
    };
    if memory >= model_size_bytes * 4 {
        HardwareFit::Fits
    } else if memory >= model_size_bytes * 2 {
        HardwareFit::Constrained
    } else {
        HardwareFit::Insufficient
    }
}
