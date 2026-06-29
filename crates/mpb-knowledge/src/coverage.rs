use std::collections::{BTreeMap, BTreeSet};
use std::fs;

use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::{
    ArtifactRef, ClaimKind, EvidenceKind, EvidenceSummary, ExtractedDraftRecord,
    ExtractionDiagnosticSeverity, ExtractionDraft, ExtractionSourceKind, KnowledgeRunPhase,
    KnowledgeRunStore, RunStateError,
};

pub const UNSUPPORTED_SOURCE_KIND: &str = "UNSUPPORTED_SOURCE_KIND";
pub const PARTIAL_EXTRACTION: &str = "PARTIAL_EXTRACTION";
pub const UNKNOWN_MECHANIC: &str = "UNKNOWN_MECHANIC";
pub const INCOMPLETE_RELATIONSHIP: &str = "INCOMPLETE_RELATIONSHIP";
pub const BEHAVIORAL_CLAIM_WITHOUT_RUNTIME_EVIDENCE: &str =
    "BEHAVIORAL_CLAIM_WITHOUT_RUNTIME_EVIDENCE";
pub const STATIC_CLAIM_WITHOUT_DETERMINISTIC_EVIDENCE: &str =
    "STATIC_CLAIM_WITHOUT_DETERMINISTIC_EVIDENCE";
