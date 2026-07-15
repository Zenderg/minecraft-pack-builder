# Worker Decisions

This document is the source of truth for ownership and retention rules for developer-side
worker-decision artifacts under `knowledge/worker-decisions/`. Worker trust and evaluation policy
belongs in [`docs/knowledge/model-workers.md`](../../docs/knowledge/model-workers.md), release
sequencing in the [pipeline operator guide](../../docs/knowledge/autonomous-release-pipeline.md), and
pack-specific reviewable decisions in each pack's `source/worker-decisions.jsonl`.

Workers are assistants only. Their prompts, raw outputs, corrections, and experiment outcomes may be
used as future training data, but worker output is not trusted knowledge. Trusted claims must be
backed by deterministic source evidence or accepted runtime lab evidence for the exact fingerprint.

Use `knowledge/worker-decisions/local/` for raw prompt/output/correction traces during experiments.
That folder is ignored and must not be shipped in runtime bundles.
