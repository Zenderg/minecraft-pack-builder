# Phase 10 Validation: Export To `.schem` And `.litematic`

## Artifacts

- Sponge/WorldEdit schematic: `docs/validation/phase-10-starter-factory.schem`
- Litematica schematic: `docs/validation/phase-10-starter-factory.litematic`

Both artifacts are generated from the same phase 7/10 demo scheme that the desktop viewer renders for `Starter Factory`.

Regenerate them with:

```bash
cargo run -p app-tauri --example export_phase_10
```

## Implemented Surface

- `mpb-export` writes gzip-compressed NBT for Sponge `.schem` and Litematica `.litematic`.
- Export includes all final blocks, including blocks assigned to `Unassigned`.
- MCP `export_scheme` now accepts `schemeId`, `format`, and `destinationPath`, then writes the selected file.
- Desktop UI exposes `Export .schem` and `Export .litematic` actions from the opened scheme workspace.
- The UI uses the native Tauri save dialog before calling the desktop export command.

## Automated Validation

- `cargo test`
- `pnpm test`
- `pnpm build`

Focused checks:

- `crates/mpb-export/tests/export_formats.rs` decodes both exported files back from gzip NBT and verifies dimensions, palette entries, and block payloads.
- `crates/mpb-agent/tests/mcp_tool_surface.rs` verifies MCP export writes a file to the requested destination.
- `src-tauri/tests/export_scheme.rs` verifies the desktop command path writes both demo export formats.
- `src/App.viewer.test.tsx` verifies the viewer export action opens the save flow and calls the backend export.

## Manual Validation

- Open `docs/validation/phase-10-starter-factory.schem` with a Sponge/WorldEdit-compatible schematic viewer or import command.
- Open `docs/validation/phase-10-starter-factory.litematic` with Litematica.
- Compare the loaded structure with the app viewer: the exported demo has the same 8 x 5 x 8 bounds and includes the unassigned casing/glass blocks.

Manual external-tool open testing is still pending on a Minecraft client installation with WorldEdit and Litematica available.
