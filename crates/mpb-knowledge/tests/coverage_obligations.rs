use std::fs;

use mpb_knowledge::{
    evaluate_extraction_coverage, persist_coverage_summary, ClaimKind, ClaimRecord, EntityKind,
    EntityRecord, EvidenceKind, EvidenceSummary, ExtractedDraftRecord, ExtractionDiagnostic,
    ExtractionDiagnosticSeverity, ExtractionDraft, ExtractionSourceKind,
    KnowledgeReleaseOrchestrator, KnowledgeRunPhase, KnowledgeRunStore, MechanicOverlay,
    MechanicTrait, OrchestratorRunStatus, PhaseCheckpointStatus, RecipeRecord, RelationshipRecord,
};
use serde_json::json;

const TARGET_FINGERPRINT: &str = "fingerprint-coverage";

#[test]
fn unsupported_config_diagnostics_become_specific_release_blockers() {
    let draft = ExtractionDraft {
        records: vec![ExtractedDraftRecord::Entity(entity("create:stressometer"))],
        diagnostics: vec![ExtractionDiagnostic {
            source: ExtractionSourceKind::Config,
            severity: ExtractionDiagnosticSeverity::Blocking,
            message: "unsupported config grammar affected discovered content".to_string(),
        }],
    };

    let evaluation = evaluate_extraction_coverage(&draft, TARGET_FINGERPRINT);

    assert!(evaluation
        .blockers
        .iter()
        .any(|blocker| blocker.code == "UNSUPPORTED_SOURCE_KIND"
            && blocker.obligation_id.as_deref() == Some("config:config")));
}

#[test]
fn unknown_mechanics_create_obligations_and_specific_blockers() {
    let mut entity = entity("create:depot");
    entity.mechanics = vec!["unknown:create_processing".to_string()];
    let draft = ExtractionDraft {
        records: vec![ExtractedDraftRecord::Entity(entity)],
        diagnostics: Vec::new(),
    };

    let evaluation = evaluate_extraction_coverage(&draft, TARGET_FINGERPRINT);

    assert!(evaluation
        .obligations
        .iter()
        .any(|obligation| obligation.id == "mechanic:unknown:create_processing"));
    assert!(evaluation
        .blockers
        .iter()
        .any(|blocker| blocker.code == "UNKNOWN_MECHANIC"));
}

#[test]
fn incomplete_relationships_block_until_accepted_evidence_exists() {
    let draft = ExtractionDraft {
        records: vec![
            ExtractedDraftRecord::Entity(entity("create:shaft")),
            ExtractedDraftRecord::Entity(entity("create:cogwheel")),
            ExtractedDraftRecord::Relationship(RelationshipRecord {
                id: "rel-shaft-cog".to_string(),
                from_entity_id: "create:shaft".to_string(),
                to_entity_id: "create:cogwheel".to_string(),
                relationship_type: "transmits_rotation".to_string(),
                evidence_ids: vec!["missing-evidence".to_string()],
            }),
        ],
        diagnostics: Vec::new(),
    };

    let evaluation = evaluate_extraction_coverage(&draft, TARGET_FINGERPRINT);

    assert!(evaluation
        .blockers
        .iter()
        .any(|blocker| blocker.code == "INCOMPLETE_RELATIONSHIP"
            && blocker.obligation_id.as_deref() == Some("relationship:rel-shaft-cog")));
}

