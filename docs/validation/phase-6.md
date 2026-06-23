# Phase 6 Validation: Modpack Asset Parsing, Block Index, And Texture Preparation

Date: 2026-06-23

## Openable Artifacts

- Browser preview screenshot: `docs/validation/media/phase-6-asset-diagnostics.png`
- Browser preview command: `pnpm dev -- --port 5173`, then open `http://127.0.0.1:5173/`
- Real desktop import artifact after a successful import: app diagnostics folder, file named `<modpack-slug>-<curseforge-file-id>-assets.json`
- Real desktop cache after a successful import: app data folder under `modpacks/<modpack-slug>-<curseforge-file-id>/`

## What Was Built

- Modpack archives are extracted into managed cache directories.
- CurseForge `manifest.json` files are parsed for mod project/file IDs.
- Required mod `.jar` files are downloaded through the Rust backend and parsed from the managed cache.
- `assets/<namespace>/blockstates`, `models`, `textures`, and `lang` files are indexed.
- A local block registry, texture atlas metadata, and diagnostics JSON report are generated.
- Imports fail before adding a library entry when no parseable block assets can be produced.
- The workspace shows a modpack asset diagnostics preview with release, Minecraft version, loader, mod file count, block count, asset count, cache path, report path, and texture samples.
- Add Modpack now creates a library entry immediately with `importing` status. Download and asset parsing continue in the background, then the sidebar updates to `imported` or `failed` through the stored import status.

## Validation Checklist

- PASS: Synthetic fixture modpack downloads manifest-listed mod jars and builds a block registry.
- PASS: Synthetic fixture report includes block names, texture paths, atlas metadata, cache location, and report path.
- PASS: Empty/unparseable modpack archives are rejected instead of silently importing.
- PASS: Tauri import path creates an `importing` library record immediately, then updates it to `imported` or `failed` from background processing.
- PASS: Browser preview renders texture samples and diagnostics without console warnings or errors.
- PASS: Layout check confirms the diagnostics panel does not overflow the viewer region at the default desktop viewport.
- NOT RUN: Manual live AOC import, because it requires a real CurseForge API key and live CurseForge downloads in the desktop app.

## Verification Commands

```bash
cargo test --workspace
pnpm test
pnpm build
rustfmt --check crates/mpb-assets/src/lib.rs crates/mpb-assets/tests/modpack_asset_index.rs src-tauri/src/lib.rs crates/mpb-storage/src/lib.rs
```

## Browser QA Notes

- URL: `http://127.0.0.1:5173/`
- Flow tested: app loads -> imported AOC fixture is selected -> modpack asset diagnostics renders -> selecting another imported modpack keeps diagnostics visible.
- Console health: no warnings or errors.
- Screenshot: `docs/validation/media/phase-6-asset-diagnostics.png`
