# Autonomous Knowledge Release Pipeline Design

Date: 2026-06-29

## 1. Purpose

This document defines the production target for MPB's first-party modded-Minecraft knowledge
pipeline. The target is not a CI/CD pipeline for general development work. It is a local
developer-side pipeline that lets an orchestrating agent take a locally installed PrismLauncher
modpack, produce a fully verified knowledge pack, embed that pack into the MPB patcher, and publish
a user-facing GitHub release.

The intended operator flow is:

1. The user points the orchestrator at a local PrismLauncher instance.
2. The orchestrator performs preflight checks and estimates the run time.
3. After required approvals, the orchestrator runs the local pipeline with minimal user involvement.
4. The user receives either a release report or a blocking report with precise next actions.

`All of Create - Aeronautics` is the first acceptance candidate for this pipeline, but the pipeline
must remain generic. No architecture, validation rule, or lab capability may be hardcoded only for
that modpack.

## 2. Product Contract

Production-ready means the pipeline can process any supported local Prism modpack with these
properties:

- The original Prism instance is treated as read-only source input.
- All destructive, experimental, or lab-only activity happens in a disposable clone.
- The pipeline discovers every local pack input it can observe: mods, versions, resources, recipes,
  tags, configs, datapacks, scripts, guide/manual/tooltip content, entities, mechanics,
  relationships, and behavioral surfaces.
- Every discovered entity, mechanic, relationship, and behavior becomes a coverage obligation.
- Release is blocked until every coverage obligation has accepted evidence.
- Static facts may be trusted only when backed by deterministic local extraction.
- Behavioral facts may be trusted only when verified in a real cloned Minecraft runtime for the
  exact target fingerprint.
- Worker output, internet sources, and decompiled/code analysis are draft sources only. They can
  propose hypotheses, adapters, and experiments, but they cannot create trusted claims by
  themselves.
- Unknown or unsupported discovered mechanics do not ship in a release. They block release until the
  pipeline gains enough extractor, adapter, or experiment support to verify them.
- Before a long run, the pipeline estimates time and resource requirements so the user knows how
  long the machine should stay active.
- Long runs are resumable through durable checkpoints and a run database.
- Local model downloads, keep-awake mode, project code changes, and GitHub release publication
  require explicit approval.
- The end-user release remains simple: download the MPB patcher for macOS, Windows, or Linux from
  GitHub Releases; the patcher installs the managed mod and any embedded matching knowledge bundle.
- Release artifacts may be unsigned initially, but user-facing release notes must document expected
  macOS Gatekeeper, Windows SmartScreen, or Linux trust warnings.

## 3. Scope Boundaries

The pipeline is local-first:

- Knowledge production runs on the local developer machine.
- Cloud GPU jobs are out of scope.
- External LLM APIs are out of scope for trusted knowledge production.
- CI may be used for cross-platform desktop app artifacts after the local knowledge pack is
  validated and committed.

The pipeline is orchestrator-facing, not user-UI-facing:

- A dedicated patcher UI for knowledge production is not required.
- CLI and service commands may exist because they are convenient for the orchestrator.
- User interaction is limited to approvals, progress/status summaries, and final reports.

Knowledge bundles remain embedded in the patcher for now:

- The patcher should not download separate knowledge bundles in the initial production pipeline.
- Bundles may be compressed before embedding.
- If many future packs make the patcher too large, bundle delivery should be redesigned separately.

## 4. Architecture

### 4.1 Release Orchestrator

The release orchestrator is a resumable state machine around the knowledge pipeline. It owns:

- run creation and run ids;
- phase ordering;
- checkpoint/resume;
- risk gates;
- blocker reporting;
- progress summaries;
- cleanup policy;
- final release or blocking reports.

The orchestrator stores durable local run state under ignored repository artifact directories so a
run can continue after sleep, crash, or process restart.

### 4.2 Environment Preflight

Preflight checks the local machine before expensive work starts:

- CPU, RAM, disk space, operating system, and architecture;
- GPU or Metal capability where available;
- Java, Gradle, Rust, Node, pnpm, Tauri, and GitHub tooling;
- PrismLauncher availability and target instance readability;
- model cache state;
- required source and artifact directories;
- likely clone size and extraction scale;
- rough phase-by-phase time estimate.

If the pipeline needs a local model that is not present, preflight must report the model candidate,
expected size, expected runtime mode, hardware fit, and why that model is needed. Downloading the
model requires explicit approval. Fine-tuning is not a default step; it is a gated fallback only
when base-model evaluations fail and the local machine can support the training run.

Preflight must also offer keep-awake mode, such as macOS `caffeinate`, but must not enable it
without explicit approval.

### 4.3 Target Manager

The target manager reads the original Prism instance and creates a disposable clone for all runtime
work. It owns:

