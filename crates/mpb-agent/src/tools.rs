use std::collections::BTreeMap;

use mpb_core::{
    BlockPlacement, Coordinate, Dimensions, MaterialLine, SchemeBlock, SchemeError,
    SchemeOperation, Selection, StageRef,
};
use mpb_export::{write_scheme_export, ExportError, ExportFormat};
use serde_json::{json, Value};
use thiserror::Error;

use crate::protocol::AgentEvent;
use crate::workspace::{AgentScheme, AgentWorkspace};

#[derive(Debug)]
pub(crate) struct ToolOutcome {
    pub(crate) value: Value,
    pub(crate) event: Option<AgentEvent>,
}

impl ToolOutcome {
    fn read(value: Value) -> Self {
        Self { value, event: None }
    }

    fn changed(value: Value, event: AgentEvent) -> Self {
        Self {
            value,
            event: Some(event),
        }
    }
}

#[derive(Debug, Error)]
#[error("{message}")]
pub(crate) struct ToolFailure {
    pub(crate) code: &'static str,
    pub(crate) message: String,
    pub(crate) data: Value,
}

impl ToolFailure {
    fn new(code: &'static str, message: impl Into<String>, data: Value) -> Self {
        Self {
            code,
            message: message.into(),
            data,
        }
    }

    fn scheme(error: SchemeError) -> Self {
        Self::new(error.code(), error.to_string(), json!({}))
    }

    fn export(error: ExportError) -> Self {
        Self::new(error.code(), error.to_string(), json!({}))
    }
}

