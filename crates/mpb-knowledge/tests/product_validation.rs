use std::fs;
use std::path::Path;

use mpb_knowledge::{
    build_runtime_bundle, KnowledgeReleaseOrchestrator, KnowledgeRunPhase, KnowledgeRunStore,
    McpQueryValidationEvidence, PatcherValidationEvidence, PhaseCheckpointStatus, ProductCheck,
    ProductValidationEvidence, ProductValidationReport, ProductValidationStatus,
    RuntimeProductValidationEvidence,
};
use serde_json::json;

const FIXTURE_FINGERPRINT: &str = "58ef12bb4c001755";

#[test]
fn product_validation_bundle_phase_builds_validated_runtime_bundle_and_records_checksum_size() {
    let temp = tempfile::tempdir().expect("tempdir");
    let artifact_root = temp.path().join("knowledge");
    let run_id = "run-bundle-product";
    let store = seed_successful_phases(
        &artifact_root,
        run_id,
        FIXTURE_FINGERPRINT,
        &[
            KnowledgeRunPhase::Intake,
            KnowledgeRunPhase::Preflight,
            KnowledgeRunPhase::Approvals,
            KnowledgeRunPhase::Fingerprint,
            KnowledgeRunPhase::Clone,
            KnowledgeRunPhase::Extraction,
            KnowledgeRunPhase::Drafting,
            KnowledgeRunPhase::ExperimentPlanning,
            KnowledgeRunPhase::AdapterExpansion,
            KnowledgeRunPhase::RuntimeVerification,
            KnowledgeRunPhase::Validation,
        ],
    );
    store
        .record_artifact_ref(
            "knowledge-source-dir",
            fixture_source_dir(),
            Some(FIXTURE_FINGERPRINT),
            json!({"fixture": "minimal"}),
        )
        .expect("record source dir");
    drop(store);

    let outcome = KnowledgeReleaseOrchestrator::new(&artifact_root)
        .run_next_required_phase(run_id)
        .expect("run bundle phase");

    assert_eq!(outcome.phase, Some(KnowledgeRunPhase::Bundle));
    assert_eq!(outcome.status.as_str(), "PhaseSucceeded");
    let store = KnowledgeRunStore::open(&artifact_root, run_id).expect("open store");
    let bundle = store
        .latest_artifact_ref("runtime-bundle")
        .expect("artifact query")
        .expect("runtime bundle artifact");
    assert_eq!(
        bundle.target_fingerprint.as_deref(),
        Some(FIXTURE_FINGERPRINT)
    );
    assert!(Path::new(&bundle.path).is_file());
    assert_eq!(
        bundle.detail["checksum"].as_str().expect("checksum").len(),
        16
    );
    assert!(
        bundle.detail["compressedSizeBytes"]
            .as_u64()
            .expect("compressed size")
            > 0
    );
}

#[test]
fn product_validation_patcher_integration_blocks_when_runtime_bundle_targets_a_different_fingerprint(
) {
    let temp = tempfile::tempdir().expect("tempdir");
    let artifact_root = temp.path().join("knowledge");
    let run_id = "run-patcher-fingerprint-block";
    let store = seed_successful_phases(
        &artifact_root,
        run_id,
        "different-target-fingerprint",
        &phases_before(KnowledgeRunPhase::PatcherIntegration),
    );
    let bundle_dir = store.run_dir().join("seed-bundle");
    build_runtime_bundle(fixture_source_dir(), &bundle_dir).expect("build fixture bundle");
    store
        .record_artifact_ref(
            "runtime-bundle",
            bundle_dir.join("knowledge-index.json"),
            Some(FIXTURE_FINGERPRINT),
            json!({"checksum": "seeded", "compressedSizeBytes": 1}),
        )
        .expect("record runtime bundle");
    drop(store);

    let outcome = KnowledgeReleaseOrchestrator::new(&artifact_root)
        .run_next_required_phase(run_id)
        .expect("run patcher integration");

    assert_eq!(outcome.phase, Some(KnowledgeRunPhase::PatcherIntegration));
    assert_eq!(outcome.status.as_str(), "Blocked");
    let store = KnowledgeRunStore::open(&artifact_root, run_id).expect("open store");
    assert!(store
        .blockers()
        .expect("blockers")
        .iter()
        .any(|blocker| blocker.code == "PATCHER_BUNDLE_FINGERPRINT_MISMATCH"));
}

