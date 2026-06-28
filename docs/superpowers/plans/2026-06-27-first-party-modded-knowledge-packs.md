# First-Party Modded Knowledge Packs Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add production-grade first-party curated modded-Minecraft knowledge packs, tied to exact Prism/modpack fingerprints, installable by the MPB patcher and queryable read-only through the MPB Minecraft mod MCP runtime.

**Architecture:** Keep generation and validation as developer-side tooling, keep source knowledge as reviewable structured repository files, generate a compact read-only runtime bundle, and let the existing patcher install that bundle only when the selected Prism instance exactly matches the bundled fingerprint. The Minecraft mod loads the installed bundle and exposes minimal read-only MCP knowledge tools alongside existing scheme-editing tools; no local model inference, lab tooling, or knowledge generation is present in the end-user runtime.

**Tech Stack:** Rust 2021 workspace, Tauri 2 patcher, React 19 patcher UI, Java Minecraft mod runtime for Fabric/Forge/NeoForge, JSON/JSONL source records, generated JSON runtime indexes, Gradle mod build, PrismLauncher instance metadata.

---

## Scope Split

This spec covers multiple independent subsystems. Implement it as a sequence of focused plans/PR-sized work units, each producing working, testable software:

1. Knowledge source schema and strict validation gates.
2. Deterministic Prism/modpack fingerprinting and extraction.
3. Runtime bundle builder and fixture bundle.
4. Patcher install/update/repair/unpatch integration for knowledge bundles.
5. Minecraft mod read-only knowledge bundle loader and MCP query tools.
6. Developer-only lab mod and batch experiment runner.
7. Model-worker harness with explicit rejection/evidence conversion.
8. `All of Create - Aeronautics` production pack run.

Do not ship a trusted first-party pack until all relevant plans for that pack are complete and the validation command passes with zero unresolved coverage.

---

### Task 1: Knowledge Schema And Validation Gates

**Files:**
- Create: `crates/mpb-knowledge/Cargo.toml`
- Create: `crates/mpb-knowledge/src/lib.rs`
- Create: `crates/mpb-knowledge/src/schema.rs`
- Create: `crates/mpb-knowledge/src/validation.rs`
- Create: `crates/mpb-knowledge/tests/validation_gates.rs`
- Modify: `Cargo.toml`
- Create: `docs/knowledge/README.md`

- [ ] Add `mpb-knowledge` to the Rust workspace with dependencies on `serde`, `serde_json`, and `thiserror`.
- [ ] Define source schema types for `KnowledgePackSource`, `KnowledgeManifest`, `EntityRecord`, `ClaimRecord`, `EvidenceSummary`, `RecipeRecord`, `MechanicTrait`, `MechanicOverlay`, `RelationshipRecord`, `CoverageSummary`, and `WorkerDecision`.
- [ ] Encode the production contract in validation errors: fingerprint mismatch, uncovered entities, incomplete overlays, behavioral claims without runtime evidence, incomplete dependency chains, unresolved placeholders, trusted worker output, runtime bundle query gaps, and missing manifest metadata.
- [ ] Write failing tests in `crates/mpb-knowledge/tests/validation_gates.rs` that construct small in-memory packs and assert every required gate fails with a specific error code.
- [ ] Implement `validate_source_pack(pack: &KnowledgePackSource) -> Result<ValidationReport, KnowledgeValidationError>` and helper checks in `validation.rs`.
- [ ] Add fixture tests proving a fully covered minimal pack passes while any `unknown`, `todo`, `stub`, `inferred_only`, unresolved conflict marker, or placeholder evidence fails.
- [ ] Document the schema, required evidence semantics, and the rule that raw lab artifacts are local developer artifacts and are not committed or shipped.
- [ ] Run `cargo test -p mpb-knowledge`.
- [ ] Run `cargo test --workspace`.
- [ ] Commit with message `feat: add knowledge schema validation gates`.

### Task 2: Exact Fingerprint Model And Deterministic Extractor Scaffold

