//! CurseForge release discovery, modpack downloads, and asset import helpers.

mod curseforge;
mod import;

use std::io::Read;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

pub use curseforge::{
    discover_modpack_releases, download_release_archive, filter_releases,
    parse_modpack_page_url, search_modpack_projects, CurseForgeHttpGateway, CurseForgeProject,
    CurseForgeRelease, DiscoveredReleases, DownloadProgress, DownloadedArchive,
    ParsedModpackUrl, ReleaseFilter, ReleaseSummary,
};
pub use import::{
    build_modpack_asset_index, build_modpack_asset_index_with_events, AssetImportEvent,
    AssetImportProgress, AssetImportReport, BlockAssetSample, ModpackAssetImportRequest,
    TextureAtlasEntry, TextureAtlasMetadata,
};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AssetError {
    #[error("only CurseForge modpack page URLs are supported")]
    UnsupportedUrl,
    #[error("CurseForge modpack was not found for slug '{slug}'")]
    ModpackNotFound { slug: String },
    #[error("CurseForge API request failed: {0}")]
    Http(String),
    #[error("CurseForge API response was not usable: {0}")]
    Api(String),
    #[error("release file {file_id} is missing a download URL")]
    MissingDownloadUrl { file_id: u64 },
    #[error("download was cancelled")]
    Cancelled,
    #[error("file operation failed: {0}")]
    Io(#[from] std::io::Error),
    #[error("zip archive could not be parsed: {0}")]
    Zip(String),
    #[error("modpack archive is missing manifest.json")]
    MissingManifest,
    #[error("modpack manifest could not be parsed: {0}")]
    Manifest(String),
    #[error("modpack did not contain any parseable block assets")]
    NoParseableBlocks,
}

#[derive(Debug, Clone, Default)]
pub struct CancellationToken {
    cancelled: Arc<AtomicBool>,
}

impl CancellationToken {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::SeqCst);
    }

    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::SeqCst)
    }
}

pub trait CurseForgeGateway {
    fn search_modpack_projects(
        &self,
        api_key: &str,
        query: &str,
    ) -> Result<Vec<CurseForgeProject>, AssetError>;

    fn find_modpack_project(
        &self,
        api_key: &str,
        slug: &str,
    ) -> Result<Option<CurseForgeProject>, AssetError>;

    fn list_project_files(
        &self,
        api_key: &str,
        project_id: u64,
    ) -> Result<Vec<CurseForgeRelease>, AssetError>;

    fn open_download(
        &self,
        api_key: &str,
        release: &CurseForgeRelease,
    ) -> Result<Box<dyn Read>, AssetError>;

    fn open_mod_file_download(
        &self,
        _api_key: &str,
        project_id: u64,
        file_id: u64,
    ) -> Result<Box<dyn Read>, AssetError> {
        Err(AssetError::Api(format!(
            "mod file download is not available for project {project_id} file {file_id}"
        )))
    }
}
