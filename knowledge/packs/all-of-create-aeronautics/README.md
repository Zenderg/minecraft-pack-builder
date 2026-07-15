# All of Create - Aeronautics Knowledge Pack

This document is the human-readable entry point for the All of Create - Aeronautics pack and the
source of truth for its operator guidance and validation commands. Machine-readable identity and
coverage live in `source/manifest.json` and `extraction-summary.json`; cross-pack contracts live in
[`docs/knowledge`](../../../docs/knowledge/README.md).

First-party curated knowledge pack for the supported exact PrismLauncher target:

- Modpack: `All of Create - Aeronautics`
- Modpack version: `v2.0`
- Minecraft: `1.21.1`
- Loader: `NeoForge 21.1.233`
- Knowledge schema: `mpb-knowledge-v1`
- Builder version: `mpb-knowledge-0.1.0`
- Lab version: `mpb-lab-0.1.0`
- Exact patch-target fingerprint: `b16b2b58a198088e`

The general fingerprint normalization and managed-file exclusions are defined in the
[fingerprint contract](../../../docs/knowledge/fingerprints.md).

## Coverage

The committed source pack is generated from deterministic static extraction. The following summary
mirrors `source/manifest.json` and `extraction-summary.json`:

- 205 mod jars
- 35,096 source entities
- 12,701 recipe records
- 33,905 relationship records
- 15 mechanic overlays
- 6 accepted evidence summaries

Runtime lab experiments and real Prism client validation are release gates for behavioral claims.
This static production run does not mark worker output as trusted and does not include raw local lab
artifacts.

The source of truth is the reviewable `source/` directory. The uncompressed runtime
`bundle/knowledge-index.json` is a generated local artifact rebuilt from source for inspection and
validation; it is not committed for large production packs. The Tauri patcher embeds
`bundle/knowledge-index.json.gz` to keep the app binary small, then writes the JSON bundle into the
managed Prism instance when the selected patch target matches the exact fingerprint.

## Validation

```sh
cargo run -p mpb-knowledge --bin mpb-knowledge -- validate-source knowledge/packs/all-of-create-aeronautics/source
cargo run -p mpb-knowledge --bin mpb-knowledge -- build-bundle knowledge/packs/all-of-create-aeronautics/source knowledge/packs/all-of-create-aeronautics/bundle
cargo run -p mpb-knowledge --bin mpb-knowledge -- inspect-bundle knowledge/packs/all-of-create-aeronautics/bundle/knowledge-index.json
```

Use the [first-party release checklist](../../../docs/validation/first-party-knowledge-release-checklist.md)
for every release. The latest concise human-reviewed result is kept in the
[AOCA validation summary](../../../docs/validation/all-of-create-aeronautics.md); exact generated run
evidence remains under the ignored `knowledge/runs/<run-id>/` tree.