**Files:**
- Create: `crates/mpb-knowledge/src/fingerprint.rs`
- Create: `crates/mpb-knowledge/src/extract.rs`
- Create: `crates/mpb-knowledge/tests/fingerprint.rs`
- Modify: `crates/mpb-assets/src/prism.rs`
- Modify: `crates/mpb-assets/tests/prism_discovery.rs`
- Create: `docs/knowledge/fingerprints.md`

- [ ] Write tests for exact fingerprint inputs: modpack identity/version, Minecraft version, loader/version, full mod list and versions, configs, datapacks, KubeJS/CraftTweaker-like scripts, resource/data packs, knowledge schema version, builder version, and lab tooling version.
- [ ] Extend Prism content fingerprinting beyond current metadata/mod/resourcepack coverage so it includes `config`, `datapacks`, `kubejs`, `scripts`, and other pack-affecting folders when present.
- [ ] Implement a stable canonical fingerprint document that records each input with sorted paths, byte length, checksum, and role.
- [ ] Add `compute_target_fingerprint(instance_path, builder_version, lab_version, schema_version)` in `mpb-knowledge`.
- [ ] Add deterministic extractor interfaces that can collect registry, block state, recipe, tag, language, config, datapack, script, resource/data-pack, guidebook, tooltip, and manual-derived facts into draft records without marking them trusted unless backed by deterministic source evidence.
- [ ] Ensure unsupported or partially collected inputs produce validation-blocking extraction diagnostics rather than soft warnings.
- [ ] Document the exact-match policy: no version ranges, no user override, no "close enough" trusted mode.
- [ ] Run `cargo test -p mpb-knowledge fingerprint`.
- [ ] Run `cargo test -p mpb-assets prism_discovery`.
- [ ] Commit with message `feat: add exact modpack fingerprinting`.

### Task 3: Runtime Bundle Builder

**Files:**
- Create: `crates/mpb-knowledge/src/bundle.rs`
- Create: `crates/mpb-knowledge/tests/bundle_queries.rs`
- Create: `crates/mpb-knowledge/src/bin/mpb-knowledge.rs`
- Create: `docs/knowledge/runtime-bundle-format.md`
- Create: `knowledge/packs/fixtures/minimal/source/manifest.json`
- Create: `knowledge/packs/fixtures/minimal/source/entities.jsonl`
- Create: `knowledge/packs/fixtures/minimal/source/claims.jsonl`
- Create: `knowledge/packs/fixtures/minimal/source/evidence.jsonl`
- Create: `knowledge/packs/fixtures/minimal/bundle/knowledge-index.json`

- [ ] Define generated runtime bundle indexes for entity lookup by id, localized name, tag, use case, mechanic, interface, recipe/dependency graph slice, mechanic details, and evidence summary.
- [ ] Add `build_runtime_bundle(source_dir, output_dir)` that refuses to generate a bundle unless `validate_source_pack` passes.
- [ ] Add CLI commands `validate-source`, `build-bundle`, and `inspect-bundle`.
- [ ] Create a minimal fixture pack with complete evidence-backed records for a tiny deterministic example such as `minecraft:stone`, `minecraft:cobblestone`, and one recipe/dependency relation.
- [ ] Write tests proving the bundle answers all required read-only query types without reading raw logs or invoking any model.
- [ ] Ensure the generated bundle manifest includes pack id, exact fingerprint, schema version, builder/lab versions, validation command, validation timestamp, checksum list, and coverage summary.
- [ ] Run `cargo test -p mpb-knowledge bundle_queries`.
- [ ] Run `cargo run -p mpb-knowledge --bin mpb-knowledge -- validate-source knowledge/packs/fixtures/minimal/source`.
- [ ] Run `cargo run -p mpb-knowledge --bin mpb-knowledge -- build-bundle knowledge/packs/fixtures/minimal/source /tmp/mpb-minimal-bundle`.
- [ ] Commit with message `feat: build read-only knowledge bundles`.

