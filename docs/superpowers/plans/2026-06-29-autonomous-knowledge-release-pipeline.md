# Autonomous Knowledge Release Pipeline Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a resumable local developer-side pipeline that turns a supported local PrismLauncher modpack into a fully validated embedded MPB knowledge bundle and a user-facing GitHub release report or precise blocking report.

**Architecture:** Build a new orchestration layer inside `mpb-knowledge` instead of moving knowledge production into the end-user patcher. The orchestrator owns run state, approvals, checkpoints, preflight, clone management, extraction, lab execution, bundle generation, patcher integration validation, release preparation, and final reporting while delegating existing schema, fingerprint, bundle, worker, and lab primitives to focused modules.

**Tech Stack:** Rust 2021 workspace, `mpb-knowledge` CLI, SQLite run database through `rusqlite`, append-only JSONL event log, PrismLauncher local instance files, Java lab mod, Tauri patcher artifacts, GitHub Actions release workflow, GitHub CLI for approved publication.

---

## Scope Split

This spec spans several independent but connected subsystems. Implement it as a sequence of working slices, each with tests and a durable validation note:

1. Local artifact layout, run database, and event log.
2. Preflight and approval gates.
3. Target fingerprint, disposable clone, and launch/intervention checkpoints.
4. Pipeline phase state machine and resume.
5. Coverage obligation expansion and extraction diagnostics.
6. Worker runtime approval, evaluation, artifact recording, and gated fine-tuning decisions.
7. Runtime experiment suite, retry policy, and adapter-expansion blocker plans.
8. Bundle embedding and product validation.
9. Release builder, GitHub publication approval, release/blocking reports.
10. End-to-end acceptance run for `All of Create - Aeronautics`.

Do not publish a release from this pipeline until every phase has a persisted successful checkpoint for the exact target fingerprint and the final validation report has zero blocking findings.

## File Structure

- `crates/mpb-knowledge/src/run_state.rs`: SQLite schema, migrations, run ids, checkpoints, events, approvals, blockers, and artifact references.
- `crates/mpb-knowledge/src/preflight.rs`: local environment inspection, resource estimates, model cache inspection, keep-awake availability, and phase duration estimates.
- `crates/mpb-knowledge/src/approvals.rs`: explicit approval model for long runs, keep-awake mode, model downloads, fine-tuning, project code changes, and GitHub release publication.
- `crates/mpb-knowledge/src/target.rs`: original Prism instance inspection, exact fingerprint attachment, disposable clone creation, clone cleanup, launch probing, and manual intervention checkpoints.
- `crates/mpb-knowledge/src/orchestrator.rs`: resumable state machine and phase ordering for intake through report generation.
- `crates/mpb-knowledge/src/coverage.rs`: coverage obligations derived from extraction, bundle, worker, and lab results.
- `crates/mpb-knowledge/src/experiments.rs`: batch experiment plan, retry policy, lab observation acceptance, and flaky experiment blockers.
- `crates/mpb-knowledge/src/adapter_plan.rs`: structured proposed code-change plans for missing extractor/lab/validation support.
- `crates/mpb-knowledge/src/release.rs`: release preparation, validation command runner, GitHub Actions trigger inputs, release notes, checksums, unsigned warnings, and report assembly.
- `crates/mpb-knowledge/src/bin/mpb-knowledge.rs`: CLI commands for preflight, start, resume, approve, status, report, and release preparation.
- `crates/mpb-knowledge/tests/*`: focused Rust integration tests for each pipeline boundary.
- `.github/workflows/release.yml`: keep matrix build behavior, add knowledge release inputs/artifact assertions after local validation is committed.
- `.gitignore`: ignore durable local artifacts required by the spec.
- `docs/knowledge/autonomous-release-pipeline.md`: operator-facing command and artifact contract.
- `docs/validation/2026-06-29-autonomous-knowledge-release-pipeline.md`: validation evidence accumulated while implementing the pipeline.

---

### Task 1: Durable Run State And Local Artifact Layout