- exact target fingerprinting;
- clone creation;
- clone patching with lab/runtime instrumentation;
- disposable world creation or selection;
- Minecraft launch where local OS and Prism allow it;
- detection of launcher or OS prompts that need user intervention;
- safe cleanup of clone artifacts.

The clone may be modified freely. The original instance must not be changed by knowledge-production
steps.

### 4.4 Extractors

Extractors produce deterministic local source records. They must cover:

- Prism metadata and instance metadata;
- mod jars and versions;
- registries where available;
- block states, item models, recipes, tags, language files, resources, and data files;
- configs, datapacks, scripts, generated packs, resource packs, texture packs, and shader packs;
- local guidebook/manual/tooltip-like content where accessible;
- initial entity, mechanic, recipe, relationship, and trait records.

Extractor output creates coverage obligations. Partial extraction or unsupported source kinds must
produce blocking diagnostics rather than soft warnings when they affect discovered content.

### 4.5 Worker Runtime

Small local models are developer-side assistants. Their allowed roles are:

- draft classification;
- claim extraction from local documentation;
- conflict detection;
- experiment proposal;
- lab-log summarization;
- structured JSON/schema repair suggestions.

The worker runtime must:

- run only after hardware preflight and model approval when a model must be downloaded;
- record prompts, inputs, outputs, model identity, model checksum, evaluation results, and
  corrections as ignored local artifacts;
- keep worker decisions untrusted until converted into deterministic or runtime evidence;
- evaluate worker quality on task-specific fixtures before using output in a release run;
- propose fine-tuning only when base model quality blocks the pack.

Fine-tuning, when approved, is task-specific and local. It is not "train a model on this modpack" as
a substitute for evidence.

### 4.6 Draft Sources: Internet And Code Analysis

Internet sources and decompiled/static code analysis may help the orchestrator understand what to
test. They may identify APIs, mechanics, edge cases, likely adapters, and experiment ideas.

They are never trusted evidence for release. Any claim derived from these draft sources must be
verified through deterministic local extraction or runtime lab evidence before it can ship.

### 4.7 Lab Runner And Adapters

The lab runner executes controlled experiments in the cloned Minecraft runtime. It must expose
high-level operations such as:

- prepare/reset isolated lab area;
- place structures;
- set valid block states;
- use items on blocks;
- run bounded ticks;
- inspect block entities, inventories, fluids, energy, kinetic state, vessel/contraption state, or
  other observable state where APIs allow;
- compare before/after snapshots;
- record structured observations.

Generic lab operations will not cover every modded mechanic. When the pipeline discovers a mechanic
that cannot be observed with existing lab support, it must prepare a proposed code-change plan for
the required extractor, lab adapter, tests, or validation rule. It may implement those changes only
after approval. The adapter is trusted only after tests pass and runtime experiments verify the
target behavior in the cloned world.

### 4.8 Coverage Validator

The coverage validator is stricter than the current static-pack validator. It must fail release for:

- uncovered discovered entities;
- uncovered discovered mechanics;
- incomplete relationships, recipes, traits, overlays, or dependency chains;
- behavioral claims without accepted runtime evidence;
- worker-only, internet-only, or decompile-only trusted claims;
- unresolved placeholders or conflict markers;
- stale or mismatched fingerprints;
- flaky runtime experiments that exceed retry policy;
- missing release metadata;
- bundle query gaps;
- missing clone/runtime validation results.

Validation success means the pack is releaseable for the exact fingerprint, not just internally
well-formed.

### 4.9 Bundle And Patcher Integration

The bundle builder produces:

- reviewable curated source records under `knowledge/packs/<pack-id>/source/`;
- generated runtime bundle under `knowledge/packs/<pack-id>/bundle/`;
- compressed embedded bundle artifact for the patcher;
- checksums and release metadata.

The patcher embeds matching first-party bundles and installs a bundle only when the selected Prism
instance exactly matches the bundle fingerprint. If no embedded bundle matches, the patcher may still
install the base MPB mod but must report curated knowledge unavailable.

### 4.10 Release Builder

The release builder coordinates:

- source validation;
- bundle generation;
- patcher metadata update;
- Rust, Java, TypeScript, Tauri, and Minecraft mod tests;
- local product smoke against the cloned runtime;
- commit/tag preparation;
- GitHub Actions desktop artifact builds for macOS, Windows, and Linux;
- release notes with unsigned-app warnings;
- checksums;
- GitHub release publication after approval and with available credentials.

## 5. Pipeline Flow

1. **Intake:** receive a Prism instance path and create a run id.
2. **Preflight:** inspect environment, model needs, disk, toolchains, and estimate duration.
3. **Approvals:** request approval for long run, keep-awake mode, model download, or other risk
   gates.
