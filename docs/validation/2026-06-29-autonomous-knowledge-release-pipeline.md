# Autonomous Knowledge Release Pipeline Validation

## 2026-06-29 Task 1: Durable Run State

Red phase:

```text
cargo test -p mpb-knowledge run_state
```

Observed result: failed to compile because `KnowledgeRunStore`, `KnowledgeRunPhase`, `PhaseCheckpointStatus`, and `RunBlockerInput` were not yet exported.

Green phase:

```text
cargo test --offline -p mpb-knowledge run_state
```

Observed result: passed. The command ran 4 `run_state_*` tests covering idempotent migrations, checkpoint resume, append-only SQLite/JSONL events, and blocker persistence.

```text
cargo test --offline -p mpb-knowledge
```

Observed result: passed. The full `mpb-knowledge` suite ran 22 integration tests plus crate/doc test targets with no failures.

Note: Cargo was run with `--offline` because the new dependencies were already present in the local Cargo cache, while sandboxed network access to crates.io was unavailable.

## 2026-06-29 Task 2: Preflight And Approval Gates

Red phase:

```text
cargo test --offline -p mpb-knowledge preflight
cargo test --offline -p mpb-knowledge approvals
```

Observed result: failed to compile because `run_preflight`, `HardwareFit`, `ApprovalKind`, and approval methods on `KnowledgeRunStore` were not yet exported.

Green phase:

```text
cargo test --offline -p mpb-knowledge preflight
```

Observed result: passed. The command ran 2 preflight tests covering read-only fixture inspection, model-cache non-mutation, JSON CLI output, and persisted `Preflight` checkpoint state for `--run-id`.

```text
cargo test --offline -p mpb-knowledge approvals
```

Observed result: passed. The command ran 3 approval tests covering all six missing-approval gates, exact fingerprint matching, append-only revocation by newer denial, unknown CLI approval kind errors, and persisted CLI approval events.

```text
cargo test --offline -p mpb-knowledge
```

Observed result: passed. The full `mpb-knowledge` suite ran 27 integration tests plus crate/doc test targets with no failures.

## 2026-06-29 Task 3: Target Manager, Disposable Clone, And Launch Checkpoints

Red phase:

```text
cargo test -p mpb-knowledge target_manager
```

Observed result: failed to compile because `CleanupPolicy`, `LaunchProbeResult`, `TargetManager`, and artifact-reference reading on `KnowledgeRunStore` did not exist.

Green phase:

```text
cargo test -p mpb-knowledge target_manager
```

Observed result: passed. The command ran 4 `target_manager_*` tests covering read-only original inspection, disposable clone creation and artifact references, clone-only instrumentation hooks, cleanup confinement, CLI clone/probe commands, and resumable manual intervention launch checkpoints.

```text
cargo fmt --check
```

Observed result: passed after formatting the new target manager module and tests.

```text
cargo test -p mpb-knowledge
```

Observed result: passed. The full `mpb-knowledge` suite ran 31 integration tests plus crate/doc test targets with no failures.

## 2026-06-29 Task 4: Resumable Orchestrator State Machine

Red phase:

```text
cargo test -p mpb-knowledge orchestrator_resume
```

Observed result: failed to compile because `KnowledgePhaseRunner`, `KnowledgeReleaseOrchestrator`, `OrchestratorError`, `PhaseRunContext`, and `PhaseRunStatus` were not yet exported.

Green phase:

```text
cargo test -p mpb-knowledge --test orchestrator_resume
```

Observed result: passed. The command ran 3 integration tests covering phase-order resume after simulated interruptions at `Preflight`, `Clone`, and `RuntimeVerification`; `release start` intake/preflight persistence and missing-`LongRun` blocking report generation; and `release resume` idempotency for an existing disposable clone artifact.

Post-review regression expansion:

```text
cargo test -p mpb-knowledge --test orchestrator_resume
```

