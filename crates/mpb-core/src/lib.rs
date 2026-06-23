//! Authoritative scheme domain model, operations, validation, and materials.

use std::collections::{BTreeMap, BTreeSet};

use serde::Serialize;
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
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

    fn contains(self, coordinate: Coordinate) -> bool {
        (0..self.x).contains(&coordinate.x)
            && (0..self.y).contains(&coordinate.y)
            && (0..self.z).contains(&coordinate.z)
    }
}

impl std::fmt::Display for Dimensions {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{} x {} x {}", self.x, self.y, self.z)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub enum StageRef {
    Unassigned,
    Stage(u32),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ConstructionStage {
    pub id: u32,
    pub name: String,
    pub order: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlockDefinition {
    allowed_states: BTreeMap<String, BTreeSet<String>>,
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
        }
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
        Self { blocks }
    }

    fn validate_block(&self, block: &SchemeBlock) -> Result<(), SchemeError> {
        let definition =
            self.blocks
                .get(&block.block_id)
                .ok_or_else(|| SchemeError::UnknownBlock {
                    block_id: block.block_id.clone(),
                })?;

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

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
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
    Resize(Dimensions),
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
pub struct ChangeRequest {
    pub selection: Selection,
    pub body: String,
    pub status: ChangeRequestStatus,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChangeRequestStatus {
    Pending,
    Resolved,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Scheme {
    name: String,
    dimensions: Dimensions,
    stages: Vec<ConstructionStage>,
    blocks: BTreeMap<Coordinate, SchemeBlock>,
    change_requests: Vec<ChangeRequest>,
    next_stage_id: u32,
}

impl Scheme {
    pub fn new(name: &str, dimensions: Dimensions) -> Self {
        Self {
            name: name.to_string(),
            dimensions,
            stages: Vec::new(),
            blocks: BTreeMap::new(),
            change_requests: Vec::new(),
            next_stage_id: 1,
        }
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
            SchemeOperation::Resize(dimensions) => {
                self.dimensions = dimensions;
            }
        }

        self.validate(registry)
    }

    pub fn validate(&self, registry: &BlockRegistry) -> Result<(), SchemeError> {
        for (coordinate, block) in &self.blocks {
            self.ensure_coordinate_in_bounds(*coordinate)?;
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
        self.dimensions
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
                self.ensure_coordinate_in_bounds(placement.coordinate)?;
                self.ensure_stage_exists(placement.block.stage)?;
                registry.validate_block(&placement.block)
            }
            SchemeOperation::Delete(coordinate) => self.ensure_coordinate_in_bounds(*coordinate),
            SchemeOperation::ReplaceAll { to, .. } => {
                self.ensure_stage_exists(to.stage)?;
                registry.validate_block(to)
            }
            SchemeOperation::BulkSet { selection, block } => {
                self.ensure_selection_in_bounds(*selection)?;
                self.ensure_stage_exists(block.stage)?;
                registry.validate_block(block)
            }
            SchemeOperation::Resize(dimensions) => {
                for coordinate in self.blocks.keys() {
                    if !dimensions.contains(*coordinate) {
                        return Err(SchemeError::ResizeWouldDropBlock {
                            coordinate: *coordinate,
                            dimensions: *dimensions,
                        });
                    }
                }
                Ok(())
            }
        }
    }

    fn ensure_coordinate_in_bounds(&self, coordinate: Coordinate) -> Result<(), SchemeError> {
        if self.dimensions.contains(coordinate) {
            Ok(())
        } else {
            Err(SchemeError::CoordinateOutOfBounds {
                coordinate,
                dimensions: self.dimensions,
            })
        }
    }

    fn ensure_selection_in_bounds(&self, selection: Selection) -> Result<(), SchemeError> {
        self.ensure_coordinate_in_bounds(selection.from)?;
        self.ensure_coordinate_in_bounds(selection.to)
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

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum SchemeError {
    #[error("dimensions must be positive, got {x} x {y} x {z}")]
    InvalidDimensions { x: i32, y: i32, z: i32 },
    #[error("coordinate {coordinate} is outside {dimensions}")]
    CoordinateOutOfBounds {
        coordinate: Coordinate,
        dimensions: Dimensions,
    },
    #[error("resize to {dimensions} would drop existing block at {coordinate}")]
    ResizeWouldDropBlock {
        coordinate: Coordinate,
        dimensions: Dimensions,
    },
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
            Self::CoordinateOutOfBounds { .. } => "coordinate_out_of_bounds",
            Self::ResizeWouldDropBlock { .. } => "resize_would_drop_block",
            Self::UnknownBlock { .. } => "unknown_block",
            Self::MissingBlockState { .. } => "missing_block_state",
            Self::UnknownBlockState { .. } => "unknown_block_state",
            Self::InvalidBlockState { .. } => "invalid_block_state",
            Self::UnknownStage { .. } => "unknown_stage",
            Self::EmptyStageName => "empty_stage_name",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DomainDemoReport {
    pub scheme_name: String,
    pub summary: DomainDemoSummary,
    pub stages: Vec<StageLine>,
    pub materials: Vec<MaterialLine>,
    pub rejected_actions: Vec<RejectedAction>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DomainDemoSummary {
    pub dimensions: Dimensions,
    pub stage_count: usize,
    pub block_count: usize,
    pub material_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StageLine {
    pub id: Option<u32>,
    pub name: String,
    pub order: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RejectedAction {
    pub action: String,
    pub code: String,
    pub message: String,
}

pub fn domain_demo_report() -> DomainDemoReport {
    let registry = BlockRegistry::synthetic_fixture();
    let mut scheme = Scheme::new(
        "Domain Demo Scheme",
        Dimensions::new(4, 3, 4).expect("valid demo dimensions"),
    );
    let foundation = scheme.add_stage("Foundation").expect("foundation stage");
    let machines = scheme.add_stage("Machines").expect("machines stage");

    for operation in [
        SchemeOperation::Place(BlockPlacement::new(
            Coordinate::new(0, 0, 0),
            "minecraft:stone_bricks",
            [("cracked", "false")],
            StageRef::Stage(foundation),
        )),
        SchemeOperation::Place(BlockPlacement::new(
            Coordinate::new(1, 0, 0),
            "minecraft:stone_bricks",
            [("cracked", "false")],
            StageRef::Stage(foundation),
        )),
        SchemeOperation::Place(BlockPlacement::new(
            Coordinate::new(0, 1, 0),
            "thermal:machine_frame",
            [("tier", "basic")],
            StageRef::Stage(machines),
        )),
        SchemeOperation::Place(BlockPlacement::new(
            Coordinate::new(2, 0, 0),
            "create:andesite_casing",
            [],
            StageRef::Unassigned,
        )),
        SchemeOperation::BulkSet {
            selection: Coordinate::new(0, 2, 0).to_selection(Coordinate::new(1, 2, 0)),
            block: BlockPlacement::new(
                Coordinate::new(0, 2, 0),
                "minecraft:glass",
                [("color", "clear")],
                StageRef::Stage(machines),
            )
            .block,
        },
    ] {
        scheme
            .apply(&registry, operation)
            .expect("demo operation should be valid");
    }

    let rejected_actions = [
        (
            "place missing block",
            SchemeOperation::Place(BlockPlacement::new(
                Coordinate::new(0, 0, 1),
                "minecraft:missing_block",
                [],
                StageRef::Unassigned,
            )),
        ),
        (
            "bulk set out of bounds",
            SchemeOperation::BulkSet {
                selection: Coordinate::new(4, 0, 0).to_selection(Coordinate::new(4, 0, 0)),
                block: BlockPlacement::new(
                    Coordinate::new(4, 0, 0),
                    "minecraft:stone_bricks",
                    [("cracked", "false")],
                    StageRef::Unassigned,
                )
                .block,
            },
        ),
    ]
    .into_iter()
    .filter_map(
        |(action, operation)| match scheme.apply(&registry, operation) {
            Ok(()) => None,
            Err(error) => Some(RejectedAction {
                action: action.to_string(),
                code: error.code().to_string(),
                message: error.to_string(),
            }),
        },
    )
    .collect::<Vec<_>>();

    let mut stages = scheme
        .stages()
        .iter()
        .map(|stage| StageLine {
            id: Some(stage.id),
            name: stage.name.clone(),
            order: Some(stage.order),
        })
        .collect::<Vec<_>>();
    stages.push(StageLine {
        id: None,
        name: "Unassigned".to_string(),
        order: None,
    });

    let materials = scheme.materials();
    let dimensions = scheme.dimensions();
    let block_count = scheme.block_count();
    DomainDemoReport {
        scheme_name: scheme.name,
        summary: DomainDemoSummary {
            dimensions,
            stage_count: stages.len(),
            block_count,
            material_count: materials.len(),
        },
        stages,
        materials,
        rejected_actions,
    }
}
