//! MCP-compatible AI tool integration for Minecraft Pack Builder.

mod http;
mod protocol;
mod server;
mod tool_schemas;
mod tools;
mod workspace;

pub use http::{start_streamable_http_server, McpHttpServerHandle};
pub(crate) use protocol::json_rpc_error;
pub use protocol::{
    AgentError, AgentEvent, AgentStatus, ClientIdentity, JsonRpcOutcome, JsonRpcRequest,
};
pub use server::AgentServer;
pub use workspace::AgentWorkspace;

pub const MCP_PROTOCOL_VERSION: &str = "2025-06-18";
pub const DEFAULT_MCP_PORT: u16 = 47392;
pub(crate) const MCP_TRANSPORT: &str = "streamable-http";
const MCP_ENDPOINT_PATH: &str = "/mcp";

pub fn default_mcp_endpoint() -> String {
    format!("http://127.0.0.1:{DEFAULT_MCP_PORT}{MCP_ENDPOINT_PATH}")
}