### Task 4: Patcher Knowledge Bundle Integration

**Files:**
- Modify: `crates/mpb-assets/src/patcher.rs`
- Modify: `crates/mpb-assets/src/lib.rs`
- Modify: `crates/mpb-assets/tests/patcher.rs`
- Create: `crates/mpb-assets/src/knowledge_bundle.rs`
- Create: `crates/mpb-assets/src/mpb_knowledge_fixture_bundle.hex`
- Modify: `src/patcher/patcherState.ts`
- Modify: `src/patcher/patcherState.test.ts`
- Modify: `src/patcher/PatcherApp.tsx`
- Modify: `src/i18n.ts`

- [ ] Extend `MpbPatchManifest` with `knowledge_pack_id`, `knowledge_fingerprint`, `knowledge_schema_version`, compatibility metadata, and managed knowledge bundle files.
- [ ] Add bundled knowledge artifact metadata for fixture and future first-party pack artifacts; keep the artifact read-only and checksummed under `<instance>/mpb/knowledge/<pack-id>/`.
- [ ] Write tests for patch statuses with knowledge files: `Patched`, `NeedsUpdate`, `NeedsRepair`, `Unsupported`, and `Conflict`.
- [ ] Implement exact fingerprint matching before installing knowledge. If the base MPB mod is supported but the knowledge fingerprint does not match, install/repair the base mod only and report knowledge unavailable.
- [ ] Ensure unpatch removes managed knowledge files and never removes `mpb/schemes` unless `delete_schemes` is explicitly true.
- [ ] Update patcher UI state to show knowledge availability as data from patch evaluation, without adding user-facing flows not specified by the product specs.
- [ ] Update next-step text only where it needs to distinguish "curated knowledge available" from "curated knowledge unsupported for this instance."
- [ ] Run `cargo test -p mpb-assets patcher`.
- [ ] Run `pnpm test src/patcher/patcherState.test.ts`.
- [ ] Run `pnpm build`.
- [ ] Commit with message `feat: install matching knowledge bundles`.

### Task 5: Minecraft Runtime Knowledge Loader And MCP Tools

**Files:**
- Create: `mods/mpb-minecraft-mod/common/src/main/java/com/mpb/runtime/knowledge/MpbKnowledgePack.java`
- Create: `mods/mpb-minecraft-mod/common/src/main/java/com/mpb/runtime/knowledge/MpbKnowledgeRepository.java`
- Create: `mods/mpb-minecraft-mod/common/src/main/java/com/mpb/runtime/knowledge/MpbKnowledgeQuery.java`
- Modify: `mods/mpb-minecraft-mod/common/src/main/java/com/mpb/runtime/MpbRuntimePaths.java`
- Modify: `mods/mpb-minecraft-mod/common/src/main/java/com/mpb/runtime/MpbManagerSnapshot.java`
- Modify: `mods/mpb-minecraft-mod/common/src/main/java/com/mpb/runtime/MpbAgentPrompt.java`
- Modify: `mods/mpb-minecraft-mod/common/src/main/java/com/mpb/runtime/MpbMcpToolCatalog.java`
- Modify: `mods/mpb-minecraft-mod/common/src/main/java/com/mpb/runtime/MpbMcpHttpServer.java`
- Create: `mods/mpb-minecraft-mod/tests/src/com/mpb/runtime/MpbKnowledgeRuntimeTest.java`
- Modify: `mods/mpb-minecraft-mod/tests/src/com/mpb/runtime/MpbMcpToolCatalogTest.java`
- Modify: `mods/mpb-minecraft-mod/tests/src/com/mpb/runtime/MpbRuntimeConfigTest.java`

