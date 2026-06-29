use mpb_knowledge::{
    validate_source_pack, ClaimKind, ClaimRecord, CoverageSummary, EntityKind, EntityRecord,
    EvidenceKind, EvidenceSummary, KnowledgeManifest, KnowledgePackSource, MechanicOverlay,
    MechanicTrait, RecipeRecord, RelationshipRecord, ValidationCode, WorkerDecision,
};

#[test]
fn minimal_fully_covered_pack_passes() {
    let report = validate_source_pack(&valid_pack()).expect("valid pack");

    assert_eq!(report.entity_count, 2);
    assert_eq!(report.claim_count, 2);
    assert_eq!(report.evidence_count, 2);
}

#[test]
fn detects_every_required_validation_gate_with_specific_codes() {
    let mut pack = valid_pack();
    pack.manifest.computed_fingerprint = "different".to_string();
    pack.entities[0].covered = false;
    pack.overlays[0].complete = false;
    pack.claims[1].evidence_ids = vec!["ev-static-stone".to_string()];
    pack.recipes[0]
        .input_entity_ids
        .push("minecraft:missing".to_string());
    pack.entities[1]
        .localized_names
        .insert("en_us".to_string(), "TODO".to_string());
    pack.worker_decisions.push(WorkerDecision {
        id: "worker-trusted".to_string(),
        task: "classify".to_string(),
        model: "Qwen2.5-Coder-1.5B-Instruct".to_string(),
        output_ref: "local-worker-output.json".to_string(),
        trusted: true,
        converted_evidence_ids: Vec::new(),
    });
    pack.coverage
        .runtime_bundle_query_indexes
        .retain(|query| query != "evidence");
    pack.manifest.builder_version.clear();

    let error = validate_source_pack(&pack).expect_err("invalid pack");
    let codes = error.codes();

    assert!(codes.contains(&ValidationCode::FingerprintMismatch));
    assert!(codes.contains(&ValidationCode::UncoveredEntities));
    assert!(codes.contains(&ValidationCode::IncompleteOverlays));
    assert!(codes.contains(&ValidationCode::BehavioralClaimWithoutRuntimeEvidence));
    assert!(codes.contains(&ValidationCode::IncompleteDependencyChains));
    assert!(codes.contains(&ValidationCode::UnresolvedPlaceholders));
    assert!(codes.contains(&ValidationCode::TrustedWorkerOutput));
    assert!(codes.contains(&ValidationCode::RuntimeBundleQueryGaps));
    assert!(codes.contains(&ValidationCode::MissingManifestMetadata));
}

#[test]
fn placeholder_terms_and_conflict_markers_fail_validation() {
    for placeholder in [
        "unknown",
        "todo",
        "stub",
        "inferred_only",
        "<<<<<<< HEAD",
        "${entity}",
    ] {
        let mut pack = valid_pack();
        pack.evidence[0].summary = placeholder.to_string();

        let error = validate_source_pack(&pack).expect_err("placeholder must fail");

        assert!(error
            .codes()
            .contains(&ValidationCode::UnresolvedPlaceholders));
    }
}

#[test]
fn behavioral_claim_requires_accepted_runtime_evidence() {
    let mut pack = valid_pack();
    pack.evidence[1].accepted = false;

    let error = validate_source_pack(&pack).expect_err("runtime evidence is required");

    assert!(error
        .codes()
        .contains(&ValidationCode::BehavioralClaimWithoutRuntimeEvidence));
}

#[test]
fn release_validation_blocks_partial_extraction_and_untrusted_source_paths() {
    let mut pack = valid_pack();
    pack.coverage.partial_extraction = true;
    pack.coverage
        .unsupported_source_kinds
        .push("config:create-client.toml".to_string());
    pack.coverage.clone_runtime_validated = false;
    pack.coverage
        .flaky_experiment_ids
        .push("exp-create-kinetics-flaky".to_string());
    pack.claims[0].evidence_ids = vec!["ev-internet-only".to_string()];
    pack.evidence.push(EvidenceSummary {
        id: "ev-internet-only".to_string(),
        kind: EvidenceKind::InternetSource,
        summary: "A web page says stone is a solid block.".to_string(),
        fingerprint: "fingerprint-fixture".to_string(),
        accepted: true,
    });
    pack.claims.push(ClaimRecord {
        id: "claim-decompile-only".to_string(),
        entity_id: "minecraft:stone".to_string(),
        kind: ClaimKind::Static,
        statement: "A private implementation detail exists.".to_string(),
        evidence_ids: vec!["ev-decompile-only".to_string()],
        worker_decision_ids: Vec::new(),
    });
    pack.evidence.push(EvidenceSummary {
        id: "ev-decompile-only".to_string(),
        kind: EvidenceKind::DecompileOutput,
        summary: "A decompiler showed one branch in a mod class.".to_string(),
        fingerprint: "fingerprint-fixture".to_string(),
        accepted: true,
    });

    let error = validate_source_pack(&pack).expect_err("release blockers must fail validation");
    let codes = error.codes();

    assert!(codes.contains(&ValidationCode::PartialExtraction));
    assert!(codes.contains(&ValidationCode::UnsupportedSourceKind));
    assert!(codes.contains(&ValidationCode::MissingCloneRuntimeValidation));
    assert!(codes.contains(&ValidationCode::InternetOnlyTrust));
    assert!(codes.contains(&ValidationCode::DecompileOnlyTrust));
    assert!(codes.contains(&ValidationCode::FlakyExperiments));
}