pub(crate) fn dispatch_tool(
    workspace: &mut AgentWorkspace,
    name: &str,
    arguments: Value,
) -> Result<ToolOutcome, ToolFailure> {
    match name {
        "list_imported_modpacks" => Ok(ToolOutcome::read(json!({
            "modpacks": workspace.modpacks
        }))),
        "add_modpack" => Err(ToolFailure::new(
            "curseforge_import_requires_desktop_backend",
            "Modpack import must run through the desktop backend with secure CurseForge credentials.",
            json!({ "acceptedArguments": ["pageUrl", "fileId"] }),
        )),
        "list_schemes" => {
            let modpack_id = required_i64(&arguments, "modpackId")?;
            Ok(ToolOutcome::read(json!({
                "schemes": workspace
                    .schemes
                    .values()
                    .filter(|scheme| scheme.modpack_id == modpack_id)
                    .map(scheme_summary)
                    .collect::<Vec<_>>()
            })))
        }
        "create_scheme" => {
            let modpack_id = required_i64(&arguments, "modpackId")?;
            ensure_modpack_exists(workspace, modpack_id)?;
            let name = required_string(&arguments, "name")?;
            let dimensions = parse_dimensions(&arguments)?;
            let id = workspace.next_scheme_id;
            workspace.next_scheme_id += 1;
            let scheme = AgentScheme {
                id,
                modpack_id,
                name: name.clone(),
                scheme: mpb_core::Scheme::new(&name, dimensions),
            };
            let summary = scheme_summary(&scheme);
            workspace.schemes.insert(id, scheme);
            Ok(ToolOutcome::changed(
                json!({ "scheme": summary }),
                AgentEvent::LibraryChanged {},
            ))
        }
        "rename_scheme" => {
            let scheme_id = required_i64(&arguments, "schemeId")?;
            let name = required_string(&arguments, "name")?;
            let scheme = workspace_scheme_mut(workspace, scheme_id)?;
            scheme.name = name;
            Ok(ToolOutcome::changed(
                json!({ "scheme": scheme_summary(scheme) }),
                AgentEvent::LibraryChanged {},
            ))
        }
        "delete_scheme" => {
            let scheme_id = required_i64(&arguments, "schemeId")?;
            workspace
                .schemes
                .remove(&scheme_id)
                .ok_or_else(|| not_found("scheme", scheme_id))?;
            Ok(ToolOutcome::changed(
                json!({ "deletedSchemeId": scheme_id }),
                AgentEvent::LibraryChanged {},
            ))
        }
        "read_scheme_content" => {
            let scheme_id = required_i64(&arguments, "schemeId")?;
            let scheme = workspace_scheme(workspace, scheme_id)?;
            Ok(ToolOutcome::read(scheme_content(scheme)))
        }
        "read_current_selection" => Ok(ToolOutcome::read(selection_content(
            workspace.current_selection,
        ))),
        "place_block" => {
            let scheme_id = required_i64(&arguments, "schemeId")?;
            let coordinate = parse_coordinate_field(&arguments, "coordinate")?;
            let block = parse_block(&arguments["block"])?;
            let registry = workspace.registry.clone();
            let scheme = workspace_scheme_mut(workspace, scheme_id)?;
            scheme
                .scheme
                .apply(
                    &registry,
                    SchemeOperation::Place(BlockPlacement { coordinate, block }),
                )
                .map_err(ToolFailure::scheme)?;
            Ok(ToolOutcome::changed(
                scheme_content(scheme),
                AgentEvent::SchemeChanged { scheme_id },
            ))
        }
        "delete_block" => {
            let scheme_id = required_i64(&arguments, "schemeId")?;
            let coordinate = parse_coordinate_field(&arguments, "coordinate")?;
            let registry = workspace.registry.clone();
            let scheme = workspace_scheme_mut(workspace, scheme_id)?;
            scheme
                .scheme
                .apply(&registry, SchemeOperation::Delete(coordinate))
                .map_err(ToolFailure::scheme)?;
            Ok(ToolOutcome::changed(
                scheme_content(scheme),
                AgentEvent::SchemeChanged { scheme_id },
            ))
        }
        "replace_blocks" => {
            let scheme_id = required_i64(&arguments, "schemeId")?;
            let from_block_id = required_string(&arguments, "fromBlockId")?;
            let to = parse_block(&arguments["to"])?;
            let registry = workspace.registry.clone();
            let scheme = workspace_scheme_mut(workspace, scheme_id)?;
            scheme
                .scheme
                .apply(&registry, SchemeOperation::ReplaceAll { from_block_id, to })
                .map_err(ToolFailure::scheme)?;
            Ok(ToolOutcome::changed(
                scheme_content(scheme),
                AgentEvent::SchemeChanged { scheme_id },
            ))
        }
        "bulk_set_area" => {
            let scheme_id = required_i64(&arguments, "schemeId")?;
            let selection = parse_selection(&arguments)?;
            let block = parse_block(&arguments["block"])?;
            let registry = workspace.registry.clone();
            let scheme = workspace_scheme_mut(workspace, scheme_id)?;
            scheme
                .scheme
                .apply(&registry, SchemeOperation::BulkSet { selection, block })
                .map_err(ToolFailure::scheme)?;
            Ok(ToolOutcome::changed(
                scheme_content(scheme),
                AgentEvent::SchemeChanged { scheme_id },
            ))
        }
        "resize_scheme" => {
            let scheme_id = required_i64(&arguments, "schemeId")?;
            let dimensions = parse_dimensions(&arguments)?;
            let registry = workspace.registry.clone();
            let scheme = workspace_scheme_mut(workspace, scheme_id)?;
            scheme
                .scheme
                .apply(&registry, SchemeOperation::Resize(dimensions))
                .map_err(ToolFailure::scheme)?;
            Ok(ToolOutcome::changed(
                scheme_content(scheme),
                AgentEvent::SchemeChanged { scheme_id },
            ))
        }
        "create_stage" => {
            let scheme_id = required_i64(&arguments, "schemeId")?;
            let name = required_string(&arguments, "name")?;
            let scheme = workspace_scheme_mut(workspace, scheme_id)?;
            let stage_id = scheme.scheme.add_stage(&name).map_err(ToolFailure::scheme)?;
            Ok(ToolOutcome::changed(
                json!({ "stageId": stage_id, "scheme": scheme_content(scheme) }),
                AgentEvent::SchemeChanged { scheme_id },
            ))
        }
        "rename_stage" => {
            let scheme_id = required_i64(&arguments, "schemeId")?;
            let stage_id = required_u32(&arguments, "stageId")?;
            let name = required_string(&arguments, "name")?;
            let scheme = workspace_scheme_mut(workspace, scheme_id)?;
            scheme
                .scheme
                .rename_stage(stage_id, &name)
                .map_err(ToolFailure::scheme)?;
            Ok(ToolOutcome::changed(
                scheme_content(scheme),
                AgentEvent::SchemeChanged { scheme_id },
            ))
        }
        "assign_blocks_to_stage" => {
            let scheme_id = required_i64(&arguments, "schemeId")?;
            let selection = parse_selection(&arguments)?;
            let stage = parse_stage_ref(arguments.get("stageId"))?;
            let registry = workspace.registry.clone();
            let scheme = workspace_scheme_mut(workspace, scheme_id)?;
            scheme
                .scheme
                .apply(&registry, SchemeOperation::AssignStage { selection, stage })
                .map_err(ToolFailure::scheme)?;
            Ok(ToolOutcome::changed(
                scheme_content(scheme),
                AgentEvent::SchemeChanged { scheme_id },
            ))
        }
        "validate_scheme" => {
            let scheme_id = required_i64(&arguments, "schemeId")?;
            let scheme = workspace_scheme(workspace, scheme_id)?;
            match scheme.scheme.validate(&workspace.registry) {
                Ok(()) => Ok(ToolOutcome::read(json!({
                    "valid": true,
                    "errors": [],
                    "diagnostic": validation_diagnostic(scheme_id, "success", None, None)
                }))),
                Err(error) => Ok(ToolOutcome::read(json!({
                    "valid": false,
                    "errors": [{ "code": error.code(), "message": error.to_string() }],
                    "diagnostic": validation_diagnostic(
                        scheme_id,
                        "failed",
                        Some(error.code()),
                        Some(error.to_string())
                    )
                }))),
            }
        }
        "get_materials" => {
            let scheme_id = required_i64(&arguments, "schemeId")?;
            let scheme = workspace_scheme(workspace, scheme_id)?;
            Ok(ToolOutcome::read(json!({ "materials": scheme.scheme.materials() })))
        }
        "export_scheme" => {
            let scheme_id = required_i64(&arguments, "schemeId")?;
            let format = parse_export_format(&arguments)?;
            let destination_path = required_string(&arguments, "destinationPath")?;
            let scheme = workspace_scheme(workspace, scheme_id)?;
            let artifact = write_scheme_export(&scheme.scheme, format, &destination_path)
                .map_err(ToolFailure::export)?;
            Ok(ToolOutcome::read(json!({
                "schemeId": scheme_id,
                "format": artifact.format,
                "path": artifact.path,
                "byteLen": artifact.byte_len,
                "blockCount": artifact.block_count,
            })))
        }
        _ => Err(ToolFailure::new(
            "unknown_tool",
            format!("Unknown MCP tool '{name}'"),
            json!({ "tool": name }),
        )),
    }
}

