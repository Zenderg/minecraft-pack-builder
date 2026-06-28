# All of Create - Aeronautics Knowledge Pack Validation

Date: 2026-06-28

## Target

- Prism instance: local developer PrismLauncher instance named `All of Create - Aeronautics`
- Modpack identity: `All of Create - Aeronautics`
- Modpack version: `v2.0`
- Minecraft version: `1.21.1`
- Loader: `NeoForge`
- Loader version: `21.1.233`
- Knowledge schema: `mpb-knowledge-v1`
- Builder version: `mpb-knowledge-0.1.0`
- Lab version: `mpb-lab-0.1.0`
- Exact patch-target fingerprint: `4cdf224f36c11b8a`

The initial raw fingerprint command returned `04b92af60c4afcbd` for the already patched local
instance. The production pack uses `4cdf224f36c11b8a`, computed from the same fingerprint document
after excluding the MPB managed file `mods/mpb-minecraft-mod.jar`; this matches the patcher
compatibility policy and prevents MPB installation from making its own knowledge bundle stale.

## Extraction Summary

Deterministic static extraction covered local Prism metadata and runtime-affecting inputs:

- 205 mod jars
- 7,024 blockstate-derived block records
- 8,850 item-model-derived item records
- 12,756 discovered recipe resources, normalized into 12,701 valid recipe records
- 5,886 tag resources
- 464 config files
- 34 datapack/generated datapack files
- 1,840 resourcepack/texturepack/shaderpack/generated resourcepack files
- 35,096 total source entities
- 33,905 relationship records
- 15 mechanic overlays
- 6 accepted evidence summaries

Raw local artifacts, screenshots, logs, saves, and worker traces were not committed.

## Commands Run

```sh
cargo test -p mpb-knowledge --test cli fingerprint_command_prints_exact_fingerprint_document_summary
cargo run -p mpb-knowledge --bin mpb-knowledge -- fingerprint "<local-prism-instance>/All of Create - Aeronautics" mpb-knowledge-0.1.0 mpb-lab-0.1.0 mpb-knowledge-v1
cargo run -p mpb-knowledge --bin mpb-knowledge -- validate-source knowledge/packs/all-of-create-aeronautics/source
cargo run -p mpb-knowledge --bin mpb-knowledge -- build-bundle knowledge/packs/all-of-create-aeronautics/source knowledge/packs/all-of-create-aeronautics/bundle
cargo run -p mpb-knowledge --bin mpb-knowledge -- inspect-bundle knowledge/packs/all-of-create-aeronautics/bundle/knowledge-index.json
cargo test -p mpb-assets patcher
cargo test -p mpb-assets
cargo test --workspace
pnpm test
pnpm build
pnpm tauri build --bundles app
```

## Results

- `validate-source` passed with zero validation failures.
- `build-bundle` produced the local generated runtime bundle
  `knowledge/packs/all-of-create-aeronautics/bundle/knowledge-index.json`.
- `inspect-bundle` reported `all-of-create-aeronautics mpb-knowledge-v1 entities=35096 evidence=6`.
- The exact plan command `cargo test -p mpb-assets patcher` exited successfully, but it filtered to
  0 tests because current patcher test names do not include the literal string `patcher`; run
  `cargo test -p mpb-assets` for the meaningful patcher suite.
- `cargo test -p mpb-assets` passed after embedding the AOC bundle as compressed data.
- `cargo test --workspace`, `pnpm test`, and `pnpm build` passed after the final formatting pass.
- `pnpm tauri build --bundles app` produced
  `target/release/bundle/macos/Minecraft Pack Builder.app`.

## Release App Patch Validation

User-facing validation was performed through the release Tauri bundle, not through the development
knowledge collection flow. The application path was:

```text
target/release/bundle/macos/Minecraft Pack Builder.app
```

The app patched the selected AOC Prism instance and installed:

```text
<local-prism-instance>/All of Create - Aeronautics/mpb/knowledge/all-of-create-aeronautics/knowledge-index.json
```

The installed knowledge bundle remained the production JSON payload, about 89 MB on disk, with pack
checksum `d440d9c4b2dce383`.

After launching the instance, the runtime config exposed the MCP server on:

```text
http://127.0.0.1:47392/mcp
```

Live MCP checks against the running Minecraft main menu succeeded:

- `/mpb/status` returned HTTP 200 and included the AOC curated-knowledge prompt.
- `mpb_knowledge_status` reported pack `all-of-create-aeronautics`, fingerprint
  `4cdf224f36c11b8a`, schema `mpb-knowledge-v1`.
- `mpb_search_entities` for `cogwheel` returned AOC-backed entity and recipe hits.
- `mpb_get_entity_card` for `create:cogwheel` returned an entity card.
- `mpb_get_recipe_graph` for `create:cogwheel` returned recipes and relationships in about 0.93 s.
- `mpb_get_mechanic_details` for `kinetic-networks` returned the curated mechanic overlay.
- `mpb_get_evidence` for `det-src-aoca-recipes` returned accepted deterministic-source evidence in
  about 0.98 s.

## Performance Regression Fixed

The first release bundle embedded the raw 89 MB AOC JSON with `include_bytes!`, increasing the app
executable to about 104 MB. In the user-facing patch flow this caused a black startup window for
roughly 10 seconds and another visible pause after pressing Update.

The release asset now embeds `knowledge-index.json.gz` instead and materializes the JSON only when
the patcher actually writes the managed knowledge file. After this change:

- `knowledge-index.json.gz` is about 5.4 MB.
- The release app executable is about 18 MB.
- Patcher compatibility checks use the stored checksum and do not eagerly decode the AOC payload.

## Remaining Release Gates

The following gates are still outside this deterministic/static source pass:

- Run hypothesis-driven lab experiments for behavior that cannot be proven from static resources.
- Convert accepted observations into runtime evidence summaries only when linked to exact-fingerprint
  claims.
