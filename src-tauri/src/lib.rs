use std::path::PathBuf;

use mpb_assets::{validate_prism_root, PrismRootValidation};

mod patcher_commands;

pub use patcher_commands::{
    patch_prism_instance_path, patcher_instances_for_root, remove_patch_for_instance_path,
    PatcherInstanceSummary, PatcherOperationSummary, PatcherProgressStepSummary,
};

#[tauri::command]
fn validate_prism_launcher_root(root_path: PathBuf) -> Result<PrismRootValidation, String> {
    validate_prism_root(root_path).map_err(|error| error.to_string())
}

#[tauri::command]
fn discover_prism_launcher_roots() -> Result<Vec<PrismRootValidation>, String> {
    default_prism_root_candidates()
        .into_iter()
        .map(validate_prism_root)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| error.to_string())
}

fn default_prism_root_candidates() -> Vec<PathBuf> {
    let mut candidates = Vec::new();
    let home = std::env::var_os("HOME").map(PathBuf::from);
    if let Some(home) = home {
        if cfg!(target_os = "macos") {
            candidates.push(home.join("Library/Application Support/PrismLauncher"));
        } else if cfg!(target_os = "windows") {
            if let Some(app_data) = std::env::var_os("APPDATA") {
                candidates.push(PathBuf::from(app_data).join("PrismLauncher"));
            }
            candidates.push(home.join("scoop/persist/prismlauncher"));
        } else {
            candidates.push(home.join(".local/share/PrismLauncher"));
            candidates
                .push(home.join(".var/app/org.prismlauncher.PrismLauncher/data/PrismLauncher"));
        }
    }
    candidates
}

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            discover_prism_launcher_roots,
            validate_prism_launcher_root,
            patcher_commands::list_patcher_instances,
            patcher_commands::patch_prism_instance,
            patcher_commands::remove_prism_instance_patch,
        ])
        .run(tauri::generate_context!())
        .expect("failed to run MPB Patcher");
}
