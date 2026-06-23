//! Local storage and app data path helpers.

use std::fs;
use std::path::{Path, PathBuf};

use serde::Serialize;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum StorageError {
    #[error("failed to create application data directory at {path}: {source}")]
    CreateDir {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppDataPaths {
    pub app_data_dir: PathBuf,
    pub diagnostics_dir: PathBuf,
}

pub fn ensure_app_data_dirs(base_dir: impl AsRef<Path>) -> Result<AppDataPaths, StorageError> {
    let app_data_dir = base_dir.as_ref().to_path_buf();
    let diagnostics_dir = app_data_dir.join("diagnostics");

    create_dir(&app_data_dir)?;
    create_dir(&diagnostics_dir)?;

    Ok(AppDataPaths {
        app_data_dir,
        diagnostics_dir,
    })
}

fn create_dir(path: &Path) -> Result<(), StorageError> {
    fs::create_dir_all(path).map_err(|source| StorageError::CreateDir {
        path: path.to_path_buf(),
        source,
    })
}
