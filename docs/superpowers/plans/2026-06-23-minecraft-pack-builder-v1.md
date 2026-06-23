# Minecraft Pack Builder V1 Implementation Plan

> **Repository workflow:** implement phases in the current main branch unless the user explicitly asks for a different branch or worktree.

**Goal:** Build the complete local-first v1 desktop application described in the product and technical specifications: CurseForge modpack import, local library, scheme viewing and review, external AI tool integration, validation, and export.

**Architecture:** Tauri hosts a cross-platform desktop app. Rust owns durable state, domain validation, storage, import, asset indexing, render preparation, exports, and MCP-compatible tools. React, TypeScript, Three.js, Radix UI, and Tailwind CSS own the user-facing workspace, onboarding, settings, panels, and 3D interaction.

**Tech Stack:** Tauri, Rust, SQLite, TypeScript, React, Vite, Three.js/WebGL2, Radix UI, Tailwind CSS, OS secure credential storage, MCP-compatible local server, CurseForge API, `.schem`, `.litematic`.

---

## Planning Decisions

- Scope: full v1, not an MVP track.
- First manual validation platform: macOS.
- v1 target platforms: macOS, Windows, Linux.
- Real CurseForge manual validation modpack: `https://www.curseforge.com/minecraft/modpacks/aoc`.
- Every phase must produce an openable artifact the user can inspect directly.
- A phase is not closed by tests alone. It closes only when the manual artifact exists and the validation checklist for that phase is passable.

## Visual Validation Contract

Each phase must end with:

- an app build, local page, exported file, fixture, report, or packaged artifact the user can open;
- a short `docs/validation/phase-N.md` note with the exact artifact path or command;
- screenshots or short screen recordings stored under `docs/validation/media/` when the phase includes UI or 3D behavior;
- a pass/fail checklist written in user-facing language.

The user should never need to infer what changed from code. They should be able to open the artifact and see what was completed.

## Phase 1: Desktop Shell And Product Workspace

**Purpose:** Establish the real desktop application frame that all later functionality lands inside.

**Build:**

- Create the Tauri + Vite + React + TypeScript application.
- Create the Rust workspace with the planned crate boundaries: `mpb-core`, `mpb-storage`, `mpb-assets`, `mpb-render`, `mpb-export`, `mpb-agent`, and `app-tauri`.
- Add a dark-only main window with the permanent workspace layout: sidebar, central viewer region, right-side review/materials area, top status strip, settings entry, and AI connection indicator.
- Add English and Russian i18n infrastructure with visible language switching.
- Add local app data path discovery and a diagnostics command that opens the app data folder in Finder on macOS.

**User Opens:**

- The macOS desktop app.
- The diagnostics/app data folder opened from the settings surface.

**User Validates:**

- The app launches as a desktop app, not as a browser tab.
- The main workspace shape is visible immediately after onboarding completion or skip.
- The UI is dark themed.
- Language can be switched between English and Russian.
- The app can open its local data folder from settings.

**Engineering Validation:**

- `cargo test --workspace`
- `pnpm test`
- `pnpm build`
- Tauri dev launch on macOS

## Phase 2: Onboarding, Settings, And Secure CurseForge Key Storage

**Purpose:** Make first launch and settings usable by a normal Minecraft player.

**Build:**

- Add first-launch onboarding with language selection, AI integration instructions, and CurseForge API key setup.
- Store the CurseForge API key through OS secure credential storage.
- Add settings screens for AI integration, CurseForge API key management, language, and data/diagnostics folder access.
- Add clear states for missing key, saved key, replaced key, and secure storage unavailable.
- Ensure the add-modpack flow redirects to settings if the key is missing.

**User Opens:**

- The app on a clean local profile.
- Settings after onboarding.

**User Validates:**

- Onboarding explains what the API key is for.
- The key is never shown after saving.
- The key can be replaced.
- Skipping onboarding still allows reaching the empty library.
- Starting add-modpack without a key sends the user to settings instead of asking inside the import wizard.

**Engineering Validation:**

- Secure storage integration tests where the OS allows them.
- Frontend component tests for onboarding state transitions.
- Playwright flow for onboarding skip and settings navigation.

## Phase 3: Local Library, SQLite Storage, And Scheme Records

**Purpose:** Make the local library durable and understandable before import and rendering become complex.

**Build:**

- Add SQLite migrations for imported modpacks, schemes, scheme dimensions, construction stages, settings metadata, and import status.
- Add repositories for modpack and scheme CRUD.
- Add sidebar tree with imported modpacks and schemes.
- Add create, rename, and delete flows for schemes.
- Add imported modpack rename/delete flows, including deletion of schemes and local cache for that imported instance.
- Allow duplicate imported modpack names by applying numeric suffixes.
- Add autosave after every successful user operation.

**User Opens:**

