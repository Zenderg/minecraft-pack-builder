use std::env;
use std::fs;
use std::hash::Hasher;
use std::path::PathBuf;
use std::process::ExitCode;
use std::str::FromStr;

use mpb_knowledge::{
    build_runtime_bundle, compute_target_fingerprint, read_runtime_bundle, run_preflight,
    validate_source_dir, validate_source_pack, write_release_report_artifacts, ApprovalKind,
    ExtractedDraftRecord, ExtractionDraft, HardwareFit, KnowledgePackSource,
    KnowledgeReleaseOrchestrator, KnowledgeRunPhase, KnowledgeRunStore, PhaseCheckpointStatus,
    ProductCheck, ProductValidationEvidence, TargetManager,
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
        Some("target") => match args.next().as_deref() {
            Some("clone") => {
                let run_id = args.next().ok_or("target clone requires <run-id>")?;
                let instance_path = args.next().ok_or("target clone requires <instance-path>")?;
                let options = parse_target_options(args.collect())?;
                let manager = TargetManager::new(&options.artifact_root);
                let clone = manager
                    .create_disposable_clone(&run_id, &instance_path)
                    .map_err(|error| error.to_string())?;
                println!(
                    "{}",
                    serde_json::to_string_pretty(&clone).map_err(|error| error.to_string())?
                );
                Ok(())
            }
            Some("probe-launch") => {
                let run_id = args.next().ok_or("target probe-launch requires <run-id>")?;
                let options = parse_target_options(args.collect())?;
                let manager = TargetManager::new(&options.artifact_root);
                let probe = manager
                    .probe_launch(&run_id)
                    .map_err(|error| error.to_string())?;
                println!(
                    "{}",
                    serde_json::to_string_pretty(&probe).map_err(|error| error.to_string())?
                );
                Ok(())
            }
            _ => Err(
                "usage: mpb-knowledge target <clone RUN INSTANCE|probe-launch RUN> [--artifact-root PATH]"
                .to_string(),
            ),
        },
        Some("release") => match args.next().as_deref() {
            Some("start") => {
                let instance_path = args.next().ok_or("release start requires <instance-path>")?;
                let options = parse_release_start_options(args.collect())?;
                let pack_id = options
                    .pack_id
                    .as_deref()
                    .ok_or("release start requires --pack-id <pack-id>")?;
                let orchestrator = KnowledgeReleaseOrchestrator::new(&options.artifact_root);
                let outcome = orchestrator
                    .start_release(&instance_path, pack_id)
                    .map_err(|error| error.to_string())?;
                println!(
                    "{}",
                    serde_json::to_string_pretty(&outcome).map_err(|error| error.to_string())?
                );
                Ok(())
            }
            Some("resume") => {
                let run_id = args.next().ok_or("release resume requires <run-id>")?;
                let options = parse_release_options(args.collect())?;
                let orchestrator = KnowledgeReleaseOrchestrator::new(&options.artifact_root);
                let outcome = orchestrator
                    .run_next_required_phase(&run_id)
                    .map_err(|error| error.to_string())?;
                println!(
                    "{}",
                    serde_json::to_string_pretty(&outcome).map_err(|error| error.to_string())?
                );
                Ok(())
            }
            Some("status") => {
                let run_id = args.next().ok_or("release status requires <run-id>")?;
                let options = parse_release_options(args.collect())?;
                let orchestrator = KnowledgeReleaseOrchestrator::new(&options.artifact_root);
                let status = orchestrator
                    .status(&run_id)
                    .map_err(|error| error.to_string())?;
                println!(
                    "{}",
                    serde_json::to_string_pretty(&status).map_err(|error| error.to_string())?
                );
                Ok(())
            }
            Some("report") => {
                let run_id = args.next().ok_or("release report requires <run-id>")?;
                let options = parse_release_options(args.collect())?;
                let store = KnowledgeRunStore::open(&options.artifact_root, &run_id)
                    .map_err(|error| error.to_string())?;
                let paths =
                    write_release_report_artifacts(&store, None).map_err(|error| error.to_string())?;
                println!(
                    "{}",
                    serde_json::to_string_pretty(&paths).map_err(|error| error.to_string())?
                );
                Ok(())
            }
            Some("attach-source") => {
                let run_id = args.next().ok_or("release attach-source requires <run-id>")?;
                let source_dir = args
                    .next()
                    .ok_or("release attach-source requires <source-dir>")?;
                let options = parse_release_options(args.collect())?;
                let store = KnowledgeRunStore::open(&options.artifact_root, &run_id)
                    .map_err(|error| error.to_string())?;
                let pack = mpb_knowledge::load_source_pack(&source_dir)
                    .map_err(|error| error.to_string())?;
                validate_source_pack(&pack).map_err(|error| error.to_string())?;
                let target_fingerprint = run_target_fingerprint(&store)?;
                ensure_pack_targets_run(&pack, &target_fingerprint)?;
                let draft = extraction_draft_from_pack(&pack);
                let extraction_dir = store.run_dir().join("extraction");
                fs::create_dir_all(&extraction_dir).map_err(|error| error.to_string())?;
                let draft_path = extraction_dir.join("extraction-draft.json");
                fs::write(
                    &draft_path,
                    serde_json::to_vec_pretty(&draft).map_err(|error| error.to_string())?,
                )
                .map_err(|error| error.to_string())?;
                let draft_artifact = store
                    .record_artifact_ref(
                        "extraction-draft",
                        &draft_path,
                        Some(&target_fingerprint),
                        json!({
                            "sourceDir": source_dir,
                            "recordCount": draft.records.len(),
                        }),
                    )
                    .map_err(|error| error.to_string())?;
                let source_artifact = store
                    .record_artifact_ref(
                        "knowledge-source-dir",
                        &source_dir,
                        Some(&target_fingerprint),
                        json!({
                            "packId": pack.manifest.pack_id,
                            "packVersion": pack.manifest.pack_version,
                            "schemaVersion": pack.manifest.schema_version,
                        }),
                    )
                    .map_err(|error| error.to_string())?;
                store
                    .append_event(
                        "release.source_attached",
                        Some(&target_fingerprint),
                        json!({
                            "sourceArtifactId": source_artifact.id,
                            "extractionDraftArtifactId": draft_artifact.id,
                            "sourceDir": source_dir,
                        }),
                    )
                    .map_err(|error| error.to_string())?;
                println!(
                    "{}",
                    serde_json::to_string_pretty(&json!({
                        "runId": run_id,
                        "targetFingerprint": target_fingerprint,
                        "sourceArtifact": source_artifact,
                        "extractionDraftArtifact": draft_artifact,
                    }))
                    .map_err(|error| error.to_string())?
                );
                Ok(())
            }
            Some("attach-worker-model") => {
                let run_id = args
                    .next()
                    .ok_or("release attach-worker-model requires <run-id>")?;
                let model_path = args
                    .next()
                    .ok_or("release attach-worker-model requires <model-path>")?;
                let options = parse_release_attach_worker_model_options(args.collect())?;
                let store = KnowledgeRunStore::open(&options.artifact_root, &run_id)
                    .map_err(|error| error.to_string())?;
                let target_fingerprint = run_target_fingerprint(&store)?;
                let model_bytes = fs::read(&model_path).map_err(|error| {
                    format!("worker model path must be readable before attachment: {error}")
                })?;
                let checksum = options
                    .checksum
                    .unwrap_or_else(|| stable_checksum(&model_bytes));
                let identity = options
                    .identity
                    .unwrap_or_else(|| "local-worker-model".to_string());
                let hardware_fit = options.hardware_fit.unwrap_or(HardwareFit::Unknown);
                let artifact = store
                    .record_artifact_ref(
                        "worker-model",
                        &model_path,
                        Some(&target_fingerprint),
                        json!({
                            "identity": identity,
                            "checksum": checksum,
                            "hardwareFit": format!("{hardware_fit:?}"),
                            "source": "mpb-knowledge release attach-worker-model",
                        }),
                    )
                    .map_err(|error| error.to_string())?;
                store
                    .append_event(
                        "release.worker_model_attached",
                        Some(&target_fingerprint),
                        json!({"artifactId": artifact.id, "modelPath": model_path}),
                    )
                    .map_err(|error| error.to_string())?;
                println!(
                    "{}",
                    serde_json::to_string_pretty(&artifact).map_err(|error| error.to_string())?
                );
                Ok(())
            }
            Some("attach-product-evidence") => {
                let run_id = args
                    .next()
                    .ok_or("release attach-product-evidence requires <run-id>")?;
                let evidence_path = args
                    .next()
                    .ok_or("release attach-product-evidence requires <evidence-json>")?;
                let options = parse_release_options(args.collect())?;
                let store = KnowledgeRunStore::open(&options.artifact_root, &run_id)
                    .map_err(|error| error.to_string())?;
                let target_fingerprint = run_target_fingerprint(&store)?;
                let evidence: ProductValidationEvidence =
                    serde_json::from_slice(&fs::read(&evidence_path).map_err(|error| {
                        format!("product evidence path must be readable before attachment: {error}")
                    })?)
                    .map_err(|error| format!("invalid product validation evidence JSON: {error}"))?;
                let artifact = store
                    .record_artifact_ref(
                        "product-validation-evidence",
                        &evidence_path,
                        Some(&target_fingerprint),
                        json!({
                            "source": "mpb-knowledge release attach-product-evidence",
                            "patcherInstall": evidence.patcher.install.status,
                            "runtimeClone": evidence.runtime.cloned_runtime.status,
                            "tauriDesktop": evidence.runtime.tauri_desktop.status,
                        }),
                    )
                    .map_err(|error| error.to_string())?;
                store
                    .append_event(
                        "release.product_evidence_attached",
                        Some(&target_fingerprint),
                        json!({"artifactId": artifact.id, "evidencePath": evidence_path}),
                    )
                    .map_err(|error| error.to_string())?;
                println!(
                    "{}",
                    serde_json::to_string_pretty(&artifact).map_err(|error| error.to_string())?
                );
                Ok(())
            }
            Some("attach-runtime-evidence") => {
                let run_id = args
                    .next()
                    .ok_or("release attach-runtime-evidence requires <run-id>")?;
                let evidence_path = args
                    .next()
                    .ok_or("release attach-runtime-evidence requires <evidence-json>")?;
                let options = parse_release_options(args.collect())?;
                let store = KnowledgeRunStore::open(&options.artifact_root, &run_id)
                    .map_err(|error| error.to_string())?;
                let target_fingerprint = run_target_fingerprint(&store)?;
                let evidence: ProductCheck =
                    serde_json::from_slice(&fs::read(&evidence_path).map_err(|error| {
                        format!("runtime evidence path must be readable before attachment: {error}")
                    })?)
                    .map_err(|error| format!("invalid cloned runtime evidence JSON: {error}"))?;
                let artifact = store
                    .record_artifact_ref(
                        "cloned-runtime-validation-evidence",
                        &evidence_path,
                        Some(&target_fingerprint),
                        json!({
                            "source": "mpb-knowledge release attach-runtime-evidence",
                            "status": evidence.status,
                            "label": evidence.label,
                            "artifactPaths": evidence.artifact_paths,
                        }),
                    )
                    .map_err(|error| error.to_string())?;
                store
                    .append_event(
                        "release.runtime_evidence_attached",
                        Some(&target_fingerprint),
                        json!({"artifactId": artifact.id, "evidencePath": evidence_path}),
                    )
                    .map_err(|error| error.to_string())?;
                println!(
                    "{}",
                    serde_json::to_string_pretty(&artifact).map_err(|error| error.to_string())?
                );
                Ok(())
            }
            Some("prepare-github") => {
                let run_id = args
                    .next()
                    .ok_or("release prepare-github requires <run-id>")?;
                let options = parse_release_prepare_github_options(args.collect())?;
                let tag = options
                    .tag
                    .as_deref()
                    .ok_or("release prepare-github requires --tag <tag>")?;
                let store = KnowledgeRunStore::open(&options.artifact_root, &run_id)
                    .map_err(|error| error.to_string())?;
                let preparation = mpb_knowledge::prepare_github_release_publication(&store, tag)
                    .map_err(|error| error.to_string())?;
                println!(
                    "{}",
                    serde_json::to_string_pretty(&preparation)
                        .map_err(|error| error.to_string())?
                );
                Ok(())
            }
            _ => Err(
                "usage: mpb-knowledge release <start INSTANCE --pack-id PACK|resume RUN|status RUN|report RUN|attach-source RUN SOURCE_DIR|attach-worker-model RUN MODEL_PATH|attach-runtime-evidence RUN EVIDENCE_JSON|attach-product-evidence RUN EVIDENCE_JSON|prepare-github RUN --tag TAG> [--artifact-root PATH]"
                    .to_string(),
            ),
        },
        _ => Err(
            "usage: mpb-knowledge <validate-source SOURCE|build-bundle SOURCE OUTPUT|inspect-bundle BUNDLE|fingerprint INSTANCE BUILDER LAB SCHEMA|preflight INSTANCE [--artifact-root PATH] [--run-id RUN]|approve RUN APPROVAL_KIND --reason TEXT [--artifact-root PATH] [--target-fingerprint FINGERPRINT]|target clone RUN INSTANCE [--artifact-root PATH]|target probe-launch RUN [--artifact-root PATH]|release start INSTANCE --pack-id PACK [--artifact-root PATH]|release resume RUN [--artifact-root PATH]|release status RUN [--artifact-root PATH]|release report RUN [--artifact-root PATH]|release attach-source RUN SOURCE_DIR [--artifact-root PATH]|release attach-worker-model RUN MODEL_PATH [--identity ID] [--checksum CHECKSUM] [--hardware-fit FIT] [--artifact-root PATH]|release attach-runtime-evidence RUN EVIDENCE_JSON [--artifact-root PATH]|release attach-product-evidence RUN EVIDENCE_JSON [--artifact-root PATH]|release prepare-github RUN --tag TAG [--artifact-root PATH]>"
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

struct TargetOptions {
    artifact_root: PathBuf,
}

struct ReleaseStartOptions {
    artifact_root: PathBuf,
    pack_id: Option<String>,
}

struct ReleaseOptions {
    artifact_root: PathBuf,
}

struct ReleasePrepareGithubOptions {
    artifact_root: PathBuf,
    tag: Option<String>,
}

struct ReleaseAttachWorkerModelOptions {
    artifact_root: PathBuf,
    identity: Option<String>,
    checksum: Option<String>,
    hardware_fit: Option<HardwareFit>,
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

fn parse_target_options(args: Vec<String>) -> Result<TargetOptions, String> {
    let mut options = TargetOptions {
        artifact_root: PathBuf::from("knowledge"),
    };
    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--artifact-root" => {
                index += 1;
                let value = args.get(index).ok_or("--artifact-root requires <path>")?;
                options.artifact_root = PathBuf::from(value);
            }
            other => return Err(format!("unknown target option: {other}")),
        }
        index += 1;
    }
    Ok(options)
}

