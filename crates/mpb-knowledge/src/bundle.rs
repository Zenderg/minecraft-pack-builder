use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::fingerprint::stable_checksum;
use crate::{
    validate_source_pack, ClaimRecord, CoverageSummary, EntityRecord, EvidenceSummary,
    KnowledgeManifest, KnowledgePackSource, KnowledgeValidationError, MechanicOverlay,
    RecipeRecord, RelationshipRecord,
};

#[derive(Debug, Error)]
pub enum BundleBuildError {
    #[error("file operation failed: {0}")]
    Io(#[from] std::io::Error),
    #[error("json parse failed: {0}")]
    Json(#[from] serde_json::Error),
    #[error(transparent)]
    Validation(#[from] KnowledgeValidationError),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeBundle {
    pub manifest: RuntimeBundleManifest,
    pub indexes: RuntimeBundleIndexes,
    pub checksums: Vec<BundleChecksum>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeBundleManifest {
    pub pack_id: String,
    pub pack_version: String,
    pub exact_fingerprint: String,
    pub schema_version: String,
    pub builder_version: String,
    pub lab_version: String,
    pub validation_command: String,
    pub validation_timestamp: String,
    pub coverage: CoverageSummary,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeBundleIndexes {
    pub entities_by_id: BTreeMap<String, EntityRecord>,
    pub entities_by_localized_name: BTreeMap<String, Vec<String>>,
    pub entities_by_tag: BTreeMap<String, Vec<String>>,
    pub entities_by_use_case: BTreeMap<String, Vec<String>>,
    pub entities_by_mechanic: BTreeMap<String, Vec<String>>,
    pub entities_by_interface: BTreeMap<String, Vec<String>>,
    pub recipe_graphs: BTreeMap<String, RecipeGraphSlice>,
    pub mechanic_details: BTreeMap<String, MechanicOverlay>,
    pub evidence_by_id: BTreeMap<String, EvidenceSummary>,
    pub claims_by_entity_id: BTreeMap<String, Vec<ClaimRecord>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecipeGraphSlice {
    pub entity_id: String,
    pub recipes: Vec<RecipeRecord>,
    pub relationships: Vec<RelationshipRecord>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BundleChecksum {
    pub path: String,
    pub checksum: String,
}

pub fn validate_source_dir(source_dir: impl AsRef<Path>) -> Result<(), BundleBuildError> {
    let pack = load_source_pack(source_dir)?;
    validate_source_pack(&pack)?;
    Ok(())
}

pub fn build_runtime_bundle(
    source_dir: impl AsRef<Path>,
    output_dir: impl AsRef<Path>,
) -> Result<RuntimeBundle, BundleBuildError> {
    let source_dir = source_dir.as_ref();
    let output_dir = output_dir.as_ref();
    let pack = load_source_pack(source_dir)?;
    validate_source_pack(&pack)?;

    let mut bundle = RuntimeBundle {
        manifest: runtime_manifest(&pack.manifest, &pack.coverage, source_dir),
        indexes: build_indexes(&pack),
        checksums: source_checksums(source_dir)?,
    };
    let payload = serde_json::to_vec_pretty(&bundle)?;
    bundle.checksums.push(BundleChecksum {
        path: "knowledge-index.json".to_string(),
        checksum: stable_checksum(&payload),
    });

    fs::create_dir_all(output_dir)?;
    fs::write(
        output_dir.join("knowledge-index.json"),
        serde_json::to_vec_pretty(&bundle)?,
    )?;
    Ok(bundle)
}

pub fn read_runtime_bundle(path: impl AsRef<Path>) -> Result<RuntimeBundle, BundleBuildError> {
    Ok(serde_json::from_slice(&fs::read(path)?)?)
}

pub fn load_source_pack(
    source_dir: impl AsRef<Path>,
) -> Result<KnowledgePackSource, BundleBuildError> {
    let source_dir = source_dir.as_ref();
    let manifest: KnowledgeManifest = read_json(source_dir.join("manifest.json"))?;
    let entities = read_jsonl(source_dir.join("entities.jsonl"))?;
    let claims = read_jsonl(source_dir.join("claims.jsonl"))?;
    let evidence = read_jsonl(source_dir.join("evidence.jsonl"))?;
    let recipes = read_jsonl(source_dir.join("recipes.jsonl"))?;
    let overlays = read_jsonl(source_dir.join("overlays.jsonl"))?;
    let relationships = read_jsonl(source_dir.join("relationships.jsonl"))?;
    let worker_decisions = read_jsonl(source_dir.join("worker-decisions.jsonl"))?;
    let coverage = coverage_from_records(&entities);
    Ok(KnowledgePackSource {
        manifest,
        entities,
        claims,
        evidence,
        recipes,
        overlays,
        relationships,
        coverage,
        worker_decisions,
    })
}

pub struct RuntimeBundleQuery<'a> {
    bundle: &'a RuntimeBundle,
}

impl<'a> RuntimeBundleQuery<'a> {
    pub fn new(bundle: &'a RuntimeBundle) -> Self {
        Self { bundle }
    }

    pub fn entity_by_id(&self, id: &str) -> Option<&'a EntityRecord> {
        self.bundle.indexes.entities_by_id.get(id)
    }

    pub fn search_by_localized_name(&self, query: &str) -> Vec<&'a EntityRecord> {
        let needle = normalize(query);
        self.bundle
            .indexes
            .entities_by_localized_name
            .iter()
            .filter(|(name, _)| name.contains(&needle))
            .flat_map(|(_, ids)| ids.iter())
            .filter_map(|id| self.entity_by_id(id))
            .collect()
    }

    pub fn search_by_tag(&self, tag: &str) -> Vec<&'a EntityRecord> {
        self.search_index(&self.bundle.indexes.entities_by_tag, tag)
    }

    pub fn search_by_use_case(&self, query: &str) -> Vec<&'a EntityRecord> {
        let needle = normalize(query);
        self.bundle
            .indexes
            .entities_by_use_case
            .iter()
            .filter(|(use_case, _)| use_case.contains(&needle))
            .flat_map(|(_, ids)| ids.iter())
            .filter_map(|id| self.entity_by_id(id))
            .collect()
    }

    pub fn search_by_mechanic(&self, mechanic: &str) -> Vec<&'a EntityRecord> {
        self.search_index(&self.bundle.indexes.entities_by_mechanic, mechanic)
    }

    pub fn search_by_interface(&self, interface: &str) -> Vec<&'a EntityRecord> {
        self.search_index(&self.bundle.indexes.entities_by_interface, interface)
    }

