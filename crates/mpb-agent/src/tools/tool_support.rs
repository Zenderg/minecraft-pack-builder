use std::collections::{BTreeMap, BTreeSet};

use mpb_core::{
    BlockPlacement, BlockRegistry, Coordinate, Dimensions, Scheme, SchemeBlock, SchemeOperation,
    Selection, StageRef,
};
use serde_json::{json, Value};

use super::{
    invalid_arguments, parse_block, parse_coordinate_field, parse_dimensions, parse_selection,
    parse_stage_ref, registry_material_metadata, scheme_content, scheme_materials_content,
    stored_scheme_content, ToolFailure,
};
use crate::workspace::AgentScheme;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ResponseMode {
    Content,
    Summary,
}

#[derive(Debug, Clone)]
pub(super) struct MutationImpact {
    changed_coordinates: Vec<Coordinate>,
    changed_region: Option<Selection>,
    changed_coordinate_count: usize,
}

impl MutationImpact {
    pub(super) fn single(coordinate: Coordinate) -> Self {
        Self {
            changed_coordinates: vec![coordinate],
            changed_region: Some(coordinate.to_selection(coordinate)),
            changed_coordinate_count: 1,
        }
    }

    pub(super) fn coordinates(coordinates: Vec<Coordinate>) -> Self {
        let changed_region = selection_bounds(coordinates.iter().copied());
        let changed_coordinate_count = coordinates.len();
        Self {
            changed_coordinates: coordinates,
            changed_region,
            changed_coordinate_count,
        }
    }

    pub(super) fn selection(selection: Selection) -> Self {
        Self {
            changed_coordinates: Vec::new(),
            changed_region: Some(selection),
            changed_coordinate_count: selection_volume(selection),
        }
    }
}

#[derive(Debug, Clone)]
pub(super) enum ParsedMutation {
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
    Resize(Dimensions),
}

pub(super) fn memory_mutation_response(
    scheme: &AgentScheme,
    before: &Scheme,
    impacts: Vec<MutationImpact>,
    response_mode: ResponseMode,
) -> Value {
    match response_mode {
        ResponseMode::Content => scheme_content(scheme),
        ResponseMode::Summary => json!({
            "summary": mutation_summary(
                scheme.id,
                scheme.instance_id,
                &scheme.name,
                &scheme.scheme,
                before,
                impacts,
                None
            )
        }),
    }
}

pub(super) fn stored_mutation_response(
    record: &mpb_storage::SchemeRecord,
    scheme: &Scheme,
    before: &Scheme,
    impacts: Vec<MutationImpact>,
    registry_report: Option<&Value>,
    response_mode: ResponseMode,
) -> Value {
    match response_mode {
        ResponseMode::Content => json!({
            "scheme": stored_scheme_content(record, scheme, registry_report)
        }),
        ResponseMode::Summary => json!({
            "summary": mutation_summary(
                record.id,
                record.prism_instance_id,
                &record.name,
                scheme,
                before,
                impacts,
                registry_report
            )
        }),
    }
}

fn mutation_summary(
    scheme_id: i64,
    instance_id: i64,
    name: &str,
    scheme: &Scheme,
    before: &Scheme,
    impacts: Vec<MutationImpact>,
    registry_report: Option<&Value>,
) -> Value {
    let changed_coordinate_count = impacts
        .iter()
        .map(|impact| impact.changed_coordinate_count)
        .sum::<usize>();
    let changed_coordinates = impacts
        .iter()
        .flat_map(|impact| impact.changed_coordinates.iter().copied())
        .map(coordinate_array)
        .collect::<Vec<_>>();
    let changed_region = selection_bounds(
        impacts
            .iter()
            .filter_map(|impact| impact.changed_region)
            .flat_map(|selection| [selection.from, selection.to]),
    );
    let mut overview = scheme_overview(scheme_id, instance_id, name, scheme, registry_report);
    if let Some(object) = overview.as_object_mut() {
        object.insert(
            "changedCoordinateCount".to_string(),
            json!(changed_coordinate_count),
        );
        object.insert("changedCoordinates".to_string(), json!(changed_coordinates));
        object.insert(
            "changedRegion".to_string(),
            changed_region.map(selection_summary).unwrap_or(Value::Null),
        );
        object.insert(
            "materialsDelta".to_string(),
            json!(materials_delta(before, scheme, registry_report)),
        );
        object.insert(
            "diagnostic".to_string(),
            json!({
                "operation": "mutation",
                "schemeId": scheme_id,
                "status": "success",
                "errorCode": null,
                "errorMessage": null,
                "recoveryMessage": null
            }),
        );
    }
    overview
}

