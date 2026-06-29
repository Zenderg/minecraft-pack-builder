use std::env;
use std::ffi::OsString;
use std::fmt;
use std::fs;
use std::path::{Component, Path, PathBuf};
use std::process::Command;

use serde::{Deserialize, Serialize};
use serde_json::json;
use thiserror::Error;

use crate::fingerprint::FingerprintError;
use crate::{
    compute_target_fingerprint, FingerprintDocument, KnowledgeRunPhase, KnowledgeRunStore,
    PhaseCheckpointStatus, RunStateError, TargetFingerprint,
};

const DEFAULT_LAB_TOOLING_VERSION: &str = "mpb-knowledge-lab";
const DEFAULT_KNOWLEDGE_SCHEMA_VERSION: &str = "mpb-knowledge-schema";

#[derive(Debug, Error)]
pub enum TargetError {
    #[error("file operation failed: {0}")]
    Io(#[from] std::io::Error),
    #[error("fingerprint operation failed: {0}")]
    Fingerprint(#[from] FingerprintError),
    #[error("run state operation failed: {0}")]
    RunState(#[from] RunStateError),
    #[error("json operation failed: {0}")]
    Json(#[from] serde_json::Error),
    #[error("target instance is not a directory: {0}")]
    InstanceNotDirectory(PathBuf),
    #[error(
        "original target fingerprint changed during clone creation: before {before}, after {after}"
    )]
    OriginalFingerprintChanged { before: String, after: String },
    #[error("instrumentation path must be relative and must not contain parent components: {0}")]
    UnsafeInstrumentationPath(String),
    #[error("refusing to operate outside disposable clone path: {path}")]
    UnsafeClonePath { path: PathBuf },
    #[error("run has no recorded target clone artifact")]
    MissingCloneArtifact,
    #[error("launcher command is empty")]
    EmptyLauncherCommand,
}

#[derive(Debug, Clone)]
pub struct TargetManager {
    artifact_root: PathBuf,
    builder_version: String,
    lab_tooling_version: String,
    knowledge_schema_version: String,
    launcher_command: Option<Vec<OsString>>,
}

impl TargetManager {
    pub fn new(artifact_root: impl AsRef<Path>) -> Self {
        Self {
            artifact_root: artifact_root.as_ref().to_path_buf(),
            builder_version: env!("CARGO_PKG_VERSION").to_string(),
            lab_tooling_version: DEFAULT_LAB_TOOLING_VERSION.to_string(),
            knowledge_schema_version: DEFAULT_KNOWLEDGE_SCHEMA_VERSION.to_string(),
            launcher_command: default_launcher_command(),
        }
    }

    pub fn with_fingerprint_versions(
        mut self,
        builder_version: impl Into<String>,
        lab_tooling_version: impl Into<String>,
        knowledge_schema_version: impl Into<String>,
    ) -> Self {
        self.builder_version = builder_version.into();
        self.lab_tooling_version = lab_tooling_version.into();
        self.knowledge_schema_version = knowledge_schema_version.into();
        self
    }

    pub fn with_launcher_command<I, S>(mut self, command: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<OsString>,
    {
        self.launcher_command = Some(command.into_iter().map(Into::into).collect());
        self
    }

    pub fn inspect_original(
        &self,
        instance_path: impl AsRef<Path>,
    ) -> Result<TargetInspection, TargetError> {
        let instance_path = instance_path.as_ref();
        if !instance_path.is_dir() {
            return Err(TargetError::InstanceNotDirectory(
                instance_path.to_path_buf(),
            ));
        }
        let fingerprint = self.compute_fingerprint(instance_path)?;
        Ok(TargetInspection {
            source_path: instance_path.to_path_buf(),
            metadata: TargetMetadata::from_document(&fingerprint.document),
            fingerprint,
        })
    }

    pub fn create_disposable_clone(
        &self,
        run_id: &str,
        instance_path: impl AsRef<Path>,
    ) -> Result<DisposableClone, TargetError> {
        let original = self.inspect_original(instance_path)?;
        let clone_path = self.clone_path(run_id);
        let clone_parent = clone_path
            .parent()
            .expect("clone path always has parent")
            .to_path_buf();
        fs::create_dir_all(&clone_parent)?;
        if clone_path.exists() {
            fs::remove_dir_all(&clone_path)?;
        }
        copy_dir_all(&original.source_path, &clone_path)?;

        let fingerprint_after = self.compute_fingerprint(&original.source_path)?;
        if fingerprint_after.fingerprint != original.fingerprint.fingerprint {
            return Err(TargetError::OriginalFingerprintChanged {
                before: original.fingerprint.fingerprint,
                after: fingerprint_after.fingerprint,
            });
        }

        let fingerprint_before = original.fingerprint.fingerprint.clone();
        let clone = DisposableClone {
            run_id: run_id.to_string(),
            source_path: original.source_path.clone(),
            clone_path,
            fingerprint_before,
            fingerprint_after: fingerprint_after.fingerprint.clone(),
            cleanup_policy: CleanupPolicy::DeleteAfterReport,
            metadata: original.metadata.clone(),
        };
        let store = KnowledgeRunStore::open(&self.artifact_root, run_id)?;
        store.record_run(
            Some(&clone.fingerprint_before),
            json!({
                "createdBy": "mpb-knowledge target clone",
                "instancePath": clone.source_path,
                "metadata": clone.metadata,
            }),
        )?;
        store.record_artifact_ref(
            "target-original",
            &clone.source_path,
            Some(&clone.fingerprint_before),
            json!({
                "readOnly": true,
                "metadata": clone.metadata,
            }),
        )?;
        store.record_artifact_ref(
            "target-clone",
            &clone.clone_path,
            Some(&clone.fingerprint_after),
            json!({
                "sourcePath": clone.source_path,
                "cleanupPolicy": clone.cleanup_policy,
                "fingerprintBefore": clone.fingerprint_before,
                "fingerprintAfter": clone.fingerprint_after,
            }),
        )?;
        store.record_phase_checkpoint(
            KnowledgeRunPhase::Fingerprint,
            PhaseCheckpointStatus::Succeeded,
            Some(&clone.fingerprint_before),
            serde_json::to_value(&original.fingerprint)?,
        )?;
        store.record_phase_checkpoint(
            KnowledgeRunPhase::Clone,
            PhaseCheckpointStatus::Succeeded,
            Some(&clone.fingerprint_after),
            serde_json::to_value(&clone)?,
        )?;
        Ok(clone)
    }

    pub fn install_clone_instrumentation(
        &self,
        run_id: &str,
        clone_path: impl AsRef<Path>,
        relative_path: impl AsRef<Path>,
        bytes: &[u8],
    ) -> Result<PathBuf, TargetError> {
        let clone_path = clone_path.as_ref();
        self.ensure_expected_clone_path(run_id, clone_path)?;
        let relative_path = validate_instrumentation_path(relative_path.as_ref())?;
        let target_path = clone_path.join(&relative_path);
        if let Some(parent) = target_path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&target_path, bytes)?;

        let store = KnowledgeRunStore::open(&self.artifact_root, run_id)?;
        store.record_artifact_ref(
            "target-clone-instrumentation",
            &target_path,
            None,
            json!({
                "clonePath": clone_path,
                "relativePath": relative_path,
                "byteLength": bytes.len(),
            }),
        )?;
        store.append_event(
            "target.instrumentation_installed",
            None,
            json!({
                "clonePath": clone_path,
                "path": target_path,
            }),
        )?;
        Ok(target_path)
    }

    pub fn set_cleanup_policy(
        &self,
        run_id: &str,
        policy: CleanupPolicy,
        clone: Option<&DisposableClone>,
    ) -> Result<(), TargetError> {
        let store = KnowledgeRunStore::open(&self.artifact_root, run_id)?;
        let target_fingerprint = clone.map(|clone| clone.fingerprint_after.as_str());
        store.append_event(
            "target.cleanup_policy_set",
            target_fingerprint,
            json!({
                "cleanupPolicy": policy,
                "clonePath": clone.map(|clone| clone.clone_path.clone()),
            }),
        )?;
        Ok(())
    }

    pub fn cleanup_clone(
        &self,
        run_id: &str,
        clone_path: impl AsRef<Path>,
        policy: CleanupPolicy,
        run_succeeded: bool,
    ) -> Result<CleanupOutcome, TargetError> {
        let clone_path = clone_path.as_ref();
        self.ensure_expected_clone_path(run_id, clone_path)?;
        let should_delete = match policy {
            CleanupPolicy::KeepForDebugging => false,
            CleanupPolicy::DeleteOnSuccess => run_succeeded,
            CleanupPolicy::DeleteAfterReport => false,
        };
        if should_delete && clone_path.exists() {
            fs::remove_dir_all(clone_path)?;
        }
        let outcome = CleanupOutcome {
            policy,
            deleted: should_delete,
            clone_path: clone_path.to_path_buf(),
        };
        let store = KnowledgeRunStore::open(&self.artifact_root, run_id)?;
        store.append_event(
            "target.cleanup_completed",
            None,
            serde_json::to_value(&outcome)?,
        )?;
        Ok(outcome)
    }

    pub fn probe_launch(&self, run_id: &str) -> Result<LaunchProbeCheckpoint, TargetError> {
        let store = KnowledgeRunStore::open(&self.artifact_root, run_id)?;
        let clone_artifact = store
            .latest_artifact_ref("target-clone")?
            .ok_or(TargetError::MissingCloneArtifact)?;
        let clone_path = PathBuf::from(&clone_artifact.path);
        self.ensure_expected_clone_path(run_id, &clone_path)?;

        let mut attempted_command = self
            .launcher_command
            .clone()
            .unwrap_or_else(|| vec![OsString::from("PrismLauncher"), OsString::from("--launch")]);
        attempted_command.push(clone_path.as_os_str().to_os_string());
        let command_text = attempted_command
            .iter()
            .map(|part| part.to_string_lossy().to_string())
            .collect::<Vec<_>>();

        let (result, observed_status_text) = match self.launcher_command.as_ref() {
            Some(command) if !command.is_empty() && command_path_available(&command[0]) => {
                match run_launcher_command(&attempted_command) {
                    Ok(output) if output.status.success() => {
                        let status_text = output_text(&output);
                        if status_text.contains("mpb launch ready") {
                            (LaunchProbeResult::Ready, Some(status_text))
                        } else {
                            (
                                LaunchProbeResult::ManualInterventionRequired,
                                Some(status_text),
                            )
                        }
                    }
                    Ok(output) => (LaunchProbeResult::LaunchFailed, Some(output_text(&output))),
                    Err(error) => (LaunchProbeResult::LaunchFailed, Some(error.to_string())),
                }
            }
            Some(command) if command.is_empty() => return Err(TargetError::EmptyLauncherCommand),
            _ => (
                LaunchProbeResult::LauncherUnavailable,
                Some("No launcher command configured. Set MPB_KNOWLEDGE_PRISM_LAUNCHER or pass a command through TargetManager.".to_string()),
            ),
        };

        let checkpoint = LaunchProbeCheckpoint {
            run_id: run_id.to_string(),
            phase: KnowledgeRunPhase::Clone,
            result,
            clone_path,
            operating_system: env::consts::OS.to_string(),
            launcher_command_attempted: command_text,
            observed_status_text,
            resume_command: format!(
                "mpb-knowledge target probe-launch {run_id} --artifact-root {}",
                self.artifact_root.display()
            ),
        };
        store.append_event(
            "target.launch_probe",
            clone_artifact.target_fingerprint.as_deref(),
            json!({ "probe": checkpoint }),
        )?;
        store.record_phase_checkpoint(
            KnowledgeRunPhase::Clone,
            PhaseCheckpointStatus::Succeeded,
            clone_artifact.target_fingerprint.as_deref(),
            json!({ "launchProbe": checkpoint }),
        )?;
        Ok(checkpoint)
    }

    pub fn latest_launch_probe(
        &self,
        run_id: &str,
    ) -> Result<Option<LaunchProbeCheckpoint>, TargetError> {
        let store = KnowledgeRunStore::open(&self.artifact_root, run_id)?;
        for event in store.events()?.into_iter().rev() {
            if event.event_kind == "target.launch_probe" {
                return Ok(Some(serde_json::from_value(event.detail["probe"].clone())?));
            }
        }
        Ok(None)
    }

    fn clone_path(&self, run_id: &str) -> PathBuf {
        self.artifact_root
            .join("prism-clones")
            .join(run_id)
            .join("instance")
    }

    fn ensure_expected_clone_path(
        &self,
        run_id: &str,
        clone_path: &Path,
    ) -> Result<(), TargetError> {
        let expected = self.clone_path(run_id);
        if clone_path != expected {
            return Err(TargetError::UnsafeClonePath {
                path: clone_path.to_path_buf(),
            });
        }
        Ok(())
    }

    fn compute_fingerprint(&self, instance_path: &Path) -> Result<TargetFingerprint, TargetError> {
        Ok(compute_target_fingerprint(
            instance_path,
            &self.builder_version,
            &self.lab_tooling_version,
            &self.knowledge_schema_version,
        )?)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TargetInspection {
    pub source_path: PathBuf,
    pub metadata: TargetMetadata,
    pub fingerprint: TargetFingerprint,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TargetMetadata {
    pub modpack_identity: Option<String>,
    pub modpack_version: Option<String>,
    pub minecraft_version: Option<String>,
    pub loader: Option<String>,
    pub loader_version: Option<String>,
}

impl TargetMetadata {
    fn from_document(document: &FingerprintDocument) -> Self {
        Self {
            modpack_identity: document.modpack_identity.clone(),
            modpack_version: document.modpack_version.clone(),
            minecraft_version: document.minecraft_version.clone(),
            loader: document.loader.clone(),
            loader_version: document.loader_version.clone(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DisposableClone {
    pub run_id: String,
    pub source_path: PathBuf,
    pub clone_path: PathBuf,
    pub fingerprint_before: String,
    pub fingerprint_after: String,
    pub cleanup_policy: CleanupPolicy,
    pub metadata: TargetMetadata,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CleanupPolicy {
    KeepForDebugging,
    DeleteOnSuccess,
    DeleteAfterReport,
}

impl fmt::Display for CleanupPolicy {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let value = match self {
            CleanupPolicy::KeepForDebugging => "KeepForDebugging",
            CleanupPolicy::DeleteOnSuccess => "DeleteOnSuccess",
            CleanupPolicy::DeleteAfterReport => "DeleteAfterReport",
        };
        formatter.write_str(value)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CleanupOutcome {
    pub policy: CleanupPolicy,
    pub deleted: bool,
    pub clone_path: PathBuf,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LaunchProbeResult {
    Ready,
    ManualInterventionRequired,
    LauncherUnavailable,
    LaunchFailed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LaunchProbeCheckpoint {
    pub run_id: String,
    pub phase: KnowledgeRunPhase,
    pub result: LaunchProbeResult,
    pub clone_path: PathBuf,
    pub operating_system: String,
    pub launcher_command_attempted: Vec<String>,
    pub observed_status_text: Option<String>,
    pub resume_command: String,
}

fn copy_dir_all(source: &Path, destination: &Path) -> Result<(), std::io::Error> {
    fs::create_dir_all(destination)?;
    let mut entries = fs::read_dir(source)?.collect::<Result<Vec<_>, _>>()?;
    entries.sort_by_key(|entry| entry.path());
    for entry in entries {
        let source_path = entry.path();
        let destination_path = destination.join(entry.file_name());
        let metadata = fs::symlink_metadata(&source_path)?;
        if metadata.file_type().is_symlink() {
            copy_symlink(&source_path, &destination_path)?;
        } else if metadata.is_dir() {
            copy_dir_all(&source_path, &destination_path)?;
        } else if metadata.is_file() {
            fs::copy(&source_path, &destination_path)?;
        }
    }
    Ok(())
}

#[cfg(unix)]
fn copy_symlink(source_path: &Path, destination_path: &Path) -> Result<(), std::io::Error> {
    std::os::unix::fs::symlink(fs::read_link(source_path)?, destination_path)
}

#[cfg(windows)]
fn copy_symlink(source_path: &Path, destination_path: &Path) -> Result<(), std::io::Error> {
    let target = fs::read_link(source_path)?;
    if source_path.is_dir() {
        std::os::windows::fs::symlink_dir(target, destination_path)
    } else {
        std::os::windows::fs::symlink_file(target, destination_path)
    }
}

#[cfg(not(any(unix, windows)))]
fn copy_symlink(source_path: &Path, destination_path: &Path) -> Result<(), std::io::Error> {
    fs::copy(source_path, destination_path).map(|_| ())
}

fn validate_instrumentation_path(relative_path: &Path) -> Result<PathBuf, TargetError> {
    if relative_path.is_absolute()
        || relative_path.components().any(|component| {
            matches!(
                component,
                Component::ParentDir | Component::RootDir | Component::Prefix(_)
            )
        })
    {
        return Err(TargetError::UnsafeInstrumentationPath(
            relative_path.display().to_string(),
        ));
    }
    Ok(relative_path.to_path_buf())
}

fn default_launcher_command() -> Option<Vec<OsString>> {
    env::var_os("MPB_KNOWLEDGE_PRISM_LAUNCHER").map(|path| vec![path, OsString::from("--launch")])
}

fn command_path_available(path: &OsString) -> bool {
    let path = Path::new(path);
    path.is_file()
}

fn run_launcher_command(command: &[OsString]) -> Result<std::process::Output, std::io::Error> {
    if command.is_empty() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "empty launcher command",
        ));
    }
    let mut process = Command::new(&command[0]);
    if command.len() > 1 {
        process.args(&command[1..]);
    }
    process.output()
}

fn output_text(output: &std::process::Output) -> String {
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    [stdout.trim(), stderr.trim()]
        .into_iter()
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}
