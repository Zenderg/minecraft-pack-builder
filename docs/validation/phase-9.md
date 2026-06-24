# Phase 9 Validation: MCP-Compatible AI Integration And Tool Surface

## Openable Artifacts

- Contract report: `docs/validation/phase-9-mcp-contract.json`
- AI integration settings screen in the desktop app.
- Stable MCP endpoint shown by the desktop app through the AI integration settings screen: `http://127.0.0.1:47392/mcp`.

## What Changed

- The Tauri app starts a local MCP-compatible Streamable HTTP endpoint at `http://127.0.0.1:47392/mcp` during application setup.
- The main window shows AI server/client status instead of a fixed disconnected label.
- AI integration settings show the server status, active client, endpoint, protocol version, and tool count.
- The settings prompt includes the stable MCP endpoint, asks capable agents to add it as `minecraft-pack-builder`, tells them to explain any required restart/reload, verifies tool availability after connection, and asks them to answer in the current app interface language.
- `mpb-agent` implements JSON-RPC handling for `initialize`, `notifications/initialized`, `ping`, `tools/list`, and `tools/call`.
- The server allows exactly one active external client at a time.
- Mutating scheme tools route through `mpb-core` validation and reject invalid commands atomically with structured errors.
- Successful agent mutations emit `ai_agent_event` for the frontend to refresh visible state.

## Tool Surface

The phase 9 MCP tool list contains:

- `list_imported_modpacks`
- `add_modpack`
- `list_schemes`
- `create_scheme`
- `rename_scheme`
- `delete_scheme`
- `read_scheme_content`
- `read_current_selection`
- `place_block`
- `delete_block`
- `replace_blocks`
- `bulk_set_area`
- `resize_scheme`
- `create_stage`
- `rename_stage`
- `assign_blocks_to_stage`
- `validate_scheme`
- `get_materials`
- `export_scheme`

`export_scheme` is present as part of the phase 9 tool surface. Actual `.schem` and `.litematic` file writing remains phase 10 according to the implementation plan.

## Validation Commands

Run from the repository root:

```bash
cargo test --workspace
pnpm test
pnpm build
```

## User Checklist

- [ ] Open the desktop app and go to Settings -> AI integration.
- [ ] The AI integration screen shows a running local MCP server endpoint.
- [ ] The main window status says the AI server is running when no client is connected.
- [ ] Connect one external MCP client to the shown endpoint.
- [ ] The active client name becomes visible after `initialize`.
- [ ] A second different client is rejected while the first client is active.
- [ ] The external client can list tools and see the full tool surface above.
- [ ] The external client can create or mutate a scheme through tools.
- [ ] Invalid area mutations are rejected with a structured error and do not partially change the scheme.
- [ ] The viewer refreshes after successful agent mutation events.

## Automated Evidence

- `crates/mpb-agent/tests/mcp_tool_surface.rs` checks initialize/tool listing, single-client rejection, and atomic invalid bulk mutation behavior.
- `cargo test --workspace` passed.
- `pnpm test` passed.
- `pnpm build` passed.
