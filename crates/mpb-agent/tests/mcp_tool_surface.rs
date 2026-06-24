use mpb_agent::{
    default_mcp_endpoint, AgentServer, ClientIdentity, JsonRpcOutcome, JsonRpcRequest,
};
use serde_json::json;

#[test]
fn default_mcp_endpoint_is_stable_across_app_restarts() {
    assert_eq!(default_mcp_endpoint(), "http://127.0.0.1:47392/mcp");
}

#[test]
fn initializes_mcp_server_and_lists_full_phase_9_tool_surface() {
    let server = AgentServer::new_demo();
    let initialize = server
        .handle(JsonRpcRequest::new(
            1,
            "initialize",
            json!({
                "protocolVersion": "2025-06-18",
                "clientInfo": { "name": "Codex", "version": "test" },
                "capabilities": {}
            }),
            ClientIdentity::new("127.0.0.1", None),
        ))
        .expect("initialize response");

    assert_eq!(initialize["jsonrpc"], "2.0");
    assert_eq!(initialize["result"]["protocolVersion"], "2025-06-18");
    assert_eq!(initialize["result"]["serverInfo"]["name"], "minecraft-pack-builder");
    assert_eq!(
        initialize["result"]["capabilities"]["tools"]["listChanged"],
        true
    );

    let tools = server
        .handle(JsonRpcRequest::new(
            2,
            "tools/list",
            json!({}),
            ClientIdentity::new("127.0.0.1", Some("Codex")),
        ))
        .expect("tools list response");
    let names = tools["result"]["tools"]
        .as_array()
        .expect("tools")
        .iter()
        .map(|tool| tool["name"].as_str().expect("tool name"))
        .collect::<Vec<_>>();

    assert_eq!(
        names,
        vec![
            "list_imported_modpacks",
            "add_modpack",
            "list_schemes",
            "create_scheme",
            "rename_scheme",
            "delete_scheme",
            "read_scheme_content",
            "read_current_selection",
            "place_block",
            "delete_block",
            "replace_blocks",
            "bulk_set_area",
            "resize_scheme",
            "create_stage",
            "rename_stage",
            "assign_blocks_to_stage",
            "validate_scheme",
            "get_materials",
            "export_scheme",
        ]
    );
    assert!(tools["result"]["tools"][0]["inputSchema"].is_object());
}

#[test]
fn tool_input_schemas_define_every_required_property() {
    let tools = list_tools();
    for tool in &tools {
        let name = tool["name"].as_str().expect("tool name");
        let schema = &tool["inputSchema"];
        let properties = schema["properties"]
            .as_object()
            .unwrap_or_else(|| panic!("{name} properties must be an object"));
        let required = schema["required"]
            .as_array()
            .unwrap_or_else(|| panic!("{name} required must be an array"));

        for field in required {
            let field = field.as_str().expect("required field name");
            assert!(
                properties.contains_key(field),
                "{name} requires {field}, but inputSchema.properties does not define it"
            );
        }
    }

    let add_modpack = tools
        .iter()
        .find(|tool| tool["name"] == "add_modpack")
        .expect("add_modpack tool");
    assert_eq!(
        add_modpack["inputSchema"]["properties"]["pageUrl"]["type"],
        "string"
    );
    assert_eq!(
        add_modpack["inputSchema"]["properties"]["fileId"]["type"],
        "integer"
    );
}

#[test]
fn tool_input_schemas_use_any_of_for_nullable_fields() {
    let tools = list_tools();
    for tool in &tools {
        assert_schema_has_no_type_arrays(&tool["inputSchema"]);
    }

    let place_block = tools
        .iter()
        .find(|tool| tool["name"] == "place_block")
        .expect("place_block tool");
    let stage_id = &place_block["inputSchema"]["properties"]["block"]["properties"]["stageId"];
    assert_eq!(stage_id["anyOf"][0]["type"], "integer");
    assert_eq!(stage_id["anyOf"][1]["type"], "null");
}

#[test]
fn accepts_json_rpc_notifications_without_returning_a_response() {
    let server = AgentServer::new_demo();
    let outcome = server
        .handle_json_message(
            r#"{"jsonrpc":"2.0","method":"notifications/initialized","params":{}}"#,
            ClientIdentity::new("127.0.0.1", Some("Codex")),
        )
        .expect("notification accepted");

    assert_eq!(outcome, JsonRpcOutcome::Accepted);
}

#[test]
fn accepts_json_rpc_responses_without_returning_a_response() {
    let server = AgentServer::new_demo();
    let outcome = server
        .handle_json_message(
            r#"{"jsonrpc":"2.0","id":7,"result":{"ok":true}}"#,
            ClientIdentity::new("127.0.0.1", Some("Codex")),
        )
        .expect("response accepted");

    assert_eq!(outcome, JsonRpcOutcome::Accepted);
}

