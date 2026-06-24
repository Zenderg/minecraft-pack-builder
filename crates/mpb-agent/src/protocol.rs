use serde::Serialize;
use serde_json::{json, Value};
use thiserror::Error;

use crate::MCP_PROTOCOL_VERSION;

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
    pub(crate) id: Value,
    pub(crate) method: String,
    pub(crate) params: Value,
    pub(crate) identity: ClientIdentity,
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

    pub(crate) fn from_value(value: Value, identity: ClientIdentity) -> Result<Self, AgentError> {
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

pub(crate) fn json_rpc_result(id: Value, result: Value) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "result": result })
}

pub(crate) fn json_rpc_error(
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

pub(crate) fn initialize_result(id: Value) -> Value {
    json_rpc_result(
        id,
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
    )
}
