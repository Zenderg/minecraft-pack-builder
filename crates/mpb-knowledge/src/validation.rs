use std::collections::BTreeSet;

use thiserror::Error;

use crate::schema::{
    ClaimKind, EvidenceKind, EvidenceSummary, KnowledgePackSource, MechanicOverlay,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ValidationCode {
    FingerprintMismatch,
    UncoveredEntities,
    IncompleteOverlays,
    BehavioralClaimWithoutRuntimeEvidence,
    IncompleteDependencyChains,
    UnresolvedPlaceholders,
    TrustedWorkerOutput,
    RuntimeBundleQueryGaps,
    MissingManifestMetadata,
}

impl ValidationCode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::FingerprintMismatch => "fingerprint_mismatch",
            Self::UncoveredEntities => "uncovered_entities",
            Self::IncompleteOverlays => "incomplete_overlays",
            Self::BehavioralClaimWithoutRuntimeEvidence => {
                "behavioral_claim_without_runtime_evidence"
            }
            Self::IncompleteDependencyChains => "incomplete_dependency_chains",
            Self::UnresolvedPlaceholders => "unresolved_placeholders",
            Self::TrustedWorkerOutput => "trusted_worker_output",
            Self::RuntimeBundleQueryGaps => "runtime_bundle_query_gaps",
            Self::MissingManifestMetadata => "missing_manifest_metadata",
        }
    }
}

