//! Authoritative scheme domain model, operations, validation, and materials.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Dimensions {
    pub x: i32,
    pub y: i32,
    pub z: i32,
}

impl Dimensions {
    pub fn new(x: i32, y: i32, z: i32) -> Result<Self, SchemeError> {
        if x <= 0 || y <= 0 || z <= 0 {
            return Err(SchemeError::InvalidDimensions { x, y, z });
        }
        Ok(Self { x, y, z })
    }
}

impl std::fmt::Display for Dimensions {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{} x {} x {}", self.x, self.y, self.z)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Coordinate {
    pub x: i32,
    pub y: i32,
    pub z: i32,
}

impl Coordinate {
    pub fn new(x: i32, y: i32, z: i32) -> Self {
        Self { x, y, z }
    }

    pub fn to_selection(self, other: Coordinate) -> Selection {
        Selection::new(self, other)
    }
}

impl std::fmt::Display for Coordinate {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "({}, {}, {})", self.x, self.y, self.z)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Selection {
    pub from: Coordinate,
    pub to: Coordinate,
}

impl Selection {
    pub fn new(first: Coordinate, second: Coordinate) -> Self {
        Self {
            from: Coordinate::new(
                first.x.min(second.x),
                first.y.min(second.y),
                first.z.min(second.z),
            ),
            to: Coordinate::new(
                first.x.max(second.x),
                first.y.max(second.y),
                first.z.max(second.z),
            ),
        }
    }