pub(super) fn scheme_overview(
    scheme_id: i64,
    instance_id: i64,
    name: &str,
    scheme: &Scheme,
    registry_report: Option<&Value>,
) -> Value {
    let dimensions = scheme.dimensions();
    json!({
        "schemeId": scheme_id,
        "instanceId": instance_id,
        "name": name,
        "dimensions": [dimensions.x, dimensions.y, dimensions.z],
        "blockCount": scheme.block_count(),
        "filledBounds": selection_bounds(scheme.blocks().map(|(coordinate, _)| *coordinate))
            .map(selection_summary)
            .unwrap_or(Value::Null),
        "stages": scheme.stages(),
        "stageBlockCounts": stage_block_counts(scheme),
        "topMaterials": top_materials(scheme, registry_report, 10),
    })
}

fn selection_bounds(coordinates: impl IntoIterator<Item = Coordinate>) -> Option<Selection> {
    let mut iter = coordinates.into_iter();
    let first = iter.next()?;
    let mut min = first;
    let mut max = first;
    for coordinate in iter {
        min = Coordinate::new(
            min.x.min(coordinate.x),
            min.y.min(coordinate.y),
            min.z.min(coordinate.z),
        );
        max = Coordinate::new(
            max.x.max(coordinate.x),
            max.y.max(coordinate.y),
            max.z.max(coordinate.z),
        );
    }
    Some(min.to_selection(max))
}

fn selection_volume(selection: Selection) -> usize {
    let x = i64::from(selection.to.x - selection.from.x + 1);
    let y = i64::from(selection.to.y - selection.from.y + 1);
    let z = i64::from(selection.to.z - selection.from.z + 1);
    usize::try_from(x * y * z).unwrap_or(usize::MAX)
}

fn selection_summary(selection: Selection) -> Value {
    json!({
        "from": coordinate_array(selection.from),
        "to": coordinate_array(selection.to),
    })
}

fn coordinate_array(coordinate: Coordinate) -> Value {
    json!([coordinate.x, coordinate.y, coordinate.z])
}

fn stage_block_counts(scheme: &Scheme) -> Vec<Value> {
    let mut counts: BTreeMap<Option<u32>, u32> = BTreeMap::new();
    for (_, block) in scheme.blocks() {
        let key = match block.stage {
            StageRef::Stage(id) => Some(id),
            StageRef::Unassigned => None,
        };
        *counts.entry(key).or_default() += 1;
    }
    let mut ordered = scheme
        .stages()
        .iter()
        .map(|stage| {
            json!({
                "stageId": stage.id,
                "name": stage.name,
                "blockCount": counts.remove(&Some(stage.id)).unwrap_or(0)
            })
        })
        .collect::<Vec<_>>();
    if let Some(block_count) = counts.remove(&None) {
        ordered.push(json!({
            "stageId": null,
            "name": "Unassigned",
            "blockCount": block_count
        }));
    }
    ordered
}

fn top_materials(scheme: &Scheme, registry_report: Option<&Value>, limit: usize) -> Vec<Value> {
    let mut materials = scheme_materials_content(scheme, registry_report);
    materials.sort_by(|left, right| {
        let left_count = left["count"].as_u64().unwrap_or(0);
        let right_count = right["count"].as_u64().unwrap_or(0);
        right_count
            .cmp(&left_count)
            .then_with(|| left["blockId"].as_str().cmp(&right["blockId"].as_str()))
    });
    materials.truncate(limit);
    materials
}

fn materials_delta(before: &Scheme, after: &Scheme, registry_report: Option<&Value>) -> Vec<Value> {
    let metadata = registry_report
        .map(registry_material_metadata)
        .unwrap_or_default();
    let before_counts = material_count_map(before);
    let after_counts = material_count_map(after);
    let mut block_ids = before_counts.keys().cloned().collect::<BTreeSet<_>>();
    block_ids.extend(after_counts.keys().cloned());
    block_ids
        .into_iter()
        .filter_map(|block_id| {
            let before = before_counts.get(&block_id).copied().unwrap_or(0);
            let after = after_counts.get(&block_id).copied().unwrap_or(0);
            let delta = after - before;
            (delta != 0).then(|| {
                let material = metadata.get(&block_id).cloned().unwrap_or_default();
                json!({
                    "blockId": block_id,
                    "displayName": material.display_name.unwrap_or_else(|| block_id.clone()),
                    "delta": delta,
                    "before": before,
                    "after": after,
                })
            })
        })
        .collect()
}

