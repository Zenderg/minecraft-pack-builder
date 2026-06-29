use std::fs;
use std::hash::Hasher;
use std::io::Write;
use std::path::{Path, PathBuf};

use flate2::write::GzEncoder;
use flate2::Compression;
use serde::{Deserialize, Serialize};
use serde_json::json;
use thiserror::Error;

use crate::orchestrator::{
    current_target_fingerprint, OrchestratorError, PhaseRunContext, PhaseRunStatus,
};
use crate::{
    build_runtime_bundle, load_source_pack, read_runtime_bundle, validate_source_pack,
    KnowledgePackSource, KnowledgeRunPhase, RunBlockerInput, RuntimeBundleQuery,
};

const RUNTIME_BUNDLE_MISSING: &str = "RUNTIME_BUNDLE_MISSING";
const PATCHER_BUNDLE_FINGERPRINT_MISMATCH: &str = "PATCHER_BUNDLE_FINGERPRINT_MISMATCH";
const PRODUCT_VALIDATION_EVIDENCE_MISSING: &str = "PRODUCT_VALIDATION_EVIDENCE_MISSING";
const PATCHER_INTEGRATION_EVIDENCE_MISSING: &str = "PATCHER_INTEGRATION_EVIDENCE_MISSING";
const PATCHER_INTEGRATION_BEHAVIOR_FAILED: &str = "PATCHER_INTEGRATION_BEHAVIOR_FAILED";
const PRODUCT_VALIDATION_PATCHER_BEHAVIOR_FAILED: &str =
    "PRODUCT_VALIDATION_PATCHER_BEHAVIOR_FAILED";
const MCP_QUERY_COVERAGE_MISSING: &str = "MCP_QUERY_COVERAGE_MISSING";
const REAL_CLONED_RUNTIME_VALIDATION_MISSING: &str = "REAL_CLONED_RUNTIME_VALIDATION_MISSING";
const TAURI_DESKTOP_VALIDATION_MISSING: &str = "TAURI_DESKTOP_VALIDATION_MISSING";