**Files:**
- Modify: `crates/mpb-knowledge/Cargo.toml`
- Modify: `crates/mpb-knowledge/src/lib.rs`
- Create: `crates/mpb-knowledge/src/run_state.rs`
- Create: `crates/mpb-knowledge/tests/run_state.rs`
- Modify: `.gitignore`
- Create: `docs/knowledge/autonomous-release-pipeline.md`
- Create: `docs/validation/2026-06-29-autonomous-knowledge-release-pipeline.md`

- [ ] Add dependencies to `crates/mpb-knowledge/Cargo.toml`: `rusqlite = { version = "0.32", features = ["bundled"] }`, `time = { version = "0.3", features = ["formatting", "macros"] }`, and `uuid = { version = "1", features = ["v4", "serde"] }`.
- [ ] Create `run_state.rs` with a `KnowledgeRunStore` that opens `knowledge/runs/<run-id>/run.sqlite3`, applies idempotent migrations, and writes an append-only `events.jsonl`.
- [ ] Use SQLite tables `runs`, `phase_checkpoints`, `approvals`, `blockers`, `artifact_refs`, and `events`; every row stores `run_id`, `target_fingerprint` where applicable, `created_at`, and enough JSON detail to resume without re-reading transient terminal output.
- [ ] Define run phases as an enum with these stable variants: `Intake`, `Preflight`, `Approvals`, `Fingerprint`, `Clone`, `Extraction`, `Drafting`, `ExperimentPlanning`, `AdapterExpansion`, `RuntimeVerification`, `Validation`, `Bundle`, `PatcherIntegration`, `ProductValidation`, `Release`, `Report`.
- [ ] Write `crates/mpb-knowledge/tests/run_state.rs` to prove migrations are idempotent, phase checkpoints can be resumed, event log order is append-only, and blockers survive closing and reopening the database.
- [ ] Update `.gitignore` with `knowledge/runs/`, `knowledge/model-cache/`, `knowledge/model-datasets/`, and `knowledge/prism-clones/` while keeping reviewable pack sources under `knowledge/packs/` tracked.
- [ ] Export run-state types from `crates/mpb-knowledge/src/lib.rs`.
- [ ] Document the artifact layout in `docs/knowledge/autonomous-release-pipeline.md`, including that raw worker outputs, raw lab logs, cloned instances, downloaded models, and run databases are local ignored artifacts.
- [ ] Start `docs/validation/2026-06-29-autonomous-knowledge-release-pipeline.md` with the run-state test commands and observed results.
- [ ] Run: `cargo test -p mpb-knowledge run_state`
- [ ] Run: `cargo test -p mpb-knowledge`
- [ ] Commit with message `feat: add durable knowledge run state`.

### Task 2: Preflight, Estimates, And Approval Gates

**Files:**
- Modify: `crates/mpb-knowledge/src/lib.rs`
- Create: `crates/mpb-knowledge/src/preflight.rs`
- Create: `crates/mpb-knowledge/src/approvals.rs`
- Create: `crates/mpb-knowledge/tests/preflight.rs`
- Create: `crates/mpb-knowledge/tests/approvals.rs`
- Modify: `crates/mpb-knowledge/src/bin/mpb-knowledge.rs`
- Modify: `docs/knowledge/autonomous-release-pipeline.md`
- Modify: `docs/validation/2026-06-29-autonomous-knowledge-release-pipeline.md`

