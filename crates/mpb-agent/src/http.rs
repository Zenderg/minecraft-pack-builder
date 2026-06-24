use std::collections::BTreeMap;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{IpAddr, SocketAddr, TcpListener, TcpStream};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use serde_json::{json, Value};

use crate::{
    default_mcp_endpoint, json_rpc_error, AgentError, AgentEvent, AgentServer, ClientIdentity,
    JsonRpcOutcome, MCP_ENDPOINT_PATH, MCP_PROTOCOL_VERSION, DEFAULT_MCP_PORT,
};

pub struct McpHttpServerHandle {
    endpoint: String,
    shutdown: Arc<AtomicBool>,
    join: Option<JoinHandle<()>>,
}

impl McpHttpServerHandle {
    pub fn endpoint(&self) -> &str {
        &self.endpoint
    }

    pub fn stop(mut self) {
        self.shutdown.store(true, Ordering::SeqCst);
        let _ = TcpStream::connect(self.endpoint.replace("http://", "").replace(MCP_ENDPOINT_PATH, ""));
        if let Some(join) = self.join.take() {
            let _ = join.join();
        }
    }
}

impl Drop for McpHttpServerHandle {
    fn drop(&mut self) {
        self.shutdown.store(true, Ordering::SeqCst);
    }
}

pub fn start_streamable_http_server(
    server: AgentServer,
    on_events: impl Fn(Vec<AgentEvent>) + Send + Sync + 'static,
) -> Result<McpHttpServerHandle, AgentError> {
    let listener = TcpListener::bind((IpAddr::from([127, 0, 0, 1]), DEFAULT_MCP_PORT))
        .map_err(|error| {
            AgentError::Http(format!(
                "Could not bind MCP server at {}: {error}",
                default_mcp_endpoint()
            ))
        })?;
    listener
        .set_nonblocking(true)
        .map_err(|error| AgentError::Http(format!("Could not configure MCP listener: {error}")))?;
    let local_addr = listener
        .local_addr()
        .map_err(|error| AgentError::Http(format!("Could not read MCP listener address: {error}")))?;
    let endpoint = format!("http://{local_addr}{MCP_ENDPOINT_PATH}");
    let shutdown = Arc::new(AtomicBool::new(false));
    let thread_shutdown = shutdown.clone();
    let thread_endpoint = endpoint.clone();
    let events = Arc::new(on_events);
    server.set_server_endpoint(Some(endpoint.clone()));
    let thread_server = server.clone();

    let join = thread::spawn(move || {
        while !thread_shutdown.load(Ordering::SeqCst) {
            match listener.accept() {
                Ok((stream, addr)) => {
                    let server = thread_server.clone();
                    let events = events.clone();
                    thread::spawn(move || {
                        if let Err(error) = handle_http_stream(stream, addr, &server) {
                            eprintln!("MCP HTTP request failed: {error}");
                        }
                        let pending = server.drain_events();
                        if !pending.is_empty() {
                            events(pending);
                        }
                    });
                }
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                    thread::sleep(Duration::from_millis(20));
                }
                Err(error) => {
                    eprintln!("MCP listener at {thread_endpoint} failed: {error}");
                    break;
                }
            }
        }
    });

    Ok(McpHttpServerHandle {
        endpoint,
        shutdown,
        join: Some(join),
    })
}

