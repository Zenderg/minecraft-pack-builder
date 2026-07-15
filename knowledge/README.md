# Knowledge Artifact Tree

The `knowledge/` tree contains reviewable pack sources alongside generated and local release
artifacts. This document is the source of truth for directory ownership, tracking, and retention;
stable knowledge contracts live under [`docs/knowledge`](../docs/knowledge/README.md), and
pack-specific operator guidance lives in each pack directory.

## Reviewable Sources

- `packs/<pack-id>/source/` contains manifest, entity, claim, evidence, recipe, overlay, relationship, and worker-decision records intended for review and version control.
- `packs/<pack-id>/README.md` contains stable human guidance for that pack; machine-readable identity remains in `source/manifest.json`.
- `packs/<pack-id>/bundle/knowledge-index.json.gz` may be tracked when the patcher intentionally embeds that compressed artifact.
- `packs/fixtures/` contains small committed source and bundle fixtures used by automated tests.
- `worker-decisions/README.md` defines retention rules for worker experiment metadata.

## Local And Generated Artifacts

The following paths are ignored and must not become reviewable sources of truth:

- `runs/` stores resumable SQLite state, events, generated reports, and run-scoped worker artifacts.
- `lab-artifacts/` stores raw experiment logs, snapshots, screenshots, saves, and notebooks.
- `model-cache/` and `model-datasets/` store approved local model files and training/evaluation data.
- `prism-clones/` stores disposable PrismLauncher validation clones.
- `worker-decisions/local/` stores raw prompts, outputs, corrections, and experiment traces.
- Production `packs/*/bundle/knowledge-index.json` files are regenerated locally; only fixture JSON bundles are tracked.

Durable conclusions discovered while operating the pipeline belong in the owning product,
architecture, knowledge, module, or validation document. Do not promote raw run reports or local
artifact paths into long-lived documentation.
