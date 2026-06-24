use serde_json::{json, Value};

pub(crate) fn tool_definitions() -> Vec<Value> {
    vec![
        tool_schema(
            "list_instances",
            "List PrismLauncher instances from the active Launcher Root.",
            json!({}),
            vec![],
        ),
        tool_schema(
            "list_schemes",
            "List schemes inside a PrismLauncher instance.",
            json!({ "instanceId": integer_schema("Prism instance id.") }),
            vec!["instanceId"],
        ),
        tool_schema(
            "create_scheme",
            "Create a scheme in a ready PrismLauncher instance.",
            json!({
                "instanceId": integer_schema("Prism instance id."),
                "name": string_schema("Scheme name."),
                "dimensions": vector3_schema("Scheme dimensions [x, y, z].")
            }),
            vec!["instanceId", "name", "dimensions"],
        ),
        tool_schema(
            "rename_scheme",
            "Rename a scheme.",
            json!({
                "schemeId": integer_schema("Scheme id."),
                "name": string_schema("New scheme name.")
            }),
            vec!["schemeId", "name"],
        ),
        tool_schema(
            "delete_scheme",
            "Delete a scheme.",
            json!({ "schemeId": integer_schema("Scheme id.") }),
            vec!["schemeId"],
        ),
        tool_schema(
            "read_scheme_content",
            "Read dimensions, stages, blocks, and materials for a scheme.",
            json!({ "schemeId": integer_schema("Scheme id.") }),
            vec!["schemeId"],
        ),
        tool_schema(
            "read_current_selection",
            "Read the user's current viewer selection.",
            json!({}),
            vec![],
        ),
        tool_schema(
            "summarize_scheme",
            "Read a compact scheme overview: dimensions, filled bounds, stages, stage block counts, and top materials.",
            json!({ "schemeId": integer_schema("Scheme id.") }),
            vec!["schemeId"],
        ),
        tool_schema(
            "search_blocks",
            "Search block ids and display names in a ready Prism instance registry before placing blocks.",
            json!({
                "instanceId": integer_schema("Prism instance id."),
                "query": string_schema("Case-insensitive block id or display-name search text."),
                "limit": optional_integer_schema("Maximum results to return. Defaults to 20.")
            }),
            vec!["instanceId", "query"],
        ),
        tool_schema(
            "get_block_definition",
            "Inspect one block id, including valid block states and model variants for orientation-sensitive blocks.",
            json!({
                "instanceId": integer_schema("Prism instance id."),
                "blockId": string_schema("Minecraft block identifier, for example minecraft:furnace.")
            }),
            vec!["instanceId", "blockId"],
        ),
        tool_schema(
            "place_block",
            "Place or replace one block.",
            json!({
                "schemeId": integer_schema("Scheme id."),
                "coordinate": vector3_schema("Block coordinate [x, y, z]."),
                "block": block_schema(),
                "responseMode": response_mode_schema()
            }),
            vec!["schemeId", "coordinate", "block"],
        ),
        tool_schema(
            "delete_block",
            "Delete one block.",
            json!({
                "schemeId": integer_schema("Scheme id."),
                "coordinate": vector3_schema("Block coordinate [x, y, z]."),
                "responseMode": response_mode_schema()
            }),
            vec!["schemeId", "coordinate"],
        ),
        tool_schema(
            "replace_blocks",
            "Replace all blocks with a matching block id.",
            json!({
                "schemeId": integer_schema("Scheme id."),
                "fromBlockId": string_schema("Existing block identifier to replace."),
                "to": block_schema(),
                "responseMode": response_mode_schema()
            }),
            vec!["schemeId", "fromBlockId", "to"],
        ),
        tool_schema(
            "bulk_set_area",
            "Set every block in a rectangular area.",
            json!({
                "schemeId": integer_schema("Scheme id."),
                "from": vector3_schema("First area corner [x, y, z]."),
                "to": vector3_schema("Opposite area corner [x, y, z]."),
                "block": block_schema(),
                "responseMode": response_mode_schema()
            }),
            vec!["schemeId", "from", "to", "block"],
        ),
        tool_schema(
            "apply_mutations",
            "Apply an ordered array of place/delete/replace/bulk/stage-assignment mutations transactionally.",
            json!({
                "schemeId": integer_schema("Scheme id."),
                "mutations": {
                    "type": "array",
                    "description": "Ordered mutations. If any mutation is invalid, none are saved.",
                    "items": mutation_schema(),
                    "minItems": 1
                },
                "responseMode": response_mode_schema()
            }),
            vec!["schemeId", "mutations"],
        ),
        tool_schema(
            "resize_scheme",
            "Resize the scheme if no existing blocks would be dropped.",
            json!({
                "schemeId": integer_schema("Scheme id."),
                "dimensions": vector3_schema("New dimensions [x, y, z]."),
                "responseMode": response_mode_schema()
            }),
            vec!["schemeId", "dimensions"],
        ),
        tool_schema(
            "create_stage",
            "Create a construction stage.",
            json!({
                "schemeId": integer_schema("Scheme id."),
                "name": string_schema("Stage name.")
            }),
            vec!["schemeId", "name"],
        ),
        tool_schema(
            "rename_stage",
            "Rename a construction stage.",
            json!({
                "schemeId": integer_schema("Scheme id."),
                "stageId": integer_schema("Construction stage id."),
                "name": string_schema("New stage name.")
            }),
            vec!["schemeId", "stageId", "name"],
        ),
        tool_schema(
            "assign_blocks_to_stage",
            "Assign all blocks in a rectangular area to a construction stage.",
            json!({
                "schemeId": integer_schema("Scheme id."),
                "from": vector3_schema("First area corner [x, y, z]."),
                "to": vector3_schema("Opposite area corner [x, y, z]."),
                "stageId": nullable_integer_schema("Construction stage id, or null for Unassigned."),
                "responseMode": response_mode_schema()
            }),
            vec!["schemeId", "from", "to", "stageId"],
        ),
        tool_schema(
            "validate_scheme",
            "Run technical validation for a scheme.",
            json!({ "schemeId": integer_schema("Scheme id.") }),
            vec!["schemeId"],
        ),
        tool_schema(
            "get_materials",
            "Get material counts for a scheme, enriched with local Prism registry names, item ids, texture paths, and stack counts when authoritative max stack sizes are available.",
            json!({ "schemeId": integer_schema("Scheme id.") }),
            vec!["schemeId"],
        ),
        tool_schema(
            "export_scheme",
            "Export a scheme to a selected desktop file path.",
            json!({
                "schemeId": integer_schema("Scheme id."),
                "format": {
                    "type": "string",
                    "enum": ["schem", "litematic"],
                    "description": "Export format."
                },
                "destinationPath": string_schema("Absolute destination file path chosen by the user.")
            }),
            vec!["schemeId", "format", "destinationPath"],
        ),
    ]
}