fn handle_http_stream(
    mut stream: TcpStream,
    addr: SocketAddr,
    server: &AgentServer,
) -> Result<(), AgentError> {
    let mut reader = BufReader::new(stream.try_clone().map_err(|error| {
        AgentError::Http(format!("Could not clone MCP HTTP stream: {error}"))
    })?);
    let mut request_line = String::new();
    reader
        .read_line(&mut request_line)
        .map_err(|error| AgentError::Http(format!("Could not read request line: {error}")))?;
    let parts = request_line.split_whitespace().collect::<Vec<_>>();
    if parts.len() < 2 {
        write_http_json(
            &mut stream,
            400,
            json_rpc_error(Value::Null, -32600, "Invalid HTTP request", "invalid_http_request", json!({})),
        )?;
        return Ok(());
    }
    let method = parts[0];
    let path = parts[1];
    let mut content_length = 0usize;
    let mut origin: Option<String> = None;
    let mut headers = BTreeMap::<String, String>::new();

    loop {
        let mut header = String::new();
        reader
            .read_line(&mut header)
            .map_err(|error| AgentError::Http(format!("Could not read header: {error}")))?;
        let trimmed = header.trim_end();
        if trimmed.is_empty() {
            break;
        }
        if let Some((name, value)) = trimmed.split_once(':') {
            headers.insert(name.to_ascii_lowercase(), value.trim().to_string());
            if name.eq_ignore_ascii_case("content-length") {
                content_length = value.trim().parse().unwrap_or(0);
            }
            if name.eq_ignore_ascii_case("origin") {
                origin = Some(value.trim().to_string());
            }
        }
    }

    if path != MCP_ENDPOINT_PATH {
        write_http_json(
            &mut stream,
            404,
            json_rpc_error(Value::Null, -32601, "MCP endpoint not found", "endpoint_not_found", json!({})),
        )?;
        return Ok(());
    }

    if !is_allowed_origin(origin.as_deref()) {
        write_http_json(
            &mut stream,
            403,
            json_rpc_error(Value::Null, -32010, "Origin is not allowed", "origin_not_allowed", json!({ "origin": origin })),
        )?;
        return Ok(());
    }

    if method == "GET" {
        write_http_empty(&mut stream, 405, &[("Allow", "POST, DELETE")])?;
        return Ok(());
    }

    if method == "DELETE" {
        server.clear_active_client();
        write_http_empty(&mut stream, 202, &[])?;
        return Ok(());
    }

    if method != "POST" {
        write_http_empty(&mut stream, 405, &[("Allow", "POST, DELETE")])?;
        return Ok(());
    }

    if !accept_supports_streamable_http(headers.get("accept").map(String::as_str)) {
        write_http_json(
            &mut stream,
            406,
            json_rpc_error(
                Value::Null,
                -32020,
                "MCP Streamable HTTP requests must accept application/json and text/event-stream",
                "unsupported_accept_header",
                json!({ "accept": headers.get("accept") }),
            ),
        )?;
        return Ok(());
    }

    if let Some(protocol_version) = headers.get("mcp-protocol-version") {
        if protocol_version != MCP_PROTOCOL_VERSION {
            write_http_json(
                &mut stream,
                400,
                json_rpc_error(
                    Value::Null,
                    -32021,
                    "Unsupported MCP protocol version header",
                    "unsupported_protocol_version_header",
                    json!({
                        "requested": protocol_version,
                        "supported": MCP_PROTOCOL_VERSION
                    }),
                ),
            )?;
            return Ok(());
        }
    }

    if content_length == 0 || content_length > 1024 * 1024 {
        write_http_json(
            &mut stream,
            400,
            json_rpc_error(Value::Null, -32600, "Invalid MCP request body", "invalid_body", json!({ "contentLength": content_length })),
        )?;
        return Ok(());
    }

    let mut body = vec![0_u8; content_length];
    reader
        .read_exact(&mut body)
        .map_err(|error| AgentError::Http(format!("Could not read body: {error}")))?;
    let body = String::from_utf8(body)
        .map_err(|error| AgentError::Http(format!("MCP body must be UTF-8: {error}")))?;
    let identity = ClientIdentity::new(addr.ip().to_string(), None);
    let outcome = server.handle_json_message(&body, identity).unwrap_or_else(|error| {
        JsonRpcOutcome::Response(json_rpc_error(
            Value::Null,
            -32603,
            "Internal MCP server error",
            "internal_error",
            json!({ "message": error.to_string() }),
        ))
    });
    match outcome {
        JsonRpcOutcome::Accepted => write_http_empty(&mut stream, 202, &[]),
        JsonRpcOutcome::Response(response) => write_http_json(&mut stream, 200, response),
    }
}

fn accept_supports_streamable_http(accept: Option<&str>) -> bool {
    let Some(accept) = accept else {
        return false;
    };
    accept
        .split(',')
        .map(|value| value.split(';').next().unwrap_or("").trim())
        .any(|value| value == "application/json" || value == "*/*")
        && accept
            .split(',')
            .map(|value| value.split(';').next().unwrap_or("").trim())
            .any(|value| value == "text/event-stream" || value == "*/*")
}

fn write_http_json(stream: &mut TcpStream, status: u16, body: Value) -> Result<(), AgentError> {
    let body = serde_json::to_vec(&body)
        .map_err(|error| AgentError::Http(format!("Could not serialize response: {error}")))?;
    write!(
        stream,
        "HTTP/1.1 {status} {}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        http_reason_phrase(status),
        body.len()
    )
    .map_err(|error| AgentError::Http(format!("Could not write response headers: {error}")))?;
    stream
        .write_all(&body)
        .map_err(|error| AgentError::Http(format!("Could not write response body: {error}")))
}

fn write_http_empty(
    stream: &mut TcpStream,
    status: u16,
    headers: &[(&str, &str)],
) -> Result<(), AgentError> {
    write!(
        stream,
        "HTTP/1.1 {status} {}\r\nContent-Length: 0\r\n",
        http_reason_phrase(status)
    )
    .map_err(|error| AgentError::Http(format!("Could not write response: {error}")))?;
    for (name, value) in headers {
        write!(stream, "{name}: {value}\r\n")
            .map_err(|error| AgentError::Http(format!("Could not write response: {error}")))?;
    }
    write!(stream, "Connection: close\r\n\r\n")
        .map_err(|error| AgentError::Http(format!("Could not write response: {error}")))
}

fn http_reason_phrase(status: u16) -> &'static str {
    match status {
        200 => "OK",
        202 => "Accepted",
        400 => "Bad Request",
        403 => "Forbidden",
        404 => "Not Found",
        405 => "Method Not Allowed",
        406 => "Not Acceptable",
        _ => "OK",
    }
}

fn is_allowed_origin(origin: Option<&str>) -> bool {
    match origin {
        None => true,
        Some(value) => {
            value == "tauri://localhost"
                || value.starts_with("http://localhost")
                || value.starts_with("http://127.0.0.1")
                || value.starts_with("https://localhost")
        }
    }
}