fn parse_release_start_options(args: Vec<String>) -> Result<ReleaseStartOptions, String> {
    let mut options = ReleaseStartOptions {
        artifact_root: PathBuf::from("knowledge"),
        pack_id: None,
    };
    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--artifact-root" => {
                index += 1;
                let value = args.get(index).ok_or("--artifact-root requires <path>")?;
                options.artifact_root = PathBuf::from(value);
            }
            "--pack-id" => {
                index += 1;
                let value = args.get(index).ok_or("--pack-id requires <pack-id>")?;
                options.pack_id = Some(value.clone());
            }
            other => return Err(format!("unknown release start option: {other}")),
        }
        index += 1;
    }
    Ok(options)
}

fn parse_release_options(args: Vec<String>) -> Result<ReleaseOptions, String> {
    let mut options = ReleaseOptions {
        artifact_root: PathBuf::from("knowledge"),
    };
    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--artifact-root" => {
                index += 1;
                let value = args.get(index).ok_or("--artifact-root requires <path>")?;
                options.artifact_root = PathBuf::from(value);
            }
            other => return Err(format!("unknown release option: {other}")),
        }
        index += 1;
    }
    Ok(options)
}

fn parse_release_prepare_github_options(
    args: Vec<String>,
) -> Result<ReleasePrepareGithubOptions, String> {
    let mut options = ReleasePrepareGithubOptions {
        artifact_root: PathBuf::from("knowledge"),
        tag: None,
    };
    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--artifact-root" => {
                index += 1;
                let value = args.get(index).ok_or("--artifact-root requires <path>")?;
                options.artifact_root = PathBuf::from(value);
            }
            "--tag" => {
                index += 1;
                let value = args.get(index).ok_or("--tag requires <tag>")?;
                options.tag = Some(value.clone());
            }
            other => return Err(format!("unknown release prepare-github option: {other}")),
        }
        index += 1;
    }
    Ok(options)
}

