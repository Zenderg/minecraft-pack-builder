# Patcher And Minecraft Runtime Release Checklist

This checklist is the source of truth for release validation of the desktop patcher and embedded
Fabric, Forge, and NeoForge Minecraft runtimes. Product behavior belongs in the
[product contract](../product/patcher-and-minecraft-mod.md), architecture ownership in the
[architecture overview](../architecture/README.md), and individual command output and platform
session logs in generated release evidence.

## Build And Artifact Integrity

- [ ] `pnpm test` passes.
- [ ] `pnpm build` passes.
- [ ] `cargo test --workspace` passes.
- [ ] `tools/build-minecraft-mod-container.sh` builds all supported loader artifacts with the toolchains documented in the [runtime README](../../mods/mpb-minecraft-mod/README.md).
- [ ] Fabric, Forge, and NeoForge jars contain the expected loader metadata, thin entrypoint, common runtime, MPB Manager, MCP tool catalog, and runtime path classes.
- [ ] Embedded jar assets and compatibility metadata are regenerated from the same build.
- [ ] Repeated builds of unchanged sources produce identical loader jar hashes and do not create metadata-only hex diffs.
- [ ] A release-mode Tauri desktop artifact builds successfully on every target release platform.

## Patcher Behavior

- [ ] The desktop app discovers supported PrismLauncher roots and lists instances without taking ownership of them.
- [ ] Compatibility uses an exact bundled loader/Minecraft artifact match and reports unsupported combinations without guessing.
- [ ] Install writes only MPB-managed files and records their paths, checksums, and ownership in `<instance>/mpb/patch-manifest.json`.
- [ ] Repair restores missing or changed managed files.
- [ ] Pre-existing unmanaged files at managed paths produce a conflict instead of being overwritten.
- [ ] Unpatch removes managed files and preserves `<instance>/mpb/schemes` unless scheme deletion is explicitly selected.
- [ ] A running instance is detected and protected from unsafe patch mutation on Windows, macOS, and Linux.
- [ ] A compatible base runtime can install without curated knowledge; a fingerprint mismatch never installs or activates mismatched knowledge.

## Real Minecraft Client Smoke

- [ ] Launch a patched supported Fabric instance from PrismLauncher and reach the Minecraft main menu.
- [ ] Launch a patched supported Forge instance from PrismLauncher and reach the Minecraft main menu.
- [ ] Launch a patched supported NeoForge instance from PrismLauncher and reach the Minecraft main menu.
- [ ] The MPB runtime starts its Streamable HTTP MCP endpoint from the main menu and binds to localhost by default.
- [ ] `/mpb`, the loader config entry when available, and an assigned keybinding open MPB Manager.
- [ ] MPB Manager reports runtime/patch status, active scheme state, endpoint, prompt, and curated-knowledge availability accurately.
- [ ] LAN mode requires an explicit warning, persists in instance config, and the runtime enforces one active MCP client.
- [ ] Scheme creation, edits, batch geometry, stages, regions, validation, and failure responses work through MCP without partial invalid mutations.
- [ ] Build/View mode, anchoring, ordinary block ghosts, modded-block fallbacks, occupied-position warnings, and built-block hiding are exercised in-world.
- [ ] `.litematic` and `.schem` import/export work through Minecraft UI actions and are not exposed as MCP tools.

Record target versions, artifact hashes, platform, Prism instances, endpoint probes, observed results,
and unavailable steps in generated release evidence. Browser/Vite-only validation is not a substitute
for the Tauri patcher and real Minecraft clients.