- [ ] Define `PreflightReport` with CPU architecture, operating system, memory estimate, disk free estimate for the repository and Prism clone location, Java/Gradle/Rust/Node/pnpm/Tauri/GitHub CLI availability, Prism instance readability, expected clone size, extraction scale estimate, model cache status, keep-awake availability, and phase-by-phase duration estimates.
- [ ] Implement `run_preflight(instance_path, artifact_root)` so it never downloads models, never enables keep-awake mode, and never mutates the target instance.
- [ ] Add hardware-fit model planning as data, not action: `ModelNeed { task, candidate_label, expected_size_bytes, runtime_mode, hardware_fit, reason }`.
- [ ] Define `ApprovalKind` values for `LongRun`, `KeepAwake`, `ModelDownload`, `FineTuning`, `ProjectCodeChange`, and `GitHubReleasePublication`.
- [ ] Make approval records fingerprint-aware and revocable only by creating a newer denial/approval event; do not overwrite historical approval rows.
- [ ] Add CLI command `mpb-knowledge preflight <instance-path> --artifact-root knowledge` that prints a JSON preflight report and writes it to the run store when used with `--run-id`.
- [ ] Add CLI command `mpb-knowledge approve <run-id> <approval-kind> --reason <text>` that persists an approval event and exits with a clear error for unknown approval kinds.
- [ ] Add tests proving long-run work is blocked without `LongRun`, keep-awake is blocked without `KeepAwake`, model download is blocked without `ModelDownload`, fine-tuning is blocked without `FineTuning`, project edits are blocked without `ProjectCodeChange`, and GitHub publication is blocked without `GitHubReleasePublication`.
- [ ] Document approval examples and the rule that approvals are required even when the orchestrator is otherwise autonomous.
- [ ] Update the validation note with preflight fixture output and approval-gate test results.
- [ ] Run: `cargo test -p mpb-knowledge preflight`
- [ ] Run: `cargo test -p mpb-knowledge approvals`
- [ ] Commit with message `feat: add knowledge pipeline preflight approvals`.

### Task 3: Target Manager, Disposable Clone, And Launch Intervention Checkpoints

**Files:**
- Modify: `crates/mpb-knowledge/src/lib.rs`
- Create: `crates/mpb-knowledge/src/target.rs`
- Create: `crates/mpb-knowledge/tests/target_manager.rs`
- Modify: `crates/mpb-knowledge/src/bin/mpb-knowledge.rs`
- Modify: `docs/knowledge/autonomous-release-pipeline.md`
- Modify: `docs/validation/2026-06-29-autonomous-knowledge-release-pipeline.md`

- [ ] Implement `TargetManager::inspect_original(instance_path)` using the existing fingerprint functions and Prism metadata parsing without writing to the original instance.
- [ ] Implement `TargetManager::create_disposable_clone(run_id, instance_path)` that copies the target into `knowledge/prism-clones/<run-id>/instance`, records source and clone paths in `artifact_refs`, and verifies the original fingerprint remains unchanged after clone creation.
- [ ] Add clone patching hooks that can install lab/runtime instrumentation only into the clone path.
- [ ] Add cleanup policy values `KeepForDebugging`, `DeleteOnSuccess`, and `DeleteAfterReport`, persisted in the run database.
- [ ] Add launch probe results `Ready`, `ManualInterventionRequired`, `LauncherUnavailable`, and `LaunchFailed`; manual intervention records must include the OS, launcher command attempted, observed prompt/status text when available, and resume command.
- [ ] Add CLI command `mpb-knowledge target clone <run-id> <instance-path>` and `mpb-knowledge target probe-launch <run-id>`.
- [ ] Write tests with a fixture Prism instance proving clone creation preserves file content, patch hooks write only under `knowledge/prism-clones/<run-id>`, cleanup never touches the original, and manual intervention checkpoints are resumable.
- [ ] Document that the original Prism instance is read-only source input and all destructive work happens in the clone.
- [ ] Update the validation note with clone safety test output.
- [ ] Run: `cargo test -p mpb-knowledge target_manager`
- [ ] Commit with message `feat: add disposable Prism target manager`.

### Task 4: Resumable Orchestrator State Machine

**Files:**
- Modify: `crates/mpb-knowledge/src/lib.rs`
- Create: `crates/mpb-knowledge/src/orchestrator.rs`
- Create: `crates/mpb-knowledge/tests/orchestrator_resume.rs`
- Modify: `crates/mpb-knowledge/src/bin/mpb-knowledge.rs`
- Modify: `docs/knowledge/autonomous-release-pipeline.md`
- Modify: `docs/validation/2026-06-29-autonomous-knowledge-release-pipeline.md`

