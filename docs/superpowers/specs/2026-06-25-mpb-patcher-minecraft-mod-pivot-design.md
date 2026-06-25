# Minecraft Pack Builder: Patcher And Minecraft Mod Pivot

Date: 2026-06-25

## 1. Product Direction

Minecraft Pack Builder is no longer centered on a desktop viewer, global scheme library, or app-owned Minecraft renderer. The product pivots to two concrete pieces:

- MPB Patcher: a GUI patcher for PrismLauncher instances.
- MPB Minecraft Mod: a client-only in-game runtime that hosts the MCP server, stores schemes in the instance, and shows the build guide inside Minecraft.

The desktop application model, SQLite global library, and Tauri-hosted 3D viewer are not the product core anymore. Existing code may be reused only when it helps the new direction. Code that exists mainly to support the old desktop viewer/library model should be removed or rewritten instead of preserved through half-measures.

The important product truth is that Minecraft itself is the honest visual environment. MPB should not build a second approximate Minecraft renderer outside the game.

## 2. MPB Patcher

MPB Patcher is a regular GUI application for Windows, macOS, and Linux. It is localized from the operating system locale: Russian for Russian locales, English for everything else.

The patcher supports PrismLauncher only.

Main flow:

1. Detect the PrismLauncher Launcher Root automatically.
2. If detection fails, let the user choose a path and validate it as a Prism Launcher Root.
3. Show Prism instances with name, Minecraft version, loader, path, and patch status.
4. Let the user apply, update, repair, or remove the MPB patch.
5. Show step-by-step progress while patching.
6. After success, show a short next step: start the instance in PrismLauncher, open MPB Manager in Minecraft with `/mpb` or assigned keybindings, then copy the agent prompt from MPB Manager.

Supported patch statuses:

- `Not patched`
- `Patched`
- `Needs update`
- `Needs repair`
- `Conflict`
- `Unsupported`
- `Instance running`

The patcher installs everything required for MPB to work. MPB-owned mod artifacts are bundled inside the patcher release. Third-party runtime dependencies may be downloaded and cached when needed. If the current patcher release does not contain an MPB artifact compatible with the selected instance loader and Minecraft version, the instance is `Unsupported` with a clear reason.

The supported loader surface is Fabric, Forge, and NeoForge for Minecraft 1.20 and later. Vanilla instances, unknown loaders, ambiguous loader metadata, missing Minecraft versions, and versions below the supported range are unsupported. The patcher must not guess.

Before patching or unpatching, the patcher should block or warn if the selected instance appears to be running. Mod installation and removal happen while the instance is closed.

The patcher writes a technical manifest in the instance root, for example:

```text
<instance>/mpb/patch-manifest.json
```

The manifest records managed files, versions, checksums, dependency ownership, and patch metadata. It is for safe patch/update/repair/unpatch behavior and does not rename the instance or affect the user's visible instance name.

Dependency rules:

- If a required dependency already exists and is compatible, use it and mark it as preexisting.
- Preexisting dependencies are never removed by unpatch.
- If a dependency is missing, the patcher may install a managed copy.
- If a dependency exists but is incompatible, stop with a conflict instead of installing a second copy next to it.
- MPB-owned files may be updated by the patcher.
- If a managed file is missing or changed, show `Needs repair` and restore it when the user repairs the patch.

Unpatch removes managed mods, dependencies, bridge files, config, cache, and manifest. It does not remove user-owned or preexisting mods. Schemes are user data: unpatch asks separately whether to delete MPB schemes from the instance.

The patcher does not create a full backup of the instance and does not launch PrismLauncher or the selected instance.

## 3. MPB Minecraft Mod

MPB Minecraft Mod is the main runtime. It is client-only and does not require installation on a server. It does not place blocks, use server commands, or send schemes to a server.

The mod targets Fabric, Forge, and NeoForge for Minecraft 1.20 and later. A multi-loader toolchain may be used when it genuinely reduces maintenance. Runtime dependencies are allowed if they are technically justified and fully managed by the patcher.

The MCP server starts in the Minecraft main menu. This lets an external agent create and update schemes before the user enters a world. In-world display requires a selected scheme, a world/server session, and an anchor.

MCP transport remains Streamable HTTP on `/mcp`. By default, the server binds to localhost. MPB Manager can enable LAN mode, which persists in instance config. LAN mode uses no token or pairing code, but the UI warns that MCP-compatible clients on the local network can connect while Minecraft is running. Only one active MCP client is supported at a time.

