# 2026-06-25 Patcher / Mod Pivot Validation

## Implemented

- `mpb-core` schemes are sparse:
  - no fixed dimensions at creation time;
  - coordinates must be non-negative;
  - bounds and dimensions are computed from placed blocks;
  - incomplete stage assignments fall back to single-stage build planning.
- `mpb-storage` has a new instance-local JSON repository:
  - prepares `<instance>/mpb/config.json`;
  - prepares `<instance>/mpb/schemes/`;
  - prepares `<instance>/mpb/cache/`;
  - writes `.mpb.json` scheme files atomically with temp-file plus rename;
  - uses palette-based scheme files with schema/version/timestamps.
- `mpb-assets` has patcher status and operations:
  - supports Fabric, Forge, NeoForge only;
  - requires Minecraft 1.20+;
  - detects unsupported vanilla/unknown/old instances without guessing;
  - writes `<instance>/mpb/patch-manifest.json`;
  - detects changed/missing managed files as `Needs repair`;
  - blocks unmanaged preexisting `mpb-minecraft-mod.jar` files as `Conflict`;
  - rejects loaders/Minecraft versions without an exact bundled artifact compatibility match;
  - installs real bundled MPB mod jars instead of placeholder bytes;
  - preserves schemes during unpatch unless explicitly requested.
- Tauri exposes patcher commands through `src-tauri/src/patcher_commands.rs`.
- React active app surface is now the MPB Patcher workflow. The old desktop viewer/library frontend files were removed.
- Minecraft mod runtime source exists under `mods/mpb-minecraft-mod/`:
  - real Gradle loader toolchains for Fabric Loom, ForgeGradle, and NeoForge ModDevGradle;
  - Fabric, Forge, and NeoForge thin entrypoints;
  - real `/mpb` client command registration and unbound manager keybinding registration in all three loader builds;
  - real Minecraft `Screen` for MPB Manager entry with scheme summaries, active scheme selection, rename, confirmed active scheme delete, import from `<instance>/mpb/import`, export to `<instance>/mpb/export`, re-anchor entry, endpoint, prompt copy, LAN toggle, refresh, and version/protocol/manifest status;
  - unbound Build/View mode keybinding with shared runtime guide state, active scheme state, anchor state, and client feedback message;
  - Fabric, Forge, and NeoForge right-click anchor placement plus in-world line overlay rendering for active schemes;
  - Build mode hides matching already-built blocks and highlights occupied wrong-block positions in red; View mode shows the full anchored scheme overlay;
  - common Java runtime;
  - instance-local config/scheme/cache folder preparation;
  - local Streamable HTTP-compatible `/mcp` endpoint;
  - full MCP tool catalog plus core scheme, batch point edit, fill/clear/copy/paste/mirror/replace/translate/rotate geometry, stage reorder/assign/unassign, and region management operations;
  - generated loader-specific jars embedded in the Rust patcher as hex assets.

## Automated Validation Run

- `MPB_GRADLE=/private/tmp/gradle-8.14.3/bin/gradle MPB_GRADLE_EXTRA_ARGS='-Porg.gradle.java.installations.paths=/private/tmp/jdk-17-adoptium/Contents/Home' mods/mpb-minecraft-mod/build.sh`: passed; generated Fabric, Forge, and NeoForge jars through real loader Gradle toolchains plus embedded Rust hex assets. Fabric uses Loom `1.12.7`, Forge uses ForgeGradle `6.0.54`, and NeoForge uses ModDevGradle `2.0.141`.
- `cargo fmt --all`: passed.
- `cargo test --workspace`: passed.
- `pnpm test`: passed, 3 files / 8 tests.
- `pnpm build`: passed.
- `pnpm tauri build`: passed; built the release binary and macOS `.app` bundle at `target/release/bundle/macos/Minecraft Pack Builder.app`.
- JVM smoke: loaded `MpbClientRuntime` from a generated real loader jar, started MCP on `http://127.0.0.1:47392/mcp`, then shut it down cleanly.
- Jar inspection verified the generated Fabric, Forge, and NeoForge artifacts contain their loader metadata, loader entrypoint class, MPB Manager screen class, MCP tool catalog, and runtime path classes.
- Local Prism metadata inspection found installed Forge `1.20.1`, NeoForge `1.21.1`, and Fabric `26.x` instances. The patcher now marks Fabric `26.x` unsupported because this release bundles only Fabric `1.20.1`, Forge `1.20.1`, and NeoForge `1.21.1` artifacts.
- Follow-up cleanup removed the old desktop viewer/library frontend, desktop MCP server, SQLite library repository, render crate, asset-index extractor, runtime-extractor tool, Three.js dependency, and updater wiring.
- GitHub release workflow now installs JDK 17 and JDK 21, installs Gradle `8.14.3`, runs `mods/mpb-minecraft-mod/build.sh`, and only then runs Rust workspace tests and Tauri bundling. This ensures CI release artifacts embed freshly built Fabric, Forge, and NeoForge mod jars instead of depending on stale checked-out/generated hex assets.
- `mods/mpb-minecraft-mod/build.sh` refreshes embedded hex assets with `xxd` when available and falls back to Node.js for Windows CI runners.

## Manual / External Validation Still Needed

- Launch patched Fabric, Forge, and NeoForge Prism instances with Minecraft 1.20+.
- Verify the mod starts MCP from the Minecraft main menu inside each real Prism client.
- Verify MPB Manager `/mpb`, unbound keybindings, LAN mode warning/config persistence, prompt copy, and in-world ghost rendering in a real client.
- Validate PrismLauncher running-instance detection on Windows, macOS, and Linux before mutating files.

## Packaging Note

The local macOS packaging target is now the `.app` bundle. The previous DMG target failed in Tauri's generated `bundle_dmg.sh`; since the product requirement is a macOS GUI application rather than a specific DMG container, the failing DMG target was removed from local release targets instead of keeping a broken packaging step.