- [ ] Implement `KnowledgeReleaseOrchestrator` as a phase runner that loads the latest successful checkpoint, runs only the next required phase, records phase start/success/failure events, and exits with a blocking report path when a blocker is created.
- [ ] Make every phase idempotent by reading durable inputs from the run database and artifact references before doing filesystem work.
- [ ] Add `mpb-knowledge release start <instance-path> --pack-id <pack-id>` to create a run id, persist intake, run preflight, and stop before long-run work if approval is missing.
- [ ] Add `mpb-knowledge release resume <run-id>` to resume from the latest checkpoint and `mpb-knowledge release status <run-id>` to print phase, blockers, approval status, artifact paths, and next command.
- [ ] Ensure a process interruption after any phase can be simulated by reopening the run store and continuing without repeating completed clone, extraction, bundle, or release phases.
- [ ] Write tests that inject failures after `Preflight`, `Clone`, and `RuntimeVerification`, then prove resume continues from the next phase and does not duplicate append-only events except for the new resume event.
- [ ] Document the phase order from the spec verbatim and map each phase to CLI status names.
- [ ] Update the validation note with resume tests and a sample status output.
- [ ] Run: `cargo test -p mpb-knowledge orchestrator_resume`
- [ ] Commit with message `feat: orchestrate resumable knowledge releases`.

### Task 5: Coverage Obligations And Strict Extraction Blocking

**Files:**
- Modify: `crates/mpb-knowledge/src/extract.rs`
- Modify: `crates/mpb-knowledge/src/validation.rs`
- Modify: `crates/mpb-knowledge/src/lib.rs`
- Create: `crates/mpb-knowledge/src/coverage.rs`
- Create: `crates/mpb-knowledge/tests/coverage_obligations.rs`
- Modify: `crates/mpb-knowledge/src/orchestrator.rs`
- Modify: `docs/knowledge/autonomous-release-pipeline.md`
- Modify: `docs/validation/2026-06-29-autonomous-knowledge-release-pipeline.md`

- [ ] Define `CoverageObligation` records for discovered entities, mechanics, relationships, recipes, traits, overlays, configs, datapacks, scripts, resources, guide/manual/tooltip content, and behavior claims.
- [ ] Convert extraction diagnostics that affect discovered content into release blockers rather than warnings.
- [ ] Require every obligation to have accepted deterministic evidence for static claims or accepted runtime evidence for behavioral claims.
- [ ] Extend validation failures so unsupported source kinds, partial extraction, stale fingerprints, invalid bundle query indexes, missing clone/runtime validation, worker-only trust, internet-only trust, decompile-only trust, and flaky experiments all block release.
- [ ] Add a coverage summary persisted in the run database after extraction and updated after lab verification.
- [ ] Write tests that create synthetic extraction drafts with unsupported configs, unknown mechanics, incomplete relationships, behavioral claims without runtime observations, and stale fingerprints; each must produce a specific blocker code.
- [ ] Wire the orchestrator so `Extraction` and `Validation` phases stop with a blocking report when obligations remain uncovered.
- [ ] Document obligation semantics and the accepted evidence matrix.
- [ ] Update the validation note with the obligation blocker matrix.
- [ ] Run: `cargo test -p mpb-knowledge coverage_obligations`
- [ ] Run: `cargo test -p mpb-knowledge validation_gates`
- [ ] Commit with message `feat: enforce coverage obligations`.

### Task 6: Worker Runtime Artifacts, Evaluation, And Fine-Tuning Gate

