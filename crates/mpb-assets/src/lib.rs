//! PrismLauncher discovery and local Minecraft asset indexing helpers.

mod asset_index;
mod prism;

use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

pub use asset_index::{
    build_prism_asset_index, build_prism_asset_index_with_events, AssetIndexEvent,
    AssetIndexProgress, BlockAssetSample, PrismAssetIndexReport, PrismAssetIndexRequest,
    TextureAtlasEntry, TextureAtlasMetadata,
};
pub use prism::{
    validate_prism_root, PrismInstanceDescriptor, PrismInstanceStatus, PrismRootValidation,
};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AssetError {
    #[error("download was cancelled")]
    Cancelled,
    #[error("file operation failed: {0}")]
    Io(#[from] std::io::Error),
    #[error("zip archive could not be parsed: {0}")]
    Zip(String),
    #[error("modpack did not contain any parseable block assets")]
    NoParseableBlocks,
    #[error("asset index could not be parsed: {0}")]
    InvalidAssetIndex(String),
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