pub fn valid_pack() -> KnowledgePackSource {
    KnowledgePackSource {
        manifest: KnowledgeManifest {
            pack_id: "fixture-minimal".to_string(),
            pack_version: "1.0.0".to_string(),
            schema_version: "mpb-knowledge-v1".to_string(),
            modpack_id: "fixture-pack".to_string(),
            modpack_version: "1.0.0".to_string(),
            minecraft_version: "1.21.1".to_string(),
            loader: "NeoForge".to_string(),
            loader_version: "21.1.233".to_string(),
            target_fingerprint: "fingerprint-fixture".to_string(),
            computed_fingerprint: "fingerprint-fixture".to_string(),
            builder_version: "mpb-knowledge-test".to_string(),
            lab_version: "mpb-lab-test".to_string(),
        },
        entities: vec![
            EntityRecord {
                id: "minecraft:stone".to_string(),
                kind: EntityKind::Block,
                localized_names: [("en_us".to_string(), "Stone".to_string())].into(),
                tags: vec!["minecraft:mineable/pickaxe".to_string()],
                use_cases: vec!["build foundations".to_string()],
                interfaces: vec!["solid_block".to_string()],
                mechanics: vec!["mining".to_string()],
                covered: true,
            },
            EntityRecord {
                id: "minecraft:cobblestone".to_string(),
                kind: EntityKind::Block,
                localized_names: [("en_us".to_string(), "Cobblestone".to_string())].into(),
                tags: vec!["minecraft:mineable/pickaxe".to_string()],
                use_cases: vec!["craft stone tools".to_string()],
                interfaces: vec!["solid_block".to_string()],
                mechanics: vec!["mining".to_string()],
                covered: true,
            },
        ],
        claims: vec![
            ClaimRecord {
                id: "claim-stone-static".to_string(),
                entity_id: "minecraft:stone".to_string(),
                kind: ClaimKind::Static,
                statement: "Stone is a solid block.".to_string(),
                evidence_ids: vec!["ev-static-stone".to_string()],
                worker_decision_ids: Vec::new(),
            },
            ClaimRecord {
                id: "claim-stone-drops-cobble".to_string(),
                entity_id: "minecraft:stone".to_string(),
                kind: ClaimKind::Behavioral,
                statement: "Mining stone without Silk Touch drops cobblestone.".to_string(),
                evidence_ids: vec!["ev-runtime-drop".to_string()],
                worker_decision_ids: Vec::new(),
            },
        ],
        evidence: vec![
            EvidenceSummary {
                id: "ev-static-stone".to_string(),
                kind: EvidenceKind::DeterministicSource,
                summary: "Registry and language extraction identify minecraft:stone as Stone."
                    .to_string(),
                fingerprint: "fingerprint-fixture".to_string(),
                accepted: true,
            },
            EvidenceSummary {
                id: "ev-runtime-drop".to_string(),
                kind: EvidenceKind::RuntimeObservation,
                summary: "Lab observation mined stone and observed cobblestone output.".to_string(),
                fingerprint: "fingerprint-fixture".to_string(),
                accepted: true,
            },
        ],
        recipes: vec![RecipeRecord {
            id: "recipe-stone-to-cobble".to_string(),
            output_entity_id: "minecraft:cobblestone".to_string(),
            input_entity_ids: vec!["minecraft:stone".to_string()],
            mechanic: "mining".to_string(),
            evidence_ids: vec!["ev-runtime-drop".to_string()],
        }],
        overlays: vec![MechanicOverlay {
            id: "mining".to_string(),
            entity_ids: vec![
                "minecraft:stone".to_string(),
                "minecraft:cobblestone".to_string(),
            ],
            traits: vec![MechanicTrait {
                id: "drops".to_string(),
                name: "Drop behavior".to_string(),
                evidence_ids: vec!["ev-runtime-drop".to_string()],
                complete: true,
            }],
            evidence_ids: vec!["ev-runtime-drop".to_string()],
            complete: true,
        }],
        relationships: vec![RelationshipRecord {
            id: "rel-stone-cobble".to_string(),
            from_entity_id: "minecraft:stone".to_string(),
            to_entity_id: "minecraft:cobblestone".to_string(),
            relationship_type: "drops".to_string(),
            evidence_ids: vec!["ev-runtime-drop".to_string()],
        }],
        coverage: CoverageSummary {
            expected_entity_ids: vec![
                "minecraft:stone".to_string(),
                "minecraft:cobblestone".to_string(),
            ],
            covered_entity_ids: vec![
                "minecraft:stone".to_string(),
                "minecraft:cobblestone".to_string(),
            ],
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
            clone_runtime_validated: true,
            partial_extraction: false,
            unsupported_source_kinds: Vec::new(),
            flaky_experiment_ids: Vec::new(),
        },
        worker_decisions: Vec::new(),
    }
}
