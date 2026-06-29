# Autonomous Knowledge Release Pipeline

The autonomous knowledge release pipeline is a local developer-side workflow for turning a supported PrismLauncher instance into a validated embedded MPB knowledge bundle and release report. It keeps the original Prism instance as read-only input; durable run state and large transient artifacts live under the local `knowledge/` artifact tree.

## Artifact Layout

- `knowledge/runs/<run-id>/run.sqlite3` stores resumable run state for one pipeline run.
- `knowledge/runs/<run-id>/events.jsonl` is an append-only event log mirroring important state transitions, blockers, approvals, and artifact references.
- `knowledge/runs/<run-id>/workers/` is reserved for raw local worker prompts, inputs, outputs, model identity, checksums, fixture-evaluation results, and corrections.
- `knowledge/runs/<run-id>/reports/` is reserved for generated blocking and release reports.
- `knowledge/model-cache/` stores downloaded or prepared local model files after explicit approval.
- `knowledge/model-datasets/` stores local fine-tuning and evaluation datasets after explicit approval.
- `knowledge/prism-clones/<run-id>/` stores disposable PrismLauncher clones used for lab instrumentation and runtime validation.
- `knowledge/packs/` remains the reviewable source tree for curated knowledge packs and is intentionally not ignored as a whole.

The local ignored artifacts include raw worker outputs, raw lab logs, cloned Prism instances, downloaded models, model datasets, and run databases. These files are durable for local resume/debugging, but they are not reviewable source artifacts.

## Run State Contract

Each run is addressed by a stable run id. The run store applies idempotent SQLite migrations and creates these tables:

- `runs`
- `phase_checkpoints`
- `approvals`
- `blockers`
- `artifact_refs`
- `events`

Rows include the `run_id`, a `created_at` timestamp, JSON details with enough context to resume without terminal output, and a `target_fingerprint` where the row is tied to a specific pack fingerprint.

The stable phase order is: `Intake`, `Preflight`, `Approvals`, `Fingerprint`, `Clone`, `Extraction`, `Drafting`, `ExperimentPlanning`, `AdapterExpansion`, `RuntimeVerification`, `Validation`, `Bundle`, `PatcherIntegration`, `ProductValidation`, `Release`, `Report`.
