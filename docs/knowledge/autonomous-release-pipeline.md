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

Implemented phases are idempotent. Preflight reuses an existing `preflight-report` artifact, clone resume reuses an existing `target-clone` artifact instead of deleting and recreating the disposable clone, worker drafting reuses an existing `worker-output` artifact, experiment planning reuses an existing `experiment-plan` artifact, and adapter expansion reuses an existing `adapter-expansion-plan` artifact. Unsupported future phases create a blocking report under `knowledge/runs/<run-id>/reports/` rather than pretending the release is complete.

## Bundle Embedding And Product Validation

The `Bundle` phase runs only after strict validation has succeeded. It accepts either a persisted `knowledge-source-dir` artifact or a `knowledge-source-pack` artifact that can be materialized back into the source-record layout. The phase rebuilds `knowledge-index.json`, writes `knowledge-index.json.gz`, and records a `runtime-bundle` artifact with:

- exact fingerprint;
- pack id and schema version;
- uncompressed checksum and size;
- compressed artifact path and compressed size.

The `PatcherIntegration` phase verifies that the runtime bundle manifest and artifact reference both point at the exact target fingerprint. If they do not, the run blocks with `PATCHER_BUNDLE_FINGERPRINT_MISMATCH`. The phase also requires exact-fingerprint patcher evidence, either as a dedicated `patcher-validation-evidence` artifact or as the patcher section of the shared `product-validation-evidence` artifact. That evidence must prove exact-match metadata and mismatched-fingerprint behavior for the current target fingerprint. A fingerprint mismatch must install only the base MPB mod and report curated knowledge unavailable; the patcher package owns that behavior and keeps it covered with `cargo test -p mpb-assets --test patcher`.

The `ProductValidation` phase requires a `product-validation-evidence` artifact containing structured Tauri desktop, patcher, MCP, and cloned runtime results. It writes `reports/product-validation-report.json` and records a `product-validation-report` artifact. Browser/Vite checks may be attached as supplemental evidence, but they are never sufficient for release acceptance.

The product report records:

- patcher install, update, repair, unpatch, exact-fingerprint match, and mismatched-fingerprint behavior;
- MCP knowledge status plus search/entity/recipe/mechanic/evidence query checks;
- runtime bundle query coverage against the generated bundle indexes;
- real cloned Prism/Minecraft runtime validation status;
- Tauri desktop validation status;
- release-blocking product validation findings.

## Release Reports And GitHub Preparation

Generate the final local report pair with:

```text
cargo run -p mpb-knowledge --bin mpb-knowledge -- release report <run-id> --artifact-root knowledge
```

The command writes `knowledge/runs/<run-id>/reports/release-report.json` and `knowledge/runs/<run-id>/reports/release-report.md`, records a `release-report` artifact reference, and does not publish anything. Blocking phases write `blocking-<id>-<phase>.json` and `.md` through the same report contract. The JSON schemas are documented in `docs/knowledge/release-report-schema.md`.

Prepare the GitHub publication command and local notes after local validation has been committed:

```text
cargo run -p mpb-knowledge --bin mpb-knowledge -- release prepare-github <run-id> --artifact-root knowledge --tag <tag>
```

`release prepare-github` writes local release notes and prints the `gh workflow run release.yml` command with the knowledge run id, pack id, fingerprint, and report artifact path. It does not invoke `gh`, create a release, dispatch the workflow, or publish release notes. Missing credentials or missing approval therefore produce a prepared local command/report rather than a failed or partial publication.

Before an operator runs the prepared `gh` command or otherwise publishes release notes, record exact-fingerprint publication approval:

```text
cargo run -p mpb-knowledge --bin mpb-knowledge -- approve <run-id> GitHubReleasePublication --artifact-root knowledge --target-fingerprint <fingerprint> --reason "operator approved publishing the prepared release"
```

Prepared output includes `publicationApproved` and `missingApproval` so publication tooling can hard-stop without discarding the local release report.