- The app with a seeded local library fixture.
- The SQLite-backed app after quitting and relaunching.

**User Validates:**

- Sidebar shows modpacks and schemes as a two-level tree.
- A scheme belongs to exactly one imported modpack.
- Renames survive app restart.
- Deletes ask for confirmation and then disappear after restart.
- Duplicate local names become distinct with numeric suffixes.

**Engineering Validation:**

- SQLite migration tests.
- Repository tests for CRUD and duplicate naming.
- Playwright smoke test for sidebar persistence after restart.

## Phase 4: Scheme Domain Model, Operations, Validation, And Materials

**Purpose:** Make Rust the authoritative owner of scheme correctness.

**Build:**

- Implement scheme dimensions, block coordinates, block identifiers, block states, stages, `Unassigned`, and selections in `mpb-core`.
- Implement atomic operations for placing, deleting, replacing, and bulk-changing blocks.
- Implement resize validation and rejection of operations that would move blocks out of bounds.
- Implement construction stage ordering and cumulative visibility rules.
- Implement technical validation for block existence, allowed states, coordinates, bulk bounds, and post-operation structure.
- Implement material list generation for the complete scheme.
- Add synthetic block registry fixtures for tests and local demo data.

**User Opens:**

- A local “domain demo scheme” screen or diagnostic page inside the app.
- A JSON validation report generated from the synthetic demo scheme.

**User Validates:**

- The demo scheme shows dimensions, stage list, block count, and material count.
- Invalid operations appear as rejected actions in the validation report.
- The materials list updates after valid operations.
- `Unassigned` blocks are included in final counts.

**Engineering Validation:**

- Rust unit tests for every scheme operation.
- Property-style tests for coordinate bounds and bulk operation atomicity.
- Snapshot tests for structured validation errors.

## Phase 5: CurseForge Release Discovery And Modpack Download

**Purpose:** Let the user add a real CurseForge modpack release through the intended wizard.

**Build:**

- Parse CurseForge modpack page URLs and reject non-modpack URLs.
- Search CurseForge modpacks by name and let the user select a project result.
- Fetch available releases through Rust backend code only.
- Show a single release list with filters for Minecraft version and loader.
- Select the latest available release by default.
- Download the selected file into managed app data directories.
- Show progress, cancellation, success, and user-readable failure states.
- Record the imported modpack as a fixed local snapshot of the selected release.
- Support AI-initiated modpack import through the same backend path, without direct filesystem access.

**User Opens:**

- Add Modpack wizard in the app.
- The imported AOC modpack entry after searching for `AOC` or another CurseForge modpack name and selecting the intended project.
- The local app data folder containing the downloaded archive.

**User Validates:**

- Searching by a modpack name shows CurseForge project results.
- Selecting the intended project shows real releases.
- Filters narrow releases by Minecraft version and loader.
- The selected release downloads with progress.
- The imported modpack appears in the sidebar after success.
- Cancelled or failed downloads leave a clear error and no broken sidebar entry.

**Engineering Validation:**

- URL parsing tests.
- Mocked CurseForge API tests.
- Download progress and cancellation tests.
- Manual macOS import test using the AOC modpack.

## Phase 6: Modpack Asset Parsing, Block Index, And Texture Preparation

**Purpose:** Convert an imported modpack into enough local block, model, texture, and language data for rendering and validation.

**Build:**

- Extract downloaded modpack archives into managed cache directories.
- Discover mod files and relevant resource assets.
- Parse blockstates, models, textures, and language names where available.
- Build a local block registry for validation and AI context.
- Generate or cache texture atlas metadata suitable for Three.js.
- Treat an unparseable modpack as a failed import rather than silently degrading.
- Add synthetic asset fixtures for automated tests.
- Add an import diagnostics report for each imported modpack.

**User Opens:**

- The AOC import diagnostics report from the app.
- A block/texture sample preview page for the imported AOC modpack.

**User Validates:**

- The report shows import status, selected release, Minecraft version, loader, block count, asset count, and cache location.
- The preview shows real block names and rendered texture samples from the imported modpack.
- If parsing fails, the app explains why the modpack was not imported successfully.

**Engineering Validation:**

- Fixture-based asset parsing tests.
- Cache integrity tests.
- Manual AOC parsing report review.

## Phase 7: Render Preparation And Three.js 3D Viewer

**Purpose:** Show schemes as real 3D structures with responsive camera and stage visibility.

**Build:**

- Implement Rust render chunk preparation and mesh buffer generation.
- Avoid one mesh per block by chunking and preparing compact buffer data.
- Skip internal opaque faces where valid.
- Provide picking metadata for block and area selection.
- Implement the Three.js viewer with camera controls, resize handling, selection overlay, and stage visibility.
- Add cumulative stage view as the default.
- Add optional future-stage translucent mode if it remains low-risk during implementation.
- Add viewer empty, loading, error, and large-scheme states.

