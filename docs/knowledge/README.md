# Curated Knowledge Packs

MPB first-party knowledge packs are reviewable source records plus a generated read-only runtime bundle. Source records live under `knowledge/packs/<pack-id>/source/`; generated bundles live under `knowledge/packs/<pack-id>/bundle/`.

The production contract is strict:

- A pack is trusted only for the exact Prism/modpack fingerprint in its manifest.
- Fingerprints are exact-match only: no version ranges, no user override, and no "close enough" trusted mode.
- Every expected entity must be covered before a bundle can be generated.
- Behavioral claims require accepted runtime observation evidence.
- Deterministic extraction can create trusted static facts; model or worker output can only create drafts until converted into accepted evidence.
- Placeholders such as `unknown`, `todo`, `stub`, `inferred_only`, conflict markers, and template variables fail validation.
- Runtime query gaps, incomplete overlays, incomplete dependency chains, stale fingerprints, missing manifest metadata, and trusted worker-only output fail validation.
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

## Installation And Runtime Contract

The patcher installs a generated bundle only when the selected Prism instance exactly matches a bundled first-party fingerprint. A successful knowledge install writes a managed read-only bundle to:

```text
<instance>/mpb/knowledge/<pack-id>/knowledge-index.json
```

The patch manifest records the pack id, fingerprint, schema version, compatibility metadata, and checksums for managed knowledge files. Repair and update operations restore managed knowledge files when the exact fingerprint still matches. Unpatch removes managed knowledge files and must not remove `<instance>/mpb/schemes` unless the user explicitly selected scheme deletion.

If the base MPB mod is compatible but the knowledge fingerprint does not match, the patcher may still install or repair the base mod. It must report curated knowledge as unsupported/unavailable and must not install mismatched first-party knowledge. In Minecraft, the runtime keeps the pack inactive, `mpb_knowledge_status` reports unsupported/unavailable, the other read-only knowledge tools return unsupported, and the copied agent prompt says not to claim curated modpack support.

End users do not manage source records or lab artifacts. Their flow is: download the MPB Patcher, select a Prism instance, install the managed mod plus any matching knowledge bundle, start Minecraft, open MPB Manager, copy the MCP endpoint and prompt, and connect an external agent.

## Release Gates

No first-party pack is trusted or shipped until all of these gates pass:

- `cargo run -p mpb-knowledge --bin mpb-knowledge -- validate-source <pack>/source` passes with zero unresolved coverage, placeholders, stale fingerprints, incomplete overlays, incomplete dependency chains, behavioral claims without runtime evidence, trusted worker-only claims, missing manifest metadata, or runtime bundle query gaps.
- `cargo run -p mpb-knowledge --bin mpb-knowledge -- build-bundle <pack>/source <pack>/bundle` produces the runtime bundle from validated source records.
- `cargo run -p mpb-knowledge --bin mpb-knowledge -- inspect-bundle <pack>/bundle/knowledge-index.json` confirms the expected pack id, schema version, coverage counts, and query indexes.
- Patcher validation covers install, repair, update, unpatch, unsupported fingerprint behavior, and unmanaged-file conflict handling.
- Java runtime validation covers bundle loading, checksum and metadata rejection, knowledge MCP tool schemas, successful read-only query responses, unsupported responses, and prompt text for active and inactive packs.
- Product validation includes a real Prism client smoke run through MPB Manager and the MCP endpoint; browser-only validation is not sufficient for release packaging.
