# MPB Minecraft Mod Runtime

This module is the source tree for the MPB client-only Minecraft runtime.

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
- Opens MPB Manager through `/mpb`, loader config entry when available, or an unbound keybinding assigned by the user.
- Provides an unbound keybinding for Build/View mode.
- Does not place blocks, use server commands, require a server mod, or expose player world position/progress through MCP.
- Import/export of `.litematic` and `.schem` is a Minecraft UI action, not an MCP tool.

## Runtime Implementation Notes

The patcher ships real loader-specific jars generated from this source tree. Loader-specific entrypoints are thin and delegate to a common runtime service with these lifecycle hooks:

- `onClientMainMenuReady`
- `startMcpServer`
- `openManager`
- `toggleBuildViewMode`
- `reloadSchemes`
- `shutdown`

Ghost rendering must use Minecraft client rendering for ordinary block states and reject unsupported orientation rotation instead of saving partial mutations.
Ghost rendering must preserve the `RenderType` requested by Minecraft's renderer and may only wrap the returned `VertexConsumer` for guide alpha. Forcing every block through a generic translucent render type breaks texture/material lookup for vanilla block-entity-style blocks such as chests and for modded special renderers.
Modded blocks can still produce no visible ghost pixels even when their registry id, block state, and `renderSingleBlock` call are valid. This was reproduced with Create cogwheels in an All of Create / Aeronautics NeoForge instance: the guide outlines were positioned correctly, but the kinetic block-entity render path did not produce visible block-model ghosts. For non-vanilla blocks whose render shape is not `MODEL`, whose state has a block entity, whose baked block model is custom, missing, or has no quads, the guide should keep the normal block render attempt and also render an alpha-wrapped item-model fallback. This is a generic modded-block fallback, not a Create-specific special case.
Runtime block registry lookups must check explicit registry membership before reading `BuiltInRegistries.BLOCK.get(...)`; otherwise missing modded ids can be masked by Minecraft's fallback block and MCP clients receive a false "known block with no properties" response.

The current runtime is self-contained and does not require a separate managed dependency jar. It starts a local Streamable HTTP-compatible MCP endpoint at `/mcp`, creates instance-local MPB folders/config on startup, exposes core scheme management tools over JSON-RPC, and registers a real `/mpb` client command plus an unbound `key.mpb.open_manager` keybinding in each loader build.

## Build

```bash
MPB_GRADLE=/path/to/gradle \
MPB_GRADLE_EXTRA_ARGS='-Porg.gradle.java.installations.paths=/path/to/jdk17' \
mods/mpb-minecraft-mod/build.sh
```

The script requires Gradle and builds through Fabric Loom, ForgeGradle, and NeoForge ModDevGradle. ForgeGradle requires a JDK 17 toolchain for the Minecraft 1.20.1 setup step. The script writes loader-specific jars to `artifacts/generated/` and refreshes:

- `crates/mpb-assets/src/mpb_mod_fabric_jar.hex`
- `crates/mpb-assets/src/mpb_mod_forge_jar.hex`
- `crates/mpb-assets/src/mpb_mod_neoforge_jar.hex`