    pub fn recipe_graph_for(&self, entity_id: &str) -> Option<&'a RecipeGraphSlice> {
        self.bundle.indexes.recipe_graphs.get(entity_id)
    }

    pub fn mechanic_details(&self, mechanic: &str) -> Option<&'a MechanicOverlay> {
        self.bundle.indexes.mechanic_details.get(mechanic)
    }

    pub fn evidence(&self, evidence_id: &str) -> Option<&'a EvidenceSummary> {
        self.bundle.indexes.evidence_by_id.get(evidence_id)
    }

    fn search_index(
        &self,
        index: &'a BTreeMap<String, Vec<String>>,
        key: &str,
    ) -> Vec<&'a EntityRecord> {
        index
            .get(&normalize(key))
            .into_iter()
            .flat_map(|ids| ids.iter())
            .filter_map(|id| self.entity_by_id(id))
            .collect()
    }
}

fn runtime_manifest(
    manifest: &KnowledgeManifest,
    coverage: &CoverageSummary,
    source_dir: &Path,
) -> RuntimeBundleManifest {
    RuntimeBundleManifest {
        pack_id: manifest.pack_id.clone(),
        pack_version: manifest.pack_version.clone(),
        exact_fingerprint: manifest.target_fingerprint.clone(),
        schema_version: manifest.schema_version.clone(),
        builder_version: manifest.builder_version.clone(),
        lab_version: manifest.lab_version.clone(),
        validation_command: format!("mpb-knowledge validate-source {}", source_dir.display()),
        validation_timestamp: validation_timestamp(),
        coverage: coverage.clone(),
    }
}

fn build_indexes(pack: &KnowledgePackSource) -> RuntimeBundleIndexes {
    let mut entities_by_id = BTreeMap::new();
    let mut entities_by_localized_name = BTreeMap::<String, Vec<String>>::new();
    let mut entities_by_tag = BTreeMap::<String, Vec<String>>::new();
    let mut entities_by_use_case = BTreeMap::<String, Vec<String>>::new();
    let mut entities_by_mechanic = BTreeMap::<String, Vec<String>>::new();
    let mut entities_by_interface = BTreeMap::<String, Vec<String>>::new();

    for entity in &pack.entities {
        for name in entity.localized_names.values() {
            push_index(
                &mut entities_by_localized_name,
                &normalize(name),
                &entity.id,
            );
        }
        for tag in &entity.tags {
            push_index(&mut entities_by_tag, &normalize(tag), &entity.id);
        }
        for use_case in &entity.use_cases {
            push_index(&mut entities_by_use_case, &normalize(use_case), &entity.id);
        }
        for mechanic in &entity.mechanics {
            push_index(&mut entities_by_mechanic, &normalize(mechanic), &entity.id);
        }
        for interface in &entity.interfaces {
            push_index(
                &mut entities_by_interface,
                &normalize(interface),
                &entity.id,
            );
        }
        entities_by_id.insert(entity.id.clone(), entity.clone());
    }

    RuntimeBundleIndexes {
        entities_by_id,
        entities_by_localized_name,
        entities_by_tag,
        entities_by_use_case,
        entities_by_mechanic,
        entities_by_interface,
        recipe_graphs: recipe_graphs(pack),
        mechanic_details: pack
            .overlays
            .iter()
            .map(|overlay| (overlay.id.clone(), overlay.clone()))
            .collect(),
        evidence_by_id: pack
            .evidence
            .iter()
            .map(|evidence| (evidence.id.clone(), evidence.clone()))
            .collect(),
        claims_by_entity_id: claims_by_entity(pack),
    }
}

