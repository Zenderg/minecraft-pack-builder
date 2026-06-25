# MPB Patcher And Minecraft Mod Pivot Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Rebuild Minecraft Pack Builder around a PrismLauncher patcher and an instance-local Minecraft mod/runtime contract.

**Architecture:** Keep the Tauri shell as the cross-platform GUI patcher, but move product state out of the app-global SQLite library and into each Prism instance under `mpb/`. Keep Rust crates as the shared implementation boundary: `mpb-core` owns scheme semantics, `mpb-storage` owns atomic instance files, `mpb-assets` owns Prism discovery and patch application, and the React UI becomes a patcher dashboard.

**Tech Stack:** Rust 2021 workspace, Tauri 2, React 19, TypeScript, PrismLauncher instance metadata, JSON scheme/manifest files, Streamable HTTP MCP contract.

---

### Task 1: Sparse Scheme Domain

**Files:**
- Modify: `crates/mpb-core/src/lib.rs`
- Modify: `crates/mpb-core/tests/scheme_domain.rs`

- [ ] Write failing tests that `Scheme::new` has no fixed dimensions, rejects negative coordinates, computes bounds/dimensions from stored blocks, and treats incomplete stages as a single build stage.
- [ ] Run `cargo test -p mpb-core scheme_domain`.
- [ ] Replace fixed-dimension validation with sparse non-negative coordinate validation.
- [ ] Add computed `bounds()`, `computed_dimensions()`, and `stage_plan()` APIs.
- [ ] Run `cargo test -p mpb-core`.

### Task 2: Instance-Local Storage

**Files:**
- Modify: `crates/mpb-storage/src/lib.rs`
- Create: `crates/mpb-storage/tests/instance_storage.rs`

- [ ] Write failing tests for `mpb/config.json`, `mpb/schemes/*.mpb.json`, `mpb/cache/`, and atomic temp-file-plus-rename saves.
- [ ] Run `cargo test -p mpb-storage instance_storage`.
- [ ] Add `InstanceMpbLayout`, `SchemeFile`, and `InstanceSchemeRepository`.
- [ ] Preserve old SQLite repository code only as legacy support while new commands use instance files.
- [ ] Run `cargo test -p mpb-storage`.

### Task 3: Prism Patch Status And Manifest

**Files:**
- Modify: `crates/mpb-assets/src/prism.rs`
- Create: `crates/mpb-assets/src/patcher.rs`
- Modify: `crates/mpb-assets/src/lib.rs`
- Create: `crates/mpb-assets/tests/patcher.rs`

- [ ] Write failing tests for supported loaders, Minecraft `1.20+`, unsupported vanilla/unknown/old versions, managed manifest creation, dependency conflict detection, repair detection, and unpatch preserving preexisting files.
- [ ] Run `cargo test -p mpb-assets patcher`.
- [ ] Add patch status evaluation and manifest schema under `<instance>/mpb/patch-manifest.json`.
- [ ] Add apply/repair/remove operations that manage MPB-owned files and never remove preexisting dependencies.
- [ ] Run `cargo test -p mpb-assets`.

### Task 4: Tauri Patcher Commands

**Files:**
- Create: `src-tauri/src/patcher_commands.rs`
- Modify: `src-tauri/src/lib.rs`
- Create: `src-tauri/tests/patcher_commands.rs`

- [ ] Write failing tests for command-facing patch status summaries and operation progress output.
- [ ] Run `cargo test -p app-tauri patcher_commands`.
- [ ] Register commands for root discovery, root validation, instance listing, patch apply/repair/remove, and scheme deletion choice plumbing.
- [ ] Keep the MCP server startup out of the patcher process except where legacy tests still cover it.
- [ ] Run `cargo test -p app-tauri`.

### Task 5: React Patcher UI

**Files:**
- Replace/decompose: `src/App.tsx`
- Create: `src/patcher/*`
- Modify: `src/tauri.ts`
- Modify: `src/i18n.ts`
- Create: `src/patcher/patcherState.test.ts`

- [ ] Write failing tests for OS-locale language selection, instance status actions, progress state, and next-step text.
- [ ] Run `pnpm test src/patcher/patcherState.test.ts`.
- [ ] Replace the old desktop viewer/library UI with the patcher workflow from the spec.
- [ ] Remove viewer-only controls from the active app surface.
- [ ] Run `pnpm test` and `pnpm build`.

### Task 6: Minecraft Mod Contract Scaffold

**Files:**
- Create: `mods/mpb-minecraft-mod/README.md`
- Create: `mods/mpb-minecraft-mod/common/src/main/java/com/mpb/contract/MpbRuntimeContract.java`
- Create: `mods/mpb-minecraft-mod/fabric/src/main/resources/fabric.mod.json`
- Create: `mods/mpb-minecraft-mod/forge/src/main/resources/META-INF/mods.toml`
- Create: `mods/mpb-minecraft-mod/neoforge/src/main/resources/META-INF/neoforge.mods.toml`

- [ ] Add a durable multi-loader contract scaffold documenting client-only startup, `/mcp`, `/mpb`, unbound keybindings, instance-local paths, LAN mode, and no server-side placement.
- [ ] Add patcher artifact compatibility metadata so unsupported combinations can be reported without guessing.
- [ ] Record remaining Minecraft runtime implementation risks in project docs instead of leaving them implicit.

### Task 7: Validation And Cleanup

**Files:**
- Modify: `README.md`
- Create or modify: `docs/validation/2026-06-25-patcher-mod-pivot.md`

- [ ] Remove active references to the old desktop 3D viewer/global SQLite library as product core.
- [ ] Run Rust, TypeScript, and Tauri checks that are available locally.
- [ ] Validate against a local PrismLauncher root when present.
- [ ] Record what passed, what could not be launched automatically, and what still requires a real Minecraft mod run.
