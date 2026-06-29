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