pub const STALE_FINGERPRINT: &str = "STALE_FINGERPRINT";
pub const UNCOVERED_OBLIGATION: &str = "UNCOVERED_OBLIGATION";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CoverageObligationKind {
    DiscoveredEntity,
    Mechanic,
    Relationship,
    Recipe,
    Trait,
    Overlay,
    Config,
    Datapack,
    Script,
    Resource,
    GuideContent,
    ManualContent,
    TooltipContent,
    StaticClaim,
    BehaviorClaim,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CoverageEvidenceRequirement {
    Deterministic,
    Runtime,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CoverageObligation {
    pub id: String,
    pub kind: CoverageObligationKind,
    pub subject_id: String,
    pub evidence_requirement: CoverageEvidenceRequirement,
    #[serde(default)]
    pub evidence_ids: Vec<String>,
    pub covered: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CoverageBlocker {
    pub code: String,
    pub message: String,
    pub obligation_id: Option<String>,
    #[serde(default)]
    pub affected_evidence_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CoverageEvaluation {
    pub target_fingerprint: String,
    pub obligations: Vec<CoverageObligation>,
    pub blockers: Vec<CoverageBlocker>,
    pub summary: ObligationCoverageSummary,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ObligationCoverageSummary {
    pub total_obligations: usize,
    pub covered_obligations: usize,
    pub deterministic_obligations: usize,
    pub runtime_obligations: usize,
    pub blocker_count: usize,
}

pub fn evaluate_extraction_coverage(
    draft: &ExtractionDraft,
    target_fingerprint: &str,
) -> CoverageEvaluation {
    let mut obligations = Vec::new();
    let mut blockers = Vec::new();
    let mut evidence = BTreeMap::<String, EvidenceSummary>::new();
    let mut entity_ids = BTreeSet::<String>::new();

    for record in &draft.records {
        match record {
            ExtractedDraftRecord::Evidence(summary) => {
                if summary.fingerprint != target_fingerprint {
                    blockers.push(CoverageBlocker {
                        code: STALE_FINGERPRINT.to_string(),
                        message: format!(
                            "evidence {} targets stale fingerprint {}",
                            summary.id, summary.fingerprint
                        ),
                        obligation_id: None,
                        affected_evidence_ids: vec![summary.id.clone()],
                    });
                }
                evidence.insert(summary.id.clone(), summary.clone());
            }
            ExtractedDraftRecord::Entity(entity) => {
                entity_ids.insert(entity.id.clone());
                push_obligation(
                    &mut obligations,
                    CoverageObligation {
                        id: format!("entity:{}", entity.id),
                        kind: CoverageObligationKind::DiscoveredEntity,
                        subject_id: entity.id.clone(),
                        evidence_requirement: CoverageEvidenceRequirement::Deterministic,
                        evidence_ids: Vec::new(),
                        covered: entity.covered,
                    },
                );
                if !entity.covered {
                    blockers.push(CoverageBlocker {
                        code: UNCOVERED_OBLIGATION.to_string(),
                        message: format!("entity {} is not marked covered", entity.id),
                        obligation_id: Some(format!("entity:{}", entity.id)),
                        affected_evidence_ids: Vec::new(),
                    });
                }
                for mechanic in &entity.mechanics {
                    push_obligation(
                        &mut obligations,
                        CoverageObligation {
                            id: format!("mechanic:{mechanic}"),
                            kind: CoverageObligationKind::Mechanic,
                            subject_id: mechanic.clone(),
                            evidence_requirement: CoverageEvidenceRequirement::Runtime,
                            evidence_ids: Vec::new(),
                            covered: !is_unknown(mechanic),
                        },
                    );
                    if is_unknown(mechanic) {
                        blockers.push(CoverageBlocker {
                            code: UNKNOWN_MECHANIC.to_string(),
                            message: format!("mechanic {mechanic} requires adapter support"),
                            obligation_id: Some(format!("mechanic:{mechanic}")),
                            affected_evidence_ids: Vec::new(),
                        });
                    }
                }
            }
            _ => {}
        }
    }

    for diagnostic in &draft.diagnostics {
        if diagnostic.severity == ExtractionDiagnosticSeverity::Blocking {
            let obligation = source_obligation(diagnostic.source.clone());
            push_obligation(&mut obligations, obligation.clone());
            blockers.push(CoverageBlocker {
                code: UNSUPPORTED_SOURCE_KIND.to_string(),
                message: diagnostic.message.clone(),
                obligation_id: Some(obligation.id),
                affected_evidence_ids: Vec::new(),
            });
        }
    }

    for record in &draft.records {
        match record {
            ExtractedDraftRecord::Claim(claim) => {
                let (kind, requirement, code) = match claim.kind {
                    ClaimKind::Static => (
                        CoverageObligationKind::StaticClaim,
                        CoverageEvidenceRequirement::Deterministic,
                        STATIC_CLAIM_WITHOUT_DETERMINISTIC_EVIDENCE,
                    ),
                    ClaimKind::Behavioral => (
                        CoverageObligationKind::BehaviorClaim,
                        CoverageEvidenceRequirement::Runtime,
                        BEHAVIORAL_CLAIM_WITHOUT_RUNTIME_EVIDENCE,
                    ),
                };
                let covered = evidence_satisfies(&evidence, &claim.evidence_ids, requirement);
                let obligation_id = match claim.kind {
                    ClaimKind::Static => format!("static_claim:{}", claim.id),
                    ClaimKind::Behavioral => format!("behavior_claim:{}", claim.id),
                };
                push_obligation(
                    &mut obligations,
                    CoverageObligation {
                        id: obligation_id.clone(),
                        kind,
                        subject_id: claim.id.clone(),
                        evidence_requirement: requirement,
                        evidence_ids: claim.evidence_ids.clone(),
                        covered,
                    },
                );
                if !covered {
                    blockers.push(CoverageBlocker {
                        code: code.to_string(),
                        message: format!("claim {} lacks accepted required evidence", claim.id),
                        obligation_id: Some(obligation_id),
                        affected_evidence_ids: claim.evidence_ids.clone(),
                    });
                }
            }
            ExtractedDraftRecord::Recipe(recipe) => {
                let covered = all_evidence_accepted(&evidence, &recipe.evidence_ids)
                    && entity_ids.contains(&recipe.output_entity_id)
                    && recipe
                        .input_entity_ids
                        .iter()
                        .all(|id| entity_ids.contains(id));
                let obligation_id = format!("recipe:{}", recipe.id);
                push_obligation(
                    &mut obligations,
                    CoverageObligation {
                        id: obligation_id.clone(),
                        kind: CoverageObligationKind::Recipe,
                        subject_id: recipe.id.clone(),
                        evidence_requirement: CoverageEvidenceRequirement::Deterministic,
                        evidence_ids: recipe.evidence_ids.clone(),
                        covered,
                    },
                );
                if !covered {
                    blockers.push(CoverageBlocker {
                        code: PARTIAL_EXTRACTION.to_string(),
                        message: format!("recipe {} has incomplete extracted coverage", recipe.id),
                        obligation_id: Some(obligation_id),
                        affected_evidence_ids: recipe.evidence_ids.clone(),
                    });
                }
            }
            ExtractedDraftRecord::Relationship(relationship) => {
                let covered = all_evidence_accepted(&evidence, &relationship.evidence_ids)
                    && entity_ids.contains(&relationship.from_entity_id)
                    && entity_ids.contains(&relationship.to_entity_id);
                let obligation_id = format!("relationship:{}", relationship.id);
                push_obligation(
                    &mut obligations,
                    CoverageObligation {
                        id: obligation_id.clone(),
                        kind: CoverageObligationKind::Relationship,
                        subject_id: relationship.id.clone(),
                        evidence_requirement: CoverageEvidenceRequirement::Runtime,
                        evidence_ids: relationship.evidence_ids.clone(),
                        covered,
                    },
                );
                if !covered {
                    blockers.push(CoverageBlocker {
                        code: INCOMPLETE_RELATIONSHIP.to_string(),
                        message: format!(
                            "relationship {} has incomplete extracted coverage",
                            relationship.id
                        ),
                        obligation_id: Some(obligation_id),
                        affected_evidence_ids: relationship.evidence_ids.clone(),
                    });
                }
            }
            ExtractedDraftRecord::Overlay(overlay) => {
                let covered =
                    overlay.complete && all_evidence_accepted(&evidence, &overlay.evidence_ids);
                let obligation_id = format!("overlay:{}", overlay.id);
                push_obligation(
                    &mut obligations,
                    CoverageObligation {
                        id: obligation_id.clone(),
                        kind: CoverageObligationKind::Overlay,
                        subject_id: overlay.id.clone(),
                        evidence_requirement: CoverageEvidenceRequirement::Runtime,
                        evidence_ids: overlay.evidence_ids.clone(),
                        covered,
                    },
                );
                if !covered {
                    blockers.push(CoverageBlocker {
                        code: PARTIAL_EXTRACTION.to_string(),
                        message: format!("overlay {} is incomplete", overlay.id),
                        obligation_id: Some(obligation_id),
                        affected_evidence_ids: overlay.evidence_ids.clone(),
                    });
                }
                for trait_record in &overlay.traits {
                    let trait_covered = trait_record.complete
                        && all_evidence_accepted(&evidence, &trait_record.evidence_ids);
                    let trait_obligation_id = format!("trait:{}:{}", overlay.id, trait_record.id);
                    push_obligation(
                        &mut obligations,
                        CoverageObligation {
                            id: trait_obligation_id.clone(),
                            kind: CoverageObligationKind::Trait,
                            subject_id: trait_record.id.clone(),
                            evidence_requirement: CoverageEvidenceRequirement::Runtime,
                            evidence_ids: trait_record.evidence_ids.clone(),
                            covered: trait_covered,
                        },
                    );
                    if !trait_covered {
                        blockers.push(CoverageBlocker {
                            code: PARTIAL_EXTRACTION.to_string(),
                            message: format!(
                                "trait {} on overlay {} is incomplete",
                                trait_record.id, overlay.id
                            ),
                            obligation_id: Some(trait_obligation_id),
                            affected_evidence_ids: trait_record.evidence_ids.clone(),
                        });
                    }
                }
            }
            ExtractedDraftRecord::Entity(_) | ExtractedDraftRecord::Evidence(_) => {}
        }
    }

    obligations.sort_by(|left, right| left.id.cmp(&right.id));
    blockers.sort_by(|left, right| {
        left.code
            .cmp(&right.code)
            .then_with(|| left.obligation_id.cmp(&right.obligation_id))
    });
    let summary = ObligationCoverageSummary {
        total_obligations: obligations.len(),
        covered_obligations: obligations
            .iter()
            .filter(|obligation| obligation.covered)
            .count(),
        deterministic_obligations: obligations
            .iter()
            .filter(|obligation| {
                obligation.evidence_requirement == CoverageEvidenceRequirement::Deterministic
            })
            .count(),
        runtime_obligations: obligations
            .iter()
            .filter(|obligation| {
                obligation.evidence_requirement == CoverageEvidenceRequirement::Runtime
            })
            .count(),
        blocker_count: blockers.len(),
    };

    CoverageEvaluation {
        target_fingerprint: target_fingerprint.to_string(),
        obligations,
        blockers,
        summary,
    }
}

pub fn persist_coverage_summary(
    store: &KnowledgeRunStore,
    phase: KnowledgeRunPhase,
    evaluation: &CoverageEvaluation,
) -> Result<ArtifactRef, RunStateError> {
    let coverage_dir = store.run_dir().join("coverage");
    fs::create_dir_all(&coverage_dir)?;
    let path = coverage_dir.join(format!("{}-summary.json", phase.as_str()));
    fs::write(&path, serde_json::to_vec_pretty(evaluation)?)?;
    let artifact = store.record_artifact_ref(
        "coverage-summary",
        &path,
        Some(&evaluation.target_fingerprint),
        json!({
            "phase": phase.as_str(),
            "totalObligations": evaluation.summary.total_obligations,
            "coveredObligations": evaluation.summary.covered_obligations,
            "blockerCount": evaluation.summary.blocker_count,
        }),
    )?;
    store.append_event(
        "coverage.summary",
        Some(&evaluation.target_fingerprint),
        json!({
            "phase": phase.as_str(),
            "artifactId": artifact.id,
            "path": artifact.path,
            "summary": evaluation.summary,
            "blockerCodes": evaluation
                .blockers
                .iter()
                .map(|blocker| blocker.code.as_str())
                .collect::<BTreeSet<_>>(),
        }),
    )?;
    Ok(artifact)
}

fn push_obligation(obligations: &mut Vec<CoverageObligation>, obligation: CoverageObligation) {
    if !obligations
        .iter()
        .any(|existing| existing.id == obligation.id)
    {
        obligations.push(obligation);
    }
}

fn source_obligation(source: ExtractionSourceKind) -> CoverageObligation {
    let (id, kind) = match source {
        ExtractionSourceKind::Config => ("config:config", CoverageObligationKind::Config),
        ExtractionSourceKind::Datapack => ("datapack:datapack", CoverageObligationKind::Datapack),
        ExtractionSourceKind::Script => ("script:script", CoverageObligationKind::Script),
        ExtractionSourceKind::ResourcePack => {
            ("resource:resource_pack", CoverageObligationKind::Resource)
        }
        ExtractionSourceKind::Guidebook => {
            ("guide:guidebook", CoverageObligationKind::GuideContent)
        }
        ExtractionSourceKind::Manual => ("manual:manual", CoverageObligationKind::ManualContent),
        ExtractionSourceKind::Tooltip => {
            ("tooltip:tooltip", CoverageObligationKind::TooltipContent)
        }
        ExtractionSourceKind::Registry => ("resource:registry", CoverageObligationKind::Resource),
        ExtractionSourceKind::BlockState => {
            ("resource:block_state", CoverageObligationKind::Resource)
        }
        ExtractionSourceKind::Recipe => ("recipe:recipe_source", CoverageObligationKind::Recipe),
        ExtractionSourceKind::Tag => ("resource:tag", CoverageObligationKind::Resource),
        ExtractionSourceKind::Language => ("resource:language", CoverageObligationKind::Resource),
    };
    CoverageObligation {
        id: id.to_string(),
        kind,
        subject_id: id.to_string(),
        evidence_requirement: CoverageEvidenceRequirement::Deterministic,
        evidence_ids: Vec::new(),
        covered: false,
    }
}

fn is_unknown(value: &str) -> bool {
    let normalized = value.trim().to_lowercase();
    normalized.is_empty()
        || normalized == "unknown"
        || normalized.starts_with("unknown:")
        || normalized.contains("unknown_mechanic")
}

fn evidence_satisfies(
    evidence: &BTreeMap<String, EvidenceSummary>,
    ids: &[String],
    requirement: CoverageEvidenceRequirement,
) -> bool {
    !ids.is_empty()
        && ids
            .iter()
            .filter_map(|id| evidence.get(id))
            .any(|summary| match requirement {
                CoverageEvidenceRequirement::Deterministic => {
                    summary.accepted
                        && matches!(
                            summary.kind,
                            EvidenceKind::DeterministicSource | EvidenceKind::ManualDocumentation
                        )
                }
                CoverageEvidenceRequirement::Runtime => {
                    summary.accepted && summary.kind == EvidenceKind::RuntimeObservation
                }
            })
}

fn all_evidence_accepted(evidence: &BTreeMap<String, EvidenceSummary>, ids: &[String]) -> bool {
    !ids.is_empty()
        && ids
            .iter()
            .all(|id| evidence.get(id).is_some_and(|summary| summary.accepted))
}