#[test]
fn product_validation_patcher_integration_blocks_without_mismatch_behavior_evidence() {
    let temp = tempfile::tempdir().expect("tempdir");
    let artifact_root = temp.path().join("knowledge");
    let run_id = "run-patcher-evidence-block";
    let store = seed_successful_phases(
        &artifact_root,
        run_id,
        FIXTURE_FINGERPRINT,
        &phases_before(KnowledgeRunPhase::PatcherIntegration),
    );
    seed_runtime_bundle(&store, FIXTURE_FINGERPRINT);
    drop(store);

    let outcome = KnowledgeReleaseOrchestrator::new(&artifact_root)
        .run_next_required_phase(run_id)
        .expect("run patcher integration");

    assert_eq!(outcome.phase, Some(KnowledgeRunPhase::PatcherIntegration));
    assert_eq!(outcome.status.as_str(), "Blocked");
    let store = KnowledgeRunStore::open(&artifact_root, run_id).expect("open store");
    assert!(store
        .blockers()
        .expect("blockers")
        .iter()
        .any(|blocker| blocker.code == "PATCHER_INTEGRATION_EVIDENCE_MISSING"));
}

#[test]
fn product_validation_patcher_integration_accepts_product_evidence_for_exact_fingerprint() {
    let temp = tempfile::tempdir().expect("tempdir");
    let artifact_root = temp.path().join("knowledge");
    let run_id = "run-patcher-product-evidence";
    let store = seed_successful_phases(
        &artifact_root,
        run_id,
        FIXTURE_FINGERPRINT,
        &phases_before(KnowledgeRunPhase::PatcherIntegration),
    );
    seed_runtime_bundle(&store, FIXTURE_FINGERPRINT);
    seed_product_evidence(&store, FIXTURE_FINGERPRINT);
    drop(store);

    let outcome = KnowledgeReleaseOrchestrator::new(&artifact_root)
        .run_next_required_phase(run_id)
        .expect("run patcher integration");

    assert_eq!(outcome.phase, Some(KnowledgeRunPhase::PatcherIntegration));
    assert_eq!(outcome.status.as_str(), "PhaseSucceeded");
    let store = KnowledgeRunStore::open(&artifact_root, run_id).expect("open store");
    assert!(store
        .latest_artifact_ref("patcher-integration-report")
        .expect("artifact query")
        .is_some());
}

#[test]
fn product_validation_patcher_integration_blocks_stale_evidence_fingerprint() {
    let temp = tempfile::tempdir().expect("tempdir");
    let artifact_root = temp.path().join("knowledge");
    let run_id = "run-patcher-stale-evidence";
    let store = seed_successful_phases(
        &artifact_root,
        run_id,
        FIXTURE_FINGERPRINT,
        &phases_before(KnowledgeRunPhase::PatcherIntegration),
    );
    seed_runtime_bundle(&store, FIXTURE_FINGERPRINT);
    seed_product_evidence(&store, "other-fingerprint");
    drop(store);

    let outcome = KnowledgeReleaseOrchestrator::new(&artifact_root)
        .run_next_required_phase(run_id)
        .expect("run patcher integration");

    assert_eq!(outcome.phase, Some(KnowledgeRunPhase::PatcherIntegration));
    assert_eq!(outcome.status.as_str(), "Blocked");
    let store = KnowledgeRunStore::open(&artifact_root, run_id).expect("open store");
    assert!(store
        .blockers()
        .expect("blockers")
        .iter()
        .any(|blocker| { blocker.code == "PATCHER_INTEGRATION_EVIDENCE_FINGERPRINT_MISMATCH" }));
}

