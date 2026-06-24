use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use mpb_core::{
    BlockPlacement, BlockRegistry, Coordinate, Dimensions, MaterialLine, Scheme, SchemeBlock,
    SchemeError, SchemeOperation, Selection, StageRef,
};
use mpb_export::{write_scheme_export, ExportError, ExportFormat};
use mpb_storage::{
    LibraryDatabase, LibraryInstance, LibraryRepository, NewScheme, PrismInstanceStatus,
    StoredScheme,
};
use serde_json::{json, Value};
use thiserror::Error;

use crate::protocol::AgentEvent;
use crate::workspace::{AgentScheme, AgentWorkspace};

pub(crate) type StoredSelection = Option<Selection>;

pub(crate) struct StorageWorkspaceConfig<'a> {
    pub(crate) database_path: &'a PathBuf,
    pub(crate) diagnostics_dir: &'a PathBuf,
}

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

    fn storage(error: mpb_storage::StorageError) -> Self {
        Self::new("storage_error", error.to_string(), json!({}))
    }
}

pub(crate) fn dispatch_tool(
    workspace: &mut AgentWorkspace,
    name: &str,
    arguments: Value,
) -> Result<ToolOutcome, ToolFailure> {
    match name {
        "list_instances" => Ok(ToolOutcome::read(json!({
            "instances": workspace.instances
        }))),
        "list_schemes" => {
            let instance_id = required_i64(&arguments, "instanceId")?;
            Ok(ToolOutcome::read(json!({
                "schemes": workspace
                    .schemes
                    .values()
                    .filter(|scheme| scheme.instance_id == instance_id)
                    .map(scheme_summary)
                    .collect::<Vec<_>>()
            })))
        }
        "create_scheme" => {
            let instance_id = required_i64(&arguments, "instanceId")?;
            ensure_instance_ready(workspace, instance_id)?;
            let name = required_string(&arguments, "name")?;
            let dimensions = parse_dimensions(&arguments)?;
            let id = workspace.next_scheme_id;
            workspace.next_scheme_id += 1;
            let scheme = AgentScheme {
                id,
                instance_id,
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
            let stage_id = scheme
                .scheme
                .add_stage(&name)
                .map_err(ToolFailure::scheme)?;
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
            Ok(ToolOutcome::read(json!({
                "materials": scheme_materials_content(&scheme.scheme, None)
            })))
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

pub(crate) fn dispatch_storage_tool(
    config: StorageWorkspaceConfig<'_>,
    selection: &mut StoredSelection,
    name: &str,
    arguments: Value,
) -> Result<ToolOutcome, ToolFailure> {
    let repository = open_repository(config.database_path)?;
    match name {
        "list_instances" => {
            let library = repository.list_library().map_err(ToolFailure::storage)?;
            Ok(ToolOutcome::read(json!({
                "instances": library.iter().map(storage_instance_summary).collect::<Vec<_>>()
            })))
        }
        "list_schemes" => {
            let instance_id = required_i64(&arguments, "instanceId")?;
            let library = repository.list_library().map_err(ToolFailure::storage)?;
            let instance = library
                .iter()
                .find(|instance| instance.id == instance_id)
                .ok_or_else(|| not_found("instance", instance_id))?;
            Ok(ToolOutcome::read(json!({
                "schemes": instance.schemes.iter().map(storage_scheme_summary).collect::<Vec<_>>()
            })))
        }
        "create_scheme" => {
            let instance_id = required_i64(&arguments, "instanceId")?;
            ensure_storage_instance_ready(&repository, instance_id)?;
            let name = required_string(&arguments, "name")?;
            let dimensions = parse_dimensions(&arguments)?;
            let record = repository
                .create_scheme(NewScheme {
                    prism_instance_id: instance_id,
                    name,
                    size_x: i64::from(dimensions.x),
                    size_y: i64::from(dimensions.y),
                    size_z: i64::from(dimensions.z),
                })
                .map_err(ToolFailure::storage)?;
            let stored = repository
                .load_scheme(record.id)
                .map_err(ToolFailure::storage)?;
            Ok(ToolOutcome::changed(
                json!({ "scheme": stored_scheme_summary(&stored.record, &stored.scheme) }),
                AgentEvent::LibraryChanged {},
            ))
        }
        "rename_scheme" => {
            let scheme_id = required_i64(&arguments, "schemeId")?;
            let name = required_string(&arguments, "name")?;
            let record = repository
                .rename_scheme(scheme_id, &name)
                .map_err(ToolFailure::storage)?;
            let stored = repository
                .load_scheme(record.id)
                .map_err(ToolFailure::storage)?;
            Ok(ToolOutcome::changed(
                json!({ "scheme": stored_scheme_summary(&stored.record, &stored.scheme) }),
                AgentEvent::LibraryChanged {},
            ))
        }
        "delete_scheme" => {
            let scheme_id = required_i64(&arguments, "schemeId")?;
            repository
                .delete_scheme(scheme_id)
                .map_err(ToolFailure::storage)?;
            Ok(ToolOutcome::changed(
                json!({ "deletedSchemeId": scheme_id }),
                AgentEvent::LibraryChanged {},
            ))
        }
        "read_scheme_content" => {
            let scheme_id = required_i64(&arguments, "schemeId")?;
            let stored = repository
                .load_scheme(scheme_id)
                .map_err(ToolFailure::storage)?;
            let registry_report =
                registry_report_for_scheme(&repository, config.diagnostics_dir, &stored)?;
            Ok(ToolOutcome::read(stored_scheme_content(
                &stored.record,
                &stored.scheme,
                Some(&registry_report),
            )))
        }
        "read_current_selection" => Ok(ToolOutcome::read(selection_content(*selection))),
        "place_block" => mutate_stored_scheme(
            &repository,
            config.diagnostics_dir,
            &arguments,
            |scheme, registry, arguments| {
                let coordinate = parse_coordinate_field(arguments, "coordinate")?;
                let block = parse_block(&arguments["block"])?;
                scheme
                    .apply(
                        registry,
                        SchemeOperation::Place(BlockPlacement { coordinate, block }),
                    )
                    .map_err(ToolFailure::scheme)
            },
        ),
        "delete_block" => mutate_stored_scheme(
            &repository,
            config.diagnostics_dir,
            &arguments,
            |scheme, registry, arguments| {
                let coordinate = parse_coordinate_field(arguments, "coordinate")?;
                scheme
                    .apply(registry, SchemeOperation::Delete(coordinate))
                    .map_err(ToolFailure::scheme)
            },
        ),
        "replace_blocks" => mutate_stored_scheme(
            &repository,
            config.diagnostics_dir,
            &arguments,
            |scheme, registry, arguments| {
                let from_block_id = required_string(arguments, "fromBlockId")?;
                let to = parse_block(&arguments["to"])?;
                scheme
                    .apply(registry, SchemeOperation::ReplaceAll { from_block_id, to })
                    .map_err(ToolFailure::scheme)
            },
        ),
        "bulk_set_area" => mutate_stored_scheme(
            &repository,
            config.diagnostics_dir,
            &arguments,
            |scheme, registry, arguments| {
                let selection = parse_selection(arguments)?;
                let block = parse_block(&arguments["block"])?;
                scheme
                    .apply(registry, SchemeOperation::BulkSet { selection, block })
                    .map_err(ToolFailure::scheme)
            },
        ),
        "resize_scheme" => mutate_stored_scheme(
            &repository,
            config.diagnostics_dir,
            &arguments,
            |scheme, registry, arguments| {
                let dimensions = parse_dimensions(arguments)?;
                scheme
                    .apply(registry, SchemeOperation::Resize(dimensions))
                    .map_err(ToolFailure::scheme)
            },
        ),
        "create_stage" => {
            let scheme_id = required_i64(&arguments, "schemeId")?;
            let name = required_string(&arguments, "name")?;
            let mut stored = repository
                .load_scheme(scheme_id)
                .map_err(ToolFailure::storage)?;
            let stage_id = stored
                .scheme
                .add_stage(&name)
                .map_err(ToolFailure::scheme)?;
            repository
                .save_scheme(scheme_id, &stored.scheme)
                .map_err(ToolFailure::storage)?;
            let registry_report =
                registry_report_for_scheme(&repository, config.diagnostics_dir, &stored)?;
            Ok(ToolOutcome::changed(
                json!({
                    "stageId": stage_id,
                    "scheme": stored_scheme_content(&stored.record, &stored.scheme, Some(&registry_report))
                }),
                AgentEvent::SchemeChanged { scheme_id },
            ))
        }
        "rename_stage" => {
            let scheme_id = required_i64(&arguments, "schemeId")?;
            let stage_id = required_u32(&arguments, "stageId")?;
            let name = required_string(&arguments, "name")?;
            let mut stored = repository
                .load_scheme(scheme_id)
                .map_err(ToolFailure::storage)?;
            stored
                .scheme
                .rename_stage(stage_id, &name)
                .map_err(ToolFailure::scheme)?;
            repository
                .save_scheme(scheme_id, &stored.scheme)
                .map_err(ToolFailure::storage)?;
            let registry_report =
                registry_report_for_scheme(&repository, config.diagnostics_dir, &stored)?;
            Ok(ToolOutcome::changed(
                json!({
                    "scheme": stored_scheme_content(&stored.record, &stored.scheme, Some(&registry_report))
                }),
                AgentEvent::SchemeChanged { scheme_id },
            ))
        }
        "assign_blocks_to_stage" => mutate_stored_scheme(
            &repository,
            config.diagnostics_dir,
            &arguments,
            |scheme, registry, arguments| {
                let selection = parse_selection(arguments)?;
                let stage = parse_stage_ref(arguments.get("stageId"))?;
                scheme
                    .apply(registry, SchemeOperation::AssignStage { selection, stage })
                    .map_err(ToolFailure::scheme)
            },
        ),
        "validate_scheme" => {
            let scheme_id = required_i64(&arguments, "schemeId")?;
            let stored = repository
                .load_scheme(scheme_id)
                .map_err(ToolFailure::storage)?;
            let registry = registry_for_scheme(&repository, config.diagnostics_dir, &stored)?;
            match stored.scheme.validate(&registry) {
                Ok(()) => Ok(ToolOutcome::read(json!({
                    "valid": true,
                    "diagnostic": validation_diagnostic(scheme_id, "success", None, None)
                }))),
                Err(error) => Ok(ToolOutcome::read(json!({
                    "valid": false,
                    "error": { "code": error.code(), "message": error.to_string() },
                    "diagnostic": validation_diagnostic(scheme_id, "failed", Some(error.code()), Some(error.to_string()))
                }))),
            }
        }
        "get_materials" => {
            let scheme_id = required_i64(&arguments, "schemeId")?;
            let stored = repository
                .load_scheme(scheme_id)
                .map_err(ToolFailure::storage)?;
            let registry_report =
                registry_report_for_scheme(&repository, config.diagnostics_dir, &stored)?;
            Ok(ToolOutcome::read(json!({
                "materials": scheme_materials_content(&stored.scheme, Some(&registry_report))
            })))
        }
        "export_scheme" => {
            let scheme_id = required_i64(&arguments, "schemeId")?;
            let format = parse_export_format(&arguments)?;
            let destination_path = PathBuf::from(required_string(&arguments, "destinationPath")?);
            let stored = repository
                .load_scheme(scheme_id)
                .map_err(ToolFailure::storage)?;
            let artifact = write_scheme_export(&stored.scheme, format, destination_path)
                .map_err(ToolFailure::export)?;
            Ok(ToolOutcome::read(json!({
                "path": artifact.path,
                "byteLength": artifact.byte_len,
                "blockCount": artifact.block_count,
            })))
        }
        _ => Err(ToolFailure::new(
            "unknown_tool",
            format!("Unknown MCP tool '{name}'"),
            json!({}),
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
            "Use a block id and states from the ready Prism instance registry, then call the tool again."
        }
        "not_found" => "Refresh the library context and retry with an existing instance or scheme id.",
        "instance_not_ready" => {
            "Wait until Prism instance indexing reaches Ready, or fix the instance diagnostics in the desktop app."
        }
        "missing_asset_registry" | "invalid_asset_registry" => {
            "Let the desktop app finish Prism instance indexing so it can build the block registry."
        }
        "storage_error" => "Refresh the desktop app state and retry. If the error repeats, inspect the app diagnostics.",
        _ => "Adjust the request, keep the current scheme open, and call the tool again.",
    }
}

fn open_repository(database_path: &Path) -> Result<LibraryRepository, ToolFailure> {
    let database = LibraryDatabase::open(database_path).map_err(ToolFailure::storage)?;
    Ok(LibraryRepository::new(database))
}

fn storage_instance_summary(instance: &LibraryInstance) -> Value {
    json!({
        "id": instance.id,
        "instanceId": &instance.instance_id,
        "displayName": &instance.display_name,
        "instancePath": &instance.instance_path,
        "minecraftVersion": &instance.minecraft_version,
        "loader": &instance.loader,
        "loaderVersion": &instance.loader_version,
        "status": instance.status,
        "statusMessage": &instance.status_message,
        "schemeCount": instance.schemes.len(),
    })
}

fn storage_scheme_summary(scheme: &mpb_storage::SchemeRecord) -> Value {
    json!({
        "id": scheme.id,
        "instanceId": scheme.prism_instance_id,
        "name": &scheme.name,
        "dimensions": [scheme.dimensions.0, scheme.dimensions.1, scheme.dimensions.2],
    })
}

fn stored_scheme_summary(record: &mpb_storage::SchemeRecord, scheme: &Scheme) -> Value {
    let dimensions = scheme.dimensions();
    json!({
        "id": record.id,
        "instanceId": record.prism_instance_id,
        "name": scheme.name(),
        "dimensions": [dimensions.x, dimensions.y, dimensions.z],
        "blockCount": scheme.block_count(),
    })
}

fn stored_scheme_content(
    record: &mpb_storage::SchemeRecord,
    scheme: &Scheme,
    registry_report: Option<&Value>,
) -> Value {
    let dimensions = scheme.dimensions();
    json!({
        "id": record.id,
        "instanceId": record.prism_instance_id,
        "name": scheme.name(),
        "dimensions": [dimensions.x, dimensions.y, dimensions.z],
        "stages": scheme.stages(),
        "blocks": scheme
            .blocks()
            .map(|(coordinate, block)| block_content(*coordinate, block))
            .collect::<Vec<_>>(),
        "blockCount": scheme.block_count(),
        "materials": scheme_materials_content(scheme, registry_report),
    })
}

fn ensure_storage_instance_ready(
    repository: &LibraryRepository,
    instance_id: i64,
) -> Result<(), ToolFailure> {
    let instance = repository
        .get_prism_instance(instance_id)
        .map_err(ToolFailure::storage)?;
    if instance.status == PrismInstanceStatus::Ready {
        Ok(())
    } else {
        Err(ToolFailure::new(
            "instance_not_ready",
            format!(
                "Prism instance {} is {} and cannot accept scheme changes yet.",
                instance.display_name, instance.status
            ),
            json!({
                "instanceId": instance_id,
                "status": instance.status,
                "statusMessage": instance.status_message,
            }),
        ))
    }
}

fn mutate_stored_scheme(
    repository: &LibraryRepository,
    diagnostics_dir: &Path,
    arguments: &Value,
    operation: impl FnOnce(&mut Scheme, &BlockRegistry, &Value) -> Result<(), ToolFailure>,
) -> Result<ToolOutcome, ToolFailure> {
    let scheme_id = required_i64(arguments, "schemeId")?;
    let mut stored = repository
        .load_scheme(scheme_id)
        .map_err(ToolFailure::storage)?;
    ensure_storage_instance_ready(repository, stored.record.prism_instance_id)?;
    let registry = registry_for_scheme(repository, diagnostics_dir, &stored)?;
    operation(&mut stored.scheme, &registry, arguments)?;
    repository
        .save_scheme(scheme_id, &stored.scheme)
        .map_err(ToolFailure::storage)?;
    let registry_report = registry_report_for_scheme(repository, diagnostics_dir, &stored)?;
    Ok(ToolOutcome::changed(
        json!({
            "scheme": stored_scheme_content(&stored.record, &stored.scheme, Some(&registry_report))
        }),
        AgentEvent::SchemeChanged { scheme_id },
    ))
}

fn registry_for_scheme(
    repository: &LibraryRepository,
    diagnostics_dir: &Path,
    stored: &StoredScheme,
) -> Result<BlockRegistry, ToolFailure> {
    let instance = repository
        .get_prism_instance(stored.record.prism_instance_id)
        .map_err(ToolFailure::storage)?;
    if instance.status != PrismInstanceStatus::Ready {
        ensure_storage_instance_ready(repository, instance.id)?;
    }
    let report = registry_report_for_instance(diagnostics_dir, &instance)?;
    block_registry_from_report(&report, instance.id)
}

fn registry_report_for_scheme(
    repository: &LibraryRepository,
    diagnostics_dir: &Path,
    stored: &StoredScheme,
) -> Result<Value, ToolFailure> {
    let instance = repository
        .get_prism_instance(stored.record.prism_instance_id)
        .map_err(ToolFailure::storage)?;
    if instance.status != PrismInstanceStatus::Ready {
        ensure_storage_instance_ready(repository, instance.id)?;
    }
    registry_report_for_instance(diagnostics_dir, &instance)
}

fn registry_report_for_instance(
    diagnostics_dir: &Path,
    instance: &mpb_storage::PrismInstanceRecord,
) -> Result<Value, ToolFailure> {
    let report_path = diagnostics_dir.join(format!(
        "{}-registry.json",
        safe_report_stem(&instance.identity_fingerprint)
    ));
    let json_text = std::fs::read_to_string(&report_path).map_err(|error| {
        ToolFailure::new(
            "missing_asset_registry",
            format!(
                "Could not read Prism instance block registry at {}: {error}",
                report_path.display()
            ),
            json!({ "instanceId": instance.id, "path": report_path }),
        )
    })?;
    serde_json::from_str(&json_text).map_err(|error| {
        ToolFailure::new(
            "invalid_asset_registry",
            format!(
                "Could not parse Prism instance block registry at {}: {error}",
                report_path.display()
            ),
            json!({ "instanceId": instance.id, "path": report_path }),
        )
    })
}

fn block_registry_from_report(
    report: &Value,
    instance_id: i64,
) -> Result<BlockRegistry, ToolFailure> {
    let blocks = report["blocks"]
        .as_array()
        .ok_or_else(|| {
            ToolFailure::new(
                "invalid_asset_registry",
                "Prism registry report has no blocks array",
                json!({ "instanceId": instance_id }),
            )
        })?
        .iter()
        .filter_map(|block| block["identifier"].as_str().map(ToString::to_string))
        .collect::<Vec<_>>();
    Ok(BlockRegistry::from_block_ids(blocks))
}

fn scheme_summary(scheme: &AgentScheme) -> Value {
    let dimensions = scheme.scheme.dimensions();
    json!({
        "id": scheme.id,
        "instanceId": scheme.instance_id,
        "name": scheme.name,
        "dimensions": [dimensions.x, dimensions.y, dimensions.z],
        "blockCount": scheme.scheme.block_count()
    })
}

fn scheme_content(scheme: &AgentScheme) -> Value {
    let dimensions = scheme.scheme.dimensions();
    json!({
        "id": scheme.id,
        "instanceId": scheme.instance_id,
        "name": scheme.name,
        "dimensions": [dimensions.x, dimensions.y, dimensions.z],
        "stages": scheme.scheme.stages(),
        "blocks": scheme
            .scheme
            .blocks()
            .map(|(coordinate, block)| block_content(*coordinate, block))
            .collect::<Vec<_>>(),
        "blockCount": scheme.scheme.block_count(),
        "materials": scheme_materials_content(&scheme.scheme, None),
    })
}

#[derive(Debug, Clone, Default)]
struct MaterialMetadata {
    display_name: Option<String>,
    item_id: Option<String>,
    max_stack_size: Option<u32>,
    texture_path: Option<String>,
}

fn scheme_materials_content(scheme: &Scheme, registry_report: Option<&Value>) -> Vec<Value> {
    let metadata = registry_report
        .map(registry_material_metadata)
        .unwrap_or_default();
    scheme
        .materials()
        .into_iter()
        .map(|line| {
            let material = metadata.get(&line.block_id).cloned().unwrap_or_default();
            let max_stack_size = material.max_stack_size;
            json!({
                "blockId": line.block_id,
                "displayName": material.display_name.unwrap_or_else(|| line.block_id.clone()),
                "count": line.count,
                "itemId": material.item_id,
                "maxStackSize": max_stack_size,
                "stackCount": max_stack_size
                    .filter(|size| *size > 0)
                    .map(|size| line.count.div_ceil(size)),
                "texturePath": material.texture_path,
            })
        })
        .collect()
}

fn registry_material_metadata(report: &Value) -> BTreeMap<String, MaterialMetadata> {
    report
        .get("blocks")
        .and_then(Value::as_array)
        .map(|blocks| {
            blocks
                .iter()
                .filter_map(|block| {
                    let identifier = block.get("identifier")?.as_str()?.to_string();
                    Some((
                        identifier,
                        MaterialMetadata {
                            display_name: block
                                .get("displayName")
                                .and_then(Value::as_str)
                                .map(ToString::to_string),
                            item_id: block
                                .get("itemId")
                                .and_then(Value::as_str)
                                .map(ToString::to_string),
                            max_stack_size: block
                                .get("maxStackSize")
                                .and_then(Value::as_u64)
                                .and_then(|value| u32::try_from(value).ok()),
                            texture_path: block
                                .get("texturePath")
                                .and_then(Value::as_str)
                                .map(ToString::to_string),
                        },
                    ))
                })
                .collect()
        })
        .unwrap_or_default()
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

fn safe_report_stem(value: &str) -> String {
    let cleaned = value
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || matches!(character, '-' | '_') {
                character
            } else {
                '-'
            }
        })
        .collect::<String>();
    if cleaned.is_empty() {
        "prism-instance".to_string()
    } else {
        cleaned
    }
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

fn ensure_instance_ready(workspace: &AgentWorkspace, instance_id: i64) -> Result<(), ToolFailure> {
    let instance = workspace
        .instances
        .iter()
        .find(|instance| instance.id == instance_id)
        .ok_or_else(|| not_found("instance", instance_id))?;
    if instance.status == "ready" {
        Ok(())
    } else {
        Err(ToolFailure::new(
            "instance_not_ready",
            format!(
                "Prism instance {} is {} and cannot accept scheme changes yet.",
                instance.display_name, instance.status
            ),
            json!({ "instanceId": instance_id, "status": instance.status }),
        ))
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

#[cfg(test)]
mod tests {
    use std::fs;

    use mpb_core::{BlockPlacement, Coordinate};
    use mpb_storage::{NewPrismInstance, NewScheme};

    use super::*;

    #[test]
    fn storage_get_materials_uses_registry_metadata_without_faking_stack_sizes() {
        let test_dir = std::env::temp_dir().join(format!(
            "mpb-agent-materials-{}-{}",
            std::process::id(),
            std::thread::current().name().unwrap_or("test")
        ));
        let _ = fs::remove_dir_all(&test_dir);
        fs::create_dir_all(&test_dir).expect("create test dir");
        let database_path = test_dir.join("library.sqlite3");
        let diagnostics_dir = test_dir.join("diagnostics");
        fs::create_dir_all(&diagnostics_dir).expect("create diagnostics dir");

        let database = LibraryDatabase::open(&database_path).expect("open database");
        let repository = LibraryRepository::new(database);
        let instance = repository
            .upsert_prism_instance(NewPrismInstance {
                instance_id: "aoc".to_string(),
                display_name: "AOC".to_string(),
                instance_path: PathBuf::from("/PrismLauncher/instances/aoc"),
                minecraft_dir: PathBuf::from("/PrismLauncher/instances/aoc/minecraft"),
                minecraft_version: Some("1.21.1".to_string()),
                loader: Some("NeoForge".to_string()),
                loader_version: Some("21.1.233".to_string()),
                identity_fingerprint: "identity-aoc".to_string(),
                content_fingerprint: "content-aoc".to_string(),
                status: PrismInstanceStatus::Ready,
                status_message: None,
            })
            .expect("insert instance");
        let record = repository
            .create_scheme(NewScheme {
                prism_instance_id: instance.id,
                name: "Factory".to_string(),
                size_x: 5,
                size_y: 5,
                size_z: 5,
            })
            .expect("create scheme");
        let mut stored = repository.load_scheme(record.id).expect("load scheme");
        let registry = BlockRegistry::from_block_ids([
            "minecraft:stone".to_string(),
            "create:andesite_casing".to_string(),
        ]);
        for index in 0..65 {
            stored
                .scheme
                .apply(
                    &registry,
                    SchemeOperation::Place(BlockPlacement::new(
                        Coordinate::new(index % 5, (index / 5) % 5, index / 25),
                        "minecraft:stone",
                        [],
                        StageRef::Unassigned,
                    )),
                )
                .expect("place stone");
        }
        stored
            .scheme
            .apply(
                &registry,
                SchemeOperation::Place(BlockPlacement::new(
                    Coordinate::new(4, 4, 4),
                    "create:andesite_casing",
                    [],
                    StageRef::Unassigned,
                )),
            )
            .expect("place casing");
        repository
            .save_scheme(record.id, &stored.scheme)
            .expect("save scheme");

        let report_path = diagnostics_dir.join("identity-aoc-registry.json");
        fs::write(
            &report_path,
            serde_json::to_string_pretty(&json!({
                "status": "ready",
                "blocks": [
                    {
                        "identifier": "minecraft:stone",
                        "displayName": "Stone",
                        "itemId": "minecraft:stone",
                        "maxStackSize": 64,
                        "texturePath": "/tmp/stone.png"
                    },
                    {
                        "identifier": "create:andesite_casing",
                        "displayName": "Andesite Casing",
                        "itemId": "create:andesite_casing",
                        "maxStackSize": null,
                        "texturePath": null
                    }
                ]
            }))
            .expect("serialize report"),
        )
        .expect("write report");

        let outcome = dispatch_storage_tool(
            StorageWorkspaceConfig {
                database_path: &database_path,
                diagnostics_dir: &diagnostics_dir,
            },
            &mut None,
            "get_materials",
            json!({ "schemeId": record.id }),
        )
        .expect("get materials");

        let materials = outcome.value["materials"].as_array().expect("materials");
        let stone = materials
            .iter()
            .find(|line| line["blockId"] == "minecraft:stone")
            .expect("stone material");
        assert_eq!(stone["displayName"], "Stone");
        assert_eq!(stone["itemId"], "minecraft:stone");
        assert_eq!(stone["maxStackSize"], 64);
        assert_eq!(stone["stackCount"], 2);
        assert_eq!(stone["texturePath"], "/tmp/stone.png");

        let casing = materials
            .iter()
            .find(|line| line["blockId"] == "create:andesite_casing")
            .expect("casing material");
        assert_eq!(casing["displayName"], "Andesite Casing");
        assert!(casing["maxStackSize"].is_null());
        assert!(casing["stackCount"].is_null());

        let _ = fs::remove_dir_all(&test_dir);
    }
}
