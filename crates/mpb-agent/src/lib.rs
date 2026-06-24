//! MCP-compatible AI tool integration for Minecraft Pack Builder.

mod http;
mod tool_schemas;

use std::collections::{BTreeMap, VecDeque};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

pub use http::{start_streamable_http_server, McpHttpServerHandle};
use tool_schemas::tool_definitions;

use mpb_core::{
    BlockPlacement, BlockRegistry, Coordinate, Dimensions, MaterialLine, Scheme, SchemeBlock,
    SchemeError, SchemeOperation, Selection, StageRef,
};
use serde::Serialize;
use serde_json::{json, Value};
use thiserror::Error;

pub const MCP_PROTOCOL_VERSION: &str = "2025-06-18";
pub const DEFAULT_MCP_PORT: u16 = 47392;
const MCP_ENDPOINT_PATH: &str = "/mcp";
const ACTIVE_CLIENT_TTL: Duration = Duration::from_secs(30 * 60);

pub fn default_mcp_endpoint() -> String {
    format!("http://127.0.0.1:{DEFAULT_MCP_PORT}{MCP_ENDPOINT_PATH}")
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClientIdentity {
    pub remote_addr: String,
    pub client_name_hint: Option<String>,
}

impl ClientIdentity {
    pub fn new(remote_addr: impl Into<String>, client_name_hint: Option<&str>) -> Self {
        Self {
            remote_addr: remote_addr.into(),
            client_name_hint: client_name_hint.map(str::to_string),
        }
    }
}

#[derive(Debug, Clone)]
pub struct JsonRpcRequest {
    id: Value,
    method: String,
    params: Value,
    identity: ClientIdentity,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum JsonRpcOutcome {
    Response(Value),
    Accepted,
}

impl JsonRpcRequest {
    pub fn new(
        id: i64,
        method: impl Into<String>,
        params: Value,
        identity: ClientIdentity,
    ) -> Self {
        Self {
            id: json!(id),
            method: method.into(),
            params,
            identity,
        }
    }

    fn from_value(value: Value, identity: ClientIdentity) -> Result<Self, AgentError> {
        let method = value
            .get("method")
            .and_then(Value::as_str)
            .ok_or_else(|| AgentError::Protocol("JSON-RPC method is required".to_string()))?
            .to_string();
        Ok(Self {
            id: value.get("id").cloned().unwrap_or(Value::Null),
            method,
            params: value.get("params").cloned().unwrap_or_else(|| json!({})),
            identity,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentStatus {
    pub server_running: bool,
    pub transport: String,
    pub endpoint: Option<String>,
    pub protocol_version: String,
    pub active_client: Option<String>,
    pub tool_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum AgentEvent {
    LibraryChanged {},
    SchemeChanged { scheme_id: i64 },
}

#[derive(Debug, Error)]
pub enum AgentError {
    #[error("{0}")]
    Protocol(String),
    #[error("{0}")]
    Http(String),
    #[error("{0}")]
    Workspace(String),
}

#[derive(Clone)]
pub struct AgentServer {
    inner: Arc<AgentServerInner>,
}

struct AgentServerInner {
    status: Mutex<AgentStatusInner>,
    workspace: Mutex<AgentWorkspace>,
    events: Mutex<VecDeque<AgentEvent>>,
}

#[derive(Debug, Clone)]
struct AgentStatusInner {
    server_running: bool,
    endpoint: Option<String>,
    active_client: Option<String>,
    active_client_last_seen: Option<Instant>,
}

fn clear_expired_active_client(status: &mut AgentStatusInner) {
    if status
        .active_client_last_seen
        .is_some_and(|last_seen| last_seen.elapsed() > ACTIVE_CLIENT_TTL)
    {
        status.active_client = None;
        status.active_client_last_seen = None;
    }
}

impl AgentServer {
    pub fn new_demo() -> Self {
        Self::new(AgentWorkspace::demo())
    }

    pub fn new(workspace: AgentWorkspace) -> Self {
        Self {
            inner: Arc::new(AgentServerInner {
                status: Mutex::new(AgentStatusInner {
                    server_running: false,
                    endpoint: None,
                    active_client: None,
                    active_client_last_seen: None,
                }),
                workspace: Mutex::new(workspace),
                events: Mutex::new(VecDeque::new()),
            }),
        }
    }

    pub fn handle(&self, request: JsonRpcRequest) -> Result<Value, AgentError> {
        let response = match request.method.as_str() {
            "initialize" => self.initialize(&request),
            "ping" => Ok(json_rpc_result(request.id, json!({}))),
            "tools/list" => Ok(json_rpc_result(
                request.id,
                json!({ "tools": tool_definitions() }),
            )),
            "tools/call" => self.call_tool(&request),
            _ => Ok(json_rpc_error(
                request.id,
                -32601,
                "Method not found",
                "method_not_found",
                json!({ "method": request.method }),
            )),
        }?;
        Ok(response)
    }

    pub fn handle_json(&self, json_text: &str, identity: ClientIdentity) -> Result<Value, AgentError> {
        match self.handle_json_message(json_text, identity)? {
            JsonRpcOutcome::Response(response) => Ok(response),
            JsonRpcOutcome::Accepted => Ok(Value::Null),
        }
    }

    pub fn handle_json_message(
        &self,
        json_text: &str,
        identity: ClientIdentity,
    ) -> Result<JsonRpcOutcome, AgentError> {
        let value = serde_json::from_str::<Value>(json_text)
            .map_err(|error| AgentError::Protocol(format!("Invalid JSON-RPC payload: {error}")))?;
        if value.get("method").is_none() {
            return Ok(JsonRpcOutcome::Accepted);
        }
        if value.get("id").is_none() {
            return Ok(JsonRpcOutcome::Accepted);
        }
        let request = JsonRpcRequest::from_value(value, identity)?;
        self.touch_active_client();
        self.handle(request).map(JsonRpcOutcome::Response)
    }

    pub fn status(&self) -> AgentStatus {
        let mut status = self.inner.status.lock().expect("agent status lock");
        clear_expired_active_client(&mut status);
        AgentStatus {
            server_running: status.server_running,
            transport: "streamable-http".to_string(),
            endpoint: status.endpoint.clone(),
            protocol_version: MCP_PROTOCOL_VERSION.to_string(),
            active_client: status.active_client.clone(),
            tool_count: tool_definitions().len(),
        }
    }

    pub fn set_server_endpoint(&self, endpoint: Option<String>) {
        let mut status = self.inner.status.lock().expect("agent status lock");
        status.server_running = endpoint.is_some();
        status.endpoint = endpoint;
    }

    pub fn drain_events(&self) -> Vec<AgentEvent> {
        let mut events = self.inner.events.lock().expect("agent events lock");
        events.drain(..).collect()
    }

    pub fn clear_active_client(&self) {
        let mut status = self.inner.status.lock().expect("agent status lock");
        status.active_client = None;
        status.active_client_last_seen = None;
    }

    fn touch_active_client(&self) {
        let mut status = self.inner.status.lock().expect("agent status lock");
        clear_expired_active_client(&mut status);
        if status.active_client.is_some() {
            status.active_client_last_seen = Some(Instant::now());
        }
    }

    fn initialize(&self, request: &JsonRpcRequest) -> Result<Value, AgentError> {
        let client_name = request
            .params
            .get("clientInfo")
            .and_then(|client| client.get("name"))
            .and_then(Value::as_str)
            .or(request.identity.client_name_hint.as_deref())
            .unwrap_or("External MCP client")
            .to_string();
        let requested_protocol = request
            .params
            .get("protocolVersion")
            .and_then(Value::as_str)
            .unwrap_or(MCP_PROTOCOL_VERSION);

        if requested_protocol != MCP_PROTOCOL_VERSION {
            return Ok(json_rpc_error(
                request.id.clone(),
                -32001,
                "Unsupported MCP protocol version",
                "unsupported_protocol_version",
                json!({
                    "requested": requested_protocol,
                    "supported": MCP_PROTOCOL_VERSION
                }),
            ));
        }

        let mut status = self.inner.status.lock().expect("agent status lock");
        clear_expired_active_client(&mut status);
        if let Some(active_client) = status.active_client.as_ref() {
            if active_client != &client_name {
                return Ok(json_rpc_error(
                    request.id.clone(),
                    -32002,
                    "Another external AI client is already connected",
                    "active_client_already_connected",
                    json!({ "activeClient": active_client }),
                ));
            }
        }
        status.active_client = Some(client_name);
        status.active_client_last_seen = Some(Instant::now());

        Ok(json_rpc_result(
            request.id.clone(),
            json!({
                "protocolVersion": MCP_PROTOCOL_VERSION,
                "capabilities": {
                    "tools": { "listChanged": true }
                },
                "serverInfo": {
                    "name": "minecraft-pack-builder",
                    "version": env!("CARGO_PKG_VERSION")
                },
                "instructions": "Use these tools to import modpacks, inspect and mutate schemes, validate changes, read selections, and prepare exports. Mutating tools are validated atomically by the Rust core."
            }),
        ))
    }

    fn call_tool(&self, request: &JsonRpcRequest) -> Result<Value, AgentError> {
        let name = request
            .params
            .get("name")
            .and_then(Value::as_str)
            .ok_or_else(|| AgentError::Protocol("tools/call requires params.name".to_string()))?;
        let arguments = request
            .params
            .get("arguments")
            .cloned()
            .unwrap_or_else(|| json!({}));

        let result = {
            let mut workspace = self.inner.workspace.lock().expect("agent workspace lock");
            dispatch_tool(&mut workspace, name, arguments)
        };

        match result {
            Ok(outcome) => {
                if let Some(event) = outcome.event {
                    self.inner
                        .events
                        .lock()
                        .expect("agent events lock")
                        .push_back(event);
                }
                Ok(json_rpc_result(request.id.clone(), tool_success(outcome.value)))
            }
            Err(error) => Ok(json_rpc_result(
                request.id.clone(),
                tool_error(error.code, error.message, error.data),
            )),
        }
    }
}

#[derive(Debug, Clone)]
pub struct AgentWorkspace {
    registry: BlockRegistry,
    modpacks: Vec<AgentModpack>,
    schemes: BTreeMap<i64, AgentScheme>,
    next_scheme_id: i64,
    current_selection: Option<Selection>,
}

impl AgentWorkspace {
    pub fn demo() -> Self {
        let registry = BlockRegistry::synthetic_fixture();
        let mut scheme = Scheme::new(
            "Starter Factory",
            Dimensions::new(8, 5, 8).expect("valid dimensions"),
        );
        let foundation = scheme.add_stage("Stage 1").expect("stage 1");
        let machinery = scheme.add_stage("Stage 2").expect("stage 2");
        for placement in [
            BlockPlacement::new(
                Coordinate::new(0, 0, 0),
                "minecraft:stone_bricks",
                [("cracked", "false")],
                StageRef::Stage(foundation),
            ),
            BlockPlacement::new(
                Coordinate::new(1, 0, 0),
                "minecraft:stone_bricks",
                [("cracked", "false")],
                StageRef::Stage(foundation),
            ),
            BlockPlacement::new(
                Coordinate::new(1, 1, 0),
                "thermal:machine_frame",
                [("tier", "basic")],
                StageRef::Stage(machinery),
            ),
            BlockPlacement::new(
                Coordinate::new(2, 1, 0),
                "minecraft:glass",
                [("color", "clear")],
                StageRef::Stage(machinery),
            ),
            BlockPlacement::new(
                Coordinate::new(3, 0, 0),
                "create:andesite_casing",
                [],
                StageRef::Unassigned,
            ),
        ] {
            scheme
                .apply(&registry, SchemeOperation::Place(placement))
                .expect("valid demo placement");
        }

        let mut schemes = BTreeMap::new();
        schemes.insert(
            10,
            AgentScheme {
                id: 10,
                modpack_id: 1,
                name: "Starter Factory".to_string(),
                scheme,
            },
        );

        Self {
            registry,
            modpacks: vec![AgentModpack {
                id: 1,
                local_name: "AOC - 1.0.0".to_string(),
                source_url: Some("https://www.curseforge.com/minecraft/modpacks/aoc".to_string()),
                version_name: "1.0.0".to_string(),
                minecraft_version: Some("1.20.1".to_string()),
                loader: Some("Forge".to_string()),
            }],
            schemes,
            next_scheme_id: 11,
            current_selection: None,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct AgentModpack {
    id: i64,
    local_name: String,
    source_url: Option<String>,
    version_name: String,
    minecraft_version: Option<String>,
    loader: Option<String>,
}

#[derive(Debug, Clone)]
struct AgentScheme {
    id: i64,
    modpack_id: i64,
    name: String,
    scheme: Scheme,
}

#[derive(Debug)]
struct ToolOutcome {
    value: Value,
    event: Option<AgentEvent>,
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
struct ToolFailure {
    code: &'static str,
    message: String,
    data: Value,
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
}

fn dispatch_tool(
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
                scheme: Scheme::new(&name, dimensions),
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
        "read_current_selection" => Ok(ToolOutcome::read(selection_content(workspace.current_selection))),
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
                Ok(()) => Ok(ToolOutcome::read(json!({ "valid": true, "errors": [] }))),
                Err(error) => Ok(ToolOutcome::read(json!({
                    "valid": false,
                    "errors": [{ "code": error.code(), "message": error.to_string() }]
                }))),
            }
        }
        "get_materials" => {
            let scheme_id = required_i64(&arguments, "schemeId")?;
            let scheme = workspace_scheme(workspace, scheme_id)?;
            Ok(ToolOutcome::read(json!({ "materials": scheme.scheme.materials() })))
        }
        "export_scheme" => Err(ToolFailure::new(
            "export_formats_land_in_phase_10",
            "The MCP export tool is reserved and validates its request shape now; .schem and .litematic file writing lands in phase 10.",
            json!({ "supportedFormats": ["schem", "litematic"] }),
        )),
        _ => Err(ToolFailure::new(
            "unknown_tool",
            format!("Unknown MCP tool '{name}'"),
            json!({ "tool": name }),
        )),
    }
}

fn json_rpc_result(id: Value, result: Value) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "result": result })
}

fn json_rpc_error(
    id: Value,
    code: i64,
    message: &str,
    app_code: &str,
    data: Value,
) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "error": {
            "code": code,
            "message": message,
            "data": {
                "code": app_code,
                "details": data
            }
        }
    })
}

fn tool_success(value: Value) -> Value {
    json!({
        "content": [{ "type": "text", "text": value.to_string() }],
        "structuredContent": value,
        "isError": false
    })
}

fn tool_error(code: &'static str, message: String, data: Value) -> Value {
    let value = json!({
        "error": {
            "code": code,
            "message": message,
            "details": data
        }
    });
    json!({
        "content": [{ "type": "text", "text": value.to_string() }],
        "structuredContent": value,
        "isError": true
    })
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
                .filter_map(|(name, value)| value.as_str().map(|value| (name.clone(), value.to_string())))
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
        _ => Err(invalid_arguments("stageId must be a positive integer or null")),
    }
}

fn required_array3(arguments: &Value, field: &str) -> Result<[i32; 3], ToolFailure> {
    let values = arguments
        .get(field)
        .and_then(Value::as_array)
        .ok_or_else(|| invalid_arguments(format!("{field} must be [x, y, z]")))?;
    if values.len() != 3 {
        return Err(invalid_arguments(format!("{field} must contain exactly 3 values")));
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
    if workspace.modpacks.iter().any(|modpack| modpack.id == modpack_id) {
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