The release workflow accepts manual inputs for `knowledge_run_id`, `pack_id`, `fingerprint`, and `report_artifact_path`. When a report path is provided, the workflow asserts that it exists and uploads it as a release artifact. Release builds intentionally omit Tauri signing secrets and include explicit unsigned-app warning text for macOS, Windows, and Linux in release notes/artifacts.

## Coverage Obligations

Extraction produces durable coverage obligations for discovered entities, mechanics, relationships, recipes, traits, overlays, configs, datapacks, scripts, resources, guide/manual/tooltip content, static claims, and behavioral claims. The pipeline persists the obligation summary as a `coverage-summary` artifact under `knowledge/runs/<run-id>/coverage/` and records a `coverage.summary` event in the run database.

Extraction diagnostics that affect discovered content are release blockers. A blocking diagnostic for a config, datapack, script, resource pack, guidebook, manual, tooltip, registry, blockstate, recipe, tag, or language source is recorded as `UNSUPPORTED_SOURCE_KIND` until deterministic collector support exists for that source.

Accepted evidence requirements are strict:

- Static claims require accepted deterministic local source evidence, or accepted local manual/documentation evidence that has been converted into deterministic extraction evidence.
- Behavioral claims require accepted runtime observations from the exact target fingerprint.
- Relationships, overlays, mechanic traits, and recipes must reference accepted evidence and complete entity chains.
- Worker output, internet-only sources, decompile-only sources, stale-fingerprint artifacts, partial extraction, missing clone/runtime validation, invalid bundle query indexes, and flaky experiments are blocking release conditions.

The `Extraction` phase reads a persisted `extraction-draft` artifact, writes the coverage summary, and stops with a blocking report when any obligation is uncovered. The `Validation` phase rereads the persisted coverage summary after runtime verification and stops with the same specific blocker code if obligations remain uncovered.

## Worker Runtime

Worker runtime work is local, resumable, and untrusted by default. The `Drafting` phase requires a selected `worker-model` artifact reference with a model identity, checksum, file path, and hardware-fit detail. If the referenced model file is missing, `ModelDownload` approval is required before any download or preparation can occur. The phase records:

- `worker-prompt`
- `worker-input`
- `worker-output`
- `worker-model-identity`
- `worker-evaluation`
- `worker-corrections`

Each artifact is stored under `knowledge/runs/<run-id>/workers/<worker-id>/` and linked in the run database. Fixture evaluation must pass before worker output can be used by the pipeline. Base-evaluation failure may produce a fine-tuning proposal, but local fine-tuning cannot run unless the exact target fingerprint has `FineTuning` approval and preflight reports sufficient hardware fit.

Worker tasks are intentionally broad enough for the release workflow:

- draft classification
- local documentation claim extraction
- conflict detection
- experiment proposal
- lab-log summarization
- structured JSON/schema repair suggestions

Worker decisions remain draft-only. Validation still blocks worker-only claims, trusted worker decisions, and converted evidence ids that do not point to accepted non-worker evidence.

## Runtime Experiments And Adapter Plans

The `ExperimentPlanning` phase derives deterministic experiment batches from uncovered runtime coverage obligations. Each experiment records deterministic setup, bounded ticks, before/after snapshot operations, retry policy, and required observation adapters. The generic lab operation contract stays stable; mechanic-specific behavior belongs in isolated observation adapters.

Experiment attempts are durable `experiment-attempt` artifacts under `knowledge/runs/<run-id>/lab/`. Each attempt records its experiment id, attempt number, accepted/rejected/failed status, optional accepted observation, raw artifact path, and message. If all attempts are retryable failures and the retry policy is exhausted, the pipeline records `FLAKY_EXPERIMENT_RETRY_EXHAUSTED` with affected obligation ids and raw artifact paths.

The `AdapterExpansion` phase can prepare proposed code-change plans without editing project code. Plans include:

- files to change;
- affected obligation ids;
- proposed test command;
- required approval kind.

Any later implementation of extractor, lab adapter, test, or validation-rule support requires `ProjectCodeChange` approval. Without that approval, the pipeline can report the required plan but must not apply project edits.

When PrismLauncher, Minecraft, or the operating system requires manual client interaction, the runtime experiment workflow must persist the prompt/status and resume command rather than substituting browser-only validation.

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