The agent connection remains prompt-based. MPB Manager shows a ready-to-copy prompt next to the scheme list. The prompt includes the current endpoint and tells the user how to connect an MCP-compatible agent such as Codex, Claude Code, or opencode. The prompt is Russian when Minecraft uses Russian, and English otherwise.

MPB data is stored in the Prism instance root:

```text
<instance>/mpb/config.json
<instance>/mpb/schemes/*.mpb.json
<instance>/mpb/cache/
<instance>/mpb/patch-manifest.json
```

The mod should also work best-effort if installed manually without a patch manifest. In that case it creates the runtime folders/config it needs.

SQLite is not used for the new model. Scheme files are written atomically through temp-file plus rename.

MCP tools work with schemes and the runtime block registry. They do not expose the player's current world position, anchor, active build progress, or remaining materials. Import/export of `.litematic` and `.schem` files is a Minecraft UI action, not an MCP tool.

## 4. Scheme Format

The source of truth is a versioned MPB schema file. `.litematic` and `.schem` are import/export compatibility formats only.

Schemes are sparse and dynamically sized:

- no fixed dimensions at creation time;
- local origin is the lower corner;
- block coordinates must be `x >= 0`, `y >= 0`, `z >= 0`;
- dimensions and bounds are computed from placed blocks;
- air blocks are not stored;
- extra blocks in the world are not checked.

Scheme data includes:

- `schemaVersion`
- `schemeId`
- name
- created and updated timestamps
- block palette with full block state properties
- block positions
- optional block entity/NBT data
- optional construction stages
- optional semantic regions
- optional agent-facing metadata

Construction stages are optional. Semantic regions are optional. A scheme without them is valid.

If no stages exist, Build mode treats the scheme as a single stage `1/1`. If stages exist but not all blocks are assigned to stages, Build mode also treats the scheme as `1/1` and the UI may show `Stages incomplete`. Deleting a stage removes the assignment but does not delete blocks.

Materials are grouped by block id/type. MPB does not try to solve item mapping for modded blocks as part of the base material count.

## 5. MCP Tool Surface

The MCP surface should be useful for an agent while staying focused on schemes and registry data.

Core tools:

- list schemes
- create scheme
- read scheme
- update scheme
- rename scheme
- delete scheme
- validate scheme
- search/list block registry ids
- describe block states and allowed properties

Geometry and bulk mutation tools:

- batch point edits
- fill region
- clear region
- copy/paste region
- mirror region
- replace blocks
- translate scheme
- rotate scheme

Stage tools:

- create stage
- rename stage
- reorder stages
- delete stage
- assign blocks or regions to stage
- unassign blocks from stage
- list stages

Semantic region tools:

- create region
- update region
- delete region
- list regions

`translate_scheme` moves all blocks and associated metadata together and rejects results with negative coordinates.

`rotate_scheme` supports 90-degree rotations around the vertical axis. It must rotate both coordinates and orientation-related block state properties. If any block state cannot be rotated reliably, the operation is rejected with a diagnostic and no partial mutation is saved.

MCP uses registry ids and block state properties as the precise agent-facing vocabulary. Localized block display names are not required for the agent.

## 6. Minecraft UI

MPB has two in-game UI layers.

### MPB Manager

MPB Manager is a full-screen Minecraft GUI in the style of utility mods. It can be opened with:

- `/mpb`
- a loader/mod config entry when available
- an MPB keybinding if the user assigns one

Keybindings have no default keys. The user assigns them in Minecraft Controls. The patcher does not inspect or manage keybinding conflicts.

The Manager main screen shows:

- scheme list
- scheme name
- computed dimensions
- block count
- stage count or `No stages`
- last modified time
- selected/active scheme state
- rename/delete actions
- import/export actions for `.litematic` and `.schem`
- re-anchor action
- MCP status
- endpoint
- LAN mode toggle
- mod version
- patch manifest version when present
- Minecraft/loader version
- MCP protocol version
- copyable agent prompt

The UI does not create empty schemes. Schemes are created by the agent through MCP or imported by the user.

Diagnostics stay lightweight. Successes and errors may be shown as chat/system messages or simple UI messages. There is no dedicated log viewer.

