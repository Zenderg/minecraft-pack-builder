# Curated Knowledge Packs

MPB first-party knowledge packs are reviewable source records plus a generated read-only runtime bundle. Source records live under `knowledge/packs/<pack-id>/source/`; generated bundles live under `knowledge/packs/<pack-id>/bundle/`.

The production contract is strict:

- A pack is trusted only for the exact Prism/modpack fingerprint in its manifest.
- Every expected entity must be covered before a bundle can be generated.
- Behavioral claims require accepted runtime observation evidence.
- Deterministic extraction can create trusted static facts; model or worker output can only create drafts until converted into accepted evidence.
- Placeholders such as `unknown`, `todo`, `stub`, `inferred_only`, conflict markers, and template variables fail validation.
- Raw lab artifacts, snapshots, logs, local notebooks, and worker traces are local developer artifacts. They must not be committed or shipped in runtime bundles.

Source files are JSON/JSONL:

- `manifest.json`: pack id/version, schema version, exact fingerprint, Minecraft/loader/modpack metadata, builder version, and lab version.
- `entities.jsonl`: blocks, items, fluids, entities, tags, configs, datapacks, scripts, resource packs, and mechanics.
- `claims.jsonl`: static or behavioral claims linked to entity ids and evidence ids.
- `evidence.jsonl`: compact accepted summaries from deterministic sources or runtime observations.
- `recipes.jsonl`: recipe or dependency graph records.
- `overlays.jsonl`: complete mechanic overlays and traits.
- `relationships.jsonl`: directed entity relationships such as drops, requires, transforms, or participates-in.
- `worker-decisions.jsonl`: optional model-worker decisions. These are never trusted by themselves.

Use:

```sh
cargo run -p mpb-knowledge --bin mpb-knowledge -- validate-source knowledge/packs/fixtures/minimal/source
cargo run -p mpb-knowledge --bin mpb-knowledge -- build-bundle knowledge/packs/fixtures/minimal/source /tmp/mpb-minimal-bundle
```
