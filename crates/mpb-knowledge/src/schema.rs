use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KnowledgePackSource {
    pub manifest: KnowledgeManifest,
    #[serde(default)]
    pub entities: Vec<EntityRecord>,
    #[serde(default)]
    pub claims: Vec<ClaimRecord>,
    #[serde(default)]
    pub evidence: Vec<EvidenceSummary>,
    #[serde(default)]
    pub recipes: Vec<RecipeRecord>,
    #[serde(default)]
    pub overlays: Vec<MechanicOverlay>,
    #[serde(default)]
    pub relationships: Vec<RelationshipRecord>,
    pub coverage: CoverageSummary,
    #[serde(default)]
    pub worker_decisions: Vec<WorkerDecision>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KnowledgeManifest {
    #[serde(default)]
    pub pack_id: String,
    #[serde(default)]
    pub pack_version: String,
    #[serde(default)]
    pub schema_version: String,
    #[serde(default)]
    pub modpack_id: String,
    #[serde(default)]
    pub modpack_version: String,
    #[serde(default)]
    pub minecraft_version: String,
    #[serde(default)]
    pub loader: String,
    #[serde(default)]
    pub loader_version: String,
    #[serde(default)]
    pub target_fingerprint: String,
    #[serde(default)]
    pub computed_fingerprint: String,
    #[serde(default)]
    pub builder_version: String,
    #[serde(default)]
    pub lab_version: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EntityKind {
    Block,
    Item,
    Fluid,
    Entity,
    Tag,
    Config,
    Datapack,
    Script,
    ResourcePack,
    Mechanic,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EntityRecord {
    pub id: String,
    pub kind: EntityKind,
    #[serde(default)]
    pub localized_names: BTreeMap<String, String>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub use_cases: Vec<String>,
    #[serde(default)]
    pub interfaces: Vec<String>,
    #[serde(default)]
    pub mechanics: Vec<String>,
    pub covered: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ClaimKind {
    Static,
    Behavioral,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClaimRecord {
    pub id: String,
    pub entity_id: String,
    pub kind: ClaimKind,
    pub statement: String,
    #[serde(default)]
    pub evidence_ids: Vec<String>,
    #[serde(default)]
    pub worker_decision_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceKind {
    DeterministicSource,
    RuntimeObservation,
    WorkerOutput,
    ManualDocumentation,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EvidenceSummary {
    pub id: String,
    pub kind: EvidenceKind,
    pub summary: String,
    pub fingerprint: String,
    pub accepted: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecipeRecord {
    pub id: String,
    pub output_entity_id: String,
    #[serde(default)]
    pub input_entity_ids: Vec<String>,
    pub mechanic: String,
    #[serde(default)]
    pub evidence_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MechanicTrait {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub evidence_ids: Vec<String>,
    pub complete: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MechanicOverlay {
    pub id: String,
    #[serde(default)]
    pub entity_ids: Vec<String>,
    #[serde(default)]
    pub traits: Vec<MechanicTrait>,
    #[serde(default)]
    pub evidence_ids: Vec<String>,
    pub complete: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RelationshipRecord {
    pub id: String,
    pub from_entity_id: String,
    pub to_entity_id: String,
    pub relationship_type: String,
    #[serde(default)]
    pub evidence_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CoverageSummary {
    #[serde(default)]
    pub expected_entity_ids: Vec<String>,
    #[serde(default)]
    pub covered_entity_ids: Vec<String>,
    #[serde(default)]
    pub runtime_bundle_query_indexes: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkerDecision {
    pub id: String,
    pub task: String,
    pub model: String,
    pub output_ref: String,
    pub trusted: bool,
    #[serde(default)]
    pub converted_evidence_ids: Vec<String>,
}
