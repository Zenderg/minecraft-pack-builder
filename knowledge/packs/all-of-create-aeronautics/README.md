# All of Create - Aeronautics Knowledge Pack

First-party curated knowledge pack for the local PrismLauncher instance:

- Instance: `/Users/koshmarus/Library/Application Support/PrismLauncher/instances/All of Create - Aeronautics`
- Modpack version: `v2.0`
- Minecraft: `1.21.1`
- Loader: `NeoForge 21.1.233`
- Knowledge schema: `mpb-knowledge-v1`
- Builder version: `mpb-knowledge-0.1.0`
- Lab version: `mpb-lab-0.1.0`
- Exact patch-target fingerprint: `4cdf224f36c11b8a`

The patch-target fingerprint excludes the MPB managed runtime mod file
`mods/mpb-minecraft-mod.jar`. The patcher also excludes that file during compatibility checks so
installing or repairing MPB does not invalidate a matching curated knowledge pack.

## Coverage

The committed source pack is generated from deterministic static extraction over the selected
local Prism instance:

- 205 mod jars
- 35,096 source entities
- 12,701 recipe records
- 33,905 relationship records
- 15 mechanic overlays
- 6 accepted evidence summaries

Runtime lab experiments and real Prism client validation are release gates for behavioral claims.
This static production run does not mark worker output as trusted and does not include raw local lab
artifacts.

The source-of-truth release bundle is `bundle/knowledge-index.json`. The Tauri patcher embeds
`bundle/knowledge-index.json.gz` to keep the app binary small, then writes the JSON bundle into the
managed Prism instance when the selected patch target matches the exact fingerprint.

## Validation

```sh
cargo run -p mpb-knowledge --bin mpb-knowledge -- validate-source knowledge/packs/all-of-create-aeronautics/source
cargo run -p mpb-knowledge --bin mpb-knowledge -- build-bundle knowledge/packs/all-of-create-aeronautics/source knowledge/packs/all-of-create-aeronautics/bundle
```
