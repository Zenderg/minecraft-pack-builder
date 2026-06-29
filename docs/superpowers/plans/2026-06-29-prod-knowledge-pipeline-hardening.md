# Production Knowledge Pipeline Hardening Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make the autonomous knowledge release pipeline block production release unless it records a local worker model run and real cloned Prism/Minecraft runtime evidence.

**Architecture:** Keep the existing resumable run database and phase order. Remove the permissive fast paths in `Drafting` and `RuntimeVerification`, add an explicit `cloned-runtime-validation-evidence` artifact, and expose a CLI attachment command so the next operator can record real runtime evidence without editing SQLite by hand.

**Tech Stack:** Rust 2021, `mpb-knowledge` CLI, SQLite run state, JSON artifact refs, existing integration tests.

---

### Task 1: Drafting Requires A Worker Model

**Files:**
- Modify: `crates/mpb-knowledge/tests/worker_runtime.rs`
- Modify: `crates/mpb-knowledge/src/orchestrator_phases.rs`

- [ ] Change the deterministic-coverage drafting test so it expects `Drafting` to block with `WORKER_MODEL_MISSING`.
- [ ] Run `cargo test -p mpb-knowledge --test worker_runtime orchestrator_drafting_phase_requires_worker_model_even_when_deterministic_coverage_is_complete` and confirm it fails against the current permissive implementation.
- [ ] Remove the `workerSkipped` success path from `run_drafting_phase`.
- [ ] Run `cargo test -p mpb-knowledge --test worker_runtime` and confirm worker runtime tests pass.

### Task 2: Runtime Verification Requires Clone Evidence

**Files:**
- Modify: `crates/mpb-knowledge/tests/experiments.rs`
- Modify: `crates/mpb-knowledge/src/orchestrator_phases.rs`

- [ ] Add a test proving a zero-experiment plan still blocks with `CLONED_RUNTIME_VALIDATION_MISSING` when no real clone/runtime evidence artifact exists.
- [ ] Add a test proving passed exact-fingerprint clone/runtime evidence lets zero-experiment runtime verification succeed.
- [ ] Run `cargo test -p mpb-knowledge --test experiments runtime_verification` and confirm the new missing-evidence test fails first.
- [ ] Implement strict evidence lookup for `cloned-runtime-validation-evidence` with exact fingerprint and `status: "passed"`.
- [ ] Run `cargo test -p mpb-knowledge --test experiments`.

### Task 3: CLI Attachment For Runtime Evidence

**Files:**
- Modify: `crates/mpb-knowledge/tests/cli.rs`
- Modify: `crates/mpb-knowledge/src/bin/mpb-knowledge.rs`
- Modify: `docs/knowledge/autonomous-release-pipeline.md`

- [ ] Add a CLI test for `mpb-knowledge release attach-runtime-evidence <run-id> <evidence-json>`.
- [ ] Run `cargo test -p mpb-knowledge --test cli attach_runtime` and confirm it fails before implementation.
- [ ] Implement the CLI command so it validates readable JSON, records `cloned-runtime-validation-evidence`, and emits `release.runtime_evidence_attached`.
- [ ] Document the required evidence shape and that browser/Vite checks remain insufficient.
- [ ] Run `cargo test -p mpb-knowledge --test cli`.

### Task 4: Verification And Commit

**Files:**
- Modify: `docs/validation/2026-06-29-autonomous-knowledge-release-pipeline.md`

- [ ] Record the hardening result and the commands run.
- [ ] Run `cargo fmt --check`.
- [ ] Run `cargo test -p mpb-knowledge --test worker_runtime --test experiments --test cli`.
- [ ] Run broader project verification commands that are practical in the local environment.
- [ ] Commit the hardening changes on `main`.