pub(crate) fn tool_success(value: Value) -> Value {
    json!({
        "content": [{ "type": "text", "text": value.to_string() }],
        "structuredContent": value,
        "isError": false
    })
}

pub(crate) fn tool_error(tool: &str, code: &'static str, message: String, data: Value) -> Value {
    let value = json!({
        "error": {
            "code": code,
            "message": message,
            "details": data
        },
        "diagnostic": {
            "operation": "ai_tool_call",
            "tool": tool,
            "status": "failed",
            "errorCode": code,
            "errorMessage": message,
            "recoveryMessage": tool_recovery_message(code)
        }
    });
    json!({
        "content": [{ "type": "text", "text": value.to_string() }],
        "structuredContent": value,
        "isError": true
    })
}

fn validation_diagnostic(
    scheme_id: i64,
    status: &'static str,
    error_code: Option<&'static str>,
    error_message: Option<String>,
) -> Value {
    json!({
        "operation": "validation",
        "schemeId": scheme_id,
        "status": status,
        "errorCode": error_code,
        "errorMessage": error_message,
        "recoveryMessage": if error_code.is_some() {
            Some("Review the validation error, correct the scheme through a valid tool call, then run validation again.")
        } else {
            None
        }
    })
}

fn tool_recovery_message(code: &str) -> &'static str {
    match code {
        "invalid_arguments" => "Adjust the request arguments to match the tool schema, then call the tool again.",
        "coordinate_out_of_bounds" => {
            "Adjust the request coordinates to stay inside the scheme dimensions, then call the tool again."
        }
        "unknown_block" | "invalid_block_state" => {
            "Use a block id and states from the imported modpack registry, then call the tool again."
        }
        "not_found" => "Refresh the library context and retry with an existing modpack or scheme id.",
        "curseforge_import_requires_desktop_backend" => {
            "Start modpack import through the desktop backend so credentials and files stay controlled."
        }
        _ => "Adjust the request, keep the current scheme open, and call the tool again.",
    }
}

