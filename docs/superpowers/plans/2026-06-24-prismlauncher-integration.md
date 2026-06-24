# PrismLauncher Integration Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. This repository must stay on `main`; do not create a branch or worktree.

**Goal:** Replace app-owned CurseForge modpack import with one active PrismLauncher root, watcher-first sync, Prism-linked schemes, asset/runtime readiness, and updated UI/agent tools.

**Architecture:** PrismLauncher owns instances and runtime preparation; Minecraft Pack Builder stores only schemes, Prism root settings, instance linkage, fingerprints, and indexing state. Backend sync discovers instances, marks missing records safely, and gates viewer/agent mutations on `ready`.

**Tech Stack:** Rust workspace crates, SQLite via `rusqlite`, Tauri commands/events, React/TypeScript frontend, Vitest, Cargo tests.

---

### Task 1: Storage Prism Model

**Files:**
- Modify: `crates/mpb-storage/src/lib.rs`
- Modify: `crates/mpb-storage/tests/library_repositories.rs`

- [x] Replace old imported modpack schema with Prism settings, Prism instances, scheme documents, and index status tables.
- [x] Add repository APIs for setting Prism root, upserting Prism instances, marking missing instances, possible relink support, readiness checks, and scheme CRUD by instance.
- [x] Add migration behavior that drops old imported-modpack data from unreleased schemas.
- [x] Verify with storage tests.

### Task 2: Prism Discovery And Sync Core

**Files:**
- Create: `crates/mpb-assets/src/prism.rs` or a new `mpb-prism` module if boundaries demand it
- Modify: `crates/mpb-assets/src/lib.rs`
- Add tests under `crates/mpb-assets/tests/`

- [x] Implement Prism root default candidates, validation, instance parsing, metadata normalization, identity/content fingerprints, and possible-match scoring.
- [x] Implement local static asset scan entry points decoupled from CurseForge download.
- [x] Implement loader-aware runtime extraction adapters for NeoForge, Forge, and Fabric with cached authoritative stack sizes when local Prism artifacts allow it.
- [x] Add readiness orchestration that writes a real local registry report or marks the instance `failed` with an actionable message, without faking data.
- [x] Verify with parser/fingerprint tests.

### Task 3: Tauri Commands And Watcher-First Sync

**Files:**
- Modify: `src-tauri/src/lib.rs`
- Delete: `src-tauri/src/credentials.rs` if no longer referenced
- Modify: `src-tauri/Cargo.toml`
- Modify: `src-tauri/tests/*`

- [x] Remove CurseForge commands and import controller.
- [x] Add Prism commands: discover roots, validate/select root, list library, and confirm possible relink.
- [x] Start backend sync on app setup and after root selection.
- [x] Add watcher-first filesystem sync with debounced events.
- [x] Verify Tauri command tests.

### Task 4: Agent Tool Surface

**Files:**
- Modify: `crates/mpb-agent/src/tool_schemas.rs`
- Modify: `crates/mpb-agent/src/tools.rs`
- Modify: `crates/mpb-agent/src/workspace.rs`
- Modify: `crates/mpb-agent/tests/mcp_tool_surface.rs`

- [x] Replace imported-modpack naming with Prism instances.
- [x] Remove `add_modpack`.
- [x] Gate create/mutate tools on ready instance status.
- [x] Keep read/export for local scheme documents when possible.
- [x] Verify MCP tool tests.

### Task 5: Frontend Flow

**Files:**
- Modify: `src/App.tsx`
- Modify: `src/tauri.ts`
- Modify: `src/onboarding.ts`
- Modify: `src/i18n.ts`
- Modify: `src/app/*`
- Delete or stop using `src/importWizard.tsx` and `src/importWizard.css`
- Modify tests under `src/*.test.ts*`

- [x] Replace CurseForge onboarding/settings with Prism root detection and selection.
- [x] Remove Add Modpack and import dialogs.
- [x] Render Prism instances with ready/indexing/failed/missing statuses.
- [x] Block create/open/viewer editing for non-ready instances while keeping export/rename/delete safe actions.
- [x] Add possible-match confirmation modal.
- [x] Verify Vitest suite.

### Task 6: Cleanup, Build, Desktop Validation

**Files:**
- Modify: docs, configs, package metadata as needed

- [x] Remove unused CurseForge code, types, strings, tests, and dependencies.
- [x] Run `pnpm test`, `pnpm build`, and `cargo test --workspace`.
- [x] Run Tauri desktop build validation through the macOS app bundle; report that DMG/updater signing still requires local packaging credentials.
