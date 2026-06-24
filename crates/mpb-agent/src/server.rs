use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use serde_json::{json, Value};

use crate::protocol::{
    initialize_result, json_rpc_error, json_rpc_result, AgentError, AgentEvent, AgentStatus,
    ClientIdentity, JsonRpcOutcome, JsonRpcRequest,
};
use crate::tool_schemas::tool_definitions;
use crate::tools::{
    dispatch_storage_tool, dispatch_tool, tool_error, tool_success, StorageWorkspaceConfig,
    StoredSelection,
};
use crate::workspace::AgentWorkspace;
use crate::{MCP_PROTOCOL_VERSION, MCP_TRANSPORT};

const ACTIVE_CLIENT_TTL: Duration = Duration::from_secs(30 * 60);

#[derive(Clone)]
pub struct AgentServer {
    inner: Arc<AgentServerInner>,
}

struct AgentServerInner {
    status: Mutex<AgentStatusInner>,
    workspace: AgentWorkspaceBackend,
    events: Mutex<VecDeque<AgentEvent>>,
}

enum AgentWorkspaceBackend {
    Memory(Mutex<AgentWorkspace>),
    Storage {
        database_path: PathBuf,
        diagnostics_dir: PathBuf,
        selection: Mutex<StoredSelection>,
    },
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
    pub fn new_test_fixture() -> Self {
        Self::new(AgentWorkspace::test_fixture())
    }

    pub fn new_storage(database_path: PathBuf, diagnostics_dir: PathBuf) -> Self {
        Self {
            inner: Arc::new(AgentServerInner {
                status: Mutex::new(AgentStatusInner {
                    server_running: false,
                    endpoint: None,
                    active_client: None,
                    active_client_last_seen: None,
                }),
                workspace: AgentWorkspaceBackend::Storage {
                    database_path,
                    diagnostics_dir,
                    selection: Mutex::new(None),
                },
                events: Mutex::new(VecDeque::new()),
            }),
        }
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
                workspace: AgentWorkspaceBackend::Memory(Mutex::new(workspace)),
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

    pub fn handle_json(
        &self,
        json_text: &str,
        identity: ClientIdentity,
    ) -> Result<Value, AgentError> {
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
            transport: MCP_TRANSPORT.to_string(),
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

        Ok(initialize_result(request.id.clone()))
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

        let result = match &self.inner.workspace {
            AgentWorkspaceBackend::Memory(workspace) => {
                let mut workspace = workspace.lock().expect("agent workspace lock");
                dispatch_tool(&mut workspace, name, arguments)
            }
            AgentWorkspaceBackend::Storage {
                database_path,
                diagnostics_dir,
                selection,
            } => {
                let mut selection = selection.lock().expect("agent selection lock");
                dispatch_storage_tool(
                    StorageWorkspaceConfig {
                        database_path,
                        diagnostics_dir,
                    },
                    &mut selection,
                    name,
                    arguments,
                )
            }
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
                Ok(json_rpc_result(
                    request.id.clone(),
                    tool_success(outcome.value),
                ))
            }
            Err(error) => Ok(json_rpc_result(
                request.id.clone(),
                tool_error(name, error.code, error.message, error.data),
            )),
        }
    }
}
