# PrismLauncher Integration Design

## Goal

Replace the app-owned CurseForge modpack import flow with a production-ready PrismLauncher integration. The app should show PrismLauncher instances from one active Launcher Root, keep user schemes locally, and act as a safe bridge between the user, the viewer, and AI agent tools.

## Product Decisions

- The app uses one active PrismLauncher Launcher Root at a time.
- The user selects the PrismLauncher data/root folder, not the app binary. Manual copy says: "Open PrismLauncher, use `Folders > Launcher Root`, then select that folder here."
- Onboarding includes a PrismLauncher step after language and AI setup.
- Settings include a PrismLauncher section for the current root, validation status, instance count, and choosing another root.
- All valid Prism instances are shown, including vanilla/custom instances with no mods.
- CurseForge API key setup, CurseForge search, release download, import retry/cancel, and app-owned modpack snapshots are removed.
- Existing imported modpack data from the unreleased old flow is deleted during migration. Existing schemes attached to that old model are not migrated.
- Missing Prism instances are not deleted silently. They stay visible with a missing status, their schemes remain exportable, and viewer/editing/create are blocked until the instance is restored.
- Exact instance identity matches after a root move restore automatically. Probable matches show a confirmation modal before relinking.
- Sync is watcher-first. Polling is only a reliability fallback for startup, sleep/wake, watcher failure, or filesystem edge cases.
- Instances must reach `ready` before scheme creation or viewer/editing. `ready` requires a compatible static Prism registry report. Runtime-only fields, such as authoritative stack sizes, remain nullable until a safe runtime extractor provides them.

## Backend Architecture

Storage changes from imported CurseForge snapshots to Prism-linked local data:

- `app_settings`: active `prism_root_path`, last sync time, and app-level sync metadata.
- `prism_instances`: `instance_id`, display name, paths, Minecraft version, loader, loader version, identity fingerprint, content fingerprint, status, and status message.
- `schemes`: linked to `prism_instances`, with scheme documents stored locally as today.
- Diagnostics registry reports: per-instance static indexing status, runtime extraction status, input fingerprints, blocks, names, cached texture file paths, and nullable runtime-only fields.

Prism root validation:

- A root is valid when it is a directory containing `instances`.
- A Prism instance is a child of `instances` with `instance.cfg` or `mmc-pack.json` and a usable instance folder structure.
- Validation returns the root path, validity, message, and valid instance count so onboarding/settings can immediately show whether the selection worked.

Sync behavior:

- Scan `instances/*`.
- Parse `instance.cfg` and `mmc-pack.json` for display name, component metadata, Minecraft version, loader, and loader version.
- Compute `identity_fingerprint` from stable normalized metadata, excluding absolute paths.
- Compute `content_fingerprint` from metadata plus mod/resourcepack file names, sizes, and mtimes.
- Mark disappeared instances as `missing` without deleting schemes.
- Restore exact identity matches automatically.
- Emit possible-match candidates when a missing instance and a discovered instance are similar but not exact.
- Debounce filesystem events before sync/index work.

## Asset And Runtime Indexing

Static asset indexing reads local Prism instance files:

- `minecraft/mods` or `.minecraft/mods`
- `minecraft/resourcepacks` or `.minecraft/resourcepacks`
- Prism root vanilla client libraries when available
- instance-local resource overrides where present

The existing asset collector should be retained where useful but decoupled from CurseForge download/manifest APIs.

Static registry report contract:

- `schemaVersion` guards compatibility. Old reports are rebuilt automatically.
- Block entries include identifier, namespace, display name, item id, model id, cached texture path, and nullable max stack size.
- Textures from jar/zip archives are materialized into a diagnostics texture cache. UI and agent tools must receive physical file paths, never `jar::entry` pseudo-paths.
- Do not fake stack sizes. `maxStackSize` and `stackCount` stay null unless the registry report has an authoritative value.

Runtime extraction remains the future source for authoritative data such as stack sizes:

- Use Prism-managed Java/runtime/libraries/metadata from the active Prism root and selected instance.
- Do not download Java or loader assets through this app.
- Do not mutate the user's real Prism instance.
- Run extraction in a temporary app-owned snapshot controlled by the app. Prism `libraries` and instance `mods` are copied into app diagnostics/runtime work directories before Java starts.
- Extraction is cached by content fingerprint. If the instance content fingerprint does not change and a runtime report exists, do not run Java again.
- Do not use Prism CLI as the primary extraction path because it is a user-facing launch interface and can open the game or run user launch settings.
- Forge-like loaders are handled through Prism's ForgeWrapper path. NeoForge and Forge use embedded helper mods in the app-owned snapshot; the helper reads item registries after server startup and writes `itemId -> maxStackSize` into the cached runtime report.
- Fabric uses an embedded Fabric server entrypoint when all required local server artifacts exist. If Prism has only the client jar, static indexing remains `ready`, runtime extraction is marked `unavailable`, and stack sizes remain nullable instead of being guessed.
- If Prism/loader generated runtime artifacts are missing, static indexing remains `ready`, runtime extraction is marked `unavailable`, and the diagnostic tells the user what local artifact is missing.

## Frontend Flow

Onboarding:

- Steps: language, AI integration, PrismLauncher.
- The PrismLauncher step automatically checks default Prism data locations.
- If a valid root is found, show the detected instance count and allow finishing.
- If not found, ask the user to select the Launcher Root manually and show the Prism menu hint.

Settings:

- Replace CurseForge settings with PrismLauncher settings.
- Show current root, validation status, detected instance count, latest sync/index status, and diagnostics paths.
- Allow choosing another root.

Library:

- Remove Add Modpack and the import wizard.
- Show all Prism instances with statuses:
  - `ready`: create schemes, open viewer, edit.
  - `pending` or `indexing`: visible, but create/open/edit blocked.
  - `failed`: visible with error; scheme export/rename/delete remain available.
  - `missing`: red indicator; viewer/edit/create blocked; scheme export/rename/delete remain available.
- If an open scheme becomes non-ready, preserve selection and replace the viewer with a blocked state.
- Possible-match relinking uses a modal with old and new paths; no silent relink unless identity matches exactly.

## Agent Tools

The MCP surface changes from imported modpacks to Prism instances:

- `list_instances` replaces `list_imported_modpacks`.
- `add_modpack` is removed.
- `list_schemes(instanceId)` lists schemes for a Prism instance.
- `create_scheme(instanceId, name, dimensions)` requires `ready`.
- Scheme read/export can work for missing instances where the local scheme document exists.
- Mutating tools require the parent instance to be `ready`.
- Tool descriptions state that instances come from the active PrismLauncher root.

## Testing And Validation

Meaningful tests should cover:

- Prism root validation and instance counting.
- `instance.cfg` and `mmc-pack.json` parsing.
- Identity and content fingerprint behavior.
- Missing instance sync behavior.
- Possible-match detection.
- Storage migration from the old imported-modpack schema.
- Frontend blocking for non-ready instances.
- Agent tools checking readiness before mutations.

Desktop validation should prefer the Tauri app. Browser-only Vite validation is not a substitute for user-facing confidence in this repository.

## Removal Scope

Remove:

- CurseForge credentials and secure key UI.
- CurseForge search/release/import commands.
- Import wizard UI and import progress dialogs.
- CurseForge gateway/download code.
- `add_modpack` agent tool.
- Old copy that describes app-owned imported modpacks.

Keep and adapt:

- Scheme domain model.
- Scheme storage documents.
- Export.
- Viewer rendering, with readiness gating.
- Static asset scanning internals where useful.
