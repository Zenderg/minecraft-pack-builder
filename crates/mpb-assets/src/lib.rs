//! PrismLauncher discovery and MPB patch management helpers.

mod patcher;
mod prism;

use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

pub use patcher::{
    apply_mpb_patch, evaluate_mpb_patch, remove_mpb_patch, MpbFileOwner, MpbManagedFile,
    MpbPatchAction, MpbPatchEvaluation, MpbPatchManifest, MpbPatchOperationResult,
    MpbPatchProgressStep, MpbPatchStatus,
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
    #[error("asset index could not be parsed: {0}")]
    InvalidAssetIndex(String),
    #[error("patch operation failed: {0}")]
    Patch(String),
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
