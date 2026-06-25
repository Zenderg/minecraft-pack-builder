use std::path::{Path, PathBuf};

use mpb_assets::{
    apply_mpb_patch, evaluate_mpb_patch, remove_mpb_patch, validate_prism_root, MpbPatchAction,
    MpbPatchOperationResult, MpbPatchStatus, PrismInstanceDescriptor,
};
use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PatcherInstanceSummary {
    pub instance_id: String,
    pub display_name: String,
    pub instance_path: PathBuf,
    pub minecraft_dir: PathBuf,
    pub minecraft_version: Option<String>,
    pub loader: Option<String>,
    pub loader_version: Option<String>,
    pub patch_status: String,
    pub patch_reason: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PatcherOperationSummary {
    pub status: String,
    pub steps: Vec<PatcherProgressStepSummary>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PatcherProgressStepSummary {
    pub label: String,
    pub status: String,
}

pub fn patcher_instances_for_root(
    root_path: impl AsRef<Path>,
) -> Result<Vec<PatcherInstanceSummary>, String> {
    let validation = validate_prism_root(root_path).map_err(|error| error.to_string())?;
    if !validation.valid {
        return Err(validation.message);
    }
    Ok(validation.instances.iter().map(instance_summary).collect())
}

pub fn patch_prism_instance_path(
    instance_path: impl AsRef<Path>,
    action: &str,
) -> Result<PatcherOperationSummary, String> {
    let instance = descriptor_for_instance_path(instance_path)?;
    let action = match action {
        "apply" => MpbPatchAction::Apply,
        "update" => MpbPatchAction::Update,
        "repair" => MpbPatchAction::Repair,
        other => return Err(format!("Unsupported patch action: {other}")),
    };
    apply_mpb_patch(&instance, action)
        .map(operation_summary)
        .map_err(|error| error.to_string())
}

pub fn remove_patch_for_instance_path(
    instance_path: impl AsRef<Path>,
    delete_schemes: bool,
) -> Result<PatcherOperationSummary, String> {
    let instance = descriptor_for_instance_path(instance_path)?;
    remove_mpb_patch(&instance, delete_schemes)
        .map(operation_summary)
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn list_patcher_instances(root_path: PathBuf) -> Result<Vec<PatcherInstanceSummary>, String> {
    patcher_instances_for_root(root_path)
}

#[tauri::command]
pub fn patch_prism_instance(
    instance_path: PathBuf,
    action: String,
) -> Result<PatcherOperationSummary, String> {
    patch_prism_instance_path(instance_path, &action)
}

#[tauri::command]
pub fn remove_prism_instance_patch(
    instance_path: PathBuf,
    delete_schemes: bool,
) -> Result<PatcherOperationSummary, String> {
    remove_patch_for_instance_path(instance_path, delete_schemes)
}

fn instance_summary(instance: &PrismInstanceDescriptor) -> PatcherInstanceSummary {
    let evaluation = evaluate_mpb_patch(instance);
    PatcherInstanceSummary {
        instance_id: instance.instance_id.clone(),
        display_name: instance.display_name.clone(),
        instance_path: instance.instance_path.clone(),
        minecraft_dir: instance.minecraft_dir.clone(),
        minecraft_version: instance.minecraft_version.clone(),
        loader: instance.loader.clone(),
        loader_version: instance.loader_version.clone(),
        patch_status: status_name(evaluation.status).to_string(),
        patch_reason: evaluation.reason,
    }
}

fn operation_summary(result: MpbPatchOperationResult) -> PatcherOperationSummary {
    PatcherOperationSummary {
        status: status_name(result.status).to_string(),
        steps: result
            .steps
            .into_iter()
            .map(|step| PatcherProgressStepSummary {
                label: step.label,
                status: step.status,
            })
            .collect(),
    }
}

fn descriptor_for_instance_path(
    instance_path: impl AsRef<Path>,
) -> Result<PrismInstanceDescriptor, String> {
    let instance_path = instance_path.as_ref();
    let instances_dir = instance_path
        .parent()
        .ok_or_else(|| "Instance path has no parent directory.".to_string())?;
    let root = instances_dir.parent().ok_or_else(|| {
        "Instance path is not inside a PrismLauncher instances directory.".to_string()
    })?;
    let validation = validate_prism_root(root).map_err(|error| error.to_string())?;
    validation
        .instances
        .into_iter()
        .find(|instance| same_path(&instance.instance_path, instance_path))
        .ok_or_else(|| {
            format!(
                "PrismLauncher instance was not found at {}.",
                instance_path.display()
            )
        })
}

fn same_path(left: &Path, right: &Path) -> bool {
    left == right
        || (left.exists()
            && right.exists()
            && left.canonicalize().ok() == right.canonicalize().ok())
}

fn status_name(status: MpbPatchStatus) -> &'static str {
    match status {
        MpbPatchStatus::NotPatched => "notPatched",
        MpbPatchStatus::Patched => "patched",
        MpbPatchStatus::NeedsUpdate => "needsUpdate",
        MpbPatchStatus::NeedsRepair => "needsRepair",
        MpbPatchStatus::Conflict => "conflict",
        MpbPatchStatus::Unsupported => "unsupported",
        MpbPatchStatus::InstanceRunning => "instanceRunning",
    }
}
