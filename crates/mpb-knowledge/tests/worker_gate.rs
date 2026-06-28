use mpb_knowledge::{
    validate_source_pack, ClaimKind, ClaimRecord, CoverageSummary, EntityKind, EntityRecord,
    EvidenceKind, EvidenceSummary, FineTuningDecision, KnowledgeManifest, KnowledgePackSource,
    MechanicOverlay, MechanicTrait, RecipeRecord, RelationshipRecord, ValidationCode,
    WorkerDecision, WorkerOutputEnvelope, WorkerTaskKind,
};

#[test]
fn worker_envelopes_record_allowed_roles_and_model_strategy() {
    let envelope = WorkerOutputEnvelope::new(
        "worker-classify-001",
        WorkerTaskKind::DraftClassification,
        "Qwen2.5-Coder-1.5B-Instruct",
        "fingerprint-aoca",
        "knowledge/worker-decisions/local/classify-001.prompt.json",
        "knowledge/worker-decisions/local/classify-001.output.json",
        FineTuningDecision::NoFineTuningUsed {
            reason: "worker drafts are never trusted directly".to_string(),
        },
    )
    .expect("structured classification worker should be accepted");

    assert_eq!(envelope.model, "Qwen2.5-Coder-1.5B-Instruct");
    assert_eq!(envelope.task_kind, WorkerTaskKind::DraftClassification);
    assert!(envelope.is_structured_transformation_candidate());
    assert!(!envelope.is_source_of_truth());
}

#[test]
fn broader_reasoning_workers_are_reserved_for_experiment_proposals() {
    let envelope = WorkerOutputEnvelope::new(
        "worker-propose-001",
        WorkerTaskKind::ExperimentProposal,
        "Qwen3-4B",
        "fingerprint-aoca",
        "knowledge/worker-decisions/local/propose-001.prompt.json",
        "knowledge/worker-decisions/local/propose-001.output.json",
        FineTuningDecision::FineTuningRequired {
            reason: "proposal precision is below release threshold".to_string(),
        },
    )
    .expect("reserved reasoning worker should be accepted for proposals");

    assert!(envelope.is_reasoning_experiment_candidate());
}

#[test]
fn worker_only_claims_fail_even_when_worker_output_is_marked_accepted() {
    let mut pack = valid_worker_pack();
    pack.claims.push(ClaimRecord {
        id: "claim-worker-only".to_string(),
        entity_id: "minecraft:stone".to_string(),
        kind: ClaimKind::Static,
        statement: "Worker says stone is useful for foundations.".to_string(),
        evidence_ids: vec!["ev-worker-only".to_string()],
        worker_decision_ids: vec!["worker-classify-001".to_string()],
    });
    pack.evidence.push(EvidenceSummary {
        id: "ev-worker-only".to_string(),
        kind: EvidenceKind::WorkerOutput,
        summary: "Worker draft classified minecraft:stone as foundation material.".to_string(),
        fingerprint: "fingerprint-worker".to_string(),
        accepted: true,
    });
    pack.worker_decisions.push(WorkerDecision {
        id: "worker-classify-001".to_string(),
        task: "draft_classification".to_string(),
        model: "Qwen2.5-Coder-1.5B-Instruct".to_string(),
        output_ref: "knowledge/worker-decisions/local/classify-001.output.json".to_string(),
        trusted: false,
        converted_evidence_ids: Vec::new(),
    });

    let error = validate_source_pack(&pack).expect_err("worker-only claims must fail");

    assert!(error.codes().contains(&ValidationCode::TrustedWorkerOutput));
}

#[test]
fn converted_worker_claims_pass_only_with_accepted_non_worker_evidence() {
    let mut pack = valid_worker_pack();
    pack.claims[0].worker_decision_ids = vec!["worker-summary-001".to_string()];
    pack.worker_decisions.push(WorkerDecision {
        id: "worker-summary-001".to_string(),
        task: "summarization".to_string(),
        model: "Qwen2.5-Coder-1.5B-Instruct".to_string(),
        output_ref: "knowledge/worker-decisions/local/summary-001.output.json".to_string(),
        trusted: false,
        converted_evidence_ids: vec!["ev-runtime-stone".to_string()],
    });

    validate_source_pack(&pack).expect("converted evidence-backed claim should pass");
}

fn valid_worker_pack() -> KnowledgePackSource {
    KnowledgePackSource {
        manifest: KnowledgeManifest {
            pack_id: "worker-fixture".to_string(),
            pack_version: "1.0.0".to_string(),
            schema_version: "mpb-knowledge-v1".to_string(),
            modpack_id: "fixture-pack".to_string(),
            modpack_version: "1.0.0".to_string(),
            minecraft_version: "1.21.1".to_string(),
            loader: "NeoForge".to_string(),
            loader_version: "21.1.233".to_string(),
            target_fingerprint: "fingerprint-worker".to_string(),
            computed_fingerprint: "fingerprint-worker".to_string(),
            builder_version: "mpb-knowledge-test".to_string(),
            lab_version: "mpb-lab-test".to_string(),
        },
        entities: vec![EntityRecord {
            id: "minecraft:stone".to_string(),
            kind: EntityKind::Block,
            localized_names: [("en_us".to_string(), "Stone".to_string())].into(),
            tags: vec!["minecraft:mineable/pickaxe".to_string()],
            use_cases: vec!["build foundations".to_string()],
            interfaces: vec!["solid_block".to_string()],
            mechanics: vec!["mining".to_string()],
            covered: true,
        }],
        claims: vec![ClaimRecord {
            id: "claim-runtime-backed".to_string(),
            entity_id: "minecraft:stone".to_string(),
            kind: ClaimKind::Behavioral,
            statement: "Lab observation confirms stone can be mined.".to_string(),
            evidence_ids: vec!["ev-runtime-stone".to_string()],
            worker_decision_ids: Vec::new(),
        }],
        evidence: vec![EvidenceSummary {
            id: "ev-runtime-stone".to_string(),
            kind: EvidenceKind::RuntimeObservation,
            summary: "Runtime lab observation mined minecraft:stone successfully.".to_string(),
            fingerprint: "fingerprint-worker".to_string(),
            accepted: true,
        }],
        recipes: vec![RecipeRecord {
            id: "recipe-stone-self".to_string(),
            output_entity_id: "minecraft:stone".to_string(),
            input_entity_ids: vec!["minecraft:stone".to_string()],
            mechanic: "mining".to_string(),
            evidence_ids: vec!["ev-runtime-stone".to_string()],
        }],
        overlays: vec![MechanicOverlay {
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
        }],
        relationships: vec![RelationshipRecord {
            id: "rel-stone-self".to_string(),
            from_entity_id: "minecraft:stone".to_string(),
            to_entity_id: "minecraft:stone".to_string(),
            relationship_type: "self".to_string(),
            evidence_ids: vec!["ev-runtime-stone".to_string()],
        }],
        coverage: CoverageSummary {
            expected_entity_ids: vec!["minecraft:stone".to_string()],
            covered_entity_ids: vec!["minecraft:stone".to_string()],
            runtime_bundle_query_indexes: vec![
                "entity_id".to_string(),
                "localized_name".to_string(),
                "tag".to_string(),
                "use_case".to_string(),
                "mechanic".to_string(),
                "interface".to_string(),
                "recipe_graph".to_string(),
                "mechanic_details".to_string(),
                "evidence".to_string(),
            ],
        },
        worker_decisions: Vec::new(),
    }
}