**User Opens:**

- A demo scheme with several construction stages.
- A real app viewer screen showing the scheme in 3D.

**User Validates:**

- The scheme is visible as a 3D structure.
- Camera orbit, pan, and zoom feel responsive.
- Stage 1 shows only stage 1.
- Stage 2 shows stages 1 and 2.
- `Unassigned` blocks are visible according to the app’s chosen v1 display rule.
- Clicking or dragging selection shows coordinates.
- The viewer resizes cleanly with the app window.

**Engineering Validation:**

- Rust render chunk tests.
- Mesh buffer snapshot tests on synthetic schemes.
- Playwright screenshot smoke tests for nonblank viewer output.
- Manual macOS viewer interaction recording.

## Phase 8: Viewer Selection And Materials Panel

**Purpose:** Complete the user inspection workflow around the 3D viewer.

**Build:**

- Allow selecting a single block or rectangular area in the viewer.
- Show selection coordinates in the UI.
- Add a materials panel with block type and count for the current scheme.
- Keep materials updated after successful user or agent operations.

**User Opens:**

- A scheme with several blocks, stages, and materials.

**User Validates:**

- Selecting an area shows exact coordinates.
- Materials show block identifiers/names and counts.
- Materials update after scheme changes.

**Engineering Validation:**

- Frontend tests for panel state.
- Playwright flow for selection display and materials panel.

## Phase 9: MCP-Compatible AI Integration And Tool Surface

**Purpose:** Let one external AI client safely read context and mutate schemes through controlled tools.

**Build:**

- Start the local MCP-compatible server with the application lifecycle.
- Support exactly one active external client at a time.
- Show server and active-client status in the main window and settings.
- Add connection instructions for Codex, Claude Code, opencode, and compatible clients.
- Implement tools to list imported modpacks, add a modpack, list schemes, create/rename/delete schemes, read scheme content, read current selection, mutate blocks, bulk mutate areas, resize schemes, manage stages, assign blocks to stages, validate, get materials, and export.
- Route every mutating tool through Rust core validation.
- Reject invalid commands atomically with structured errors.
- Emit UI update events after successful mutations.

**User Opens:**

- AI integration settings screen.
- A connected external AI client session.
- The app viewer while an AI client creates or changes a scheme.

**User Validates:**

- The app shows server running or stopped state.
- When a client connects, the active client is visible.
- The external client can create a scheme through tools.
- The external client can modify blocks and stages.
- Invalid operations are rejected without partial changes.
- The viewer updates after successful agent operations.

**Engineering Validation:**

- MCP tool schema tests.
- Request/response contract tests.
- Invalid-operation atomicity tests.
- End-to-end local client test against the running app.

## Phase 10: Export To `.schem` And `.litematic`

**Purpose:** Produce real Minecraft ecosystem files from the final scheme.

**Build:**

- Implement `.schem` export.
- Implement `.litematic` export.
- Use mature NBT, compression, and binary serialization crates.
- Validate exportability before writing files.
- Include all blocks, including `Unassigned`.
- Use a standard save dialog for destination and file name.
- Add structured user-facing export errors.
- Add golden fixture tests for both export formats.
- Add manual validation instructions for opening exported files in target Minecraft tools.

**User Opens:**

- Exported `.schem` file from a demo scheme.
- Exported `.litematic` file from the same demo scheme.
- Manual validation notes showing which external tool opened each file successfully.

**User Validates:**

- Export action is available from the scheme workspace.
- The save dialog allows choosing format and location.
- Exported files exist at the selected path.
- The exported files open in the target Minecraft ecosystem tools.
- The exported structure matches the app viewer closely enough to trust the workflow.

**Engineering Validation:**

- Export golden tests.
- Format-specific validation tests.
- Manual open test for both exported formats.

## Phase 11: Error Handling, Diagnostics, And Data Integrity

**Purpose:** Make failures understandable and protect local data from corruption.

**Build:**

- Normalize backend errors into user-readable frontend messages.
- Add structured diagnostic artifacts for import, validation, export, and AI operations.
- Ensure failed mutations do not change scheme state.
- Use atomic persistence for scheme writes where practical.
- Validate database migrations at startup.
- Add recovery messages for unreadable local data, failed migration, missing secure storage, failed import, failed export, and AI command rejection.
- Keep logs local and expose folders through settings rather than a built-in technical log viewer.

**User Opens:**

- A diagnostics folder containing recent import/export/validation reports.
- App screens showing representative failure states.

**User Validates:**

- Errors explain what happened and what action is available.
- Failed operations do not leave half-created schemes or half-imported modpacks in the sidebar.
- Diagnostic files are easy to find from settings.
- The app remains usable after a failed import or failed export.