fn material_count_map(scheme: &Scheme) -> BTreeMap<String, i64> {
    scheme
        .materials()
        .into_iter()
        .map(|line| (line.block_id, i64::from(line.count)))
        .collect()
}

pub(super) fn search_registry_blocks(
    registry: &BlockRegistry,
    query: &str,
    limit: usize,
) -> Vec<Value> {
    let normalized_query = query.to_ascii_lowercase();
    registry
        .block_ids()
        .filter(|block_id| block_id.to_ascii_lowercase().contains(&normalized_query))
        .take(limit)
        .filter_map(|block_id| {
            registry
                .block_definition(block_id)
                .map(|definition| block_definition_content(block_id, definition, None))
        })
        .collect()
}

pub(super) fn search_report_blocks(report: &Value, query: &str, limit: usize) -> Vec<Value> {
    let normalized_query = query.to_ascii_lowercase();
    report
        .get("blocks")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter(|block| {
            let id_match = block
                .get("identifier")
                .and_then(Value::as_str)
                .is_some_and(|id| id.to_ascii_lowercase().contains(&normalized_query));
            let name_match = block
                .get("displayName")
                .and_then(Value::as_str)
                .is_some_and(|name| name.to_ascii_lowercase().contains(&normalized_query));
            id_match || name_match
        })
        .take(limit)
        .map(report_block_definition_content)
        .collect()
}

pub(super) fn find_report_block<'a>(report: &'a Value, block_id: &str) -> Option<&'a Value> {
    report
        .get("blocks")
        .and_then(Value::as_array)?
        .iter()
        .find(|block| block.get("identifier").and_then(Value::as_str) == Some(block_id))
}

pub(super) fn block_definition_content(
    block_id: &str,
    definition: &mpb_core::BlockDefinition,
    metadata: Option<&Value>,
) -> Value {
    json!({
        "blockId": block_id,
        "displayName": metadata
            .and_then(|block| block.get("displayName"))
            .and_then(Value::as_str)
            .unwrap_or(block_id),
        "itemId": metadata
            .and_then(|block| block.get("itemId"))
            .cloned()
            .unwrap_or(Value::Null),
        "maxStackSize": metadata
            .and_then(|block| block.get("maxStackSize"))
            .cloned()
            .unwrap_or(Value::Null),
        "texturePath": metadata
            .and_then(|block| block.get("texturePath"))
            .cloned()
            .unwrap_or(Value::Null),
        "allowedStates": allowed_states_content(definition.allowed_states()),
        "allowsAnyStates": definition.allows_any_states(),
    })
}

pub(super) fn report_block_definition_content(block: &Value) -> Value {
    json!({
        "blockId": block.get("identifier").and_then(Value::as_str).unwrap_or_default(),
        "displayName": block.get("displayName").and_then(Value::as_str).unwrap_or_default(),
        "itemId": block.get("itemId").cloned().unwrap_or(Value::Null),
        "maxStackSize": block.get("maxStackSize").cloned().unwrap_or(Value::Null),
        "texturePath": block.get("texturePath").cloned().unwrap_or(Value::Null),
        "faceTexturePaths": block.get("faceTexturePaths").cloned().unwrap_or(Value::Null),
        "modelVariantsAreMultipart": block
            .get("modelVariantsAreMultipart")
            .and_then(Value::as_bool)
            .unwrap_or(false),
        "modelVariants": block.get("modelVariants").cloned().unwrap_or_else(|| json!([])),
        "allowedStates": block.get("allowedStates").cloned().unwrap_or_else(|| json!([])),
        "allowsAnyStates": block.get("allowedStates").is_none(),
    })
}

fn allowed_states_content(states: &BTreeMap<String, BTreeSet<String>>) -> Vec<Value> {
    states
        .iter()
        .map(|(name, values)| json!({ "name": name, "values": values }))
        .collect()
}

pub(super) fn unknown_block_definition(block_id: &str) -> ToolFailure {
    ToolFailure::new(
        "unknown_block",
        format!("unknown block id {block_id}"),
        json!({ "blockId": block_id }),
    )
}

pub(super) fn response_mode(arguments: &Value) -> Result<ResponseMode, ToolFailure> {
    match arguments.get("responseMode").and_then(Value::as_str) {
        None | Some("content") => Ok(ResponseMode::Content),
        Some("summary") => Ok(ResponseMode::Summary),
        Some(value) => Err(invalid_arguments(format!(
            "responseMode must be content or summary, got {value}"
        ))),
    }
}