**Files:**
- Modify: `crates/mpb-knowledge/src/workers.rs`
- Modify: `crates/mpb-knowledge/src/preflight.rs`
- Modify: `crates/mpb-knowledge/src/approvals.rs`
- Create: `crates/mpb-knowledge/tests/worker_runtime.rs`
- Modify: `crates/mpb-knowledge/src/orchestrator.rs`
- Modify: `docs/knowledge/model-workers.md`
- Modify: `docs/knowledge/autonomous-release-pipeline.md`
- Modify: `docs/validation/2026-06-29-autonomous-knowledge-release-pipeline.md`

- [ ] Define a pluggable `WorkerRuntime` trait for draft classification, claim extraction from local documentation, conflict detection, experiment proposal, lab-log summarization, and structured JSON/schema repair suggestions.
- [ ] Persist worker prompts, inputs, outputs, model identity, model checksum, evaluation fixture results, and corrections under `knowledge/runs/<run-id>/workers/` with database artifact references.
- [ ] Select the concrete local model file during preflight and approval; keep pack logic free of hardcoded model filenames.
- [ ] Add a fixture evaluation step that must pass before worker output can be used during a release run.
- [ ] Keep worker decisions untrusted until they are converted into deterministic extraction evidence or runtime lab evidence.
- [ ] Implement fine-tuning as a separate gated phase result with states `NotUsed`, `ProposedBecauseBaseEvaluationFailed`, `ApprovedAndRun`, `RejectedByUser`, and `BlockedByHardware`.
- [ ] Require `FineTuning` approval and a sufficient hardware-fit preflight result before any local fine-tuning run can start.
- [ ] Write tests proving model download requires approval, base evaluation failure proposes but does not run fine-tuning, fine-tuning cannot run without approval, and worker-only claims remain validation failures.
- [ ] Document the worker runtime contract and artifact privacy expectations.
- [ ] Update the validation note with worker gate test output.
- [ ] Run: `cargo test -p mpb-knowledge worker_runtime`
- [ ] Run: `cargo test -p mpb-knowledge worker_gate`
- [ ] Commit with message `feat: gate local worker runtime artifacts`.

### Task 7: Runtime Experiment Suite, Retry Policy, And Adapter Expansion Plans

**Files:**
- Modify: `crates/mpb-knowledge/src/lab.rs`
- Modify: `mods/mpb-knowledge-lab/common/src/main/java/com/mpb/lab/MpbLabExperimentRunner.java`
- Modify: `mods/mpb-knowledge-lab/common/src/main/java/com/mpb/lab/MpbLabObservation.java`
- Create: `crates/mpb-knowledge/src/experiments.rs`
- Create: `crates/mpb-knowledge/src/adapter_plan.rs`
- Create: `crates/mpb-knowledge/tests/experiments.rs`
- Create: `crates/mpb-knowledge/tests/adapter_plan.rs`
- Modify: `crates/mpb-knowledge/src/orchestrator.rs`
- Modify: `docs/knowledge/autonomous-release-pipeline.md`
- Modify: `docs/validation/2026-06-29-autonomous-knowledge-release-pipeline.md`

- [ ] Define `ExperimentPlan` batches derived from coverage obligations with deterministic setup, bounded ticks, before/after snapshots, retry policy, and required observation adapters.
- [ ] Record every experiment attempt, retry, accepted observation, rejected observation, and flake counter in the run database.
- [ ] Treat an experiment that exceeds retry policy as a release blocker with affected obligations and raw artifact paths.
- [ ] Extend the Java lab runner contract so generic operations stay stable while mechanic-specific adapters expose isolated observation hooks.
- [ ] Define `AdapterExpansionPlan` for missing extractor, lab adapter, tests, or validation-rule support; include files to change, affected obligations, proposed test command, and approval requirement.
- [ ] Wire `ProjectCodeChange` approval so the orchestrator can prepare a code-change plan without editing code and can only apply approved adapter work in a later implementation phase.
- [ ] Write tests proving unsupported mechanics produce adapter plans, adapter plans include affected obligation ids, project code changes are blocked without approval, and flaky experiments block release after retry exhaustion.
- [ ] Document deterministic experiment expectations and manual client requirements when Prism or OS prompts require intervention.
- [ ] Update the validation note with lab/adapter tests and Java compile command results when available.
- [ ] Run: `cargo test -p mpb-knowledge experiments`
- [ ] Run: `cargo test -p mpb-knowledge adapter_plan`
- [ ] Run: `javac --release 17 -encoding UTF-8 -d <tmpdir>/mpb-knowledge-lab-classes mods/mpb-knowledge-lab/common/src/main/java/com/mpb/lab/*.java`
- [ ] Commit with message `feat: plan runtime lab experiment batches`.