#[test]
fn product_validation_blocks_failed_patcher_behavior_and_missing_mcp_queries() {
    let report = ProductValidationReport::from_evidence(
        "run-product",
        FIXTURE_FINGERPRINT,
        product_evidence(
            ProductValidationStatus::Failed,
            ProductValidationStatus::Unavailable,
        ),
    );

    assert!(report
        .blockers
        .iter()
        .any(|blocker| blocker.code == "PRODUCT_VALIDATION_PATCHER_BEHAVIOR_FAILED"));
    assert!(report
        .blockers
        .iter()
        .any(|blocker| blocker.code == "MCP_QUERY_COVERAGE_MISSING"));
}

#[test]
fn product_validation_phase_persists_report_after_patcher_integration() {
    let temp = tempfile::tempdir().expect("tempdir");
    let artifact_root = temp.path().join("knowledge");
    let run_id = "run-product-report";
    let store = seed_successful_phases(
        &artifact_root,
        run_id,
        FIXTURE_FINGERPRINT,
        &phases_before(KnowledgeRunPhase::ProductValidation),
    );
    seed_runtime_bundle(&store, FIXTURE_FINGERPRINT);
    seed_product_evidence(&store, FIXTURE_FINGERPRINT);
    drop(store);

    let outcome = KnowledgeReleaseOrchestrator::new(&artifact_root)
        .run_next_required_phase(run_id)
        .expect("run product validation");

    assert_eq!(outcome.phase, Some(KnowledgeRunPhase::ProductValidation));
    assert_eq!(outcome.status.as_str(), "PhaseSucceeded");
    let store = KnowledgeRunStore::open(&artifact_root, run_id).expect("open store");
    let report_ref = store
        .latest_artifact_ref("product-validation-report")
        .expect("artifact query")
        .expect("product validation report");
    let report: ProductValidationReport =
        serde_json::from_slice(&fs::read(report_ref.path).expect("report bytes"))
            .expect("report json");
    assert!(report.blockers.is_empty());
    assert_eq!(report.target_fingerprint, FIXTURE_FINGERPRINT);
}

#[test]
fn product_validation_phase_blocks_stale_evidence_fingerprint() {
    let temp = tempfile::tempdir().expect("tempdir");
    let artifact_root = temp.path().join("knowledge");
    let run_id = "run-product-stale-evidence";
    let store = seed_successful_phases(
        &artifact_root,
        run_id,
        FIXTURE_FINGERPRINT,
        &phases_before(KnowledgeRunPhase::ProductValidation),
    );
    seed_runtime_bundle(&store, FIXTURE_FINGERPRINT);
    seed_product_evidence(&store, "other-fingerprint");
    drop(store);

    let outcome = KnowledgeReleaseOrchestrator::new(&artifact_root)
        .run_next_required_phase(run_id)
        .expect("run product validation");

    assert_eq!(outcome.phase, Some(KnowledgeRunPhase::ProductValidation));
    assert_eq!(outcome.status.as_str(), "Blocked");
    let store = KnowledgeRunStore::open(&artifact_root, run_id).expect("open store");
    assert!(store
        .blockers()
        .expect("blockers")
        .iter()
        .any(|blocker| blocker.code == "PRODUCT_VALIDATION_EVIDENCE_FINGERPRINT_MISMATCH"));
}

fn seed_product_evidence(store: &KnowledgeRunStore, target_fingerprint: &str) {
    let evidence_path = store.run_dir().join(format!(
        "product-validation-evidence-{target_fingerprint}.json"
    ));
    fs::write(
        &evidence_path,
        serde_json::to_vec_pretty(&product_evidence(
            ProductValidationStatus::Passed,
            ProductValidationStatus::Passed,
        ))
        .expect("evidence json"),
    )
    .expect("write evidence");
    store
        .record_artifact_ref(
            "product-validation-evidence",
            &evidence_path,
            Some(target_fingerprint),
            json!({"source": "fixture evidence"}),
        )
        .expect("record evidence");
}