#[test]
fn behavioral_claim_obligations_require_accepted_runtime_observations() {
    let draft = ExtractionDraft {
        records: vec![
            ExtractedDraftRecord::Entity(entity("minecraft:stone")),
            ExtractedDraftRecord::Evidence(EvidenceSummary {
                id: "ev-static-stone".to_string(),
                kind: EvidenceKind::DeterministicSource,
                summary: "Registry extraction found minecraft:stone.".to_string(),
                fingerprint: TARGET_FINGERPRINT.to_string(),
                accepted: true,
            }),
            ExtractedDraftRecord::Claim(ClaimRecord {
                id: "claim-stone-drop".to_string(),
                entity_id: "minecraft:stone".to_string(),
                kind: ClaimKind::Behavioral,
                statement: "Mining stone drops cobblestone.".to_string(),
                evidence_ids: vec!["ev-static-stone".to_string()],
                worker_decision_ids: Vec::new(),
            }),
        ],
        diagnostics: Vec::new(),
    };

    let evaluation = evaluate_extraction_coverage(&draft, TARGET_FINGERPRINT);

    assert!(evaluation.blockers.iter().any(|blocker| {
        blocker.code == "BEHAVIORAL_CLAIM_WITHOUT_RUNTIME_EVIDENCE"
            && blocker.obligation_id.as_deref() == Some("behavior_claim:claim-stone-drop")
    }));
}

#[test]
fn stale_fingerprint_evidence_blocks_coverage() {
    let draft = ExtractionDraft {
        records: vec![ExtractedDraftRecord::Evidence(EvidenceSummary {
            id: "ev-stale".to_string(),
            kind: EvidenceKind::DeterministicSource,
            summary: "Old extraction artifact.".to_string(),
            fingerprint: "fingerprint-old".to_string(),
            accepted: true,
        })],
        diagnostics: Vec::new(),
    };

    let evaluation = evaluate_extraction_coverage(&draft, TARGET_FINGERPRINT);

    assert!(evaluation
        .blockers
        .iter()
        .any(|blocker| blocker.code == "STALE_FINGERPRINT"));
}

#[test]
fn coverage_summary_is_persisted_as_durable_run_state() {
    let temp = tempfile::tempdir().expect("tempdir");
    let artifact_root = temp.path().join("knowledge");
    let store = KnowledgeRunStore::open(&artifact_root, "run-coverage").expect("open store");
    store
        .record_run(
            Some(TARGET_FINGERPRINT),
            json!({"createdBy": "coverage test"}),
        )
        .expect("record run");
    let draft = ExtractionDraft {
        records: vec![
            ExtractedDraftRecord::Entity(entity("minecraft:stone")),
            ExtractedDraftRecord::Evidence(EvidenceSummary {
                id: "ev-runtime-stone".to_string(),
                kind: EvidenceKind::RuntimeObservation,
                summary: "Runtime lab observed stone behavior.".to_string(),
                fingerprint: TARGET_FINGERPRINT.to_string(),
                accepted: true,
            }),
            ExtractedDraftRecord::Claim(ClaimRecord {
                id: "claim-stone-runtime".to_string(),
                entity_id: "minecraft:stone".to_string(),
                kind: ClaimKind::Behavioral,
                statement: "Stone can be mined.".to_string(),
                evidence_ids: vec!["ev-runtime-stone".to_string()],
                worker_decision_ids: Vec::new(),
            }),
            ExtractedDraftRecord::Recipe(RecipeRecord {
                id: "recipe-stone-self".to_string(),
                output_entity_id: "minecraft:stone".to_string(),
                input_entity_ids: vec!["minecraft:stone".to_string()],
                mechanic: "mining".to_string(),
                evidence_ids: vec!["ev-runtime-stone".to_string()],
            }),
            ExtractedDraftRecord::Overlay(MechanicOverlay {
                id: "mining".to_string(),
                entity_ids: vec!["minecraft:stone".to_string()],
                traits: vec![MechanicTrait {
                    id: "mineable".to_string(),
                    name: "Mineable".to_string(),
                    evidence_ids: vec!["ev-runtime-stone".to_string()],
                    complete: true,
                }],
                evidence_ids: vec!["ev-runtime-stone".to_string()],
                complete: true,
            }),
        ],
        diagnostics: Vec::new(),
    };
    let evaluation = evaluate_extraction_coverage(&draft, TARGET_FINGERPRINT);

    let artifact = persist_coverage_summary(&store, KnowledgeRunPhase::Extraction, &evaluation)
        .expect("persist summary");

    assert_eq!(artifact.artifact_kind, "coverage-summary");
    assert!(fs::metadata(&artifact.path)
        .expect("coverage summary file")
        .is_file());
    let reopened = KnowledgeRunStore::open(&artifact_root, "run-coverage").expect("reopen");
    let latest = reopened
        .latest_artifact_ref("coverage-summary")
        .expect("query artifact")
        .expect("latest coverage summary");
    assert_eq!(
        latest.target_fingerprint.as_deref(),
        Some(TARGET_FINGERPRINT)
    );
    assert!(reopened
        .events()
        .expect("events")
        .iter()
        .any(
            |event| event.event_kind == "coverage.summary" && event.detail["phase"] == "Extraction"
        ));
}

