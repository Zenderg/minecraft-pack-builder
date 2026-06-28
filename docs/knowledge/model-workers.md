# Model Workers

Model workers are developer-side assistants for first-party knowledge-pack production. They may draft
classifications, summarize extracted or lab data, detect conflicts, and propose experiments. They are
never a source of truth.

The first worker candidate is `Qwen2.5-Coder-1.5B-Instruct` for structured transformation and
classification tasks:

- draft classification;
- summarization;
- conflict detection.

`Qwen3-1.7B` and `Qwen3-4B` are reserved for broader reasoning experiments, especially experiment
proposal drafts, when the first candidate is insufficient.

Every worker output is wrapped in an envelope recording the task kind, model, exact input
fingerprint, prompt reference, output reference, and fine-tuning decision. Fine-tuning decisions are:

- no fine-tuning used, with the reason recorded;
- fine-tuning used for a named worker task, with model, dataset, evaluation threshold, and result;
- fine-tuning required because worker quality blocks the pack.

Start without fine-tuning. Record prompts, outputs, corrections, and experiment outcomes as future
training data, but keep raw local traces out of shipped runtime bundles. A worker-only claim cannot
become trusted knowledge. It must be converted into deterministic extraction or runtime evidence and
then pass the same strict validation gates as all other source records.