### Task 8: Bundle Embedding And Product Validation Phase

**Files:**
- Create: `crates/mpb-knowledge/src/release.rs`
- Modify: `crates/mpb-assets/src/knowledge_bundle.rs`
- Modify: `crates/mpb-assets/tests/patcher.rs`
- Modify: `src/patcher/patcherState.test.ts`
- Modify: `crates/mpb-knowledge/src/orchestrator.rs`
- Create: `crates/mpb-knowledge/tests/product_validation.rs`
- Modify: `docs/validation/first-party-knowledge-release-checklist.md`
- Modify: `docs/validation/2026-06-29-autonomous-knowledge-release-pipeline.md`

- [ ] Add a `ProductValidationReport` that records patcher install, update, repair, unpatch, exact fingerprint match, mismatched fingerprint behavior, MCP knowledge status, MCP search/entity/recipe/mechanic/evidence query results, and real cloned runtime validation status.
- [ ] Make the orchestrator's `Bundle` phase build the runtime bundle only after strict validation passes and write checksums plus compressed artifact size into the run database.
- [ ] Make the `PatcherIntegration` phase verify that the embedded bundle metadata points at the exact fingerprint and that mismatches install only the base MPB mod while reporting curated knowledge unavailable.
- [ ] Add tests that use fixture bundles to prove product validation blocks release when patcher install/repair/unpatch behavior fails or MCP query coverage is missing.
- [ ] Keep browser/Vite validation out of the release acceptance path unless it is explicitly supplemental; the product validation phase must prefer the Tauri desktop app and real Prism client evidence.
- [ ] Update the release checklist with the new orchestrator command names and report artifacts.
- [ ] Update the validation note with product validation test output and any unavailable manual desktop/Minecraft steps.
- [ ] Run: `cargo test -p mpb-knowledge product_validation`
- [ ] Run: `cargo test -p mpb-assets patcher`
- [ ] Run: `pnpm test src/patcher/patcherState.test.ts`
- [ ] Commit with message `feat: validate embedded knowledge release product`.

### Task 9: Release Builder, GitHub Publication Gate, And Reports

**Files:**
- Modify: `crates/mpb-knowledge/src/release.rs`
- Create: `crates/mpb-knowledge/tests/release_reports.rs`
- Modify: `crates/mpb-knowledge/src/lib.rs`
- Modify: `crates/mpb-knowledge/src/bin/mpb-knowledge.rs`
- Modify: `.github/workflows/release.yml`
- Create: `docs/knowledge/release-report-schema.md`
- Modify: `docs/knowledge/autonomous-release-pipeline.md`
- Modify: `docs/validation/2026-06-29-autonomous-knowledge-release-pipeline.md`

