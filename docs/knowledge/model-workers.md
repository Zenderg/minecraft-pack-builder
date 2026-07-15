# Model Workers

This document is the source of truth for model-worker roles, trust boundaries, evaluation
requirements, artifact envelopes, and fine-tuning policy. Pipeline sequencing and approvals belong
in the [operator guide](autonomous-release-pipeline.md); raw local traces follow the retention rules
in [`knowledge/worker-decisions`](../../knowledge/worker-decisions/README.md).

Model workers are developer-side assistants for first-party knowledge-pack production. They may draft
classifications, extract candidate claims from local documentation, summarize extracted or lab data,
detect conflicts, propose experiments, summarize lab logs, and suggest structured JSON/schema
repairs. They are never a source of truth.

The first worker candidate is `Qwen2.5-Coder-1.5B-Instruct` for structured transformation and
classification tasks:

- draft classification;
- summarization;
- conflict detection.

`Qwen3-1.7B` and `Qwen3-4B` are examples of broader reasoning candidates, especially experiment
proposal drafts, when the first candidate is insufficient. The release pipeline records the concrete
local model identity, path, checksum, and hardware fit during preflight/approval; pack logic must not
hardcode a model filename.

Every worker output is wrapped in an envelope recording the task kind, model identity, model
checksum, exact input fingerprint, prompt reference, output reference, and fine-tuning decision.
The durable run also writes the raw prompt, input, output, model identity, fixture-evaluation
result, and corrections under:

```text
knowledge/runs/<run-id>/workers/<worker-id>/
```

These files are local ignored artifacts. They can be used for resume, debugging, evaluation, and
future training data, but they must not be embedded into runtime bundles or committed as release
evidence.

Worker fixture evaluation must pass before worker output can be consumed by a release run. Worker
decisions remain untrusted until converted into deterministic extraction evidence or accepted
runtime lab evidence.

Fine-tuning phase states are:

- `NotUsed`
- `ProposedBecauseBaseEvaluationFailed`
- `ApprovedAndRun`
- `RejectedByUser`
- `BlockedByHardware`

Start without fine-tuning. A failed base fixture evaluation may propose fine-tuning, but it does not
run fine-tuning by itself. A local fine-tuning run requires both `FineTuning` approval for the exact
target fingerprint and a sufficient preflight hardware-fit result. A worker-only claim cannot become
trusted knowledge. It must be converted into deterministic extraction or runtime evidence and then
pass the same strict validation gates as all other source records.