4. **Fingerprint:** compute exact source fingerprint from the original instance.
5. **Clone:** create and prepare disposable clone.
6. **Extraction:** build full discovered inventory and coverage obligations.
7. **Drafting:** run approved local workers and optional untrusted internet/code analysis to propose
   claims and experiments.
8. **Experiment planning:** build a batch experiment suite from coverage obligations.
9. **Adapter expansion:** request approval for any project code changes required to observe missing
   mechanics.
10. **Runtime verification:** execute experiments in the cloned Minecraft runtime and convert
    accepted observations to evidence.
11. **Validation:** run full coverage and trust gates.
12. **Bundle:** build source records, runtime bundle, compressed embedded bundle, and checksums.
13. **Patcher integration:** update embedded bundle metadata and patcher behavior if needed.
14. **Product validation:** test patcher install/update/repair/unpatch and MCP knowledge queries.
15. **Release:** build or trigger app artifacts, prepare notes, publish GitHub release after approval.
16. **Report:** write release report or blocking report.

Every step must be idempotent enough to resume from recorded state.

## 6. Blocking Reports

A blocking report must include:

- run id;
- target instance and fingerprint;
- failed phase;
- exact blocker;
- affected coverage obligations;
- already accepted evidence;
- missing capability or approval;
- proposed code-change, model, fine-tuning, adapter, or manual environment action;
- resume instructions for the orchestrator;
- local artifact paths relevant to debugging.

Blocking reports are preferred over partial releases.

## 7. Release Reports

A release report must include:

- target pack identity and exact fingerprint;
- coverage summary;
- evidence summary by kind;
- model candidates, approvals, evaluations, and any fine-tuning decisions;
- experiment suite summary and failure/retry statistics;
- generated source and bundle paths;
- bundle checksums and compressed artifact size;
- patcher install/update/repair/unpatch validation results;
- cloned Minecraft runtime validation results;
- MCP knowledge query validation results;
- app artifact list for macOS, Windows, and Linux;
- unsigned-artifact warning text;
- GitHub release URL when published.

## 8. Acceptance Criteria

The pipeline is accepted only when:

- It can run against a local Prism instance through the generic flow.
- It performs preflight time/resource estimation before long work starts.
- It requires explicit approval before model download, keep-awake mode, project code changes, and
  GitHub release publication.
- It creates and uses a disposable clone without mutating the original instance.
- It produces full discovered coverage obligations.
- It blocks release on uncovered or unverified discovered mechanics.
- It can use worker output, internet sources, and code analysis without letting them become trusted
  evidence.
- It can add approved lab/extractor capabilities when the current runtime cannot observe a mechanic.
- It resumes after interruption without restarting from scratch.
- It embeds validated knowledge bundles into the patcher.
- It publishes or prepares macOS, Windows, and Linux patcher artifacts through GitHub release flow.
- It proves the generic pipeline on `All of Create - Aeronautics` as the first large acceptance
  candidate.

## 9. Non-Goals

- No cloud GPU jobs for knowledge production.
- No end-user local model inference.
- No end-user knowledge-generation mode for unsupported packs.
- No trusted claims from internet pages alone.
- No trusted claims from decompiled code alone.
- No release with known uncovered discovered mechanics.
- No separate knowledge bundle download flow in the first version of this pipeline.
- No dedicated human-facing UI for knowledge production unless a later design chooses to add one.

## 10. Implementation Planning Commitments

The implementation plan must use these concrete commitments:

- Local ignored artifacts live under `knowledge/runs/`, `knowledge/lab-artifacts/`,
  `knowledge/model-cache/`, `knowledge/model-datasets/`, and `knowledge/prism-clones/`.
- Each run stores a SQLite run database plus an append-only JSONL event log so the orchestrator can
  resume work and humans can inspect the audit trail.
- Local model execution is behind a pluggable worker-runtime interface. The first production
  adapter targets a locally downloaded small instruction model with hardware-fit checks before
  download. The exact model file is selected by preflight and approval, not hardcoded into pack
  logic.
- Fine-tuning is implemented as a separate gated local worker phase. It runs only after base-model
  evaluation fails, local hardware is judged sufficient, and the user approves the fine-tuning run.
- Prism/Minecraft launch control starts with local OS process launch and health probing. If the OS,
  launcher, account state, or permissions require a manual click, the orchestrator records a
  resumable intervention checkpoint instead of silently failing.
- Java lab adapters are isolated behind mechanic-specific interfaces and tests. Generic lab
  commands remain stable while adapters expose mod-specific observation hooks.
- Runtime experiments use deterministic setup, bounded ticks, structured before/after snapshots,
  and a retry policy recorded in the run database. A flaky experiment that exceeds retry policy
  blocks release.
- GitHub Actions build unsigned desktop artifacts in a macOS, Windows, and Linux matrix after the
  local knowledge pack is validated and committed.