Observed result: passed. The command now runs 7 integration tests, adding coverage for durable failed checkpoints/events when a phase runner returns an error, `Fingerprint` resume from a durable `target-original` artifact without touching the original instance, `Preflight` rebuilding a missing report artifact instead of treating artifact metadata as the report, `Clone` blocking when the source instance fingerprint changes after the `Fingerprint` phase, and `release status` suggesting `release resume` after a historical approval blocker has been satisfied.

```text
cargo fmt --check
```

Observed result: passed after formatting the new orchestrator module, CLI additions, and tests.

```text
cargo test -p mpb-knowledge
```

Observed result: passed. The full `mpb-knowledge` suite ran 35 integration tests plus crate/doc test targets with no failures.

Sample `release status` shape after `release start` blocks at the approval gate:

```json
{
  "latestSuccessfulPhase": "Preflight",
  "nextPhase": "Approvals",
  "blockers": [
    {
      "code": "MISSING_LONG_RUN_APPROVAL"
    }
  ],
  "nextCommand": "mpb-knowledge approve <run-id> LongRun --artifact-root <artifact-root> --reason <text>"
}
```

Durable implementation note: the orchestrator computes progress by scanning successful checkpoints in the stable phase order. It intentionally does not use the newest successful checkpoint row as the source of truth, because launch probes or other diagnostics may append a later checkpoint for an earlier phase.

## 2026-06-29 Task 5: Coverage Obligations And Strict Extraction Blocking

Red phase:

```text
cargo test -p mpb-knowledge coverage_obligations
cargo test -p mpb-knowledge validation_gates
```

Observed result: failed to compile because `evaluate_extraction_coverage`, `persist_coverage_summary`, `ExtractedDraftRecord`, new `CoverageSummary` release-gate fields, new `EvidenceKind` trust-source variants, and new `ValidationCode` blocker variants did not exist yet.

Green phase:

```text
cargo test -p mpb-knowledge --test coverage_obligations
```

Observed result: passed. The command ran 8 tests covering unsupported config diagnostics, unknown mechanics, incomplete relationships, behavioral claims without runtime observations, stale fingerprints, durable `coverage-summary` persistence, `Extraction` phase blocking reports, and `Validation` phase blocking from persisted uncovered obligations.

Post-review expansion: the command now runs 9 tests, adding a guard that `Validation` blocks with `VALIDATION_SOURCE_PACK_MISSING` when coverage obligations pass but no persisted `knowledge-source-pack` artifact exists.

The exact plan commands were also rerun after implementation:

```text
cargo test -p mpb-knowledge coverage_obligations
cargo test -p mpb-knowledge validation_gates
```

Observed result: passed. Cargo treats these trailing arguments as test-name filters, so the targeted integration test files are verified with the explicit `--test` commands above.

```text
cargo test -p mpb-knowledge --test validation_gates
```

Observed result: passed. The command ran 5 tests covering the existing validation gates plus partial extraction, unsupported source kinds, missing clone/runtime validation, internet-only trust, decompile-only trust, and flaky experiment blockers.

Obligation blocker matrix added in this slice:

| Condition | Blocking code |
| --- | --- |
| Unsupported source diagnostic affecting discovered content | `UNSUPPORTED_SOURCE_KIND` |
| Unknown mechanic requiring adapter support | `UNKNOWN_MECHANIC` |
| Relationship with missing entities or missing/rejected evidence | `INCOMPLETE_RELATIONSHIP` |
| Behavioral claim without accepted runtime observation | `BEHAVIORAL_CLAIM_WITHOUT_RUNTIME_EVIDENCE` |
| Static claim without accepted deterministic evidence | `STATIC_CLAIM_WITHOUT_DETERMINISTIC_EVIDENCE` |
| Stale-fingerprint evidence | `STALE_FINGERPRINT` |
| Partial recipe/overlay/trait extraction | `PARTIAL_EXTRACTION` |
| Missing clone/runtime validation | `missing_clone_runtime_validation` |
| Internet-only trust | `internet_only_trust` |
| Decompile-only trust | `decompile_only_trust` |
| Flaky experiments | `flaky_experiments` |
| Missing source pack at validation time | `VALIDATION_SOURCE_PACK_MISSING` |

