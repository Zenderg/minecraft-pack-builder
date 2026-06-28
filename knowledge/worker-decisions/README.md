# Worker Decisions

This directory records durable metadata about developer-side model-worker experiments for curated
knowledge packs.

Workers are assistants only. Their prompts, raw outputs, corrections, and experiment outcomes may be
used as future training data, but worker output is not trusted knowledge. Trusted claims must be
backed by deterministic source evidence or accepted runtime lab evidence for the exact fingerprint.

Use `knowledge/worker-decisions/local/` for raw prompt/output/correction traces during experiments.
That folder is ignored and must not be shipped in runtime bundles.