fn parse_release_attach_worker_model_options(
    args: Vec<String>,
) -> Result<ReleaseAttachWorkerModelOptions, String> {
    let mut options = ReleaseAttachWorkerModelOptions {
        artifact_root: PathBuf::from("knowledge"),
        identity: None,
        checksum: None,
        hardware_fit: None,
    };
    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--artifact-root" => {
                index += 1;
                let value = args.get(index).ok_or("--artifact-root requires <path>")?;
                options.artifact_root = PathBuf::from(value);
            }
            "--identity" => {
                index += 1;
                let value = args.get(index).ok_or("--identity requires <text>")?;
                options.identity = Some(value.clone());
            }
            "--checksum" => {
                index += 1;
                let value = args.get(index).ok_or("--checksum requires <checksum>")?;
                options.checksum = Some(value.clone());
            }
            "--hardware-fit" => {
                index += 1;
                let value = args.get(index).ok_or("--hardware-fit requires <fit>")?;
                options.hardware_fit = Some(parse_hardware_fit(value)?);
            }
            other => {
                return Err(format!(
                    "unknown release attach-worker-model option: {other}"
                ))
            }
        }
        index += 1;
    }
    Ok(options)
}

fn run_target_fingerprint(store: &KnowledgeRunStore) -> Result<String, String> {
    store
        .run()
        .map_err(|error| error.to_string())?
        .and_then(|run| run.target_fingerprint)
        .ok_or_else(|| {
            "run must complete the Fingerprint phase before attaching artifacts".to_string()
        })
}