## 2026-06-29 Task 6: Worker Runtime Artifacts, Evaluation, And Fine-Tuning Gate

Red phase:

```text
cargo test -p mpb-knowledge worker_runtime
```

Observed result: failed to compile because worker runtime persistence, `WorkerRuntimeTask`, `ModelSelection`, `WorkerEvaluationFixture`, `WorkerGateOutcome`, and fine-tuning phase state APIs were not exported.

Green phase:

```text
cargo test -p mpb-knowledge --test worker_runtime
```

Observed result: passed. The command ran 6 tests covering durable worker prompt/input/output/model/evaluation/correction artifacts under `knowledge/runs/<run-id>/workers/`, database artifact references, `ModelDownload` approval blocking for missing local models, base evaluation failure proposing but not running fine-tuning, `FineTuning` approval and hardware-fit gates, untrusted worker envelopes for all runtime tasks, and the orchestrator `Drafting` phase persisting worker artifacts.

The exact plan command was also rerun:

```text
cargo test -p mpb-knowledge worker_runtime
```

Observed result: passed compilation, but Cargo treated `worker_runtime` as a test-name filter and ran 0 test cases. The integration test target is verified with `--test worker_runtime` above.

Durable implementation note: concrete model identity and checksum are recorded from a `worker-model` artifact reference. Pack logic does not hardcode a local model filename. Worker output remains draft-only until converted into accepted deterministic extraction evidence or accepted runtime lab evidence.

## 2026-06-29 Task 7: Runtime Experiment Suite, Retry Policy, And Adapter Expansion Plans

Red phase:

```text
cargo test -p mpb-knowledge experiments
cargo test -p mpb-knowledge adapter_plan
```

Observed result: failed to compile because experiment planning, attempt recording, retry summary, adapter expansion planning, and `ProjectCodeChange` application gates were not exported.

Green phase:

```text
cargo test -p mpb-knowledge --test experiments
```

Observed result: passed. The command ran 3 tests covering experiment batches derived from runtime coverage obligations, deterministic setup/bounded ticks/snapshot operations/adapter requirements, durable experiment attempt artifacts and events, and retry exhaustion producing `FLAKY_EXPERIMENT_RETRY_EXHAUSTED` with affected obligation ids and raw artifact paths.

```text
cargo test -p mpb-knowledge --test adapter_plan
```

Observed result: passed. The command ran 3 tests covering unsupported mechanics producing lab-adapter plans, affected obligation ids in plans, proposed test commands, `ProjectCodeChange` approval requirements, project edits blocked without approval, and extractor/validation-rule plans for non-adapter blockers.

```text
cargo test -p mpb-knowledge --test lab_observations
```

Observed result: passed. The command ran 4 tests after adding a defaulted `required_observation_adapters` field to the Rust lab observation contract.

```text
javac --release 17 -encoding UTF-8 -d /private/tmp/mpb-knowledge-lab-classes mods/mpb-knowledge-lab/common/src/main/java/com/mpb/lab/*.java
```

Observed result: passed. The Java lab runner now preserves stable generic operations and adds isolated observation-adapter registration/execution hooks. `MpbLabObservation` records required observation adapters while keeping the existing constructor path available.

The exact plan commands were also rerun:

```text
cargo test -p mpb-knowledge experiments
cargo test -p mpb-knowledge adapter_plan
```

Observed result: passed compilation, but Cargo treated the trailing names as test-name filters and ran 0 test cases. The integration test targets are verified with the explicit `--test` commands above.

## 2026-06-29 Task 8: Bundle Embedding And Product Validation

Red phase:

```text
cargo test -p mpb-knowledge product_validation
```

Observed result: failed to compile because product validation report/evidence types and the `Bundle`, `PatcherIntegration`, and `ProductValidation` phase implementations did not exist.

Green phase:

```text
cargo test -p mpb-knowledge product_validation
```