- [ ] Add a runtime loader for `<instance>/mpb/knowledge/<pack-id>/knowledge-index.json` that validates checksums and manifest fingerprint metadata before marking the pack active.
- [ ] Add read-only MCP tools: `mpb_knowledge_status`, `mpb_search_entities`, `mpb_get_entity_card`, `mpb_get_recipe_graph`, `mpb_get_mechanic_details`, and `mpb_get_evidence`.
- [ ] Keep knowledge tools read-only and separate from scheme mutation tools.
- [ ] If no exact matching active pack exists, make `mpb_knowledge_status` report unavailable and make other knowledge tools return a clear unsupported response instead of falling back to guessing.
- [ ] Update the agent prompt so matched packs instruct the agent to query curated knowledge and mismatched instances instruct the agent not to claim curated modpack support.
- [ ] Add Java tests using a fixture bundle under a temporary instance root to verify status, search, entity card, dependency graph, mechanic detail, evidence lookup, and unsupported behavior.
- [ ] Add tests proving tool catalog names and schemas include the knowledge tools.
- [ ] Run `mods/mpb-minecraft-mod/build.sh` with the local Gradle/JDK configuration documented in `mods/mpb-minecraft-mod/README.md`.
- [ ] Run `cargo test -p mpb-assets patcher` after refreshing generated mod artifact hex files.
- [ ] Commit with message `feat: expose read-only knowledge MCP tools`.

### Task 6: Developer-Only Lab Mod And Batch Experiment Runner

**Files:**
- Create: `mods/mpb-knowledge-lab/README.md`
- Create: `mods/mpb-knowledge-lab/settings.gradle`
- Create: `mods/mpb-knowledge-lab/build.gradle`
- Create: `mods/mpb-knowledge-lab/common/src/main/java/com/mpb/lab/MpbLabCommandServer.java`
- Create: `mods/mpb-knowledge-lab/common/src/main/java/com/mpb/lab/MpbLabExperimentRunner.java`
- Create: `mods/mpb-knowledge-lab/common/src/main/java/com/mpb/lab/MpbLabObservation.java`
- Create: `crates/mpb-knowledge/src/lab.rs`
- Create: `crates/mpb-knowledge/tests/lab_observations.rs`
- Modify: `.gitignore`

- [ ] Add a separate developer-only lab mod tree that is not referenced by patcher artifacts and is not installed by `apply_mpb_patch`.
- [ ] Implement high-level experiment operations for preparing/resetting a lab area, placing structures, setting block states, using items on blocks, running bounded ticks, inspecting block entities/inventories/fluids/energy/kinetic/vessel state where APIs allow, comparing before/after snapshots, and recording structured observations.
- [ ] Add Rust-side observation conversion that turns lab observations into compact `EvidenceSummary` records only after they are linked to claims and the exact fingerprint.
- [ ] Keep raw logs, snapshots, local notebooks, and worker traces ignored under developer artifact folders such as `knowledge/lab-artifacts/`.
- [ ] Add a batch-first command contract that can run the full coverage suite and fail on uncovered entities, failed experiments, unresolved mechanics, stale fingerprint, placeholders, or invalid bundle.
- [ ] Document that dedicated-server/headless operation is outside the production contract and the canonical lab target is a local client Prism instance.
- [ ] Run `cargo test -p mpb-knowledge lab_observations`.
- [ ] Build the lab mod locally when Gradle/Minecraft toolchains are available; record manual launch requirements in `docs/validation/`.
- [ ] Commit with message `feat: add developer knowledge lab runner`.

### Task 7: Model-Worker Harness And Evidence Conversion

**Files:**
- Create: `crates/mpb-knowledge/src/workers.rs`
- Create: `crates/mpb-knowledge/tests/worker_gate.rs`
- Create: `docs/knowledge/model-workers.md`
- Create: `knowledge/worker-decisions/README.md`

