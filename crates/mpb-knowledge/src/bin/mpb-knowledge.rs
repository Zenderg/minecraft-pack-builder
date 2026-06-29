use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::ExitCode;
use std::str::FromStr;

use mpb_knowledge::{
    build_runtime_bundle, compute_target_fingerprint, read_runtime_bundle, run_preflight,
    validate_source_dir, ApprovalKind, KnowledgeRunPhase, KnowledgeRunStore, PhaseCheckpointStatus,
};
use serde_json::json;

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), String> {
    let mut args = env::args().skip(1);
    match args.next().as_deref() {
        Some("validate-source") => {
            let source_dir = args.next().ok_or("validate-source requires <source-dir>")?;
            validate_source_dir(source_dir).map_err(|error| error.to_string())?;
            println!("source pack is valid");
            Ok(())
        }
        Some("build-bundle") => {
            let source_dir = args.next().ok_or("build-bundle requires <source-dir>")?;
            let output_dir = args.next().ok_or("build-bundle requires <output-dir>")?;
            let bundle =
                build_runtime_bundle(&source_dir, &output_dir).map_err(|error| error.to_string())?;
            println!(
                "built bundle {} for {}",
                bundle.manifest.pack_id, bundle.manifest.exact_fingerprint
            );
            Ok(())
        }
        Some("inspect-bundle") => {
            let bundle_path = args.next().ok_or("inspect-bundle requires <knowledge-index.json>")?;
            let bundle = read_runtime_bundle(bundle_path).map_err(|error| error.to_string())?;
            println!(
                "{} {} entities={} evidence={}",
                bundle.manifest.pack_id,
                bundle.manifest.schema_version,
                bundle.indexes.entities_by_id.len(),
                bundle.indexes.evidence_by_id.len()
            );
            Ok(())
        }
        Some("fingerprint") => {
            let instance_path = args.next().ok_or("fingerprint requires <instance-path>")?;
            let builder_version = args.next().ok_or("fingerprint requires <builder-version>")?;
            let lab_version = args.next().ok_or("fingerprint requires <lab-version>")?;
            let schema_version = args.next().ok_or("fingerprint requires <schema-version>")?;
            let fingerprint = compute_target_fingerprint(
                instance_path,
                &builder_version,
                &lab_version,
                &schema_version,
            )
            .map_err(|error| error.to_string())?;
            println!(
                "{}",
                serde_json::to_string_pretty(&fingerprint).map_err(|error| error.to_string())?
            );
            Ok(())
        }
        Some("preflight") => {
            let instance_path = args.next().ok_or("preflight requires <instance-path>")?;
            let options = parse_preflight_options(args.collect())?;
            let report =
                run_preflight(&instance_path, &options.artifact_root).map_err(|error| {
                    format!("preflight failed for {instance_path}: {error}")
                })?;
            if let Some(run_id) = options.run_id.as_deref() {
                let store = KnowledgeRunStore::open(&options.artifact_root, run_id)
                    .map_err(|error| error.to_string())?;
                store
                    .record_run(
                        None,
                        json!({
                            "createdBy": "mpb-knowledge preflight",
                            "instancePath": instance_path,
                        }),
                    )
                    .map_err(|error| error.to_string())?;
                let report_path = store.run_dir().join("preflight-report.json");
                fs::write(
                    &report_path,
                    serde_json::to_vec_pretty(&report).map_err(|error| error.to_string())?,
                )
                .map_err(|error| error.to_string())?;
                store
                    .record_artifact_ref(
                        "preflight-report",
                        &report_path,
                        None,
                        json!({"format": "json"}),
                    )
                    .map_err(|error| error.to_string())?;
                store
                    .record_phase_checkpoint(
                        KnowledgeRunPhase::Preflight,
                        PhaseCheckpointStatus::Succeeded,
                        None,
                        serde_json::to_value(&report).map_err(|error| error.to_string())?,
                    )
                    .map_err(|error| error.to_string())?;
            }
            println!(
                "{}",
                serde_json::to_string_pretty(&report).map_err(|error| error.to_string())?
            );
            Ok(())
        }
        Some("approve") => {
            let run_id = args.next().ok_or("approve requires <run-id>")?;
            let kind_text = args.next().ok_or("approve requires <approval-kind>")?;
            let kind = ApprovalKind::from_str(&kind_text).map_err(|error| error.to_string())?;
            let options = parse_approve_options(args.collect())?;
            let reason = options
                .reason
                .as_deref()
                .ok_or("approve requires --reason <text>")?;
            let store = KnowledgeRunStore::open(&options.artifact_root, &run_id)
                .map_err(|error| error.to_string())?;
            let approval = store
                .record_approval(
                    kind,
                    options.target_fingerprint.as_deref(),
                    true,
                    reason,
                    json!({"source": "mpb-knowledge approve"}),
                )
                .map_err(|error| error.to_string())?;
            println!(
                "{}",
                serde_json::to_string_pretty(&approval).map_err(|error| error.to_string())?
            );
            Ok(())
        }
        _ => Err(
            "usage: mpb-knowledge <validate-source SOURCE|build-bundle SOURCE OUTPUT|inspect-bundle BUNDLE|fingerprint INSTANCE BUILDER LAB SCHEMA|preflight INSTANCE [--artifact-root PATH] [--run-id RUN]|approve RUN APPROVAL_KIND --reason TEXT [--artifact-root PATH] [--target-fingerprint FINGERPRINT]>"
                .to_string(),
        ),
    }
}

struct PreflightOptions {
    artifact_root: PathBuf,
    run_id: Option<String>,
}

struct ApproveOptions {
    artifact_root: PathBuf,
    reason: Option<String>,
    target_fingerprint: Option<String>,
}

fn parse_preflight_options(args: Vec<String>) -> Result<PreflightOptions, String> {
    let mut options = PreflightOptions {
        artifact_root: PathBuf::from("knowledge"),
        run_id: None,
    };
    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--artifact-root" => {
                index += 1;
                let value = args.get(index).ok_or("--artifact-root requires <path>")?;
                options.artifact_root = PathBuf::from(value);
            }
            "--run-id" => {
                index += 1;
                let value = args.get(index).ok_or("--run-id requires <run-id>")?;
                options.run_id = Some(value.clone());
            }
            other => return Err(format!("unknown preflight option: {other}")),
        }
        index += 1;
    }
    Ok(options)
}

fn parse_approve_options(args: Vec<String>) -> Result<ApproveOptions, String> {
    let mut options = ApproveOptions {
        artifact_root: PathBuf::from("knowledge"),
        reason: None,
        target_fingerprint: None,
    };
    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--artifact-root" => {
                index += 1;
                let value = args.get(index).ok_or("--artifact-root requires <path>")?;
                options.artifact_root = PathBuf::from(value);
            }
            "--reason" => {
                index += 1;
                let value = args.get(index).ok_or("--reason requires <text>")?;
                options.reason = Some(value.clone());
            }
            "--target-fingerprint" => {
                index += 1;
                let value = args
                    .get(index)
                    .ok_or("--target-fingerprint requires <fingerprint>")?;
                options.target_fingerprint = Some(value.clone());
            }
            other => return Err(format!("unknown approve option: {other}")),
        }
        index += 1;
    }
    Ok(options)
}