Observed result: passed. The command ran 8 `product_validation_*` tests covering runtime bundle artifact generation with checksum and compressed size, patcher integration blocking on exact-fingerprint mismatch, patcher integration blocking without mismatch-behavior evidence, patcher integration accepting exact-fingerprint shared product evidence, stale evidence fingerprint blockers for patcher and product validation, report-level blockers for failed patcher behavior and missing MCP query coverage, and durable product validation report persistence.

Additional product acceptance checks added in this slice:

```text
cargo test -p mpb-assets patcher
```

Observed result: passed compilation, but Cargo treated `patcher` as a test-name filter and ran 0 test cases.

```text
cargo test -p mpb-assets --test patcher
```

Observed result: passed. The command ran 9 patcher integration tests covering install, repair, unpatch, loader-specific artifacts, unmanaged-file conflicts, matching embedded curated knowledge, metadata update requirements, and mismatched-fingerprint base-mod-only behavior with curated knowledge unavailable.

```text
cargo test -p mpb-assets
```

Observed result: passed. The command ran 2 embedded knowledge bundle unit tests, 9 patcher tests, 5 Prism discovery tests, and doc-test targets with no failures.

```text
pnpm test src/patcher/patcherState.test.ts
```

Observed result: passed. Vitest ran 1 file with 3 patcher-state tests, including installed/available curated knowledge next-step text and unavailable curated knowledge fallback text.

```text
cargo test -p mpb-knowledge
```

Observed result: passed. The full `mpb-knowledge` suite ran 68 integration tests plus crate/doc test targets with no failures.

Manual Tauri desktop launch, real Prism client launch, Minecraft runtime MCP probing, and cloned runtime smoke remain unavailable in automated tests. A release run must attach those results as explicit `product-validation-evidence` before `ProductValidation` can pass.

## 2026-06-29 Task 9: Release Builder, GitHub Publication Gate, And Reports

Red phase:

```text
cargo test -p mpb-knowledge release_reports
```

Observed result: failed to compile because `BlockingReport`, `ReleaseReport`, report writers, and GitHub publication preparation APIs were not exported.

Green phase:

```text
cargo test -p mpb-knowledge --test release_reports
```

Observed result: passed. The command ran 5 tests covering complete blocking report fields, complete release report fields, macOS/Windows/Linux unsigned warnings, local GitHub preparation without approval while marking publication blocked, the exact-fingerprint `GitHubReleasePublication` gate, approved local GitHub preparation that writes notes and returns a `gh workflow run release.yml` command without invoking `gh`, and the `release report` CLI writing JSON/Markdown report paths.

Additional filtered plan command:

```text
cargo test -p mpb-knowledge release_reports
```

Observed result: passed compilation, but Cargo treated `release_reports` as a test-name filter and ran only the matching test case in `tests/release_reports.rs`. The full integration target is verified with `--test release_reports` above.

Regression check:

```text
cargo test -p mpb-knowledge
```

Observed result: passed. The command ran 73 integration tests across the `mpb-knowledge` suite plus crate and doc-test targets with no failures.

Durable implementation notes:

- Blocking reports and release reports are emitted as JSON plus Markdown under `knowledge/runs/<run-id>/reports/`.
- Blocking reports now use the shared release-report contract from `crates/mpb-knowledge/src/release.rs` instead of the previous private abbreviated orchestrator struct.
- `mpb-knowledge release report <run-id>` writes the local report pair and records a `release-report` artifact reference.
- `mpb-knowledge release prepare-github <run-id> --tag <tag>` prepares local notes and a `gh workflow run release.yml` command even when approval or credentials are missing. It marks `publicationApproved: false` until exact-fingerprint `GitHubReleasePublication` approval exists. It does not call `gh`, create a release, dispatch a workflow, or publish notes.
- `.github/workflows/release.yml` manual dispatch now accepts `knowledge_run_id`, `pack_id`, `fingerprint`, and `report_artifact_path`; it asserts the report artifact exists when supplied, uploads it, omits Tauri signing secrets from release builds, and writes explicit unsigned artifact warnings.
