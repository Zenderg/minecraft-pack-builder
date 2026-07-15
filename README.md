# Minecraft Pack Builder

Minecraft Pack Builder is a tool layer for AI-assisted building in modded Minecraft. It patches a PrismLauncher instance with an in-game MPB runtime so an external AI agent can create build schemes, while Minecraft remains the real visual and mechanical environment.

Minecraft Pack Builder consists of two pieces:

- **MPB Patcher:** a Tauri desktop GUI that discovers PrismLauncher Launcher Roots, lists Prism instances, and applies/repairs/removes the managed MPB Minecraft mod patch.
- **MPB Minecraft Mod:** buildable client-only Fabric, Forge, and NeoForge jars for Minecraft 1.20+, with instance-local schemes and MCP on `/mcp`.

Curated modded support uses first-party knowledge packs: MPB developers can ship evidence-backed knowledge for exact supported modpack fingerprints so connected agents can reason about modded blocks, items, recipes, and mechanics without guessing.

Minecraft itself is the visual environment. MPB does not maintain a second desktop 3D renderer or a global SQLite scheme library.

Runtime data lives in each Prism instance:

```text
<instance>/mpb/config.json
<instance>/mpb/schemes/*.mpb.json
<instance>/mpb/cache/
<instance>/mpb/patch-manifest.json
<instance>/mpb/knowledge/<pack-id>/knowledge-index.json
```

## End-User Flow

MPB ships as one desktop patcher. The supported flow is:

1. Download and open the MPB Patcher.
2. Select a PrismLauncher instance.
3. Install or repair the managed MPB Minecraft mod patch.
4. If the selected instance exactly matches a first-party knowledge-pack fingerprint, the patcher also installs the managed read-only knowledge bundle under `<instance>/mpb/knowledge/<pack-id>/`.
5. Start the instance in PrismLauncher.
6. Open MPB Manager in Minecraft with `/mpb`, the loader config entry when available, or the user-assigned keybinding.
7. Copy the MCP endpoint and agent prompt from MPB Manager.
8. Connect the external agent to the Streamable HTTP MCP endpoint.

Curated first-party knowledge is exact-match only. When the Minecraft version, loader, mod list, configs, datapacks, scripts, resource packs, schema version, builder version, or lab tooling version do not match the bundled fingerprint, the base MPB mod may still install if compatible, but curated knowledge is unavailable. In that state the runtime disables the knowledge tools, `mpb_knowledge_status` reports unsupported/unavailable, and the copied agent prompt tells the agent not to claim curated modpack support.

Trusted packs are release-blocked until strict validation passes with zero unresolved coverage, placeholders, query gaps, stale fingerprints, incomplete overlays, incomplete dependency chains, trusted worker-only output, or behavioral claims without accepted runtime evidence. Runtime bundles are generated artifacts; raw lab logs, notebooks, screenshots, saves, and worker traces are local developer artifacts and are not shipped.

## Development

```bash
pnpm install
pnpm test
pnpm build
cargo test --workspace
pnpm tauri dev
```

The desktop host is a Tauri app in `src-tauri`. Rust domain crates live under `crates/`, and the React/Vite frontend lives under `src/`.

The Minecraft mod runtime source lives under `mods/mpb-minecraft-mod/`:

```bash
tools/build-minecraft-mod-container.sh
```

See the [Minecraft runtime module documentation](mods/mpb-minecraft-mod/README.md) for loader,
toolchain, and artifact details. The developer-only experiment runner is documented in the
[Knowledge Lab README](mods/mpb-knowledge-lab/README.md).

## Documentation

The [documentation index](docs/README.md) links the current product, architecture, curated
knowledge, and release-validation sources of truth.