**Engineering Validation:**

- Atomicity tests for invalid operations.
- Migration failure simulation.
- Frontend error-state tests.
- Manual failure walkthrough on macOS.

## Phase 12: Cross-Platform Packaging And Update Flow

**Purpose:** Deliver v1 as installable desktop artifacts for macOS, Windows, and Linux.

**Build:**

- Configure Tauri bundler for macOS, Windows, and Linux artifacts.
- Configure GitHub Releases as the public release host.
- Configure Tauri updater signing and static `latest.json`.
- Add non-disruptive update availability UI.
- Add settings toggle for automatic update checks.
- Add manual `Check for updates` action.
- Keep network/update errors non-blocking.
- Document code signing and notarization requirements for public distribution.

**User Opens:**

- macOS packaged app artifact.
- Update settings screen.
- Release artifact folder or GitHub Release draft containing macOS, Windows, and Linux bundles.

**User Validates:**

- The macOS packaged app launches outside the dev environment.
- Update settings are visible and understandable.
- Manual update check reports either current version or available update.
- Release artifacts are grouped clearly by platform.

**Engineering Validation:**

- Tauri bundle build for macOS.
- CI build matrix for Windows, macOS, and Linux.
- Updater metadata validation.
- Smoke launch of packaged macOS app.

## Phase 13: Full V1 Acceptance Pass

**Purpose:** Validate the complete product contract end-to-end.

**Build:**

- Run the full user journey on macOS using the AOC modpack.
- Verify first launch, settings, CurseForge key storage, import, asset parsing, scheme creation, AI tool mutation, 3D inspection, material list, validation, and export.
- Run automated test suites across Rust and frontend.
- Run packaging and smoke checks.
- Produce a final v1 acceptance report.

**User Opens:**

- The packaged macOS app.
- The imported AOC modpack in the local library.
- A completed scheme in the 3D viewer.
- Exported `.schem` and `.litematic` files.
- The final acceptance report at `docs/validation/v1-acceptance.md`.

**User Validates:**

- A CurseForge URL imports a real modpack with real blocks and textures.
- An external AI client can create or modify a scheme through tools.
- The user can inspect the result in 3D.
- The user can inspect selected areas and discuss requested edits in the external AI client.
- The materials list is correct for the visible completed scheme.
- Exported `.schem` and `.litematic` files open in target tools.

**Engineering Validation:**

- `cargo test --workspace`
- Frontend unit/component tests
- Playwright smoke/e2e suite
- Fixture import tests
- AOC manual import report
- MCP local client e2e test
- Export golden tests
- Packaged app smoke test

## Suggested Phase Order

1. Desktop shell and product workspace
2. Onboarding, settings, and secure CurseForge key storage
3. Local library, SQLite storage, and scheme records
4. Scheme domain model, operations, validation, and materials
5. CurseForge release discovery and modpack download
6. Modpack asset parsing, block index, and texture preparation
7. Render preparation and Three.js 3D viewer
8. Viewer selection and materials panel
9. MCP-compatible AI integration and tool surface
10. Export to `.schem` and `.litematic`
11. Error handling, diagnostics, and data integrity
12. Cross-platform packaging and update flow
13. Full v1 acceptance pass

## Risk Register

| Risk | Where It Is Resolved | Validation Artifact |
| --- | --- | --- |
| Real CurseForge modpacks may not expose enough consistent assets | Phase 6 | AOC import diagnostics report and texture preview |
| WebGL2 behavior may differ across desktop webviews | Phase 7 and Phase 12 | Viewer screenshots/recordings and packaged app smoke checks |
| MCP Rust library or transport may need adjustment | Phase 9 | Connected external client test and tool contract report |
| Exported modded blocks may not round-trip cleanly into target tools | Phase 10 and Phase 13 | Opened `.schem` and `.litematic` validation notes |
| Secure storage behavior differs on Linux desktops | Phase 2 and Phase 12 | Platform packaging and secure-storage smoke notes |
| Large schemes may stress viewer performance | Phase 7 | Render chunk metrics and viewer interaction recording |

## Definition Of Done For V1

V1 is complete only when all of the following are true:

- The app is packaged as a desktop application.
- The user can configure language, AI integration, and CurseForge API key.
- The user can import the AOC CurseForge modpack release as a local fixed snapshot.
- The app parses enough real block/model/texture data to render imported modpack blocks.
- The user can create and open schemes under imported modpacks.
- The user can inspect a scheme in 3D with stage switching and selection.
- A single external AI client can connect and mutate schemes through controlled tools.
- Invalid AI operations are rejected atomically with useful structured errors.
- Materials list and validation are available.
- The user can export `.schem` and `.litematic`.
- Exported files open in target Minecraft ecosystem tools.
- The final acceptance report links all manual artifacts the user needs to inspect.