- [ ] Define worker output envelopes for draft classification, summarization, conflict detection, and experiment proposals.
- [ ] Use `Qwen2.5-Coder-1.5B-Instruct` as the first worker candidate for structured transformation/classification tasks, with `Qwen3-1.7B` or `Qwen3-4B` reserved for broader reasoning worker experiments if the first candidate is insufficient.
- [ ] Encode three fine-tuning decisions in source metadata: no fine-tuning used, fine-tuning used for a named worker task with model/dataset/evaluation threshold/result, or fine-tuning required because quality blocks the pack.
- [ ] Start without fine-tuning; record worker prompts, outputs, corrections, and experiment outcomes as future training data, and only introduce LoRA/parameter-efficient fine-tuning after enough corrected examples exist.
- [ ] Implement validation so raw worker output can never become trusted knowledge without deterministic extraction/runtime evidence and successful gate validation.
- [ ] Write tests proving `trusted: true` worker-only claims fail and evidence-converted claims pass only when linked to accepted evidence summaries.
- [ ] Document allowed worker roles and the explicit rule that workers are assistants, never a source of truth.
- [ ] Run `cargo test -p mpb-knowledge worker_gate`.
- [ ] Commit with message `feat: gate model worker knowledge drafts`.

### Task 8: All Of Create - Aeronautics Production Pack Run

**Files:**
- Create: `knowledge/packs/all-of-create-aeronautics/README.md`
- Create: `knowledge/packs/all-of-create-aeronautics/source/manifest.json`
- Create: `knowledge/packs/all-of-create-aeronautics/source/entities.jsonl`
- Create: `knowledge/packs/all-of-create-aeronautics/source/claims.jsonl`
- Create: `knowledge/packs/all-of-create-aeronautics/source/evidence.jsonl`
- Create: `knowledge/packs/all-of-create-aeronautics/source/recipes.jsonl`
- Create: `knowledge/packs/all-of-create-aeronautics/source/relationships.jsonl`
- Create: `knowledge/packs/all-of-create-aeronautics/source/overlays.jsonl`
- Create: `knowledge/packs/all-of-create-aeronautics/bundle/knowledge-index.json`
- Create: `docs/validation/2026-06-27-aoca-knowledge-pack.md`

- [ ] Select the exact Prism instance for `All of Create - Aeronautics` and compute the target fingerprint with the production fingerprint command.
- [ ] Run deterministic extraction over registries, block states, recipes, tags, language data, configs, datapacks, scripts, resource/data packs, and accessible guidebook/tooltip/manual data.
- [ ] Build the entity graph and mechanic overlay candidates for kinetic networks, recipe-processing, vessel/airship physics, inventory/logistics, fluids, redstone, multiblocks, contraption membership, and entity interactions.
- [ ] Run hypothesis-driven lab experiments only where structural graph membership, shared interface, recipes, tags, capabilities, documented requirements, config relationships, or evidence conflicts justify interaction testing.
- [ ] Convert accepted observations into compact evidence summaries; keep raw artifacts out of git.
- [ ] Complete every discovered block, item, recipe, fluid, entity, tag, config/datapack/script/resource/data-pack input, mechanic, relationship, trait, overlay, use case, avoid case, requirement, limitation, and dependency record.
- [ ] Run the strict validation command and resolve every uncovered entity, unresolved overlay, failed experiment, stale fingerprint, placeholder, worker-only claim, and invalid bundle response before committing the pack.
- [ ] Build the runtime bundle and wire it into the patcher artifact metadata only after validation passes.
- [ ] Record validation command, validation timestamp, coverage summary, exact fingerprint, source version, schema version, builder version, and lab version in the pack manifest and validation doc.
- [ ] Run `cargo run -p mpb-knowledge --bin mpb-knowledge -- validate-source knowledge/packs/all-of-create-aeronautics/source`.
- [ ] Run `cargo run -p mpb-knowledge --bin mpb-knowledge -- build-bundle knowledge/packs/all-of-create-aeronautics/source knowledge/packs/all-of-create-aeronautics/bundle`.
- [ ] Run `cargo test --workspace`.
- [ ] Run `pnpm test`.
- [ ] Run the Minecraft mod build and perform a real Prism client validation because this is a Tauri/Minecraft desktop product, not a browser-only workflow.
- [ ] Commit with message `feat: bundle all of create aeronautics knowledge pack`.

