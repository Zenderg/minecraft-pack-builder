# Minecraft Pack Builder

Minecraft Pack Builder is a tool layer for AI-assisted building in modded Minecraft. It patches a PrismLauncher instance with an in-game MPB runtime so an external AI agent can create build schemes, while Minecraft remains the real visual and mechanical environment.

Minecraft Pack Builder now centers on two pieces:

- **MPB Patcher:** a Tauri desktop GUI that discovers PrismLauncher Launcher Roots, lists Prism instances, and applies/repairs/removes the managed MPB Minecraft mod patch.
- **MPB Minecraft Mod:** buildable client-only Fabric, Forge, and NeoForge jars for Minecraft 1.20+, with instance-local schemes and MCP on `/mcp`.

The long-term modded direction is first-party curated knowledge packs: MPB developers can ship evidence-backed knowledge for exact supported modpack fingerprints so connected agents can reason about modded blocks, items, recipes, and mechanics without guessing.

Minecraft itself is the visual environment. The old desktop 3D viewer and global SQLite scheme library have been removed from the active project.

Runtime data lives in each Prism instance:

```text
<instance>/mpb/config.json
<instance>/mpb/schemes/*.mpb.json
<instance>/mpb/cache/
<instance>/mpb/patch-manifest.json
```

## Development

```bash
pnpm install
pnpm test
pnpm build
cargo test --workspace
pnpm tauri dev
```

The desktop host is a Tauri app in `src-tauri`. Rust domain crates live under `crates/`, and the React/Vite frontend lives under `src/`.

The Minecraft mod runtime source lives under `mods/mpb-minecraft-mod/`.

```bash
MPB_GRADLE=/path/to/gradle \
MPB_GRADLE_EXTRA_ARGS='-Porg.gradle.java.installations.paths=/path/to/jdk17' \
mods/mpb-minecraft-mod/build.sh
```

The mod build script uses real loader toolchains: Fabric Loom, ForgeGradle, and NeoForge ModDevGradle. It produces loader-specific jars in `mods/mpb-minecraft-mod/artifacts/generated/` and refreshes the hex-encoded bundled artifacts consumed by `mpb-assets`. ForgeGradle's Minecraft 1.20.1 pipeline requires a JDK 17 toolchain; the main project can still run on JDK 21.