#[derive(Debug, Error)]
pub enum ReleaseError {
    #[error("file operation failed: {0}")]
    Io(#[from] std::io::Error),
    #[error("json operation failed: {0}")]
    Json(#[from] serde_json::Error),
    #[error("bundle operation failed: {0}")]
    Bundle(#[from] crate::BundleBuildError),
    #[error("run state operation failed: {0}")]
    RunState(#[from] crate::RunStateError),
    #[error("approval operation failed: {0}")]
    Approval(#[from] crate::ApprovalError),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ProductValidationStatus {
    Passed,
    Failed,
    Unavailable,
}

impl ProductValidationStatus {
    fn is_passed(self) -> bool {
        self == ProductValidationStatus::Passed
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProductCheck {
    pub status: ProductValidationStatus,
    pub label: String,
    pub detail: String,
    #[serde(default)]
    pub artifact_paths: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PatcherValidationEvidence {
    pub install: ProductCheck,
    pub update: ProductCheck,
    pub repair: ProductCheck,
    pub unpatch: ProductCheck,
    pub exact_fingerprint_match: ProductCheck,
    pub mismatched_fingerprint_base_mod_only: ProductCheck,
    pub mismatched_fingerprint_knowledge_unavailable: ProductCheck,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct McpQueryValidationEvidence {
    pub knowledge_status: ProductCheck,
    pub search_entities: ProductCheck,
    pub entity_card: ProductCheck,
    pub recipe_graph: ProductCheck,
    pub mechanic_details: ProductCheck,
    pub evidence: ProductCheck,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeProductValidationEvidence {
    pub cloned_runtime: ProductCheck,
    pub tauri_desktop: ProductCheck,
    pub browser_vite_supplemental: Option<ProductCheck>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProductValidationEvidence {
    pub patcher: PatcherValidationEvidence,
    pub mcp: McpQueryValidationEvidence,
    pub runtime: RuntimeProductValidationEvidence,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProductValidationBlocker {
    pub code: String,
    pub message: String,
    #[serde(default)]
    pub affected_checks: Vec<String>,
    #[serde(default)]
    pub artifact_paths: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProductValidationReport {
    pub run_id: String,
    pub target_fingerprint: String,
    pub patcher: PatcherValidationEvidence,
    pub mcp: McpQueryValidationEvidence,
    pub runtime: RuntimeProductValidationEvidence,
    pub bundle_query_results: BundleQueryResults,
    pub blockers: Vec<ProductValidationBlocker>,
}

impl ProductValidationReport {
    pub fn from_evidence(
        run_id: impl Into<String>,
        target_fingerprint: impl Into<String>,
        evidence: ProductValidationEvidence,
    ) -> Self {
        Self::from_evidence_and_bundle_queries(
            run_id,
            target_fingerprint,
            evidence,
            BundleQueryResults::not_exercised(),
        )
    }

    pub fn from_evidence_and_bundle_queries(
        run_id: impl Into<String>,
        target_fingerprint: impl Into<String>,
        evidence: ProductValidationEvidence,
        bundle_query_results: BundleQueryResults,
    ) -> Self {
        let mut blockers = Vec::new();
        collect_failed_checks(
            PRODUCT_VALIDATION_PATCHER_BEHAVIOR_FAILED,
            "Patcher install, update, repair, unpatch, exact-match, and mismatch behavior must pass.",
            &mut blockers,
            [
                &evidence.patcher.install,
                &evidence.patcher.update,
                &evidence.patcher.repair,
                &evidence.patcher.unpatch,
                &evidence.patcher.exact_fingerprint_match,
                &evidence.patcher.mismatched_fingerprint_base_mod_only,
                &evidence.patcher.mismatched_fingerprint_knowledge_unavailable,
            ],
        );
        collect_failed_checks(
            MCP_QUERY_COVERAGE_MISSING,
            "MCP status, search, entity, recipe, mechanic, and evidence queries must be verified.",
            &mut blockers,
            [
                &evidence.mcp.knowledge_status,
                &evidence.mcp.search_entities,
                &evidence.mcp.entity_card,
                &evidence.mcp.recipe_graph,
                &evidence.mcp.mechanic_details,
                &evidence.mcp.evidence,
            ],
        );
        collect_failed_checks(
            REAL_CLONED_RUNTIME_VALIDATION_MISSING,
            "A real cloned Prism/Minecraft runtime validation result is required.",
            &mut blockers,
            [&evidence.runtime.cloned_runtime],
        );
        collect_failed_checks(
            TAURI_DESKTOP_VALIDATION_MISSING,
            "The release acceptance path requires Tauri desktop patcher validation.",
            &mut blockers,
            [&evidence.runtime.tauri_desktop],
        );
        if !bundle_query_results.all_passed() {
            blockers.push(ProductValidationBlocker {
                code: MCP_QUERY_COVERAGE_MISSING.to_string(),
                message: "Runtime bundle query indexes did not cover every MCP knowledge query."
                    .to_string(),
                affected_checks: bundle_query_results.failed_labels(),
                artifact_paths: Vec::new(),
            });
        }
        Self {
            run_id: run_id.into(),
            target_fingerprint: target_fingerprint.into(),
            patcher: evidence.patcher,
            mcp: evidence.mcp,
            runtime: evidence.runtime,
            bundle_query_results,
            blockers,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BundleQueryResults {
    pub knowledge_status: bool,
    pub search: bool,
    pub entity: bool,
    pub recipe: bool,
    pub mechanic: bool,
    pub evidence: bool,
}

impl BundleQueryResults {
    pub fn not_exercised() -> Self {
        Self {
            knowledge_status: false,
            search: false,
            entity: false,
            recipe: false,
            mechanic: false,
            evidence: false,
        }
    }

    fn all_passed(&self) -> bool {
        self.knowledge_status
            && self.search
            && self.entity
            && self.recipe
            && self.mechanic
            && self.evidence
    }

    fn failed_labels(&self) -> Vec<String> {
        [
            ("mpb_knowledge_status", self.knowledge_status),
            ("mpb_search_entities", self.search),
            ("mpb_get_entity_card", self.entity),
            ("mpb_get_recipe_graph", self.recipe),
            ("mpb_get_mechanic_details", self.mechanic),
            ("mpb_get_evidence", self.evidence),
        ]
        .into_iter()
        .filter_map(|(label, passed)| (!passed).then(|| label.to_string()))
        .collect()
    }
}

pub(crate) fn run_bundle_phase(
    context: &PhaseRunContext<'_>,
) -> Result<PhaseRunStatus, OrchestratorError> {
    if let Some(existing) = context.store.latest_artifact_ref("runtime-bundle")? {
        if Path::new(&existing.path).is_file() {
            return Ok(PhaseRunStatus::Succeeded {
                target_fingerprint: existing.target_fingerprint,
                detail: json!({
                    "idempotent": true,
                    "runtimeBundleArtifact": existing.path,
                    "checksum": existing.detail.get("checksum"),
                    "compressedSizeBytes": existing.detail.get("compressedSizeBytes"),
                }),
            });
        }
    }

    let target_fingerprint =
        current_target_fingerprint(context.store)?.unwrap_or_else(|| "unknown".to_string());
    let source_dir = match source_dir_for_bundle(context)? {
        Some(path) => path,
        None => {
            return Ok(PhaseRunStatus::Blocked {
                blocker: RunBlockerInput {
                    code: "BUNDLE_SOURCE_DIR_MISSING".to_string(),
                    phase: Some(KnowledgeRunPhase::Bundle),
                    target_fingerprint: Some(target_fingerprint),
                    message: "Bundle generation requires a persisted knowledge source directory or source-pack artifact.".to_string(),
                    detail: json!({
                        "acceptedArtifactKinds": ["knowledge-source-dir", "knowledge-source-pack"],
                    }),
                },
            })
        }
    };
    if let Err(error) = validate_source_pack(&load_source_pack(&source_dir)?) {
        let failure = error
            .failures()
            .first()
            .expect("validation error contains failures");
        return Ok(PhaseRunStatus::Blocked {
            blocker: RunBlockerInput {
                code: format!(
                    "BUNDLE_STRICT_VALIDATION_{}",
                    failure.code.as_str().to_ascii_uppercase()
                ),
                phase: Some(KnowledgeRunPhase::Bundle),
                target_fingerprint: Some(target_fingerprint),
                message: failure.message.clone(),
                detail: json!({
                    "sourceDir": source_dir,
                    "validationCode": failure.code.as_str(),
                }),
            },
        });
    }

    let output_dir = context.store.run_dir().join("bundle");
    let bundle = build_runtime_bundle(&source_dir, &output_dir)?;
    let bundle_path = output_dir.join("knowledge-index.json");
    let bundle_bytes = fs::read(&bundle_path)?;
    let checksum = stable_checksum(&bundle_bytes);
    let gzip_path = output_dir.join("knowledge-index.json.gz");
    let compressed = gzip_bytes(&bundle_bytes)?;
    fs::write(&gzip_path, &compressed)?;
    context.store.record_artifact_ref(
        "runtime-bundle",
        &bundle_path,
        Some(&bundle.manifest.exact_fingerprint),
        json!({
            "checksum": checksum,
            "sizeBytes": bundle_bytes.len(),
            "compressedArtifact": gzip_path,
            "compressedSizeBytes": compressed.len(),
            "packId": bundle.manifest.pack_id,
            "schemaVersion": bundle.manifest.schema_version,
            "exactFingerprint": bundle.manifest.exact_fingerprint,
        }),
    )?;
    context.store.record_artifact_ref(
        "runtime-bundle-compressed",
        &gzip_path,
        Some(&bundle.manifest.exact_fingerprint),
        json!({
            "checksum": stable_checksum(&compressed),
            "sizeBytes": compressed.len(),
            "uncompressedChecksum": checksum,
        }),
    )?;
    Ok(PhaseRunStatus::Succeeded {
        target_fingerprint: Some(bundle.manifest.exact_fingerprint),
        detail: json!({
            "runtimeBundleArtifact": bundle_path,
            "checksum": checksum,
            "compressedArtifact": gzip_path,
            "compressedSizeBytes": compressed.len(),
        }),
    })
}

pub(crate) fn run_patcher_integration_phase(
    context: &PhaseRunContext<'_>,
) -> Result<PhaseRunStatus, OrchestratorError> {
    if let Some(existing) = context
        .store
        .latest_artifact_ref("patcher-integration-report")?
    {
        if Path::new(&existing.path).is_file() {
            return Ok(PhaseRunStatus::Succeeded {
                target_fingerprint: existing.target_fingerprint,
                detail: json!({"idempotent": true, "patcherIntegrationReport": existing.path}),
            });
        }
    }
    let Some(bundle_ref) = context.store.latest_artifact_ref("runtime-bundle")? else {
        return Ok(missing_runtime_bundle(
            KnowledgeRunPhase::PatcherIntegration,
        ));
    };
    let target_fingerprint =
        current_target_fingerprint(context.store)?.unwrap_or_else(|| "unknown".to_string());
    let bundle = read_runtime_bundle(&bundle_ref.path)?;
    if bundle.manifest.exact_fingerprint != target_fingerprint
        || bundle_ref.target_fingerprint.as_deref() != Some(target_fingerprint.as_str())
    {
        return Ok(PhaseRunStatus::Blocked {
            blocker: RunBlockerInput {
                code: PATCHER_BUNDLE_FINGERPRINT_MISMATCH.to_string(),
                phase: Some(KnowledgeRunPhase::PatcherIntegration),
                target_fingerprint: Some(target_fingerprint),
                message: "Embedded bundle metadata must point at the exact target fingerprint before patcher integration can pass.".to_string(),
                detail: json!({
                    "bundleArtifact": bundle_ref.path,
                    "bundleManifestFingerprint": bundle.manifest.exact_fingerprint,
                    "artifactFingerprint": bundle_ref.target_fingerprint,
                }),
            },
        });
    }
    let (evidence, evidence_path) = match patcher_validation_evidence(context, &target_fingerprint)?
    {
        PatcherEvidenceLookup::Found { evidence, path } => (evidence, path),
        PatcherEvidenceLookup::Missing => {
            return Ok(PhaseRunStatus::Blocked {
                blocker: RunBlockerInput {
                    code: PATCHER_INTEGRATION_EVIDENCE_MISSING.to_string(),
                    phase: Some(KnowledgeRunPhase::PatcherIntegration),
                    target_fingerprint: Some(target_fingerprint),
                    message: "Patcher integration requires evidence that exact-match metadata and mismatched-fingerprint behavior were verified.".to_string(),
                    detail: json!({
                        "acceptedArtifactKinds": ["patcher-validation-evidence", "product-validation-evidence"],
                        "requiredChecks": [
                            "exact_fingerprint_match",
                            "mismatched_fingerprint_base_mod_only",
                            "mismatched_fingerprint_knowledge_unavailable"
                        ],
                    }),
                },
            });
        }
        PatcherEvidenceLookup::FingerprintMismatch {
            artifact_kind,
            path,
            artifact_fingerprint,
        } => {
            return Ok(PhaseRunStatus::Blocked {
                blocker: RunBlockerInput {
                    code: "PATCHER_INTEGRATION_EVIDENCE_FINGERPRINT_MISMATCH".to_string(),
                    phase: Some(KnowledgeRunPhase::PatcherIntegration),
                    target_fingerprint: Some(target_fingerprint),
                    message: "Patcher integration evidence must be recorded for the exact target fingerprint.".to_string(),
                    detail: json!({
                        "artifactKind": artifact_kind,
                        "evidenceArtifact": path,
                        "artifactFingerprint": artifact_fingerprint,
                    }),
                },
            });
        }
    };
    let integration_checks = [
        &evidence.exact_fingerprint_match,
        &evidence.mismatched_fingerprint_base_mod_only,
        &evidence.mismatched_fingerprint_knowledge_unavailable,
    ];
    let failed_checks = integration_checks
        .into_iter()
        .filter(|check| !check.status.is_passed())
        .map(|check| check.label.clone())
        .collect::<Vec<_>>();
    if !failed_checks.is_empty() {
        return Ok(PhaseRunStatus::Blocked {
            blocker: RunBlockerInput {
                code: PATCHER_INTEGRATION_BEHAVIOR_FAILED.to_string(),
                phase: Some(KnowledgeRunPhase::PatcherIntegration),
                target_fingerprint: Some(target_fingerprint),
                message: "Patcher integration evidence did not prove exact-match and mismatched-fingerprint behavior.".to_string(),
                detail: json!({
                    "patcherEvidenceArtifact": evidence_path,
                    "failedChecks": failed_checks,
                }),
            },
        });
    }

    let report = json!({
        "runId": context.store.run_id(),
        "targetFingerprint": target_fingerprint,
        "runtimeBundleArtifact": bundle_ref.path,
        "patcherEvidenceArtifact": evidence_path,
        "metadataExactFingerprintMatch": true,
        "mismatchContract": {
            "baseMpbModMayInstall": true,
            "curatedKnowledgeMustBeUnavailable": true,
            "authoritativeTestCommand": "cargo test -p mpb-assets --test patcher",
        },
    });
    let report_path = context
        .store
        .run_dir()
        .join("reports/patcher-integration.json");
    write_json(&report_path, &report)?;
    context.store.record_artifact_ref(
        "patcher-integration-report",
        &report_path,
        Some(&target_fingerprint),
        json!({
            "metadataExactFingerprintMatch": true,
            "patcherEvidenceArtifact": evidence_path,
            "mismatchBehaviorCoveredBy": "cargo test -p mpb-assets --test patcher",
        }),
    )?;
    Ok(PhaseRunStatus::Succeeded {
        target_fingerprint: Some(target_fingerprint),
        detail: json!({"patcherIntegrationReport": report_path}),
    })
}

pub(crate) fn run_product_validation_phase(
    context: &PhaseRunContext<'_>,
) -> Result<PhaseRunStatus, OrchestratorError> {
    if let Some(existing) = context
        .store
        .latest_artifact_ref("product-validation-report")?
    {
        if Path::new(&existing.path).is_file() {
            let report: ProductValidationReport =
                serde_json::from_slice(&fs::read(&existing.path)?)?;
            if report.blockers.is_empty() {
                return Ok(PhaseRunStatus::Succeeded {
                    target_fingerprint: existing.target_fingerprint,
                    detail: json!({"idempotent": true, "productValidationReport": existing.path}),
                });
            }
        }
    }
    let target_fingerprint =
        current_target_fingerprint(context.store)?.unwrap_or_else(|| "unknown".to_string());
    let Some(evidence_ref) = context
        .store
        .latest_artifact_ref("product-validation-evidence")?
    else {
        return Ok(PhaseRunStatus::Blocked {
            blocker: RunBlockerInput {
                code: PRODUCT_VALIDATION_EVIDENCE_MISSING.to_string(),
                phase: Some(KnowledgeRunPhase::ProductValidation),
                target_fingerprint: Some(target_fingerprint),
                message: "Product validation requires explicit Tauri desktop, patcher, MCP, and cloned runtime evidence; browser/Vite evidence is supplemental only.".to_string(),
                detail: json!({
                    "requiredArtifactKind": "product-validation-evidence",
                    "desktopRequired": true,
                    "browserViteSupplementalOnly": true,
                }),
            },
        });
    };
    if evidence_ref.target_fingerprint.as_deref() != Some(target_fingerprint.as_str()) {
        return Ok(PhaseRunStatus::Blocked {
            blocker: RunBlockerInput {
                code: "PRODUCT_VALIDATION_EVIDENCE_FINGERPRINT_MISMATCH".to_string(),
                phase: Some(KnowledgeRunPhase::ProductValidation),
                target_fingerprint: Some(target_fingerprint),
                message:
                    "Product validation evidence must be recorded for the exact target fingerprint."
                        .to_string(),
                detail: json!({
                    "evidenceArtifact": evidence_ref.path,
                    "artifactFingerprint": evidence_ref.target_fingerprint,
                }),
            },
        });
    }
    let evidence: ProductValidationEvidence =
        serde_json::from_slice(&fs::read(&evidence_ref.path)?)?;
    let bundle_queries = runtime_bundle_query_results(context)?;
    let report = ProductValidationReport::from_evidence_and_bundle_queries(
        context.store.run_id(),
        target_fingerprint.clone(),
        evidence,
        bundle_queries,
    );
    let report_path = context
        .store
        .run_dir()
        .join("reports/product-validation-report.json");
    write_json(&report_path, &report)?;
    context.store.record_artifact_ref(
        "product-validation-report",
        &report_path,
        Some(&target_fingerprint),
        json!({
            "blockerCount": report.blockers.len(),
            "evidenceArtifact": evidence_ref.path,
        }),
    )?;
    if let Some(blocker) = report.blockers.first() {
        return Ok(PhaseRunStatus::Blocked {
            blocker: RunBlockerInput {
                code: blocker.code.clone(),
                phase: Some(KnowledgeRunPhase::ProductValidation),
                target_fingerprint: Some(target_fingerprint),
                message: blocker.message.clone(),
                detail: json!({
                    "productValidationReport": report_path,
                    "affectedChecks": blocker.affected_checks,
                    "artifactPaths": blocker.artifact_paths,
                }),
            },
        });
    }
    Ok(PhaseRunStatus::Succeeded {
        target_fingerprint: Some(target_fingerprint),
        detail: json!({"productValidationReport": report_path}),
    })
}

fn source_dir_for_bundle(
    context: &PhaseRunContext<'_>,
) -> Result<Option<PathBuf>, OrchestratorError> {
    if let Some(source_dir) = context.store.latest_artifact_ref("knowledge-source-dir")? {
        let path = PathBuf::from(source_dir.path);
        if path.is_dir() {
            return Ok(Some(path));
        }
    }
    let Some(source_pack) = context.store.latest_artifact_ref("knowledge-source-pack")? else {
        return Ok(None);
    };
    let pack: KnowledgePackSource = serde_json::from_slice(&fs::read(source_pack.path)?)?;
    let generated_dir = context.store.run_dir().join("bundle/source");
    materialize_source_pack(&generated_dir, &pack)?;
    context.store.record_artifact_ref(
        "knowledge-source-dir",
        &generated_dir,
        Some(&pack.manifest.target_fingerprint),
        json!({"generatedFrom": "knowledge-source-pack"}),
    )?;
    Ok(Some(generated_dir))
}

enum PatcherEvidenceLookup {
    Found {
        evidence: PatcherValidationEvidence,
        path: String,
    },
    Missing,
    FingerprintMismatch {
        artifact_kind: &'static str,
        path: String,
        artifact_fingerprint: Option<String>,
    },
}

fn patcher_validation_evidence(
    context: &PhaseRunContext<'_>,
    target_fingerprint: &str,
) -> Result<PatcherEvidenceLookup, OrchestratorError> {
    if let Some(patcher_ref) = context
        .store
        .latest_artifact_ref("patcher-validation-evidence")?
    {
        if patcher_ref.target_fingerprint.as_deref() != Some(target_fingerprint) {
            return Ok(PatcherEvidenceLookup::FingerprintMismatch {
                artifact_kind: "patcher-validation-evidence",
                path: patcher_ref.path,
                artifact_fingerprint: patcher_ref.target_fingerprint,
            });
        }
        let evidence: PatcherValidationEvidence =
            serde_json::from_slice(&fs::read(&patcher_ref.path)?)?;
        return Ok(PatcherEvidenceLookup::Found {
            evidence,
            path: patcher_ref.path,
        });
    }

    let Some(product_ref) = context
        .store
        .latest_artifact_ref("product-validation-evidence")?
    else {
        return Ok(PatcherEvidenceLookup::Missing);
    };
    if product_ref.target_fingerprint.as_deref() != Some(target_fingerprint) {
        return Ok(PatcherEvidenceLookup::FingerprintMismatch {
            artifact_kind: "product-validation-evidence",
            path: product_ref.path,
            artifact_fingerprint: product_ref.target_fingerprint,
        });
    }
    let evidence: ProductValidationEvidence =
        serde_json::from_slice(&fs::read(&product_ref.path)?)?;
    Ok(PatcherEvidenceLookup::Found {
        evidence: evidence.patcher,
        path: product_ref.path,
    })
}

fn materialize_source_pack(
    source_dir: &Path,
    pack: &KnowledgePackSource,
) -> Result<(), OrchestratorError> {
    fs::create_dir_all(source_dir)?;
    write_json(&source_dir.join("manifest.json"), &pack.manifest)?;
    write_jsonl(&source_dir.join("entities.jsonl"), &pack.entities)?;
    write_jsonl(&source_dir.join("claims.jsonl"), &pack.claims)?;
    write_jsonl(&source_dir.join("evidence.jsonl"), &pack.evidence)?;
    write_jsonl(&source_dir.join("recipes.jsonl"), &pack.recipes)?;
    write_jsonl(&source_dir.join("overlays.jsonl"), &pack.overlays)?;
    write_jsonl(&source_dir.join("relationships.jsonl"), &pack.relationships)?;
    write_jsonl(
        &source_dir.join("worker-decisions.jsonl"),
        &pack.worker_decisions,
    )?;
    Ok(())
}

fn write_jsonl<T: Serialize>(path: &Path, records: &[T]) -> Result<(), OrchestratorError> {
    let mut bytes = Vec::new();
    for record in records {
        serde_json::to_writer(&mut bytes, record)?;
        bytes.push(b'\n');
    }
    fs::write(path, bytes)?;
    Ok(())
}

fn missing_runtime_bundle(phase: KnowledgeRunPhase) -> PhaseRunStatus {
    PhaseRunStatus::Blocked {
        blocker: RunBlockerInput {
            code: RUNTIME_BUNDLE_MISSING.to_string(),
            phase: Some(phase),
            target_fingerprint: None,
            message: "Product release validation requires a generated runtime bundle.".to_string(),
            detail: json!({"requiredArtifactKind": "runtime-bundle"}),
        },
    }
}

fn runtime_bundle_query_results(
    context: &PhaseRunContext<'_>,
) -> Result<BundleQueryResults, OrchestratorError> {
    let Some(bundle_ref) = context.store.latest_artifact_ref("runtime-bundle")? else {
        return Ok(BundleQueryResults {
            knowledge_status: false,
            search: false,
            entity: false,
            recipe: false,
            mechanic: false,
            evidence: false,
        });
    };
    let bundle = read_runtime_bundle(&bundle_ref.path)?;
    let query = RuntimeBundleQuery::new(&bundle);
    let entity_id = bundle.indexes.entities_by_id.keys().next().cloned();
    let search_name = bundle
        .indexes
        .entities_by_localized_name
        .keys()
        .next()
        .cloned();
    let recipe_entity_id = bundle.indexes.recipe_graphs.keys().next().cloned();
    let mechanic_id = bundle.indexes.mechanic_details.keys().next().cloned();
    let evidence_id = bundle.indexes.evidence_by_id.keys().next().cloned();
    Ok(BundleQueryResults {
        knowledge_status: !bundle.manifest.exact_fingerprint.is_empty(),
        search: search_name
            .as_deref()
            .map(|name| !query.search_by_localized_name(name).is_empty())
            .unwrap_or(false),
        entity: entity_id
            .as_deref()
            .and_then(|id| query.entity_by_id(id))
            .is_some(),
        recipe: recipe_entity_id
            .as_deref()
            .and_then(|id| query.recipe_graph_for(id))
            .is_some(),
        mechanic: mechanic_id
            .as_deref()
            .and_then(|id| query.mechanic_details(id))
            .is_some(),
        evidence: evidence_id
            .as_deref()
            .and_then(|id| query.evidence(id))
            .is_some(),
    })
}

fn collect_failed_checks<const N: usize>(
    code: &str,
    message: &str,
    blockers: &mut Vec<ProductValidationBlocker>,
    checks: [&ProductCheck; N],
) {
    let failed = checks
        .into_iter()
        .filter(|check| !check.status.is_passed())
        .collect::<Vec<_>>();
    if failed.is_empty() {
        return;
    }
    blockers.push(ProductValidationBlocker {
        code: code.to_string(),
        message: message.to_string(),
        affected_checks: failed.iter().map(|check| check.label.clone()).collect(),
        artifact_paths: failed
            .iter()
            .flat_map(|check| check.artifact_paths.clone())
            .collect(),
    });
}

fn gzip_bytes(bytes: &[u8]) -> Result<Vec<u8>, std::io::Error> {
    let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(bytes)?;
    encoder.finish()
}

fn write_json<T: Serialize>(path: &Path, value: &T) -> Result<(), OrchestratorError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, serde_json::to_vec_pretty(value)?)?;
    Ok(())
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