### Task 9: Release Documentation And Product Guardrails

**Files:**
- Modify: `README.md`
- Modify: `mods/mpb-minecraft-mod/README.md`
- Modify: `docs/knowledge/README.md`
- Create: `docs/validation/first-party-knowledge-release-checklist.md`

- [ ] Document the end-user flow: download one patcher, select Prism instance, install managed mod plus matching knowledge bundle, start Minecraft, open MPB Manager, copy MCP prompt/endpoint, connect external agent.
- [ ] Document unsupported fingerprint behavior: base mod may install if compatible, knowledge tools disabled/unavailable, prompt says curated modpack knowledge is unsupported, and mismatched first-party knowledge cannot be used.
- [ ] Document release-blocking validation gates and the rule that no trusted pack ships with unresolved coverage or placeholders.
- [ ] Add a release checklist covering patcher install/repair/update/unpatch, Java runtime knowledge queries, MCP prompt behavior, strict source validation, bundle query coverage, and real Prism client smoke validation.
- [ ] Run `cargo test --workspace`.
- [ ] Run `pnpm test`.
- [ ] Run `pnpm build`.
- [ ] Record any unavailable desktop/Minecraft manual validation steps in `docs/validation/`.
- [ ] Commit with message `docs: document first-party knowledge release gates`.

---

## Required Execution Order

1. Finish Tasks 1-3 before any patcher or runtime installation work; patcher/runtime code needs a stable bundle format.
2. Finish Task 4 before Task 5 integration testing; the runtime needs the same installed file layout the patcher manages.
3. Finish Tasks 6-7 before claiming any production pack readiness; they provide evidence and worker rejection mechanics.
4. Finish Task 8 only after the strict validation command exists and fails correctly on incomplete packs.
5. Finish Task 9 before release packaging.

## Validation Matrix

- Rust schema/bundle/fingerprint: `cargo test -p mpb-knowledge`.
- Patcher integration: `cargo test -p mpb-assets patcher`.
- Full Rust workspace: `cargo test --workspace`.
- React patcher state/UI checks: `pnpm test` and `pnpm build`.
- Minecraft runtime: `mods/mpb-minecraft-mod/build.sh` with the local Gradle/JDK setup from `mods/mpb-minecraft-mod/README.md`.
- Production pack: `cargo run -p mpb-knowledge --bin mpb-knowledge -- validate-source knowledge/packs/all-of-create-aeronautics/source`.
- Desktop/product validation: real PrismLauncher client run with MPB Manager and MCP endpoint/prompt copied from Minecraft.

## Spec Coverage Review

- Product contract: Tasks 1, 4, 5, 8, and 9 enforce no trusted partial packs and exact unsupported behavior.
- Fingerprint model: Task 2 defines exact fingerprinting and Task 4 enforces it during patching.
- Knowledge scope: Tasks 1, 2, 3, and 8 cover entities, recipes, relationships, mechanics, evidence, and dependency chains.
- Traits and overlays: Tasks 1 and 8 gate complete overlay coverage.
- Evidence model: Tasks 1, 6, 7, and 8 keep trusted claims evidence-backed and raw artifacts local.
- Interaction coverage: Tasks 6 and 8 use graph-driven experiments and verified negative results.
- Developer workflow: Tasks 2, 3, 6, 7, and 8 implement extraction, lab, validation, bundle build, and pack production.
- Repository/bundle separation: Tasks 1 and 3 keep source reviewable and runtime bundles generated.
- Patcher integration: Task 4 owns managed files, statuses, checksums, and unpatch behavior.
- Runtime MCP surface: Task 5 adds minimal read-only knowledge tools in the Minecraft mod.
- Agent behavior contract: Tasks 5 and 9 update prompt/status behavior so agents query knowledge or honestly report unsupported state.
- Rejected directions: Tasks 4-7 keep user-side generation, model inference, lab tooling, raw logs, version ranges, and separate daemons out of the shipped runtime.