fn tool_schema(name: &str, description: &str, properties: Value, required: Vec<&str>) -> Value {
    json!({
        "name": name,
        "description": description,
        "inputSchema": {
            "type": "object",
            "properties": properties,
            "required": required,
            "additionalProperties": false
        }
    })
}

fn string_schema(description: &str) -> Value {
    json!({ "type": "string", "description": description })
}

fn integer_schema(description: &str) -> Value {
    json!({ "type": "integer", "description": description })
}

fn optional_integer_schema(description: &str) -> Value {
    integer_schema(description)
}

fn nullable_integer_schema(description: &str) -> Value {
    json!({
        "description": description,
        "anyOf": [
            { "type": "integer" },
            { "type": "null" }
        ]
    })
}

fn vector3_schema(description: &str) -> Value {
    json!({
        "type": "array",
        "description": description,
        "items": { "type": "integer" },
        "minItems": 3,
        "maxItems": 3
    })
}

fn block_schema() -> Value {
    json!({
        "type": "object",
        "description": "Block placement payload.",
        "properties": {
            "blockId": string_schema("Minecraft block identifier, for example minecraft:stone_bricks."),
            "states": {
                "type": "object",
                "description": "Block state key/value pairs.",
                "additionalProperties": { "type": "string" }
            },
            "stageId": nullable_integer_schema("Construction stage id, or null for Unassigned.")
        },
        "required": ["blockId", "states", "stageId"],
        "additionalProperties": false
    })
}

fn response_mode_schema() -> Value {
    json!({
        "type": "string",
        "enum": ["content", "summary"],
        "description": "Return full scheme content, or a compact mutation summary. Defaults to content."
    })
}

fn mutation_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "type": {
                "type": "string",
                "enum": ["placeBlock", "deleteBlock", "replaceBlocks", "bulkSetArea", "assignBlocksToStage", "resizeScheme"]
            },
            "coordinate": vector3_schema("Block coordinate [x, y, z] for placeBlock/deleteBlock."),
            "block": block_schema(),
            "fromBlockId": string_schema("Existing block identifier to replace."),
            "toBlock": block_schema(),
            "from": vector3_schema("First area corner [x, y, z]."),
            "to": vector3_schema("Opposite area corner [x, y, z]."),
            "stageId": nullable_integer_schema("Construction stage id, or null for Unassigned."),
            "dimensions": vector3_schema("New dimensions [x, y, z].")
        },
        "required": ["type"],
        "additionalProperties": false
    })
}