#[test]
fn active_client_can_be_released_without_restarting_the_app() {
    let server = AgentServer::new_demo();
    server
        .handle(JsonRpcRequest::new(
            1,
            "initialize",
            json!({
                "protocolVersion": "2025-06-18",
                "clientInfo": { "name": "Codex", "version": "test" },
                "capabilities": {}
            }),
            ClientIdentity::new("127.0.0.1", None),
        ))
        .expect("first client initializes");
    assert_eq!(server.status().active_client.as_deref(), Some("Codex"));

    server.clear_active_client();

    server
        .handle(JsonRpcRequest::new(
            2,
            "initialize",
            json!({
                "protocolVersion": "2025-06-18",
                "clientInfo": { "name": "Claude Code", "version": "test" },
                "capabilities": {}
            }),
            ClientIdentity::new("127.0.0.1", None),
        ))
        .expect("second client initializes after release");
    assert_eq!(server.status().active_client.as_deref(), Some("Claude Code"));
}

#[test]
fn allows_only_one_active_external_client() {
    let server = AgentServer::new_demo();
    server
        .handle(JsonRpcRequest::new(
            1,
            "initialize",
            json!({
                "protocolVersion": "2025-06-18",
                "clientInfo": { "name": "Codex", "version": "test" },
                "capabilities": {}
            }),
            ClientIdentity::new("127.0.0.1", None),
        ))
        .expect("first client initializes");

    let rejected = server
        .handle(JsonRpcRequest::new(
            2,
            "initialize",
            json!({
                "protocolVersion": "2025-06-18",
                "clientInfo": { "name": "Claude Code", "version": "test" },
                "capabilities": {}
            }),
            ClientIdentity::new("127.0.0.1", None),
        ))
        .expect("second client receives json-rpc error");

    assert_eq!(rejected["error"]["code"], -32002);
    assert_eq!(
        rejected["error"]["data"]["code"],
        "active_client_already_connected"
    );
    assert_eq!(
        server.status().active_client.as_deref(),
        Some("Codex")
    );
}

#[test]
fn rejects_invalid_bulk_mutation_atomically_with_structured_error() {
    let server = AgentServer::new_demo();
    server
        .handle(JsonRpcRequest::new(
            1,
            "initialize",
            json!({
                "protocolVersion": "2025-06-18",
                "clientInfo": { "name": "opencode", "version": "test" },
                "capabilities": {}
            }),
            ClientIdentity::new("127.0.0.1", None),
        ))
        .expect("initialize");

    let before = call_tool(
        &server,
        2,
        "read_scheme_content",
        json!({ "schemeId": 10 }),
    );
    let before_block_count = before["structuredContent"]["blockCount"]
        .as_u64()
        .expect("before block count");

    let rejected = call_tool(
        &server,
        3,
        "bulk_set_area",
        json!({
            "schemeId": 10,
            "from": [0, 0, 0],
            "to": [99, 0, 0],
            "block": {
                "blockId": "minecraft:stone_bricks",
                "states": { "cracked": "false" },
                "stageId": null
            }
        }),
    );

    assert_eq!(rejected["isError"], true);
    assert_eq!(
        rejected["structuredContent"]["error"]["code"],
        "coordinate_out_of_bounds"
    );

    let after = call_tool(
        &server,
        4,
        "read_scheme_content",
        json!({ "schemeId": 10 }),
    );
    assert_eq!(
        after["structuredContent"]["blockCount"].as_u64(),
        Some(before_block_count)
    );
}

fn list_tools() -> Vec<serde_json::Value> {
    let server = AgentServer::new_demo();
    let response = server
        .handle(JsonRpcRequest::new(
            1,
            "tools/list",
            json!({}),
            ClientIdentity::new("127.0.0.1", Some("Codex")),
        ))
        .expect("tools list response");
    response["result"]["tools"]
        .as_array()
        .expect("tools")
        .clone()
}

fn assert_schema_has_no_type_arrays(value: &serde_json::Value) {
    match value {
        serde_json::Value::Object(object) => {
            if let Some(schema_type) = object.get("type") {
                assert!(
                    !schema_type.is_array(),
                    "schema type arrays are not used; prefer anyOf for nullable fields: {value}"
                );
            }
            for child in object.values() {
                assert_schema_has_no_type_arrays(child);
            }
        }
        serde_json::Value::Array(items) => {
            for item in items {
                assert_schema_has_no_type_arrays(item);
            }
        }
        _ => {}
    }
}

fn call_tool(
    server: &AgentServer,
    id: i64,
    name: &str,
    arguments: serde_json::Value,
) -> serde_json::Value {
    let response = server
        .handle(JsonRpcRequest::new(
            id,
            "tools/call",
            json!({ "name": name, "arguments": arguments }),
            ClientIdentity::new("127.0.0.1", Some("Codex")),
        ))
        .expect("tool call response");
    assert_eq!(response["jsonrpc"], "2.0");
    response["result"].clone()
}