fn scheme_summary(scheme: &AgentScheme) -> Value {
    let dimensions = scheme.scheme.dimensions();
    json!({
        "id": scheme.id,
        "modpackId": scheme.modpack_id,
        "name": scheme.name,
        "dimensions": [dimensions.x, dimensions.y, dimensions.z],
        "blockCount": scheme.scheme.block_count()
    })
}

fn scheme_content(scheme: &AgentScheme) -> Value {
    let dimensions = scheme.scheme.dimensions();
    json!({
        "id": scheme.id,
        "modpackId": scheme.modpack_id,
        "name": scheme.name,
        "dimensions": [dimensions.x, dimensions.y, dimensions.z],
        "stages": scheme.scheme.stages(),
        "blocks": scheme
            .scheme
            .blocks()
            .map(|(coordinate, block)| block_content(*coordinate, block))
            .collect::<Vec<_>>(),
        "blockCount": scheme.scheme.block_count(),
        "materials": scheme.scheme.materials(),
    })
}

fn block_content(coordinate: Coordinate, block: &SchemeBlock) -> Value {
    json!({
        "coordinate": [coordinate.x, coordinate.y, coordinate.z],
        "blockId": block.block_id,
        "states": block.states,
        "stageId": match block.stage {
            StageRef::Stage(id) => Some(id),
            StageRef::Unassigned => None,
        }
    })
}

fn selection_content(selection: Option<Selection>) -> Value {
    match selection {
        Some(selection) => json!({
            "selection": {
                "from": [selection.from.x, selection.from.y, selection.from.z],
                "to": [selection.to.x, selection.to.y, selection.to.z],
            }
        }),
        None => json!({ "selection": null }),
    }
}

fn parse_dimensions(arguments: &Value) -> Result<Dimensions, ToolFailure> {
    let values = required_array3(arguments, "dimensions")?;
    Dimensions::new(values[0], values[1], values[2]).map_err(ToolFailure::scheme)
}

fn parse_selection(arguments: &Value) -> Result<Selection, ToolFailure> {
    let from = parse_coordinate(required_array3(arguments, "from")?);
    let to = parse_coordinate(required_array3(arguments, "to")?);
    Ok(from.to_selection(to))
}

fn parse_coordinate_field(arguments: &Value, field: &str) -> Result<Coordinate, ToolFailure> {
    Ok(parse_coordinate(required_array3(arguments, field)?))
}

fn parse_coordinate(values: [i32; 3]) -> Coordinate {
    Coordinate::new(values[0], values[1], values[2])
}

fn parse_block(value: &Value) -> Result<SchemeBlock, ToolFailure> {
    let block_id = value
        .get("blockId")
        .and_then(Value::as_str)
        .ok_or_else(|| invalid_arguments("block.blockId is required"))?
        .to_string();
    let states = value
        .get("states")
        .and_then(Value::as_object)
        .map(|states| {
            states
                .iter()
                .filter_map(|(name, value)| {
                    value
                        .as_str()
                        .map(|value| (name.clone(), value.to_string()))
                })
                .collect::<BTreeMap<_, _>>()
        })
        .unwrap_or_default();
    Ok(SchemeBlock {
        block_id,
        states,
        stage: parse_stage_ref(value.get("stageId"))?,
    })
}