#[test]
fn orchestrator_extraction_phase_writes_coverage_summary_and_blocks_on_obligations() {
    let temp = tempfile::tempdir().expect("tempdir");
    let artifact_root = temp.path().join("knowledge");
    let run_id = "run-extraction-blocks";
    let store = seed_successful_phases(
        &artifact_root,
        run_id,
        &[
            KnowledgeRunPhase::Intake,
            KnowledgeRunPhase::Preflight,
            KnowledgeRunPhase::Approvals,
            KnowledgeRunPhase::Fingerprint,
            KnowledgeRunPhase::Clone,
        ],
    );
    let draft = ExtractionDraft {
        records: vec![
            ExtractedDraftRecord::Entity(entity("minecraft:stone")),
            ExtractedDraftRecord::Claim(ClaimRecord {
                id: "claim-stone-runtime".to_string(),
                entity_id: "minecraft:stone".to_string(),
                kind: ClaimKind::Behavioral,
                statement: "Stone drops cobblestone when mined.".to_string(),
                evidence_ids: Vec::new(),
                worker_decision_ids: Vec::new(),
            }),
        ],
        diagnostics: Vec::new(),
    };
    let draft_path = store.run_dir().join("extraction-draft.json");
    fs::write(
        &draft_path,
        serde_json::to_vec_pretty(&draft).expect("draft json"),
    )
    .expect("write draft");
    store
        .record_artifact_ref(
            "extraction-draft",
            &draft_path,
            Some(TARGET_FINGERPRINT),
            json!({"format": "json"}),
        )
        .expect("record draft");
    drop(store);

    let outcome = KnowledgeReleaseOrchestrator::new(&artifact_root)
        .run_next_required_phase(run_id)
        .expect("run extraction phase");

    assert_eq!(outcome.phase, Some(KnowledgeRunPhase::Extraction));
    assert_eq!(outcome.status, OrchestratorRunStatus::Blocked);
    assert!(outcome
        .blocking_report_path
        .as_ref()
        .expect("blocking report")
        .is_file());
    let reopened = KnowledgeRunStore::open(&artifact_root, run_id).expect("reopen");
    assert!(reopened
        .latest_artifact_ref("coverage-summary")
        .expect("coverage artifact query")
        .is_some());
    assert!(reopened
        .blockers()
        .expect("blockers")
        .iter()
        .any(|blocker| blocker.code == "BEHAVIORAL_CLAIM_WITHOUT_RUNTIME_EVIDENCE"));
}

#[test]
fn orchestrator_validation_phase_blocks_when_persisted_obligations_remain_uncovered() {
    let temp = tempfile::tempdir().expect("tempdir");
    let artifact_root = temp.path().join("knowledge");
    let run_id = "run-validation-blocks";
    let store = seed_successful_phases(
        &artifact_root,
        run_id,
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
        ],
    );
    let draft = ExtractionDraft {
        records: vec![ExtractedDraftRecord::Relationship(RelationshipRecord {
            id: "rel-missing".to_string(),
            from_entity_id: "minecraft:stone".to_string(),
            to_entity_id: "minecraft:cobblestone".to_string(),
            relationship_type: "drops".to_string(),
            evidence_ids: Vec::new(),
        })],
        diagnostics: Vec::new(),
    };
    let evaluation = evaluate_extraction_coverage(&draft, TARGET_FINGERPRINT);
    persist_coverage_summary(&store, KnowledgeRunPhase::Extraction, &evaluation)
        .expect("persist extraction coverage summary");
    drop(store);

    let outcome = KnowledgeReleaseOrchestrator::new(&artifact_root)
        .run_next_required_phase(run_id)
        .expect("run validation phase");

    assert_eq!(outcome.phase, Some(KnowledgeRunPhase::Validation));
    assert_eq!(outcome.status, OrchestratorRunStatus::Blocked);
    let reopened = KnowledgeRunStore::open(&artifact_root, run_id).expect("reopen");
    assert!(reopened
        .blockers()
        .expect("blockers")
        .iter()
        .any(|blocker| blocker.code == "INCOMPLETE_RELATIONSHIP"));
}