- [ ] Define `BlockingReport` with run id, target instance, fingerprint, failed phase, exact blocker, affected coverage obligations, accepted evidence, missing capability or approval, proposed code/model/fine-tuning/adapter/manual action, resume command, and local artifact paths.
- [ ] Define `ReleaseReport` with target pack identity, exact fingerprint, coverage summary, evidence summary by kind, model candidates, approvals, worker evaluations, fine-tuning decisions, experiment summary, retry statistics, generated source/bundle paths, checksums, compressed size, patcher validation, cloned runtime validation, MCP query validation, desktop artifact list, unsigned-app warnings, and GitHub release URL when published.
- [ ] Add report writers that emit JSON and Markdown under `knowledge/runs/<run-id>/reports/`.
- [ ] Add CLI commands `mpb-knowledge release report <run-id>` and `mpb-knowledge release prepare-github <run-id> --tag <tag>`.
- [ ] Require `GitHubReleasePublication` approval before invoking `gh`, creating a release, dispatching a release workflow, or publishing release notes.
- [ ] Keep release publication separate from local validation: the orchestrator may prepare a tag/release command and notes without publishing when approval or credentials are missing.
- [ ] Update `.github/workflows/release.yml` so manual dispatch can receive a knowledge run id, pack id, fingerprint, and report artifact path, and so release artifacts remain unsigned with explicit release-note warning text.
- [ ] Write tests proving blocking reports contain every required field, release reports contain every required field, unsigned warnings are present for macOS/Windows/Linux, and GitHub publication is blocked without approval.
- [ ] Document the report schemas and publication flow.
- [ ] Update the validation note with report snapshots from fixture runs.
- [ ] Run: `cargo test -p mpb-knowledge release_reports`
- [ ] Commit with message `feat: prepare knowledge release reports`.

### Task 10: End-To-End Acceptance Run For All Of Create - Aeronautics

**Files:**
- Modify: `knowledge/packs/all-of-create-aeronautics/source/manifest.json`
- Modify: `knowledge/packs/all-of-create-aeronautics/source/entities.jsonl`
- Modify: `knowledge/packs/all-of-create-aeronautics/source/claims.jsonl`
- Modify: `knowledge/packs/all-of-create-aeronautics/source/evidence.jsonl`
- Modify: `knowledge/packs/all-of-create-aeronautics/source/recipes.jsonl`
- Modify: `knowledge/packs/all-of-create-aeronautics/source/relationships.jsonl`
- Modify: `knowledge/packs/all-of-create-aeronautics/source/overlays.jsonl`
- Generate locally: `knowledge/packs/all-of-create-aeronautics/bundle/knowledge-index.json`
- Modify: `knowledge/packs/all-of-create-aeronautics/bundle/knowledge-index.json.gz`
- Modify: `crates/mpb-assets/src/knowledge_bundle.rs`
- Create: `docs/validation/2026-06-29-aoca-autonomous-release-run.md`
- Modify: `docs/validation/2026-06-29-autonomous-knowledge-release-pipeline.md`

- [ ] Run preflight against the selected local `All of Create - Aeronautics` Prism instance and record the exact command, report path, target fingerprint, estimated duration, disk estimate, model needs, and keep-awake availability.
- [ ] Request and record approvals for the long run and any keep-awake/model/fine-tuning/project-code-change/publication gate that is actually needed.
- [ ] Run `mpb-knowledge release start <aoca-instance-path> --pack-id all-of-create-aeronautics`, then use `mpb-knowledge release resume <run-id>` until the run reaches either a release report or a blocking report.
- [ ] If the run blocks on missing extractor or lab adapter support, commit the generated adapter plan to the validation note, implement the approved adapter work as a separate task, and resume the same run id after tests pass.
- [ ] Ensure every discovered entity, mechanic, relationship, recipe, trait, overlay, config/datapack/script/resource input, guide/manual/tooltip-derived claim, and behavioral claim has accepted evidence for the exact fingerprint.
- [ ] Build the source records and runtime bundle through the orchestrator, not through ad hoc commands, and verify the bundle checksum matches the report.
- [ ] Validate patcher install/update/repair/unpatch behavior and MCP knowledge queries against the cloned runtime.
- [ ] Prepare GitHub release artifacts only after local validation is committed; publish only with explicit GitHub publication approval and available credentials.
- [ ] Write `docs/validation/2026-06-29-aoca-autonomous-release-run.md` with the final release report or blocking report, including exact local paths needed to resume.
- [ ] Run: `cargo run -p mpb-knowledge --bin mpb-knowledge -- release status <run-id>`
- [ ] Run: `cargo run -p mpb-knowledge --bin mpb-knowledge -- release report <run-id>`
- [ ] Run: `cargo test --workspace`
- [ ] Run: `pnpm test`
- [ ] Run: `mods/mpb-minecraft-mod/build.sh`
- [ ] Run the release Tauri desktop app and real Prism client validation; if the current machine cannot launch it, stop publication and record the manual blocker in the validation note.
- [ ] Commit with message `feat: validate autonomous aoca knowledge release`.