impl std::fmt::Display for ValidationCode {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidationFailure {
    pub code: ValidationCode,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidationReport {
    pub entity_count: usize,
    pub claim_count: usize,
    pub evidence_count: usize,
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
#[error("knowledge validation failed: {summary}")]
pub struct KnowledgeValidationError {
    summary: String,
    failures: Vec<ValidationFailure>,
}

impl KnowledgeValidationError {
    pub fn new(failures: Vec<ValidationFailure>) -> Self {
        let summary = failures
            .iter()
            .map(|failure| failure.code.as_str())
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect::<Vec<_>>()
            .join(", ");
        Self { summary, failures }
    }

    pub fn failures(&self) -> &[ValidationFailure] {
        &self.failures
    }

    pub fn codes(&self) -> BTreeSet<ValidationCode> {
        self.failures.iter().map(|failure| failure.code).collect()
    }
}

pub fn validate_source_pack(
    pack: &KnowledgePackSource,
) -> Result<ValidationReport, KnowledgeValidationError> {
    let mut failures = Vec::new();
    check_manifest(pack, &mut failures);
    check_fingerprints(pack, &mut failures);
    check_coverage(pack, &mut failures);
    check_overlays(pack, &mut failures);
    check_claims(pack, &mut failures);
    check_dependencies(pack, &mut failures);
    check_placeholders(pack, &mut failures);
    check_workers(pack, &mut failures);
    check_runtime_query_coverage(pack, &mut failures);

    if failures.is_empty() {
        Ok(ValidationReport {
            entity_count: pack.entities.len(),
            claim_count: pack.claims.len(),
            evidence_count: pack.evidence.len(),
        })
    } else {
        Err(KnowledgeValidationError::new(failures))
    }
}

fn check_manifest(pack: &KnowledgePackSource, failures: &mut Vec<ValidationFailure>) {
    let manifest = &pack.manifest;
    let required = [
        ("pack_id", &manifest.pack_id),
        ("pack_version", &manifest.pack_version),
        ("schema_version", &manifest.schema_version),
        ("modpack_id", &manifest.modpack_id),
        ("modpack_version", &manifest.modpack_version),
        ("minecraft_version", &manifest.minecraft_version),
        ("loader", &manifest.loader),
        ("loader_version", &manifest.loader_version),
        ("target_fingerprint", &manifest.target_fingerprint),
        ("computed_fingerprint", &manifest.computed_fingerprint),
        ("builder_version", &manifest.builder_version),
        ("lab_version", &manifest.lab_version),
    ];
    for (field, value) in required {
        if value.trim().is_empty() {
            failures.push(ValidationFailure {
                code: ValidationCode::MissingManifestMetadata,
                message: format!("manifest field {field} is required"),
            });
        }
    }
}

fn check_fingerprints(pack: &KnowledgePackSource, failures: &mut Vec<ValidationFailure>) {
    if pack.manifest.target_fingerprint != pack.manifest.computed_fingerprint {
        failures.push(ValidationFailure {
            code: ValidationCode::FingerprintMismatch,
            message: "manifest target_fingerprint and computed_fingerprint differ".to_string(),
        });
    }

    for evidence in &pack.evidence {
        if evidence.fingerprint != pack.manifest.target_fingerprint {
            failures.push(ValidationFailure {
                code: ValidationCode::FingerprintMismatch,
                message: format!("evidence {} targets a different fingerprint", evidence.id),
            });
        }
    }
}

fn check_coverage(pack: &KnowledgePackSource, failures: &mut Vec<ValidationFailure>) {
    let known = pack
        .entities
        .iter()
        .map(|entity| entity.id.as_str())
        .collect::<BTreeSet<_>>();
    let covered = pack
        .coverage
        .covered_entity_ids
        .iter()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();

    for expected in &pack.coverage.expected_entity_ids {
        if !known.contains(expected.as_str()) || !covered.contains(expected.as_str()) {
            failures.push(ValidationFailure {
                code: ValidationCode::UncoveredEntities,
                message: format!("expected entity {expected} is not fully covered"),
            });
        }
    }

    for entity in &pack.entities {
        if !entity.covered || !covered.contains(entity.id.as_str()) {
            failures.push(ValidationFailure {
                code: ValidationCode::UncoveredEntities,
                message: format!("entity {} is not covered", entity.id),
            });
        }
    }
}

fn check_overlays(pack: &KnowledgePackSource, failures: &mut Vec<ValidationFailure>) {
    for overlay in &pack.overlays {
        if overlay_is_incomplete(pack, overlay) {
            failures.push(ValidationFailure {
                code: ValidationCode::IncompleteOverlays,
                message: format!("mechanic overlay {} is incomplete", overlay.id),
            });
        }
    }
}

fn overlay_is_incomplete(pack: &KnowledgePackSource, overlay: &MechanicOverlay) -> bool {
    !overlay.complete
        || overlay.entity_ids.is_empty()
        || overlay.traits.is_empty()
        || !all_evidence_accepted(pack, &overlay.evidence_ids)
        || overlay.traits.iter().any(|trait_record| {
            !trait_record.complete || !all_evidence_accepted(pack, &trait_record.evidence_ids)
        })
}

fn check_claims(pack: &KnowledgePackSource, failures: &mut Vec<ValidationFailure>) {
    for claim in &pack.claims {
        if claim.kind == ClaimKind::Behavioral
            && !claim
                .evidence_ids
                .iter()
                .filter_map(|id| evidence_by_id(pack, id))
                .any(|evidence| {
                    evidence.accepted && evidence.kind == EvidenceKind::RuntimeObservation
                })
        {
            failures.push(ValidationFailure {
                code: ValidationCode::BehavioralClaimWithoutRuntimeEvidence,
                message: format!(
                    "behavioral claim {} lacks accepted runtime evidence",
                    claim.id
                ),
            });
        }

        if !all_evidence_accepted(pack, &claim.evidence_ids) {
            failures.push(ValidationFailure {
                code: ValidationCode::IncompleteDependencyChains,
                message: format!("claim {} references missing or rejected evidence", claim.id),
            });
        }

        if !claim.evidence_ids.is_empty()
            && !claim
                .evidence_ids
                .iter()
                .filter_map(|id| evidence_by_id(pack, id))
                .any(|evidence| evidence.accepted && evidence.kind != EvidenceKind::WorkerOutput)
        {
            failures.push(ValidationFailure {
                code: ValidationCode::TrustedWorkerOutput,
                message: format!("claim {} is backed only by worker output", claim.id),
            });
        }
    }
}

fn check_dependencies(pack: &KnowledgePackSource, failures: &mut Vec<ValidationFailure>) {
    let entities = pack
        .entities
        .iter()
        .map(|entity| entity.id.as_str())
        .collect::<BTreeSet<_>>();

    for recipe in &pack.recipes {
        if !entities.contains(recipe.output_entity_id.as_str())
            || recipe
                .input_entity_ids
                .iter()
                .any(|id| !entities.contains(id.as_str()))
            || !all_evidence_accepted(pack, &recipe.evidence_ids)
        {
            failures.push(ValidationFailure {
                code: ValidationCode::IncompleteDependencyChains,
                message: format!("recipe {} has an incomplete dependency chain", recipe.id),
            });
        }
    }

    for relationship in &pack.relationships {
        if !entities.contains(relationship.from_entity_id.as_str())
            || !entities.contains(relationship.to_entity_id.as_str())
            || !all_evidence_accepted(pack, &relationship.evidence_ids)
        {
            failures.push(ValidationFailure {
                code: ValidationCode::IncompleteDependencyChains,
                message: format!(
                    "relationship {} has an incomplete dependency chain",
                    relationship.id
                ),
            });
        }
    }
}

fn check_placeholders(pack: &KnowledgePackSource, failures: &mut Vec<ValidationFailure>) {
    let serialized = serde_json::to_string(pack)
        .unwrap_or_default()
        .to_lowercase();
    for marker in [
        "unknown",
        "todo",
        "stub",
        "inferred_only",
        "<<<<<<<",
        "=======",
        ">>>>>>>",
        "${",
    ] {
        if serialized.contains(marker) {
            failures.push(ValidationFailure {
                code: ValidationCode::UnresolvedPlaceholders,
                message: format!("placeholder marker {marker:?} is unresolved"),
            });
            return;
        }
    }
}

fn check_workers(pack: &KnowledgePackSource, failures: &mut Vec<ValidationFailure>) {
    for decision in &pack.worker_decisions {
        if decision.trusted {
            failures.push(ValidationFailure {
                code: ValidationCode::TrustedWorkerOutput,
                message: format!("worker decision {} is marked trusted", decision.id),
            });
        }
        if decision
            .converted_evidence_ids
            .iter()
            .any(|id| !evidence_accepted_non_worker(pack, id))
        {
            failures.push(ValidationFailure {
                code: ValidationCode::IncompleteDependencyChains,
                message: format!(
                    "worker decision {} references invalid converted evidence",
                    decision.id
                ),
            });
        }
    }
}

fn check_runtime_query_coverage(pack: &KnowledgePackSource, failures: &mut Vec<ValidationFailure>) {
    let indexes = pack
        .coverage
        .runtime_bundle_query_indexes
        .iter()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    for required in [
        "entity_id",
        "localized_name",
        "tag",
        "use_case",
        "mechanic",
        "interface",
        "recipe_graph",
        "mechanic_details",
        "evidence",
    ] {
        if !indexes.contains(required) {
            failures.push(ValidationFailure {
                code: ValidationCode::RuntimeBundleQueryGaps,
                message: format!("runtime bundle query index {required} is required"),
            });
        }
    }
}

fn evidence_by_id<'a>(pack: &'a KnowledgePackSource, id: &str) -> Option<&'a EvidenceSummary> {
    pack.evidence.iter().find(|evidence| evidence.id == id)
}

fn all_evidence_accepted(pack: &KnowledgePackSource, ids: &[String]) -> bool {
    !ids.is_empty()
        && ids
            .iter()
            .all(|id| evidence_by_id(pack, id).is_some_and(|evidence| evidence.accepted))
}

fn evidence_accepted_non_worker(pack: &KnowledgePackSource, id: &str) -> bool {
    evidence_by_id(pack, id)
        .is_some_and(|evidence| evidence.accepted && evidence.kind != EvidenceKind::WorkerOutput)
}
