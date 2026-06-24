# PrismLauncher Integration Session Notes

Date: 2026-06-24

This file captures the practical product and engineering decisions from the PrismLauncher integration work session.

## Product Direction

- PrismLauncher is the source of truth for installed modpacks. The app should show the instances currently present in the selected local PrismLauncher root instead of searching and importing packs through the app.
- CurseForge/import/key flows are no longer needed for the current product direction and can be removed.
- The app should use one selected PrismLauncher root at a time.
- Onboarding needs a PrismLauncher step:
  - auto-detect likely launcher roots;
  - if detection fails, let the user pick the launcher root manually;
  - explain where the folder is usually located;
  - validate that the selected folder is a PrismLauncher root;
  - show the number of detected instances so the user can immediately see whether the folder is correct.
- Settings must include the same PrismLauncher root selection and validation flow.
- The library should show all PrismLauncher instances, because the app is meant to act as a bridge between the user and the agent for collecting and visualizing building schemes.
- The local database remains necessary. It stores app-owned data: selected launcher root, discovered instance records, statuses, schemes, relink state, and diagnostics. PrismLauncher remains the source of installed pack presence.
- If an instance disappears from PrismLauncher, do not silently delete schemes. Keep the app record with a missing/error status. Viewing/editing schemes should be blocked while the instance is not ready because textures and registry context may be unavailable, but export can remain possible.
- If a moved launcher root causes an instance identity mismatch, offer relink with an explicit confirmation modal.
- Library refresh should be automatic. Prefer file watchers over polling when practical. Avoid adding manual refresh buttons unless a concrete recovery case requires one.
- Scheme creation/editing for an instance should be blocked until the instance reaches a ready state.

## Runtime And Asset Strategy

- The app should not install or manage Java/runtime dependencies itself. That would make the product feel heavy and brittle.
- The app can use assets and runtime artifacts already present in the user's PrismLauncher installation.
- Static registry data should come from local assets:
  - mod/resource-pack jars and directories;
  - the vanilla client jar from PrismLauncher libraries;
  - language files for display names;
  - blockstates and models for block ids and texture/model data;
  - texture PNGs copied into the app diagnostics/cache area for viewer use.
- Runtime-derived metadata is still valuable, especially for real stack sizes and material correctness. It should be attempted during indexing/reindexing when local prerequisites exist, and should not block static registry indexing if unavailable.
- Loader handling cannot assume NeoForge only. Forge, NeoForge, and Fabric all need adapter paths, and unsupported/missing prerequisites should produce diagnostics rather than fake data.

## Viewer Findings

- Textures do not require launching the Minecraft instance. They can be read from the local asset jars and served to the Tauri frontend through the asset protocol.
- A blank/slow startup was caused by heavy Prism indexing during startup. The app should list stored library data immediately and run Prism sync/indexing in the background.
- A single texture per block is insufficient. Blocks like furnaces need per-face textures from Minecraft model JSON, including parent model resolution.
- Materials list icons should use the same indexed texture data as the viewer.
- Non-full-cube blocks need model geometry. A torch is not a full block; Minecraft describes it with `elements[]` in model JSON. Rendering it as a full cube creates the black square artifact.
- PNG transparency must be handled in the Three.js material. Textured model elements need alpha testing so transparent pixels are discarded instead of showing as black.
- The viewer should render model elements as cuboids when available, and fall back to a full cube when a block has no model element data.
- The right tool column must have constrained height and overflow scrolling; otherwise long material lists become inaccessible.

## Current Known Gaps To Revisit

- Blockstate variants, facing, multipart definitions, model rotations, UV rotations, tint indices, and more complex non-cube models are production-relevant areas to keep hardening.
- Wall torches and other orientation-sensitive blocks may require full blockstate-property support instead of only the first discovered model.
- Biome-tinted blocks, fluids, connected textures, animated textures, and custom renderer blocks may need explicit fallback behavior.
- The viewer should keep favoring correct diagnostics over silent approximation when a block cannot be rendered faithfully.

## Validation From This Session

- `cargo test --workspace` passed.
- `pnpm test` passed.
- `pnpm build` passed.
- `pnpm tauri build --bundles app` built the release binary and macOS app bundle, then stopped only because `TAURI_SIGNING_PRIVATE_KEY` is not configured for updater signing.

