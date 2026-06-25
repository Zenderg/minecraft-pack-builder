# Registry, Materials, and PrismLauncher Handoff

This note captures the useful findings from the viewer/materials/runtime discussion so another agent can continue without rediscovering the same things.

## Original Problem

The viewer showed flat, wrong-looking block colors instead of Minecraft-like textures. The materials panel only displayed block ids, so it was hard to inspect what a scheme required. After adding thumbnails, the next problem became material quantities: users need counts and stack counts, but stack size differs per item.

The important product requirement: do not fake stack sizes with defaults such as `64`. If the app cannot know the real value, it should say nothing or surface an explicit indexing failure. Fallbacks are acceptable only as graceful degradation, not as a fix.

## Static Asset Import Findings

Static mod jar/resource-pack assets are enough for many visual tasks:

- blockstates and models can identify block textures;
- `assets/*/textures/block/*.png` can feed viewer thumbnails and block face textures;
- lang files can provide display names;
- `en_us.json` should win over other locales.

Static assets are not enough for authoritative stack sizes. Modern item definitions may contain explicit `minecraft:max_stack_size`, but most modpacks and vanilla items do not expose every stack size through static JSON assets. For example, in the tested All of Create / Aeronautics import, static scan found no useful `minecraft:max_stack_size` entries for the relevant block items.

Conclusion: static import can improve textures/names, but it cannot be the authoritative source of stack sizes.

## Where Stack Size Actually Lives

For NeoForge/Minecraft 1.21.x, item stack size is runtime item state. NeoForge documents item properties via `Item.Properties#stacksTo`; internally this is represented by item components such as `MAX_STACK_SIZE`. This means the reliable source is the loaded Minecraft registry after mods finish registering items.

Relevant references:

- NeoForge Items: https://docs.neoforged.net/docs/1.21.1/items/
- NeoForge Registries: https://docs.neoforged.net/docs/1.21.1/concepts/registries/
- NeoForge Events: https://docs.neoforged.net/docs/1.21.1/concepts/events/

## Runtime Extractor Direction

The runtime approach was discussed as a one-shot process during modpack import only. It should not run when opening or editing schemes.

Important constraints:

- do not search the user's filesystem for random Java installations;
- do not require the user to install Java manually;
- do not bundle a huge Java image inside the app/repo;
- if Java/runtime pieces are missing, download them into the app data directory during import;
- cache downloaded runtime artifacts for future imports;
- run extraction in a temporary/sandboxed working directory, not in the user's real instance;
- write an explicit registry report, never silently fake values.

The useful backend contract shape:

- app-managed Java runtime under app data, e.g. `runtimes/java-21`;
- runtime downloads under app data, e.g. `runtime-downloads/java-21`;
- registry reports under app data, e.g. `registries/<fingerprint>-registry.json`;
- extractor process receives:
  - `--minecraft-version`
  - `--loader`
  - `--loader-version`
  - `--modpack-dir`
  - `--output-dir`
  - `--fingerprint`
  - `--report-path`
- extractor writes report to `--report-path`; stdout is not the source of truth because real loaders are noisy;
- process must have timeout and memory limits.

Adoptium Temurin JRE 21 is a reasonable managed Java source. Verified endpoint shape:

```text
https://api.adoptium.net/v3/binary/latest/21/ga/<os>/<arch>/jre/hotspot/normal/eclipse?project=jdk
```

For macOS/aarch64 on 2026-06-24, this redirected to a Temurin 21 JRE tar.gz on GitHub releases.

## Loader Support Reality

CurseForge-relevant loaders discussed:

- Forge
- NeoForge
- Fabric
- Quilt

Production support means all four need real loader-specific extraction paths. A single NeoForge jar pretending to support Fabric/Quilt/Forge would be a bad fallback. The better artifact layout is per loader:

```text
extractors/forge/registry-extractor.jar
extractors/neoforge/registry-extractor.jar
extractors/fabric/registry-extractor.jar
extractors/quilt/registry-extractor.jar
```

Each loader likely needs its own bootstrap/entrypoint:

- Forge/NeoForge: FML mod/event listener after server start or common setup;
- Fabric: Fabric mod initializer/server lifecycle event;
- Quilt: Quilt loader initializer/server lifecycle event.

All of them should output the same JSON registry report schema so the app does not care which loader produced it.

## NeoForge Installer Notes

For tested All of Create 1.21.1, the CurseForge manifest used:

```text
Minecraft: 1.21.1
Loader: neoforge-21.1.233
```

Official NeoForge installer URL for that version was verified:

```text
https://maven.neoforged.net/releases/net/neoforged/neoforge/21.1.233/neoforge-21.1.233-installer.jar
```

The installer supports headless server install:

```text
java -jar neoforge-21.1.233-installer.jar --install-server <dir> --server-jar
```

This can be used by a NeoForge extractor, but the overall direction may now shift to PrismLauncher instead.

## PrismLauncher Pivot

The better product direction may be PrismLauncher integration instead of building our own modpack launcher/importer.

Why this is attractive:

- Prism already handles CurseForge/Modrinth import, auth, downloads, loader libraries, assets, and Java management;
- users already have instances that are known to launch;
- our app can import from installed instances instead of reconstructing them;
- less custom launcher infrastructure and fewer failure modes.

Official Prism docs worth using:

- Data locations: https://prismlauncher.org/wiki/getting-started/data-location/
- CLI: https://prismlauncher.org/wiki/getting-started/command-line-interface/
- Java settings: https://prismlauncher.org/wiki/help-pages/java-settings/