### In-World Guide

There are two MPB keybindings, both unbound by default:

- open MPB Manager
- toggle Build/View mode

Ghost display is independent of whether the Manager is open. If an active scheme has an anchor, the ghost remains visible until the active scheme is disabled or changed.

Placement rules:

- selecting a new active scheme resets the anchor;
- disabling or deleting the active scheme hides the ghost and resets the anchor;
- re-anchoring is done through Manager;
- after selecting a scheme, the player enters `Choose anchor`;
- clicking a block places the scheme origin on the block above the clicked block;
- anchor orientation comes from the player's horizontal facing at click time;
- live updates to the same scheme preserve anchor and orientation;
- changing world/server resets active scheme and anchor;
- changing dimension inside the same world/server does not reset active scheme or anchor, but ghost display is only shown in the dimension where the anchor was placed.

Build mode:

- uses cumulative stages when stages are complete;
- treats missing/incomplete stages as single stage `1/1`;
- shows only unbuilt blocks for the current cumulative stage;
- hides blocks whose block id/type already matches the target;
- shows target ghost blocks where blocks are missing;
- highlights wrong block id/type positions with a red outline/overlay;
- counts remaining materials by block id/type;
- computes progress from the client-loaded area only;
- automatically advances to the first unfinished stage after progress changes or live scheme updates;
- shows completion when all stages are complete.

View mode:

- shows the full scheme as a ghost layer over the world;
- does not hide already built blocks;
- shows a small HUD message that View mode is active and how to return to Build mode;
- persists as the selected mode until the user toggles back.

Semantic regions, when present, can be shown as optional in-world labels using the names supplied by the agent. MPB does not translate region names. Regions do not affect build progress.

## 7. Rendering And Honesty

MPB should not build or maintain a desktop 3D viewer as a core feature. Honest visual preview belongs inside Minecraft.

MPB Minecraft Mod owns the product's ghost guide. Litematica/Forgematica are not foundational dependencies. They may be researched for ideas, format compatibility, or optional future integration, but the main MPB workflow must not depend on their UI or private APIs.

Ghost rendering principles:

- render ordinary block states through Minecraft client rendering capabilities;
- store block entity/NBT data in schemes;
- render block entities or animations only where this can be done honestly without fragile fake-world simulation;
- do not replace complex unsupported visuals with generic fallback blocks that look authoritative;
- progress and wrong-block detection still operate by block id/type.

Live scheme updates:

- do not require restarting Minecraft;
- do not require resource reload for ordinary scheme data changes;
- rebuild MPB overlay state when scheme data changes;
- survive ordinary Minecraft resource reloads by recreating any MPB render resources as needed.

Mod installation and updates still require a closed instance and are handled by the patcher.

## 8. Import And Export

Import and export are user actions in MPB Manager.

Import:

- supports `.litematic` and `.schem` as compatibility formats;
- validates all block ids and states against the current runtime registry;
- rejects the whole import if blocks or states are missing/invalid;
- does not create placeholder blocks;
- does not synthesize construction stages when the imported file has none.

Export:

- writes the final block structure to `.litematic` or `.schem`;
- exports block states and NBT as supported by the target format;
- ignores MPB-only stages, semantic regions, and agent metadata in the ordinary export.

## 9. Explicit Non-Goals

The new product does not include:

- desktop 3D viewer as a core workflow;
- global app-owned scheme library;
- SQLite storage;
- server-side Minecraft mod;
- automatic block placement in the world;
- scheme creation UI in Minecraft;
- full instance backups;
- default keybindings;
- demo/starter schemes;
- auto-update of the patcher itself;
- MCP access to player world state, anchor, active progress, import, or export.

## 10. Validation Focus

The first implementation plan should validate the highest-risk assumptions:

- multi-loader architecture for Fabric, Forge, and NeoForge on Minecraft 1.20+;
- MPB mod can start a Streamable HTTP MCP server from the main menu;
- MCP can read the runtime block registry and expose block state definitions;
- scheme files can be created, mutated, validated, and atomically saved in `instance/mpb/schemes`;
- in-world ghost rendering can display ordinary block states through Minecraft client rendering;
- Build mode can compare target block id/type against client world blocks and hide completed targets;
- patcher can discover Prism roots/instances and safely apply/unpatch managed files;
- patcher dependency conflict handling protects preexisting mods.

