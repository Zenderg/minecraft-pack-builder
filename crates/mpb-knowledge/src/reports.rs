use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::release::ReleaseError;
use crate::{
    ApprovalGateError, ApprovalKind, ApprovalRecord, ArtifactRef, KnowledgeRunPhase,
    KnowledgeRunStore, RunBlocker,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReportArtifactPaths {
    pub json_path: String,
    pub markdown_path: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BlockingReport {
    pub run_id: String,
    pub target_instance: Option<String>,
    pub fingerprint: Option<String>,
    pub failed_phase: Option<KnowledgeRunPhase>,
    pub exact_blocker: RunBlocker,
    pub affected_coverage_obligations: Vec<String>,
    pub accepted_evidence: Vec<String>,
    pub missing_capability_or_approval: Option<String>,
    pub proposed_action: serde_json::Value,
    pub resume_command: String,
    pub local_artifact_paths: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UnsignedAppWarning {
    pub platform: String,
    pub warning: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReleaseReport {
    pub run_id: String,
    pub target_pack_identity: serde_json::Value,
    pub exact_fingerprint: String,
    pub coverage_summary: serde_json::Value,
    pub evidence_summary_by_kind: serde_json::Value,
    pub model_candidates: Vec<serde_json::Value>,
    pub approvals: Vec<ApprovalRecord>,
    pub worker_evaluations: Vec<serde_json::Value>,
    pub fine_tuning_decisions: Vec<serde_json::Value>,
    pub experiment_summary: serde_json::Value,
    pub retry_statistics: serde_json::Value,
    pub generated_source_paths: Vec<String>,
    pub generated_bundle_paths: Vec<String>,
    pub checksums: serde_json::Value,
    pub compressed_size_bytes: Option<u64>,
    pub patcher_validation: serde_json::Value,
    pub cloned_runtime_validation: serde_json::Value,
    pub mcp_query_validation: serde_json::Value,
    pub desktop_artifact_list: Vec<String>,
    pub unsigned_app_warnings: Vec<UnsignedAppWarning>,
    pub github_release_url: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GithubReleasePreparation {
    pub run_id: String,
    pub tag: String,
    pub fingerprint: String,
    pub pack_id: String,
    pub report_artifact_path: String,
    pub release_notes_path: String,
    pub gh_workflow_command: String,
    pub publication_approved: bool,
    pub missing_approval: Option<String>,
}

pub fn write_blocking_report_artifacts(
    store: &KnowledgeRunStore,
    blocker: &RunBlocker,
) -> Result<ReportArtifactPaths, ReleaseError> {
    let report = build_blocking_report(store, blocker)?;
    let report_dir = store.run_dir().join("reports");
    fs::create_dir_all(&report_dir)?;
    let phase = blocker
        .phase
        .map(|phase| phase.as_str())
        .unwrap_or("unknown");
    let json_path = report_dir.join(format!("blocking-{:04}-{phase}.json", blocker.id));
    let markdown_path = json_path.with_extension("md");
    fs::write(&json_path, serde_json::to_vec_pretty(&report)?)?;
    fs::write(&markdown_path, blocking_report_markdown(&report))?;
    Ok(ReportArtifactPaths {
        json_path: json_path.display().to_string(),
        markdown_path: markdown_path.display().to_string(),
    })
}

pub fn write_release_report_artifacts(
    store: &KnowledgeRunStore,
    github_release_url: Option<String>,
) -> Result<ReportArtifactPaths, ReleaseError> {
    let report = build_release_report(store, github_release_url)?;
    let report_dir = store.run_dir().join("reports");
    fs::create_dir_all(&report_dir)?;
    let json_path = report_dir.join("release-report.json");
    let markdown_path = report_dir.join("release-report.md");
    fs::write(&json_path, serde_json::to_vec_pretty(&report)?)?;
    fs::write(&markdown_path, release_report_markdown(&report))?;
    store.record_artifact_ref(
        "release-report",
        &json_path,
        Some(&report.exact_fingerprint),
        json!({
            "format": "json",
            "markdownPath": markdown_path,
            "githubReleaseUrl": report.github_release_url,
        }),
    )?;
    Ok(ReportArtifactPaths {
        json_path: json_path.display().to_string(),
        markdown_path: markdown_path.display().to_string(),
    })
}

pub fn prepare_github_release_publication(
    store: &KnowledgeRunStore,
    tag: &str,
) -> Result<GithubReleasePreparation, ReleaseError> {
    let approval_check = require_github_release_publication_approval(store);
    let fingerprint = target_fingerprint_for_reports(store);
    let fingerprint = fingerprint.unwrap_or_else(|| "unknown".to_string());
    let pack_id = target_pack_id_for_reports(store).unwrap_or_else(|| "unknown-pack".to_string());
    let notes_path = store.run_dir().join("reports/github-release-notes.md");
    if let Some(parent) = notes_path.parent() {
        fs::create_dir_all(parent)?;
    }
    let report_paths = write_release_report_artifacts(store, None)?;
    let notes = format!(
        "# Minecraft Pack Builder Knowledge Release\n\nTag: `{tag}`\n\nPack: `{pack_id}`\n\nFingerprint: `{fingerprint}`\n\n## Unsigned App Warnings\n\nThe generated macOS, Windows, and Linux desktop artifacts are unsigned unless a separate signing/notarization process is attached.\n"
    );
    fs::write(&notes_path, notes)?;
    let report_artifact_path = report_paths.json_path;
    let gh_workflow_command = format!(
        "gh workflow run release.yml --ref {tag} -f knowledge_run_id={} -f pack_id={} -f fingerprint={} -f report_artifact_path={}",
        store.run_id(), pack_id, fingerprint, report_artifact_path
    );
    let (publication_approved, missing_approval) = match approval_check {
        Ok(()) => (true, None),
        Err(error) => (false, Some(error.to_string())),
    };
    Ok(GithubReleasePreparation {
        run_id: store.run_id().to_string(),
        tag: tag.to_string(),
        fingerprint,
        pack_id,
        report_artifact_path,
        release_notes_path: notes_path.display().to_string(),
        gh_workflow_command,
        publication_approved,
        missing_approval,
    })
}

pub fn require_github_release_publication_approval(
    store: &KnowledgeRunStore,
) -> Result<(), ApprovalGateError> {
    let fingerprint = target_fingerprint_for_reports(store);
    store.require_approval(
        ApprovalKind::GitHubReleasePublication,
        fingerprint.as_deref(),
    )
}

fn build_blocking_report(
    store: &KnowledgeRunStore,
    blocker: &RunBlocker,
) -> Result<BlockingReport, ReleaseError> {
    let artifacts = store.artifact_refs()?;
    let mut local_artifact_paths = artifacts
        .iter()
        .map(|artifact| artifact.path.clone())
        .collect::<Vec<_>>();
    local_artifact_paths.extend(string_array(blocker.detail.get("localArtifactPaths")));
    local_artifact_paths.sort();
    local_artifact_paths.dedup();
    Ok(BlockingReport {
        run_id: store.run_id().to_string(),
        target_instance: target_instance_for_reports(store),
        fingerprint: blocker
            .target_fingerprint
            .clone()
            .or_else(|| target_fingerprint_for_reports(store)),
        failed_phase: blocker.phase,
        exact_blocker: blocker.clone(),
        affected_coverage_obligations: affected_obligations(&blocker.detail),
        accepted_evidence: string_array(blocker.detail.get("acceptedEvidence")),
        missing_capability_or_approval: blocker
            .detail
            .get("missingCapabilityOrApproval")
            .and_then(serde_json::Value::as_str)
            .map(str::to_string),
        proposed_action: blocker
            .detail
            .get("proposedAction")
            .cloned()
            .unwrap_or_else(|| json!(null)),
        resume_command: resume_command_for_store(store),
        local_artifact_paths,
    })
}

fn build_release_report(
    store: &KnowledgeRunStore,
    github_release_url: Option<String>,
) -> Result<ReleaseReport, ReleaseError> {
    let artifacts = store.artifact_refs()?;
    let fingerprint =
        target_fingerprint_for_reports(store).unwrap_or_else(|| "unknown".to_string());
    let runtime_bundle = latest_artifact(&artifacts, "runtime-bundle");
    let product = latest_artifact(&artifacts, "product-validation-report");
    Ok(ReleaseReport {
        run_id: store.run_id().to_string(),
        target_pack_identity: json!({
            "packId": target_pack_id_for_reports(store).unwrap_or_else(|| "unknown-pack".to_string()),
            "targetInstance": target_instance_for_reports(store),
        }),
        exact_fingerprint: fingerprint.clone(),
        coverage_summary: artifact_detail(&artifacts, "coverage-summary"),
        evidence_summary_by_kind: artifact_detail(&artifacts, "evidence-summary"),
        model_candidates: artifact_details(&artifacts, "model-candidate"),
        approvals: approval_records_for_report(store, &fingerprint)?,
        worker_evaluations: artifact_details(&artifacts, "worker-evaluation"),
        fine_tuning_decisions: artifact_details(&artifacts, "fine-tuning-decision"),
        experiment_summary: artifact_detail(&artifacts, "experiment-summary"),
        retry_statistics: artifact_detail(&artifacts, "retry-statistics"),
        generated_source_paths: artifact_paths(&artifacts, "knowledge-source-dir"),
        generated_bundle_paths: artifact_paths(&artifacts, "runtime-bundle"),
        checksums: json!({
            "runtimeBundle": runtime_bundle
                .and_then(|artifact| artifact.detail.get("checksum"))
                .and_then(serde_json::Value::as_str)
                .unwrap_or("unknown"),
        }),
        compressed_size_bytes: runtime_bundle
            .and_then(|artifact| artifact.detail.get("compressedSizeBytes"))
            .and_then(serde_json::Value::as_u64),
        patcher_validation: nested_or_detail(product, "patcherValidation"),
        cloned_runtime_validation: nested_or_detail(product, "clonedRuntimeValidation"),
        mcp_query_validation: nested_or_detail(product, "mcpQueryValidation"),
        desktop_artifact_list: artifact_paths(&artifacts, "desktop-artifact"),
        unsigned_app_warnings: unsigned_app_warnings(),
        github_release_url,
    })
}

fn approval_records_for_report(
    store: &KnowledgeRunStore,
    fingerprint: &str,
) -> Result<Vec<ApprovalRecord>, ReleaseError> {
    let mut approvals = Vec::new();
    for kind in ApprovalKind::ALL {
        approvals.extend(store.approval_history(kind, None)?);
        approvals.extend(store.approval_history(kind, Some(fingerprint))?);
    }
    approvals.sort_by_key(|approval| approval.id);
    approvals.dedup_by_key(|approval| approval.id);
    Ok(approvals)
}

fn blocking_report_markdown(report: &BlockingReport) -> String {
    format!(
        "# Blocking Report\n\nRun: `{}`\n\nTarget instance: `{}`\n\nFingerprint: `{}`\n\nFailed phase: `{}`\n\nBlocker: `{}`\n\n{}\n\nResume:\n\n```bash\n{}\n```\n",
        report.run_id,
        report.target_instance.as_deref().unwrap_or("unknown"),
        report.fingerprint.as_deref().unwrap_or("unknown"),
        report
            .failed_phase
            .map(|phase| phase.as_str())
            .unwrap_or("unknown"),
        report.exact_blocker.code,
        report.exact_blocker.message,
        report.resume_command,
    )
}

fn release_report_markdown(report: &ReleaseReport) -> String {
    let warnings = report
        .unsigned_app_warnings
        .iter()
        .map(|warning| format!("- {}: {}", warning.platform, warning.warning))
        .collect::<Vec<_>>()
        .join("\n");
    format!(
        "# Release Report\n\nRun: `{}`\n\nPack: `{}`\n\nFingerprint: `{}`\n\nRuntime bundle checksum: `{}`\n\nCompressed size: `{}` bytes\n\n## Unsigned App Warnings\n\n{}\n",
        report.run_id,
        report
            .target_pack_identity
            .get("packId")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("unknown-pack"),
        report.exact_fingerprint,
        report
            .checksums
            .get("runtimeBundle")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("unknown"),
        report.compressed_size_bytes.unwrap_or(0),
        warnings,
    )
}

fn target_instance_for_reports(store: &KnowledgeRunStore) -> Option<String> {
    store
        .run()
        .ok()
        .flatten()
        .and_then(|run| string_field(&run.detail, "instancePath"))
        .or_else(|| {
            store
                .phase_checkpoints()
                .ok()?
                .into_iter()
                .find(|checkpoint| checkpoint.phase == KnowledgeRunPhase::Intake)
                .and_then(|checkpoint| string_field(&checkpoint.detail, "instancePath"))
        })
}

fn target_fingerprint_for_reports(store: &KnowledgeRunStore) -> Option<String> {
    store
        .run()
        .ok()
        .flatten()
        .and_then(|run| run.target_fingerprint)
        .or_else(|| {
            store
                .phase_checkpoints()
                .ok()?
                .into_iter()
                .rev()
                .find_map(|checkpoint| checkpoint.target_fingerprint)
        })
}

fn target_pack_id_for_reports(store: &KnowledgeRunStore) -> Option<String> {
    store
        .run()
        .ok()
        .flatten()
        .and_then(|run| string_field(&run.detail, "packId"))
        .or_else(|| {
            store
                .phase_checkpoints()
                .ok()?
                .into_iter()
                .find(|checkpoint| checkpoint.phase == KnowledgeRunPhase::Intake)
                .and_then(|checkpoint| string_field(&checkpoint.detail, "packId"))
        })
}

fn resume_command_for_store(store: &KnowledgeRunStore) -> String {
    format!(
        "mpb-knowledge release resume {} --artifact-root {}",
        store.run_id(),
        store
            .run_dir()
            .parent()
            .and_then(Path::parent)
            .unwrap_or_else(|| Path::new("knowledge"))
            .display()
    )
}

fn latest_artifact<'a>(artifacts: &'a [ArtifactRef], kind: &str) -> Option<&'a ArtifactRef> {
    artifacts
        .iter()
        .rev()
        .find(|artifact| artifact.artifact_kind == kind)
}

fn artifact_detail(artifacts: &[ArtifactRef], kind: &str) -> serde_json::Value {
    latest_artifact(artifacts, kind)
        .map(|artifact| artifact.detail.clone())
        .unwrap_or_else(|| json!({}))
}

fn artifact_details(artifacts: &[ArtifactRef], kind: &str) -> Vec<serde_json::Value> {
    artifacts
        .iter()
        .filter(|artifact| artifact.artifact_kind == kind)
        .map(|artifact| artifact.detail.clone())
        .collect()
}

fn artifact_paths(artifacts: &[ArtifactRef], kind: &str) -> Vec<String> {
    artifacts
        .iter()
        .filter(|artifact| artifact.artifact_kind == kind)
        .map(|artifact| artifact.path.clone())
        .collect()
}

fn nested_or_detail(artifact: Option<&ArtifactRef>, field: &str) -> serde_json::Value {
    artifact
        .and_then(|artifact| artifact.detail.get(field).cloned())
        .or_else(|| artifact.map(|artifact| artifact.detail.clone()))
        .unwrap_or_else(|| json!({}))
}

fn affected_obligations(detail: &serde_json::Value) -> Vec<String> {
    let mut obligations = string_array(detail.get("affectedCoverageObligations"));
    if let Some(obligation_id) = detail
        .get("obligationId")
        .and_then(serde_json::Value::as_str)
    {
        obligations.push(obligation_id.to_string());
    }
    obligations.sort();
    obligations.dedup();
    obligations
}

fn string_array(value: Option<&serde_json::Value>) -> Vec<String> {
    value
        .and_then(serde_json::Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(serde_json::Value::as_str)
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default()
}

fn string_field(value: &serde_json::Value, field: &str) -> Option<String> {
    value
        .get(field)
        .and_then(serde_json::Value::as_str)
        .map(str::to_string)
}

fn unsigned_app_warnings() -> Vec<UnsignedAppWarning> {
    [
        (
            "macOS",
            "macOS desktop artifacts are unsigned and not notarized unless a separate signing pipeline is attached.",
        ),
        (
            "Windows",
            "Windows desktop artifacts are unsigned unless a separate code-signing certificate step is attached.",
        ),
        (
            "Linux",
            "Linux desktop artifacts are unsigned and should be verified by checksum before distribution.",
        ),
    ]
    .into_iter()
    .map(|(platform, warning)| UnsignedAppWarning {
        platform: platform.to_string(),
        warning: warning.to_string(),
    })
    .collect()
}