pub(super) fn parse_mutations(arguments: &Value) -> Result<Vec<ParsedMutation>, ToolFailure> {
    let mutations = arguments
        .get("mutations")
        .and_then(Value::as_array)
        .ok_or_else(|| invalid_arguments("mutations must be a non-empty array"))?;
    if mutations.is_empty() {
        return Err(invalid_arguments("mutations must be a non-empty array"));
    }
    mutations.iter().map(parse_mutation).collect()
}

fn parse_mutation(value: &Value) -> Result<ParsedMutation, ToolFailure> {
    let mutation_type = value
        .get("type")
        .and_then(Value::as_str)
        .ok_or_else(|| invalid_arguments("mutation.type is required"))?;
    match mutation_type {
        "placeBlock" => Ok(ParsedMutation::Place(BlockPlacement {
            coordinate: parse_coordinate_field(value, "coordinate")?,
            block: parse_block(&value["block"])?,
        })),
        "deleteBlock" => Ok(ParsedMutation::Delete(parse_coordinate_field(
            value,
            "coordinate",
        )?)),
        "replaceBlocks" => Ok(ParsedMutation::ReplaceAll {
            from_block_id: super::required_string(value, "fromBlockId")?,
            to: parse_block(value.get("toBlock").unwrap_or(&value["to"]))?,
        }),
        "bulkSetArea" => Ok(ParsedMutation::BulkSet {
            selection: parse_selection(value)?,
            block: parse_block(&value["block"])?,
        }),
        "assignBlocksToStage" => Ok(ParsedMutation::AssignStage {
            selection: parse_selection(value)?,
            stage: parse_stage_ref(value.get("stageId"))?,
        }),
        "resizeScheme" => Ok(ParsedMutation::Resize(parse_dimensions(value)?)),
        _ => Err(invalid_arguments(format!(
            "unsupported mutation.type {mutation_type}"
        ))),
    }
}

pub(super) fn apply_parsed_mutations(
    scheme: &mut Scheme,
    registry: &BlockRegistry,
    mutations: &[ParsedMutation],
) -> Result<Vec<MutationImpact>, ToolFailure> {
    let mut impacts = Vec::with_capacity(mutations.len());
    for mutation in mutations {
        match mutation {
            ParsedMutation::Place(placement) => {
                scheme
                    .apply(registry, SchemeOperation::Place(placement.clone()))
                    .map_err(ToolFailure::scheme)?;
                impacts.push(MutationImpact::single(placement.coordinate));
            }
            ParsedMutation::Delete(coordinate) => {
                scheme
                    .apply(registry, SchemeOperation::Delete(*coordinate))
                    .map_err(ToolFailure::scheme)?;
                impacts.push(MutationImpact::single(*coordinate));
            }
            ParsedMutation::ReplaceAll { from_block_id, to } => {
                let impact = MutationImpact::coordinates(
                    scheme
                        .blocks()
                        .filter_map(|(coordinate, block)| {
                            (block.block_id == *from_block_id).then_some(*coordinate)
                        })
                        .collect(),
                );
                scheme
                    .apply(
                        registry,
                        SchemeOperation::ReplaceAll {
                            from_block_id: from_block_id.clone(),
                            to: to.clone(),
                        },
                    )
                    .map_err(ToolFailure::scheme)?;
                impacts.push(impact);
            }
            ParsedMutation::BulkSet { selection, block } => {
                scheme
                    .apply(
                        registry,
                        SchemeOperation::BulkSet {
                            selection: *selection,
                            block: block.clone(),
                        },
                    )
                    .map_err(ToolFailure::scheme)?;
                impacts.push(MutationImpact::selection(*selection));
            }
            ParsedMutation::AssignStage { selection, stage } => {
                scheme
                    .apply(
                        registry,
                        SchemeOperation::AssignStage {
                            selection: *selection,
                            stage: *stage,
                        },
                    )
                    .map_err(ToolFailure::scheme)?;
                impacts.push(MutationImpact::selection(*selection));
            }
            ParsedMutation::Resize(dimensions) => {
                scheme
                    .apply(registry, SchemeOperation::Resize(*dimensions))
                    .map_err(ToolFailure::scheme)?;
                impacts.push(MutationImpact::coordinates(Vec::new()));
            }
        }
    }
    Ok(impacts)
}