fn recipe_graphs(pack: &KnowledgePackSource) -> BTreeMap<String, RecipeGraphSlice> {
    let mut entity_ids = BTreeSet::new();
    for recipe in &pack.recipes {
        entity_ids.insert(recipe.output_entity_id.clone());
        entity_ids.extend(recipe.input_entity_ids.iter().cloned());
    }
    for relationship in &pack.relationships {
        entity_ids.insert(relationship.from_entity_id.clone());
        entity_ids.insert(relationship.to_entity_id.clone());
    }

    entity_ids
        .into_iter()
        .map(|entity_id| {
            let recipes = pack
                .recipes
                .iter()
                .filter(|recipe| {
                    recipe.output_entity_id == entity_id
                        || recipe
                            .input_entity_ids
                            .iter()
                            .any(|input| input == &entity_id)
                })
                .cloned()
                .collect();
            let relationships = pack
                .relationships
                .iter()
                .filter(|relationship| {
                    relationship.from_entity_id == entity_id
                        || relationship.to_entity_id == entity_id
                })
                .cloned()
                .collect();
            (
                entity_id.clone(),
                RecipeGraphSlice {
                    entity_id,
                    recipes,
                    relationships,
                },
            )
        })
        .collect()
}

fn claims_by_entity(pack: &KnowledgePackSource) -> BTreeMap<String, Vec<ClaimRecord>> {
    let mut claims = BTreeMap::<String, Vec<ClaimRecord>>::new();
    for claim in &pack.claims {
        claims
            .entry(claim.entity_id.clone())
            .or_default()
            .push(claim.clone());
    }
    claims
}

fn coverage_from_records(entities: &[EntityRecord]) -> CoverageSummary {
    CoverageSummary {
        expected_entity_ids: entities.iter().map(|entity| entity.id.clone()).collect(),
        covered_entity_ids: entities
            .iter()
            .filter(|entity| entity.covered)
            .map(|entity| entity.id.clone())
            .collect(),
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
    }
}

fn read_json<T: for<'de> Deserialize<'de>>(path: PathBuf) -> Result<T, BundleBuildError> {
    Ok(serde_json::from_slice(&fs::read(path)?)?)
}

fn read_jsonl<T: for<'de> Deserialize<'de>>(path: PathBuf) -> Result<Vec<T>, BundleBuildError> {
    if !path.is_file() {
        return Ok(Vec::new());
    }
    fs::read_to_string(&path)?
        .lines()
        .enumerate()
        .filter(|(_, line)| !line.trim().is_empty())
        .map(|(index, line)| {
            serde_json::from_str(line).map_err(|error| {
                serde_json::Error::io(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    format!("{} line {}: {error}", path.display(), index + 1),
                ))
            })
        })
        .collect::<Result<Vec<_>, _>>()
        .map_err(BundleBuildError::Json)
}

fn source_checksums(source_dir: &Path) -> Result<Vec<BundleChecksum>, BundleBuildError> {
    let mut checksums = Vec::new();
    for file in [
        "manifest.json",
        "entities.jsonl",
        "claims.jsonl",
        "evidence.jsonl",
        "recipes.jsonl",
        "overlays.jsonl",
        "relationships.jsonl",
        "worker-decisions.jsonl",
    ] {
        let path = source_dir.join(file);
        if path.is_file() {
            checksums.push(BundleChecksum {
                path: format!("source/{file}"),
                checksum: stable_checksum(&fs::read(path)?),
            });
        }
    }
    Ok(checksums)
}

fn push_index(index: &mut BTreeMap<String, Vec<String>>, key: &str, entity_id: &str) {
    let ids = index.entry(key.to_string()).or_default();
    if !ids.iter().any(|id| id == entity_id) {
        ids.push(entity_id.to_string());
    }
}

fn normalize(value: &str) -> String {
    value.trim().to_lowercase()
}

fn validation_timestamp() -> String {
    let seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0);
    format!("unix:{seconds}")
}