fn seed_runtime_bundle(store: &KnowledgeRunStore, target_fingerprint: &str) {
    let bundle_dir = store.run_dir().join("seed-bundle");
    build_runtime_bundle(fixture_source_dir(), &bundle_dir).expect("build fixture bundle");
    store
        .record_artifact_ref(
            "runtime-bundle",
            bundle_dir.join("knowledge-index.json"),
            Some(target_fingerprint),
            json!({"checksum": "seeded", "compressedSizeBytes": 1}),
        )
        .expect("record runtime bundle");
}

fn product_evidence(
    patcher_status: ProductValidationStatus,
    mcp_status: ProductValidationStatus,
) -> ProductValidationEvidence {
    ProductValidationEvidence {
        patcher: PatcherValidationEvidence {
            install: check(patcher_status, "patcher install"),
            update: check(ProductValidationStatus::Passed, "patcher update"),
            repair: check(ProductValidationStatus::Passed, "patcher repair"),
            unpatch: check(ProductValidationStatus::Passed, "patcher unpatch"),
            exact_fingerprint_match: check(ProductValidationStatus::Passed, "exact match"),
            mismatched_fingerprint_base_mod_only: check(
                ProductValidationStatus::Passed,
                "base mod only on mismatch",
            ),
            mismatched_fingerprint_knowledge_unavailable: check(
                ProductValidationStatus::Passed,
                "knowledge unavailable on mismatch",
            ),
        },
        mcp: McpQueryValidationEvidence {
            knowledge_status: check(mcp_status, "mpb_knowledge_status"),
            search_entities: check(mcp_status, "mpb_search_entities"),
            entity_card: check(mcp_status, "mpb_get_entity_card"),
            recipe_graph: check(mcp_status, "mpb_get_recipe_graph"),
            mechanic_details: check(mcp_status, "mpb_get_mechanic_details"),
            evidence: check(mcp_status, "mpb_get_evidence"),
        },
        runtime: RuntimeProductValidationEvidence {
            cloned_runtime: check(ProductValidationStatus::Passed, "cloned runtime"),
            tauri_desktop: check(ProductValidationStatus::Passed, "tauri desktop"),
            browser_vite_supplemental: Some(check(
                ProductValidationStatus::Unavailable,
                "browser validation intentionally supplemental",
            )),
        },
    }
}

fn check(status: ProductValidationStatus, label: &str) -> ProductCheck {
    ProductCheck {
        status,
        label: label.to_string(),
        detail: format!("{label} fixture result"),
        artifact_paths: Vec::new(),
    }
}

fn seed_successful_phases(
    artifact_root: &Path,
    run_id: &str,
    target_fingerprint: &str,
    phases: &[KnowledgeRunPhase],
) -> KnowledgeRunStore {
    let store = KnowledgeRunStore::open(artifact_root, run_id).expect("open store");
    store
        .record_run(
            Some(target_fingerprint),
            json!({"createdBy": "product validation test"}),
        )
        .expect("record run");
    for phase in phases {
        store
            .record_phase_checkpoint(
                *phase,
                PhaseCheckpointStatus::Succeeded,
                Some(target_fingerprint),
                json!({
                    "seeded": true,
                    "phase": phase.as_str(),
                    "instancePath": artifact_root.join("missing-instance"),
                }),
            )
            .expect("seed phase");
    }
    store
}

fn phases_before(phase: KnowledgeRunPhase) -> Vec<KnowledgeRunPhase> {
    KnowledgeRunPhase::ALL
        .into_iter()
        .take_while(|candidate| *candidate != phase)
        .collect()
}

fn fixture_source_dir() -> &'static str {
    concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../knowledge/packs/fixtures/minimal/source"
    )
}
