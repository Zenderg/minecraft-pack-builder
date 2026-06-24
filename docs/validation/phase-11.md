# Phase 11 Validation: Error Handling, Diagnostics, And Data Integrity

## Openable Artifacts

- Diagnostics index: `docs/validation/phase-11-diagnostics-index.json`
- Runtime import diagnostics pattern: app data `diagnostics/<modpack-slug>-<file-id>-assets.json`
- Runtime export diagnostics pattern: app data `diagnostics/export-scheme-<scheme-id>-<format>.json`
- MCP validation diagnostic: `validate_scheme` response `structuredContent.diagnostic`
- MCP AI rejection diagnostic: failed `tools/call` response `structuredContent.diagnostic`

## What Changed

- Backend storage rejects local databases with a future migration version and gives a recovery-oriented message.
- Scheme creation is persisted atomically across the scheme row, dimensions row, and initial `Unassigned` stage row.
- Export writes a structured diagnostic report for both successful and failed desktop exports.
- MCP validation responses include structured validation diagnostics.
- Rejected MCP tool calls include structured AI operation diagnostics with a recovery message.
- Frontend error formatting removes JavaScript-only `Error:` noise and can surface structured message, recovery action, and diagnostic report path.

## User Validation Checklist

- [ ] Open Settings and confirm the app data/diagnostics folder location is visible.
- [ ] Export a demo scheme and confirm `export-scheme-<scheme-id>-<format>.json` appears in the diagnostics folder.
- [ ] Try exporting to an unwritable or missing destination folder and confirm the app keeps running and shows a recovery-oriented error.
- [ ] Ask a connected MCP client to run `validate_scheme` and confirm the response contains `operation: validation`.
- [ ] Ask a connected MCP client to make an invalid out-of-bounds edit and confirm the response contains `operation: ai_tool_call` and a recovery message.
- [ ] Confirm the sidebar does not show a half-created scheme after a failed scheme persistence operation.

## Engineering Validation

- `cargo test -p mpb-storage`
- `cargo test -p mpb-agent --test mcp_tool_surface`
- `cargo test -p app-tauri --test export_scheme`
- `pnpm test src/backendErrors.test.ts src/App.importModal.test.tsx src/App.keyCheck.test.tsx src/App.viewer.test.tsx`
- Full workspace verification before closing the phase.