---

## Required Execution Order

1. Finish Tasks 1-2 before any long-running pipeline work; run state and approvals are the safety boundary.
2. Finish Task 3 before runtime verification; all lab work must happen in a disposable clone.
3. Finish Task 4 before integrating extraction, worker, lab, bundle, and release phases; every later task relies on resume/checkpoint semantics.
4. Finish Task 5 before trusting any pack output; coverage obligations are the release gate.
5. Finish Tasks 6-7 before claiming behavioral knowledge support; worker output and runtime experiments must be gated and auditable.
6. Finish Task 8 before release preparation; embedded bundle behavior must be product-validated.
7. Finish Task 9 before publication; reports and GitHub approval gates are required by the spec.
8. Finish Task 10 only after all generic pipeline phases are implemented and passing.

## Validation Matrix

- Run state and resume: `cargo test -p mpb-knowledge run_state orchestrator_resume`
- Preflight and approvals: `cargo test -p mpb-knowledge preflight approvals`
- Target clone safety: `cargo test -p mpb-knowledge target_manager`
- Coverage and trust gates: `cargo test -p mpb-knowledge coverage_obligations validation_gates worker_gate`
- Worker runtime: `cargo test -p mpb-knowledge worker_runtime`
- Runtime experiments and adapter plans: `cargo test -p mpb-knowledge experiments adapter_plan`
- Product integration: `cargo test -p mpb-knowledge product_validation`, `cargo test -p mpb-assets patcher`, `pnpm test src/patcher/patcherState.test.ts`
- Release reports: `cargo test -p mpb-knowledge release_reports`
- Full local gate: `cargo test --workspace`, `pnpm test`, `mods/mpb-minecraft-mod/build.sh`
- Desktop/product gate: release Tauri app plus real PrismLauncher cloned runtime validation, recorded in `docs/validation/`.
- AOC acceptance: `mpb-knowledge release report <run-id>` must produce either a complete release report or a blocking report with resume instructions.

## Spec Coverage Review

- Product contract: Tasks 1, 2, 4, 5, 8, 9, and 10 enforce read-only original input, explicit approvals, complete coverage, trusted evidence, resumability, embedded bundles, and final reporting.
- Local-first boundaries: Tasks 2, 6, and 9 keep model execution local, cloud GPU work out of scope, and GitHub publication behind approval.
- Release orchestrator: Tasks 1 and 4 implement run ids, checkpoints, phase ordering, blockers, progress/status, cleanup policy, resume, and reports.
- Environment preflight: Task 2 covers hardware, toolchains, Prism readability, model cache state, clone/extraction scale, estimates, and keep-awake gating.
- Target manager: Task 3 covers exact fingerprinting, disposable clone creation, clone patching, launch probing, manual interventions, and cleanup.
- Extractors and coverage obligations: Task 5 converts discovered content into strict obligations and release blockers.
- Worker runtime: Task 6 records prompts/outputs/model identity/evaluation/corrections and prevents worker-only trust.
- Internet/code analysis trust boundary: Task 5 keeps draft-only sources from becoming trusted claims without deterministic or runtime evidence.
- Lab runner and adapters: Task 7 covers bounded experiments, snapshots, retry policy, structured observations, missing adapter plans, and project-code approval.
- Bundle and patcher integration: Task 8 covers strict bundle generation, compressed artifacts, exact fingerprint install behavior, and MCP query validation.
- Release builder: Task 9 covers validation, tests, GitHub Actions artifacts, unsigned warnings, checksums, approval-gated publication, and reports.
- Acceptance candidate: Task 10 proves the generic flow on `All of Create - Aeronautics`.
