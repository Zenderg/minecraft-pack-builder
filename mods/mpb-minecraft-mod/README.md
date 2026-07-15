# MPB Minecraft Mod Runtime

This document is the source of truth for the MPB Minecraft runtime module's product contract,
implementation invariants, MCP tools, and build procedure. Application-level setup belongs in the
[project README](../../README.md), knowledge formats under [`docs/knowledge`](../../docs/knowledge/README.md),
and release gates in the [validation checklists](../../docs/validation/README.md).

## Product Contract

- Client-only mod for Fabric, Forge, and NeoForge.
- Supported Minecraft versions: `1.20` and newer.
- Starts a Streamable HTTP MCP server on `/mcp` from the Minecraft main menu.
- Binds to localhost by default.
- LAN mode is persisted in `<instance>/mpb/config.json` and warns that local-network MCP clients can connect while Minecraft is running.
- Supports one active MCP client at a time.
- Stores runtime data in the Prism instance root:
  - `<instance>/mpb/config.json`
  - `<instance>/mpb/schemes/*.mpb.json`
  - `<instance>/mpb/cache/`
  - `<instance>/mpb/patch-manifest.json`
  - `<instance>/mpb/knowledge/<pack-id>/knowledge-index.json`
- Opens MPB Manager through `/mpb`, loader config entry when available, or an unbound keybinding assigned by the user.
- Provides an unbound keybinding for Build/View mode.
- Does not place blocks, use server commands, require a server mod, or expose player world position/progress through MCP.
- Import/export of `.litematic` and `.schem` is a Minecraft UI action, not an MCP tool.
- Loads first-party curated knowledge only when the patch manifest and runtime bundle metadata agree on pack id, exact fingerprint, schema version, and checksum.
- If no exact curated knowledge pack is active, the agent prompt says curated modpack knowledge is unsupported and read-only knowledge tools return unsupported/unavailable instead of falling back to guesses.

## Runtime Implementation Notes

The patcher ships real loader-specific jars generated from this source tree. Loader-specific entrypoints are thin and delegate to a common runtime service with these lifecycle hooks:

- `onClientMainMenuReady`
- `startMcpServer`
- `openManager`
- `toggleBuildViewMode`
- `reloadSchemes`
- `shutdown`

Ghost rendering and registry-safety invariants are documented in the focused
[ghost rendering guide](docs/ghost-rendering.md).

The runtime is self-contained and does not require a separate managed dependency jar. It starts a local Streamable HTTP-compatible MCP endpoint at `/mcp`, creates instance-local MPB folders/config on startup, exposes core scheme management tools over JSON-RPC, and registers a real `/mpb` client command plus an unbound `key.mpb.open_manager` keybinding in each loader build.

## Curated Knowledge MCP Tools

The runtime exposes first-party knowledge as read-only MCP tools separate from scheme mutation tools:

- `mpb_knowledge_status`: reports whether an exact curated pack is active and returns pack metadata when available.
- `mpb_search_entities`: searches active curated entities by id, localized name, tag, use case, mechanic, or interface.
- `mpb_get_entity_card`: returns one curated entity card.
- `mpb_get_recipe_graph`: returns one recipe or dependency graph slice.
- `mpb_get_mechanic_details`: returns one curated mechanic overlay.
- `mpb_get_evidence`: returns one accepted evidence summary.

These tools never generate knowledge, inspect raw lab artifacts, or trust model output at runtime. They only read the installed `knowledge-index.json` bundle. If the selected Prism instance is compatible with the base MPB mod but does not exactly match a bundled first-party knowledge fingerprint, the patcher may still install the base mod; the runtime keeps curated knowledge inactive, `mpb_knowledge_status` explains why it is unavailable, and all other knowledge tools return an unsupported response.

MPB Manager copies an agent prompt that mirrors this state. With an active pack, the prompt instructs the agent to call `mpb_knowledge_status` and the read-only knowledge tools for supported modpack questions. Without an exact active pack, the prompt explicitly tells the agent not to claim curated modpack support.

## Build

```bash
../../tools/build-minecraft-mod-container.sh
```

The repository-supported path runs the build in official Gradle Docker images. Fabric and NeoForge use `MPB_GRADLE_CONTAINER_JDK21_IMAGE` (default `gradle:8.14.3-jdk21`), while Forge uses `MPB_GRADLE_CONTAINER_JDK17_IMAGE` (default `gradle:8.14.3-jdk17`) because ForgeGradle's Minecraft 1.20.1 setup step requires a Java 17 toolchain. `MPB_GRADLE_CONTAINER_IMAGE` remains a compatibility alias for the JDK 21 image. The wrapper keeps Gradle caches in the ignored `.gradle-container-cache/` directory. Gradle archive tasks use reproducible file ordering and timestamps, and every supported hex encoder writes 128-byte rows so rebuilding unchanged jars does not create metadata-only or line-wrapping diffs. Local machines only need a Docker-compatible daemon; they do not need host Java or Gradle installs.

The underlying `build.sh` still supports direct local execution when a developer intentionally provides Gradle:

```bash
MPB_GRADLE=/path/to/gradle \
MPB_GRADLE_EXTRA_ARGS='-Porg.gradle.java.installations.paths=/path/to/jdk17' \
mods/mpb-minecraft-mod/build.sh
```

The script builds through Fabric Loom, ForgeGradle, and NeoForge ModDevGradle. ForgeGradle requires a JDK 17 toolchain for the Minecraft 1.20.1 setup step. The script writes loader-specific jars to `artifacts/generated/` and refreshes:

- `crates/mpb-assets/src/mpb_mod_fabric_jar.hex`
- `crates/mpb-assets/src/mpb_mod_forge_jar.hex`
- `crates/mpb-assets/src/mpb_mod_neoforge_jar.hex`