Known/default Prism data root on macOS:

```text
~/Library/Application Support/PrismLauncher
```

Useful Prism instance files/folders to inspect:

```text
instances/<instance name>/instance.cfg
instances/<instance name>/mmc-pack.json
instances/<instance name>/.minecraft/mods
instances/<instance name>/.minecraft/resourcepacks
```

The recommended product flow:

1. Add `Import from PrismLauncher`.
2. Discover Prism root from default locations, with manual root selection for portable/custom installs.
3. Scan `instances/*`.
4. Read `instance.cfg` and `mmc-pack.json` for name, Minecraft version, loader, loader version, and component metadata.
5. Use `.minecraft/mods` as the source of already-downloaded mod jars.
6. Build static asset index from those jars.
7. For stack sizes, run a one-shot registry extractor against a temporary copy/snapshot of the instance.
8. Never mutate the user's real Prism instance.
9. Prefer Prism-managed Java when it is explicitly referenced by the instance/launcher metadata; otherwise download app-managed Java into app data.

Open design question: whether to launch through Prism's CLI or reproduce the instance launch in our own temporary directory. The safer production path is probably a temporary copy/snapshot controlled by our app, because registry extraction should be headless and should not alter or visibly launch the user's game instance.

## Registry Performance Contract

Prism registry diagnostics can become very large once block states, model variants, texture paths, and modded blocks are indexed. Do not put startup, freshness checks, or render-scene loading on a path that parses the full `*-registry.json`.

Current constraints:

- freshness checks should read the small `*-registry-meta.json` sidecar, or only the header of a legacy registry report;
- render-scene loading should parse full block metadata only for block ids that are actually present in the scheme;
- extracting a block id from a raw registry block should avoid full JSON deserialization, because unknown fields can include huge legacy `modelElements` arrays;
- the main diagnostics registry should stay scan-friendly and should not store full geometry for every block as the primary index;
- Tauri commands that may touch registry data should run blocking work off the WebView/main invoke path.

This was validated after a macOS Tauri dev hang where the app UI was drawn but the cursor showed a spinner. `sample` showed hot paths in `serde_json` while reading or writing huge registry reports. After moving freshness to sidecar metadata, keeping registry output lightweight, and filtering render-scene registry reads by scheme block ids, the desktop process returned to idle CPU after startup.

## Viewer/Materials UI Decisions

Materials should show:

- texture thumbnail;
- display name if authoritative;
- block id as secondary text only when it adds information;
- count;
- stack count only when authoritative max stack size exists.

Do not show `[x stacks]` if max stack size is unknown.

Duplicate name/id rows should be avoided. If the display name equals the id, show it once.

## Runtime-Baked Viewer Contract

The production viewer direction is runtime-authored baked render assets, not a hand-written Minecraft renderer in TypeScript or Rust. The app should keep Three.js as the local viewport, but the geometry it draws should come from the loaded Minecraft/loader/modpack runtime whenever that runtime can provide it.

Registry reports may now include optional block render assets:

```json
{
  "blocks": [
    {
      "identifier": "mod:complex_block",
      "renderAssets": [
        {
          "fidelity": "runtimeBaked",
          "source": "minecraft-runtime",
          "condition": { "anyOf": [{ "facing": ["east"] }] },
          "model": "mod:block/runtime_complex_block",
          "elements": [
            {
              "from": [0, 0, 0],
              "to": [16, 16, 16],
              "faceTexturePaths": { "north": "/path/to/texture.png" },
              "faceUvs": { "north": [0, 0, 16, 16] }
            }
          ]
        }
      ]
    }
  ]
}
```

Render-scene loading prefers matching authoritative runtime `renderAssets` over static `modelVariants`. If no authoritative runtime asset matches the block states, it falls back to the existing static JSON model path. Runtime-baked elements are treated as already transformed; the viewer should not re-apply blockstate rotations to them.

The bundled Forge, NeoForge, and Fabric runtime extractor jars now write both authoritative runtime item stack sizes and server-side runtime shape render assets when Minecraft can expose bounded static voxel-shape boxes for a block state. These shape-derived assets use `"fidelity": "approximation"` and `"source": "minecraft-runtime-shape"`. The extractor intentionally skips single full-cube shapes to keep registry reports smaller and to avoid replacing better static JSON models with untextured shape boxes.

`approximation` render assets are a fallback, not a higher-fidelity source. The app should use them only when the static JSON model path has no render payload. Full production fidelity for blocks whose appearance comes from client-only baked models, block entity renderers, animation, custom renderers, or world context still requires a future client-baked extractor path that can emit `exact` or `runtimeBaked` assets.

Fidelity should remain explicit. Good values are:

- `exact` for assets known to match in-game static rendering;
- `runtimeBaked` for geometry authored by the Minecraft runtime/model system;
- `staticModel` for the JSON model fallback;
- `approximation` for deliberate substitutes;
- `unsupportedDynamic` for blocks that depend on dynamic renderers, animation, block entities, or world context the extractor could not freeze.

## Important Product Principle

Treat missing registry data as missing data. Do not mask it with `64`, regex guesses, pretty-name transforms, or static heuristics that look correct only for common vanilla blocks.

The app should either:

- display authoritative runtime-derived stack sizes; or
- omit stack summary and provide diagnostics explaining why registry extraction did not run.