fn ensure_pack_targets_run(
    pack: &KnowledgePackSource,
    target_fingerprint: &str,
) -> Result<(), String> {
    if pack.manifest.target_fingerprint != target_fingerprint {
        return Err(format!(
            "source pack fingerprint {} does not match run target fingerprint {}",
            pack.manifest.target_fingerprint, target_fingerprint
        ));
    }
    if pack.manifest.computed_fingerprint != target_fingerprint {
        return Err(format!(
            "source pack computed fingerprint {} does not match run target fingerprint {}",
            pack.manifest.computed_fingerprint, target_fingerprint
        ));
    }
    Ok(())
}

fn extraction_draft_from_pack(pack: &KnowledgePackSource) -> ExtractionDraft {
    let mut records = Vec::new();
    records.extend(
        pack.evidence
            .iter()
            .cloned()
            .map(ExtractedDraftRecord::Evidence),
    );
    records.extend(
        pack.entities
            .iter()
            .cloned()
            .map(ExtractedDraftRecord::Entity),
    );
    records.extend(pack.claims.iter().cloned().map(ExtractedDraftRecord::Claim));
    records.extend(
        pack.recipes
            .iter()
            .cloned()
            .map(ExtractedDraftRecord::Recipe),
    );
    records.extend(
        pack.relationships
            .iter()
            .cloned()
            .map(ExtractedDraftRecord::Relationship),
    );
    records.extend(
        pack.overlays
            .iter()
            .cloned()
            .map(ExtractedDraftRecord::Overlay),
    );
    ExtractionDraft {
        records,
        diagnostics: Vec::new(),
    }
}

fn parse_hardware_fit(value: &str) -> Result<HardwareFit, String> {
    match value {
        "Fits" | "fits" => Ok(HardwareFit::Fits),
        "Constrained" | "constrained" => Ok(HardwareFit::Constrained),
        "Insufficient" | "insufficient" => Ok(HardwareFit::Insufficient),
        "Unknown" | "unknown" => Ok(HardwareFit::Unknown),
        other => Err(format!(
            "unknown hardware fit {other}; expected Fits, Constrained, Insufficient, or Unknown"
        )),
    }
}

fn stable_checksum(bytes: &[u8]) -> String {
    let mut hasher = Fnv1a64::default();
    hasher.write(bytes);
    format!("{:016x}", hasher.finish())
}

struct Fnv1a64(u64);

impl Default for Fnv1a64 {
    fn default() -> Self {
        Self(0xcbf29ce484222325)
    }
}

impl Hasher for Fnv1a64 {
    fn finish(&self) -> u64 {
        self.0
    }

    fn write(&mut self, bytes: &[u8]) {
        for byte in bytes {
            self.0 ^= u64::from(*byte);
            self.0 = self.0.wrapping_mul(0x100000001b3);
        }
    }
}
