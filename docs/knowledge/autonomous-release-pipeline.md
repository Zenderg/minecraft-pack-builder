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

## Release Orchestrator

The release orchestrator advances durable runs one required phase at a time. It determines progress by scanning successful checkpoints in the stable phase order above, not by trusting the most recent checkpoint row. That keeps resume behavior stable when a later command records additional diagnostic checkpoints for an earlier phase.

Start a release run with:

```text
cargo run -p mpb-knowledge --bin mpb-knowledge -- release start <instance-path> --pack-id <pack-id> --artifact-root knowledge
```

`release start` creates a run id, records the `Intake` checkpoint, writes the `Preflight` report, and stops at `Approvals` until `LongRun` approval is explicitly recorded. The command prints an `OrchestratorOutcome` JSON object with these status names:

- `PhaseSucceeded`
- `Blocked`
- `Complete`

Resume the next required phase with:

```text
cargo run -p mpb-knowledge --bin mpb-knowledge -- release resume <run-id> --artifact-root knowledge
```

Inspect durable state with:

```text
cargo run -p mpb-knowledge --bin mpb-knowledge -- release status <run-id> --artifact-root knowledge
```

Status output includes `latestSuccessfulPhase`, `nextPhase`, blockers, approval status, artifact references, and the next command to run. CLI phase names match the stable Rust phase variants verbatim:

- `Intake`
- `Preflight`
- `Approvals`
- `Fingerprint`
- `Clone`
- `Extraction`
- `Drafting`
- `ExperimentPlanning`
- `AdapterExpansion`
- `RuntimeVerification`
- `Validation`
- `Bundle`
- `PatcherIntegration`
- `ProductValidation`
- `Release`
- `Report`

Implemented phases are idempotent. Preflight reuses an existing `preflight-report` artifact, clone resume reuses an existing `target-clone` artifact instead of deleting and recreating the disposable clone, and unsupported future phases create a blocking report under `knowledge/runs/<run-id>/reports/` rather than pretending the release is complete.

## Preflight

Run preflight before any long-running or mutating pipeline work:

```text
cargo run -p mpb-knowledge --bin mpb-knowledge -- preflight <instance-path> --artifact-root knowledge
```

To persist the JSON preflight report into a run store:

```text
cargo run -p mpb-knowledge --bin mpb-knowledge -- preflight <instance-path> --artifact-root knowledge --run-id <run-id>
```

Preflight inspects local CPU architecture, operating system, memory, disk estimates, tool availability, Prism instance readability, clone size, extraction scale, model cache status, keep-awake availability, phase duration estimates, and model needs. It never downloads models, never enables keep-awake mode, and never mutates the target Prism instance.

Model planning is reported as data through `ModelNeed` records. A `ModelNeed` names the task, candidate label, expected model size, runtime mode, hardware fit, and reason; it is not permission to download or run the model.

## Approval Gates

Approvals are required even when the orchestrator is otherwise autonomous. The stable approval kinds are:

- `LongRun`
- `KeepAwake`
- `ModelDownload`
- `FineTuning`
- `ProjectCodeChange`
- `GitHubReleasePublication`

Record a fingerprint-scoped approval with:

```text
cargo run -p mpb-knowledge --bin mpb-knowledge -- approve <run-id> LongRun --artifact-root knowledge --target-fingerprint <fingerprint> --reason "operator approved the local long-running release"
```

Approval rows are append-only and fingerprint-aware. A later denial or approval is a new event; historical rows are not overwritten. Gate checks use the newest matching approval row for the exact approval kind and target fingerprint.

For `release start` / `release resume`, approve the initial long-running local work before fingerprinting with a run-scoped approval:

```text
cargo run -p mpb-knowledge --bin mpb-knowledge -- approve <run-id> LongRun --artifact-root knowledge --reason "operator approved the local long-running release"
```

## Target Clone And Launch Checkpoints

The original PrismLauncher instance is read-only source input. The target manager computes the exact fingerprint from the original instance metadata and Minecraft input files, then copies the instance to:

```text
knowledge/prism-clones/<run-id>/instance
```

All destructive work, lab instrumentation, runtime probes, and cleanup operate only inside that disposable clone path. Clone creation records both `target-original` and `target-clone` artifact references in the run database and verifies that the original fingerprint is unchanged after copying.

Create a clone with:

```text
cargo run -p mpb-knowledge --bin mpb-knowledge -- target clone <run-id> <instance-path> --artifact-root knowledge
```

Clone cleanup policy values are:

- `KeepForDebugging`
- `DeleteOnSuccess`
- `DeleteAfterReport`

The default policy is `DeleteAfterReport`, so artifacts remain available until the final release or blocking report has been produced.

Launch probing is persisted as a resumable checkpoint:

```text
cargo run -p mpb-knowledge --bin mpb-knowledge -- target probe-launch <run-id> --artifact-root knowledge
```

Probe results are `Ready`, `ManualInterventionRequired`, `LauncherUnavailable`, or `LaunchFailed`. Manual intervention records include the operating system, launcher command attempted, observed prompt/status text when available, and an exact resume command. The CLI does not mutate or launch the original instance; it reads the clone artifact from the run store and targets only the disposable clone.
