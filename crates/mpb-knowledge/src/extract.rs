use serde::{Deserialize, Serialize};

use crate::{
    ClaimRecord, EntityRecord, EvidenceSummary, MechanicOverlay, RecipeRecord, RelationshipRecord,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExtractionSourceKind {
    Registry,
    BlockState,
    Recipe,
    Tag,
    Language,
    Config,
    Datapack,
    Script,
    ResourcePack,
    Guidebook,
    Tooltip,
    Manual,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExtractionDiagnosticSeverity {
    Blocking,
    Informational,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExtractionDiagnostic {
    pub source: ExtractionSourceKind,
    pub severity: ExtractionDiagnosticSeverity,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ExtractionDraft {
    #[serde(default)]
    pub records: Vec<ExtractedDraftRecord>,
    #[serde(default)]
    pub diagnostics: Vec<ExtractionDiagnostic>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ExtractedDraftRecord {
    Entity(EntityRecord),
    Claim(ClaimRecord),
    Evidence(EvidenceSummary),
    Recipe(RecipeRecord),
    Relationship(RelationshipRecord),
    Overlay(MechanicOverlay),
}

impl ExtractionDraft {
    pub fn from_sources(sources: Vec<ExtractionSourceKind>) -> Self {
        let diagnostics = sources
            .into_iter()
            .filter_map(|source| match source {
                ExtractionSourceKind::Registry
                | ExtractionSourceKind::BlockState
                | ExtractionSourceKind::Recipe
                | ExtractionSourceKind::Tag
                | ExtractionSourceKind::Language
                | ExtractionSourceKind::Config
                | ExtractionSourceKind::Datapack
                | ExtractionSourceKind::Script
                | ExtractionSourceKind::ResourcePack => None,
                unsupported => Some(ExtractionDiagnostic {
                    source: unsupported,
                    severity: ExtractionDiagnosticSeverity::Blocking,
                    message:
                        "source requires deterministic collector support before records can be trusted"
                            .to_string(),
                }),
            })
            .collect();
        Self {
            records: Vec::new(),
            diagnostics,
        }
    }
}