#[test]
fn orchestrator_validation_phase_blocks_without_source_pack_after_coverage_passes() {
    let temp = tempfile::tempdir().expect("tempdir");
    let artifact_root = temp.path().join("knowledge");
    let run_id = "run-validation-missing-source-pack";
    let store = seed_successful_phases(
        &artifact_root,
        run_id,
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
        ],
    );
    let draft = ExtractionDraft {
        records: vec![
            ExtractedDraftRecord::Entity(entity("minecraft:stone")),
            ExtractedDraftRecord::Evidence(EvidenceSummary {
                id: "ev-static-stone".to_string(),
                kind: EvidenceKind::DeterministicSource,
                summary: "Registry extraction found minecraft:stone.".to_string(),
                fingerprint: TARGET_FINGERPRINT.to_string(),
                accepted: true,
            }),
            ExtractedDraftRecord::Claim(ClaimRecord {
                id: "claim-stone-static".to_string(),
                entity_id: "minecraft:stone".to_string(),
                kind: ClaimKind::Static,
                statement: "Stone is present.".to_string(),
                evidence_ids: vec!["ev-static-stone".to_string()],
                worker_decision_ids: Vec::new(),
            }),
        ],
        diagnostics: Vec::new(),
    };
    let evaluation = evaluate_extraction_coverage(&draft, TARGET_FINGERPRINT);
    assert!(evaluation.blockers.is_empty());
    persist_coverage_summary(&store, KnowledgeRunPhase::Extraction, &evaluation)
        .expect("persist extraction coverage summary");
    drop(store);

    let outcome = KnowledgeReleaseOrchestrator::new(&artifact_root)
        .run_next_required_phase(run_id)
        .expect("run validation phase");

    assert_eq!(outcome.phase, Some(KnowledgeRunPhase::Validation));
    assert_eq!(outcome.status, OrchestratorRunStatus::Blocked);
    let reopened = KnowledgeRunStore::open(&artifact_root, run_id).expect("reopen");
    assert!(reopened
        .blockers()
        .expect("blockers")
        .iter()
        .any(|blocker| blocker.code == "VALIDATION_SOURCE_PACK_MISSING"));
}

fn entity(id: &str) -> EntityRecord {
    EntityRecord {
        id: id.to_string(),
        kind: EntityKind::Block,
        localized_names: [("en_us".to_string(), id.to_string())].into(),
        tags: Vec::new(),
        use_cases: Vec::new(),
        interfaces: Vec::new(),
        mechanics: vec!["mining".to_string()],
        covered: true,
    }
}

fn seed_successful_phases(
    artifact_root: &std::path::Path,
    run_id: &str,
    phases: &[KnowledgeRunPhase],
) -> KnowledgeRunStore {
    let store = KnowledgeRunStore::open(artifact_root, run_id).expect("open store");
    store
        .record_run(
            Some(TARGET_FINGERPRINT),
            json!({"createdBy": "coverage orchestrator test"}),
        )
        .expect("record run");
    for phase in phases {
        store
            .record_phase_checkpoint(
                *phase,
                PhaseCheckpointStatus::Succeeded,
                Some(TARGET_FINGERPRINT),
                json!({
                    "seeded": true,
                    "phase": phase.as_str(),
                    "instancePath": artifact_root.join("missing-instance"),
                }),
            )
            .expect("record checkpoint");
    }
    store
}