fn parse_stage_ref(value: Option<&Value>) -> Result<StageRef, ToolFailure> {
    match value {
        None | Some(Value::Null) => Ok(StageRef::Unassigned),
        Some(Value::Number(number)) => {
            let id = number
                .as_u64()
                .and_then(|value| u32::try_from(value).ok())
                .ok_or_else(|| invalid_arguments("stageId must be a positive integer or null"))?;
            Ok(StageRef::Stage(id))
        }
        _ => Err(invalid_arguments(
            "stageId must be a positive integer or null",
        )),
    }
}

fn parse_export_format(arguments: &Value) -> Result<ExportFormat, ToolFailure> {
    let format = required_string(arguments, "format")?;
    ExportFormat::from_extension(&format).ok_or_else(|| {
        invalid_arguments(format!(
            "format must be one of: {}, {}",
            ExportFormat::Schem.extension(),
            ExportFormat::Litematic.extension()
        ))
    })
}

fn required_array3(arguments: &Value, field: &str) -> Result<[i32; 3], ToolFailure> {
    let values = arguments
        .get(field)
        .and_then(Value::as_array)
        .ok_or_else(|| invalid_arguments(format!("{field} must be [x, y, z]")))?;
    if values.len() != 3 {
        return Err(invalid_arguments(format!(
            "{field} must contain exactly 3 values"
        )));
    }
    let mut parsed = [0_i32; 3];
    for (index, value) in values.iter().enumerate() {
        parsed[index] = value
            .as_i64()
            .and_then(|value| i32::try_from(value).ok())
            .ok_or_else(|| invalid_arguments(format!("{field}[{index}] must be an integer")))?;
    }
    Ok(parsed)
}

fn required_string(arguments: &Value, field: &str) -> Result<String, ToolFailure> {
    arguments
        .get(field)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .ok_or_else(|| invalid_arguments(format!("{field} is required")))
}

fn required_i64(arguments: &Value, field: &str) -> Result<i64, ToolFailure> {
    arguments
        .get(field)
        .and_then(Value::as_i64)
        .ok_or_else(|| invalid_arguments(format!("{field} is required")))
}

fn required_u32(arguments: &Value, field: &str) -> Result<u32, ToolFailure> {
    required_i64(arguments, field)?
        .try_into()
        .map_err(|_| invalid_arguments(format!("{field} must be a positive integer")))
}

fn ensure_modpack_exists(workspace: &AgentWorkspace, modpack_id: i64) -> Result<(), ToolFailure> {
    if workspace
        .modpacks
        .iter()
        .any(|modpack| modpack.id == modpack_id)
    {
        Ok(())
    } else {
        Err(not_found("modpack", modpack_id))
    }
}

fn workspace_scheme(
    workspace: &AgentWorkspace,
    scheme_id: i64,
) -> Result<&AgentScheme, ToolFailure> {
    workspace
        .schemes
        .get(&scheme_id)
        .ok_or_else(|| not_found("scheme", scheme_id))
}

fn workspace_scheme_mut(
    workspace: &mut AgentWorkspace,
    scheme_id: i64,
) -> Result<&mut AgentScheme, ToolFailure> {
    workspace
        .schemes
        .get_mut(&scheme_id)
        .ok_or_else(|| not_found("scheme", scheme_id))
}

fn invalid_arguments(message: impl Into<String>) -> ToolFailure {
    ToolFailure::new("invalid_arguments", message, json!({}))
}

fn not_found(entity: &'static str, id: i64) -> ToolFailure {
    ToolFailure::new(
        "not_found",
        format!("{entity} {id} was not found"),
        json!({ "entity": entity, "id": id }),
    )
}

#[allow(dead_code)]
fn material_count(materials: &[MaterialLine]) -> u32 {
    materials.iter().map(|line| line.count).sum()
}