    fn coordinates(self) -> impl Iterator<Item = Coordinate> {
        (self.from.x..=self.to.x).flat_map(move |x| {
            (self.from.y..=self.to.y)
                .flat_map(move |y| (self.from.z..=self.to.z).map(move |z| Coordinate::new(x, y, z)))
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SchemeBounds {
    pub min: Coordinate,
    pub max: Coordinate,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StagePlan {
    pub complete: bool,
    pub effective_stage_count: usize,
    pub message: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum StageRef {
    Unassigned,
    Stage(u32),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConstructionStage {
    pub id: u32,
    pub name: String,
    pub order: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlockDefinition {
    allowed_states: BTreeMap<String, BTreeSet<String>>,
    allow_any_states: bool,
}

impl BlockDefinition {
    fn new(states: &[(&str, &[&str])]) -> Self {
        Self {
            allowed_states: states
                .iter()
                .map(|(name, values)| {
                    (
                        (*name).to_string(),
                        values.iter().map(|value| (*value).to_string()).collect(),
                    )
                })
                .collect(),
            allow_any_states: false,
        }
    }

    fn permissive() -> Self {
        Self {
            allowed_states: BTreeMap::new(),
            allow_any_states: true,
        }
    }

    pub fn allowed_states(&self) -> &BTreeMap<String, BTreeSet<String>> {
        &self.allowed_states
    }

    pub fn allows_any_states(&self) -> bool {
        self.allow_any_states
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlockRegistry {
    blocks: BTreeMap<String, BlockDefinition>,
}

impl BlockRegistry {
    pub fn synthetic_fixture() -> Self {
        let mut blocks = BTreeMap::new();
        blocks.insert(
            "minecraft:stone_bricks".to_string(),
            BlockDefinition::new(&[("cracked", &["false", "true"])]),
        );
        blocks.insert(
            "minecraft:glass".to_string(),
            BlockDefinition::new(&[("color", &["clear", "white"])]),
        );
        blocks.insert(
            "thermal:machine_frame".to_string(),
            BlockDefinition::new(&[("tier", &["basic", "reinforced"])]),
        );
        blocks.insert(
            "create:andesite_casing".to_string(),
            BlockDefinition::new(&[]),
        );
        blocks.insert(
            "minecraft:furnace".to_string(),
            BlockDefinition::new(&[("facing", &["east", "north", "south", "west"])]),
        );
        Self { blocks }
    }

    pub fn from_block_ids(block_ids: impl IntoIterator<Item = String>) -> Self {
        Self {
            blocks: block_ids
                .into_iter()
                .map(|block_id| (block_id, BlockDefinition::permissive()))
                .collect(),
        }
    }

    pub fn from_block_state_definitions(
        definitions: impl IntoIterator<Item = (String, BTreeMap<String, BTreeSet<String>>)>,
    ) -> Self {
        Self {
            blocks: definitions
                .into_iter()
                .map(|(block_id, allowed_states)| {
                    (
                        block_id,
                        BlockDefinition {
                            allowed_states,
                            allow_any_states: false,
                        },
                    )
                })
                .collect(),
        }
    }

    pub fn from_mixed_block_state_definitions(
        definitions: impl IntoIterator<Item = (String, Option<BTreeMap<String, BTreeSet<String>>>)>,
    ) -> Self {
        Self {
            blocks: definitions
                .into_iter()
                .map(|(block_id, allowed_states)| {
                    let definition = allowed_states
                        .map(|allowed_states| BlockDefinition {
                            allowed_states,
                            allow_any_states: false,
                        })
                        .unwrap_or_else(BlockDefinition::permissive);
                    (block_id, definition)
                })
                .collect(),
        }
    }

    pub fn block_definition(&self, block_id: &str) -> Option<&BlockDefinition> {
        self.blocks.get(block_id)
    }

    pub fn block_ids(&self) -> impl Iterator<Item = &str> {
        self.blocks.keys().map(String::as_str)
    }

    fn validate_block(&self, block: &SchemeBlock) -> Result<(), SchemeError> {
        let definition =
            self.blocks
                .get(&block.block_id)
                .ok_or_else(|| SchemeError::UnknownBlock {
                    block_id: block.block_id.clone(),
                })?;

        if definition.allow_any_states {
            return Ok(());
        }

        for (state, allowed_values) in &definition.allowed_states {
            let value = block
                .states
                .get(state)
                .ok_or_else(|| SchemeError::MissingBlockState {
                    block_id: block.block_id.clone(),
                    state: state.clone(),
                })?;
            if !allowed_values.contains(value) {
                return Err(SchemeError::InvalidBlockState {
                    block_id: block.block_id.clone(),
                    state: state.clone(),
                    value: value.clone(),
                });
            }
        }

        for state in block.states.keys() {
            if !definition.allowed_states.contains_key(state) {
                return Err(SchemeError::UnknownBlockState {
                    block_id: block.block_id.clone(),
                    state: state.clone(),
                });
            }
        }

        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SchemeBlock {
    pub block_id: String,
    pub states: BTreeMap<String, String>,
    pub stage: StageRef,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlockPlacement {
    pub coordinate: Coordinate,
    pub block: SchemeBlock,
}

impl BlockPlacement {
    pub fn new(
        coordinate: Coordinate,
        block_id: &str,
        states: impl IntoIterator<Item = (&'static str, &'static str)>,
        stage: StageRef,
    ) -> Self {
        Self {
            coordinate,
            block: SchemeBlock {
                block_id: block_id.to_string(),
                states: states
                    .into_iter()
                    .map(|(name, value)| (name.to_string(), value.to_string()))
                    .collect(),
                stage,
            },
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SchemeOperation {
    Place(BlockPlacement),
    Delete(Coordinate),
    ReplaceAll {
        from_block_id: String,
        to: SchemeBlock,
    },
    BulkSet {
        selection: Selection,
        block: SchemeBlock,
    },
    AssignStage {
        selection: Selection,
        stage: StageRef,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MaterialLine {
    pub block_id: String,
    pub count: u32,
}

impl MaterialLine {
    pub fn new(block_id: &str, count: u32) -> Self {
        Self {
            block_id: block_id.to_string(),
            count,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Scheme {
    name: String,
    stages: Vec<ConstructionStage>,
    blocks: BTreeMap<Coordinate, SchemeBlock>,
    next_stage_id: u32,
}

impl Scheme {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            stages: Vec::new(),
            blocks: BTreeMap::new(),
            next_stage_id: 1,
        }
    }

    pub fn from_persisted(
        name: &str,
        _legacy_dimensions: Dimensions,
        stages: Vec<ConstructionStage>,
        blocks: Vec<(Coordinate, SchemeBlock)>,
    ) -> Result<Self, SchemeError> {
        let next_stage_id = stages
            .iter()
            .map(|stage| stage.id)
            .max()
            .unwrap_or(0)
            .saturating_add(1);
        let scheme = Self {
            name: name.to_string(),
            stages,
            blocks: blocks.into_iter().collect(),
            next_stage_id,
        };
        for (coordinate, block) in &scheme.blocks {
            ensure_coordinate_non_negative(*coordinate)?;
            scheme.ensure_stage_exists(block.stage)?;
        }
        Ok(scheme)
    }

    pub fn set_name(&mut self, name: &str) {
        self.name = name.trim().to_string();
    }

    pub fn add_stage(&mut self, name: &str) -> Result<u32, SchemeError> {
        let trimmed = name.trim();
        if trimmed.is_empty() {
            return Err(SchemeError::EmptyStageName);
        }
        let id = self.next_stage_id;
        self.next_stage_id += 1;
        self.stages.push(ConstructionStage {
            id,
            name: trimmed.to_string(),
            order: self.stages.len() as u32 + 1,
        });
        Ok(id)
    }

    pub fn rename_stage(&mut self, id: u32, name: &str) -> Result<(), SchemeError> {
        let trimmed = name.trim();
        if trimmed.is_empty() {
            return Err(SchemeError::EmptyStageName);
        }
        let stage = self
            .stages
            .iter_mut()
            .find(|stage| stage.id == id)
            .ok_or(SchemeError::UnknownStage { id })?;
        stage.name = trimmed.to_string();
        Ok(())
    }

    pub fn apply(
        &mut self,
        registry: &BlockRegistry,
        operation: SchemeOperation,
    ) -> Result<(), SchemeError> {
        self.validate_operation(registry, &operation)?;

        match operation {
            SchemeOperation::Place(placement) => {
                self.blocks.insert(placement.coordinate, placement.block);
            }
            SchemeOperation::Delete(coordinate) => {
                self.blocks.remove(&coordinate);
            }
            SchemeOperation::ReplaceAll { from_block_id, to } => {
                for block in self.blocks.values_mut() {
                    if block.block_id == from_block_id {
                        *block = to.clone();
                    }
                }
            }
            SchemeOperation::BulkSet { selection, block } => {
                for coordinate in selection.coordinates() {
                    self.blocks.insert(coordinate, block.clone());
                }
            }
            SchemeOperation::AssignStage { selection, stage } => {
                for coordinate in selection.coordinates() {
                    if let Some(block) = self.blocks.get_mut(&coordinate) {
                        block.stage = stage;
                    }
                }
            }
        }

        self.validate(registry)
    }

    pub fn validate(&self, registry: &BlockRegistry) -> Result<(), SchemeError> {
        for (coordinate, block) in &self.blocks {
            ensure_coordinate_non_negative(*coordinate)?;
            self.ensure_stage_exists(block.stage)?;
            registry.validate_block(block)?;
        }
        Ok(())
    }

    pub fn materials(&self) -> Vec<MaterialLine> {
        let mut counts: BTreeMap<&str, u32> = BTreeMap::new();
        for block in self.blocks.values() {
            *counts.entry(&block.block_id).or_default() += 1;
        }
        counts
            .into_iter()
            .map(|(block_id, count)| MaterialLine::new(block_id, count))
            .collect()
    }

    pub fn visible_blocks(&self, stage: StageRef) -> Vec<(&Coordinate, &SchemeBlock)> {
        self.blocks
            .iter()
            .filter(|(_, block)| self.is_visible_at_stage(block.stage, stage))
            .collect()
    }

    pub fn blocks(&self) -> impl Iterator<Item = (&Coordinate, &SchemeBlock)> {
        self.blocks.iter()
    }

    pub fn block_count(&self) -> usize {
        self.blocks.len()
    }

    pub fn dimensions(&self) -> Dimensions {
        self.computed_dimensions()
            .unwrap_or(Dimensions { x: 0, y: 0, z: 0 })
    }

    pub fn bounds(&self) -> Option<SchemeBounds> {
        let mut coordinates = self.blocks.keys();
        let first = *coordinates.next()?;
        let mut min = first;
        let mut max = first;
        for coordinate in coordinates {
            min.x = min.x.min(coordinate.x);
            min.y = min.y.min(coordinate.y);
            min.z = min.z.min(coordinate.z);
            max.x = max.x.max(coordinate.x);
            max.y = max.y.max(coordinate.y);
            max.z = max.z.max(coordinate.z);
        }
        Some(SchemeBounds { min, max })
    }

    pub fn computed_dimensions(&self) -> Option<Dimensions> {
        let bounds = self.bounds()?;
        Some(Dimensions {
            x: bounds.max.x + 1,
            y: bounds.max.y + 1,
            z: bounds.max.z + 1,
        })
    }

    pub fn stage_plan(&self) -> StagePlan {
        if self.stages.is_empty() {
            return StagePlan {
                complete: true,
                effective_stage_count: 1,
                message: None,
            };
        }
        let complete = self
            .blocks
            .values()
            .all(|block| matches!(block.stage, StageRef::Stage(_)));
        if complete {
            StagePlan {
                complete: true,
                effective_stage_count: self.stages.len(),
                message: None,
            }
        } else {
            StagePlan {
                complete: false,
                effective_stage_count: 1,
                message: Some("Stages incomplete".to_string()),
            }
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn stages(&self) -> &[ConstructionStage] {
        &self.stages
    }

    pub fn is_visible_at(&self, block_stage: StageRef, selected_stage: StageRef) -> bool {
        self.is_visible_at_stage(block_stage, selected_stage)
    }

    pub fn is_future_stage(&self, block_stage: StageRef, selected_stage: StageRef) -> bool {
        match (block_stage, selected_stage) {
            (StageRef::Stage(block_id), StageRef::Stage(selected_id)) => {
                self.stage_order(block_id) > self.stage_order(selected_id)
            }
            _ => false,
        }
    }

    fn validate_operation(
        &self,
        registry: &BlockRegistry,
        operation: &SchemeOperation,
    ) -> Result<(), SchemeError> {
        match operation {
            SchemeOperation::Place(placement) => {
                ensure_coordinate_non_negative(placement.coordinate)?;
                self.ensure_stage_exists(placement.block.stage)?;
                registry.validate_block(&placement.block)
            }
            SchemeOperation::Delete(coordinate) => ensure_coordinate_non_negative(*coordinate),
            SchemeOperation::ReplaceAll { to, .. } => {
                self.ensure_stage_exists(to.stage)?;
                registry.validate_block(to)
            }
            SchemeOperation::BulkSet { selection, block } => {
                self.ensure_selection_in_bounds(*selection)?;
                self.ensure_stage_exists(block.stage)?;
                registry.validate_block(block)
            }
            SchemeOperation::AssignStage { selection, stage } => {
                self.ensure_selection_in_bounds(*selection)?;
                self.ensure_stage_exists(*stage)
            }
        }
    }

    fn ensure_selection_in_bounds(&self, selection: Selection) -> Result<(), SchemeError> {
        ensure_coordinate_non_negative(selection.from)?;
        ensure_coordinate_non_negative(selection.to)
    }

    fn ensure_stage_exists(&self, stage: StageRef) -> Result<(), SchemeError> {
        match stage {
            StageRef::Unassigned => Ok(()),
            StageRef::Stage(id) if self.stages.iter().any(|stage| stage.id == id) => Ok(()),
            StageRef::Stage(id) => Err(SchemeError::UnknownStage { id }),
        }
    }

    fn is_visible_at_stage(&self, block_stage: StageRef, selected_stage: StageRef) -> bool {
        match (block_stage, selected_stage) {
            (StageRef::Unassigned, StageRef::Unassigned) => true,
            (StageRef::Unassigned, StageRef::Stage(_)) => false,
            (StageRef::Stage(_), StageRef::Unassigned) => false,
            (StageRef::Stage(block_id), StageRef::Stage(selected_id)) => {
                self.stage_order(block_id) <= self.stage_order(selected_id)
            }
        }
    }

    fn stage_order(&self, id: u32) -> u32 {
        self.stages
            .iter()
            .find(|stage| stage.id == id)
            .map(|stage| stage.order)
            .unwrap_or(u32::MAX)
    }
}

fn ensure_coordinate_non_negative(coordinate: Coordinate) -> Result<(), SchemeError> {
    if coordinate.x >= 0 && coordinate.y >= 0 && coordinate.z >= 0 {
        Ok(())
    } else {
        Err(SchemeError::NegativeCoordinate { coordinate })
    }
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum SchemeError {
    #[error("dimensions must be positive, got {x} x {y} x {z}")]
    InvalidDimensions { x: i32, y: i32, z: i32 },
    #[error("coordinate {coordinate} must not contain negative values")]
    NegativeCoordinate { coordinate: Coordinate },
    #[error("unknown block id {block_id}")]
    UnknownBlock { block_id: String },
    #[error("missing state {state} for block {block_id}")]
    MissingBlockState { block_id: String, state: String },
    #[error("unknown state {state} for block {block_id}")]
    UnknownBlockState { block_id: String, state: String },
    #[error("invalid state {state}={value} for block {block_id}")]
    InvalidBlockState {
        block_id: String,
        state: String,
        value: String,
    },
    #[error("unknown construction stage {id}")]
    UnknownStage { id: u32 },
    #[error("construction stage name cannot be empty")]
    EmptyStageName,
}

impl SchemeError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::InvalidDimensions { .. } => "invalid_dimensions",
            Self::NegativeCoordinate { .. } => "negative_coordinate",
            Self::UnknownBlock { .. } => "unknown_block",
            Self::MissingBlockState { .. } => "missing_block_state",
            Self::UnknownBlockState { .. } => "unknown_block_state",
            Self::InvalidBlockState { .. } => "invalid_block_state",
            Self::UnknownStage { .. } => "unknown_stage",
            Self::EmptyStageName => "empty_stage_name",
        }
    }
}
