# AOCA Current Local Release Run

Date: 2026-06-29

## Corrected Authoritative Run

The earlier `run-0a837879-9eff-433e-a564-967be5ae509a` evidence below is superseded for release authority because it attached `local-ollama-llama3.2` as the `worker-model`. The corrected authoritative local run is `run-964c7049-1e6f-4c8d-9d99-7a955d68de04`.

Corrected run facts:

- Target fingerprint: `b16b2b58a198088e`.
- Worker model: `Qwen2.5-Coder-1.5B-Instruct`.
- Model source: `Qwen/Qwen2.5-Coder-1.5B-Instruct`.
- Model artifact: `knowledge/model-cache/Qwen2.5-Coder-1.5B-Instruct/model.safetensors`.
- Model SHA-256: `c1b9b30e907950516ba3c646bdf570d8084c25a6410a0cdca80cf04b11bc13a8`.
- Model download approval: `ModelDownload` recorded for `run-964c7049-1e6f-4c8d-9d99-7a955d68de04` and fingerprint `b16b2b58a198088e`.
- Runtime clone: `/Users/koshmarus/Library/Application Support/PrismLauncher/instances/MPB AOCA Qwen Release Clone`.
- Live MCP probe: `knowledge/runs/run-964c7049-1e6f-4c8d-9d99-7a955d68de04/reports/mcp-live-probe-2026-06-29-qwen.json`.
- Release report: `knowledge/runs/run-964c7049-1e6f-4c8d-9d99-7a955d68de04/reports/release-report.json`.

The corrected run passed `Extraction`, `Drafting`, `ExperimentPlanning`, `AdapterExpansion`, `RuntimeVerification`, `Validation`, `Bundle`, `PatcherIntegration`, and `ProductValidation`. `Release` still blocks with `PHASE_NOT_IMPLEMENTED`, and `prepare-github` produced a command but did not dispatch because `GitHubReleasePublication` approval was not recorded.

The old `ccd83746388f873b` and transient `5da85c573dced774` fingerprints are superseded. The pipeline now canonicalizes Prism `mmc-pack.json` component metadata before hashing so Prism cache fields such as `cachedVolatile` do not invalidate an otherwise identical AOCA pack. The reviewable source pack and embedded AOCA bundle were rebuilt for `b16b2b58a198088e`.

Historical notes below are retained for investigation context only; any publish/release decision must use `run-964c7049-1e6f-4c8d-9d99-7a955d68de04`.

## Target

- Pack id: `all-of-create-aeronautics`
- Prism instance: `/Users/koshmarus/Library/Application Support/PrismLauncher/instances/All of Create - Aeronautics`
- Superseded run id: `run-374aaf1e-ed08-4e44-9522-c19a9ac7570e`
- Active run id: `run-0a837879-9eff-433e-a564-967be5ae509a`
- Artifact root: `knowledge`
- Exact patch-target fingerprint: `ccd83746388f873b`
- Disposable clone: `knowledge/prism-clones/run-374aaf1e-ed08-4e44-9522-c19a9ac7570e/instance`
- Active run disposable clone: `knowledge/prism-clones/run-0a837879-9eff-433e-a564-967be5ae509a/instance`
- Prism-visible runtime clone: `/Users/koshmarus/Library/Application Support/PrismLauncher/instances/MPB AOCA Release Clone`

## Fingerprint Note

The release-canonical AOCA patch-target fingerprint is `ccd83746388f873b`.

The apparent `f7884143886091c7` mismatch came from running the manual `mpb-knowledge fingerprint` CLI with display-oriented versions (`mpb-knowledge-0.1.0`, `mpb-lab-0.1.0`, `mpb-knowledge-v1`). The release orchestrator's `TargetManager::new()` computes the gate fingerprint with `builderVersion=0.1.0`, `labToolingVersion=mpb-knowledge-lab`, and `knowledgeSchemaVersion=mpb-knowledge-schema`, producing `ccd83746388f873b`.

The MPB assets patcher originally reused the runtime bundle schema version as the fingerprint schema salt. That made the visible Prism clone report `f7884143886091c7` and skip the curated AOCA bundle. The patcher now separates runtime bundle metadata (`mpb-knowledge-v1`) from fingerprint salt metadata (`mpb-knowledge-schema`), so the installed runtime manifest can remain compatible with the Java knowledge loader while matching the release fingerprint.

## Bundle

Commands:

```sh
cargo run -p mpb-knowledge --bin mpb-knowledge -- validate-source knowledge/packs/all-of-create-aeronautics/source
cargo run -p mpb-knowledge --bin mpb-knowledge -- build-bundle knowledge/packs/all-of-create-aeronautics/source knowledge/packs/all-of-create-aeronautics/bundle
cargo run -p mpb-knowledge --bin mpb-knowledge -- inspect-bundle knowledge/packs/all-of-create-aeronautics/bundle/knowledge-index.json
```

Observed:

- `validate-source` passed.
- `build-bundle` rebuilt `knowledge-index.json` for `ccd83746388f873b`.
- `build-bundle` does not refresh `knowledge-index.json.gz`; the compressed tracked artifact was regenerated from the current JSON for patcher embedding.
- `inspect-bundle` reported `all-of-create-aeronautics mpb-knowledge-v1 entities=35096 evidence=6`.
- Current uncompressed bundle stable checksum: `20418df8e492cc34`.
- Current uncompressed bundle SHA-256: `a68a654d1b9e37d8bfc90a4c5c3d65b04a30fae1fa32bf2719f3a3d39c51c258`.
- Current compressed bundle SHA-256: `f461a3304a2aa1a185f8a2e15dce9f9f56eec4bff4763a404484427147863a63`.
- Current compressed bundle size: `4847761` bytes.

## Release Run

Commands:

```sh
cargo run -p mpb-knowledge --bin mpb-knowledge -- release start "$HOME/Library/Application Support/PrismLauncher/instances/All of Create - Aeronautics" --pack-id all-of-create-aeronautics --artifact-root knowledge
cargo run -p mpb-knowledge --bin mpb-knowledge -- approve run-374aaf1e-ed08-4e44-9522-c19a9ac7570e LongRun --artifact-root knowledge --reason "User requested continuing AOCA local knowledge processing and release run on 2026-06-29; approval covers local long-running pipeline phases only, not GitHub publication."
cargo run -p mpb-knowledge --bin mpb-knowledge -- release attach-source run-374aaf1e-ed08-4e44-9522-c19a9ac7570e knowledge/packs/all-of-create-aeronautics/source --artifact-root knowledge
cargo run -p mpb-knowledge --bin mpb-knowledge -- release attach-worker-model run-374aaf1e-ed08-4e44-9522-c19a9ac7570e "$HOME/.ollama/models/blobs/sha256-dde5aa3fc5ffc17176b5e8bdc82f587b24b2678c6c66101bf7da77af9f7ccdff" --identity local-ollama-llama3.2 --checksum sha256:dde5aa3fc5ffc17176b5e8bdc82f587b24b2678c6c66101bf7da77af9f7ccdff --hardware-fit Fits --artifact-root knowledge
cargo run -p mpb-knowledge --bin mpb-knowledge -- target probe-launch run-374aaf1e-ed08-4e44-9522-c19a9ac7570e --artifact-root knowledge
cargo run -p mpb-knowledge --bin mpb-knowledge -- release status run-374aaf1e-ed08-4e44-9522-c19a9ac7570e --artifact-root knowledge
cargo run -p mpb-knowledge --bin mpb-knowledge -- release report run-374aaf1e-ed08-4e44-9522-c19a9ac7570e --artifact-root knowledge
```

Phase results:

- `Intake`: succeeded.
- `Preflight`: succeeded.
- `Approvals`: succeeded after run-scoped `LongRun` approval.
- `Fingerprint`: succeeded with patch-target fingerprint `ccd83746388f873b`.
- `Clone`: succeeded and created the disposable clone.
- `Extraction`: succeeded after attaching the AOCA source pack; coverage summary recorded 81,776 covered obligations and zero blockers.
- `Drafting`: initially blocked on `WORKER_MODEL_MISSING`, then succeeded after attaching the existing local Ollama `llama3.2` model blob. No model download approval was required or recorded.
- `ExperimentPlanning`: succeeded with zero experiment batches.
- `AdapterExpansion`: succeeded with zero project-code-change plans.
- `RuntimeVerification`: blocked on `CLONED_RUNTIME_VALIDATION_MISSING`.

The launch probe recorded `LauncherUnavailable` because `PrismLauncher` was not available in PATH and no `MPB_KNOWLEDGE_PRISM_LAUNCHER` command was configured.

After that probe, the disposable clone was copied into the PrismLauncher instances directory with `name=MPB AOCA Release Clone` so it is visible in the launcher UI after Prism refresh/restart. The stale copied `mpb/runtime.pid` file was removed from this visible clone. A previous duplicate visible clone named `All of Create - Aeronautics MPB Release Clone` had inherited the original instance display name and was moved out of PrismLauncher to `/private/tmp/mpb-prism-cleanup-20260629/`.

PrismLauncher 11.0.2 keeps loaded instance settings in memory and can rewrite `instance.cfg` on save. Do not create a visible Prism clone by copying a folder and editing `instance.cfg` while Prism is open. The Prism source shows that the instance list displays `BaseInstance::name()`, standard Copy uses `CopyInstanceDialog::instName()`, and `InstanceCopyTask::copyFinished()` applies that value with `inst->setName(name())`. For a visible validation clone, either use Prism's built-in Copy action and enter the desired name in the dialog, or fully quit Prism before applying offline `instance.cfg` edits.

## Active Run

The active release run is `run-0a837879-9eff-433e-a564-967be5ae509a`. It was started from the clean disposable AOCA clone after the patcher/runtime-schema fingerprint mismatch was fixed. It reached `ProductValidation` with zero product blockers and target fingerprint `ccd83746388f873b`.

The Prism-visible runtime clone has the corrected patch manifest:

- `knowledgePackId`: `all-of-create-aeronautics`
- `knowledgeFingerprint`: `ccd83746388f873b`
- `knowledgeSchemaVersion`: `mpb-knowledge-v1`
- `knowledgeCompatibility.matched`: `true`
- Managed knowledge bundle checksum: `20418df8e492cc34`

Manual monitoring from 16:17:59 to 16:22:01 Europe/Moscow did not observe a fresh Minecraft launch: `latest.log` stayed at `2026-06-29 15:36:49`, and `http://127.0.0.1:47392/mcp` did not respond.

The later direct PrismLauncher CLI launch succeeded:

```sh
"/Applications/Prism Launcher.app/Contents/MacOS/prismlauncher" --launch "MPB AOCA Release Clone" --show-window
```

Observed runtime evidence:

- Minecraft launched from Prism as `Prism Launcher: MPB AOCA Release Clone`.
- `latest.log` refreshed at 16:35 Europe/Moscow and reported `Minecraft Pack Builder 0.1.0 (mpb)`.
- The MPB runtime logged `[MPB] MCP server listening on http://127.0.0.1:47392/mcp`.
- Minecraft reached the title screen; ModernFix reported `Game took 42.338 seconds to start`.
- `GET http://127.0.0.1:47392/mcp` returned `{"status":"ready","transport":"streamable-http","path":"/mcp"}`.
- `mpb_knowledge_status` returned `available` for pack `all-of-create-aeronautics`, fingerprint `ccd83746388f873b`, schema `mpb-knowledge-v1`.
- Live MCP probes passed for `mpb_search_entities` (`cogwheel`), `mpb_get_entity_card` (`create:cogwheel`), `mpb_get_recipe_graph` (`create:cogwheel`), `mpb_get_mechanic_details` (`kinetic-networks`), and `mpb_get_evidence` (`det-src-aoca-recipes`).

The live MCP probe artifact is `knowledge/runs/run-0a837879-9eff-433e-a564-967be5ae509a/reports/mcp-live-probe-2026-06-29.json`.

The runtime emitted existing-pack warnings/errors that did not prevent title-screen startup or MPB MCP availability:

- `PowerGrid` native backend reported unsupported platform, disabling the accelerated solver.
- `AllTheLeaks` reported a `NoClassDefFoundError` for `net.mehvahdjukaar.supplementaries.common.misc.map_data.ColoredMapHandler`.
- Multiple AOCA/resource-pack model and language warnings were logged.

## Current Blocker

Local release validation is complete through `ProductValidation`, but the orchestrator's `Release` phase is not implemented in this pipeline slice. `release resume` now blocks with `PHASE_NOT_IMPLEMENTED` at `Release`.

Artifacts generated after live runtime validation:

- `knowledge/runs/run-0a837879-9eff-433e-a564-967be5ae509a/cloned-runtime-validation-evidence.json`
- `knowledge/runs/run-0a837879-9eff-433e-a564-967be5ae509a/product-validation-evidence.json`
- `knowledge/runs/run-0a837879-9eff-433e-a564-967be5ae509a/reports/mcp-live-probe-2026-06-29.json`
- `knowledge/runs/run-0a837879-9eff-433e-a564-967be5ae509a/reports/patcher-integration.json`
- `knowledge/runs/run-0a837879-9eff-433e-a564-967be5ae509a/reports/product-validation-report.json`
- `knowledge/runs/run-0a837879-9eff-433e-a564-967be5ae509a/reports/release-report.json`
- `knowledge/runs/run-0a837879-9eff-433e-a564-967be5ae509a/reports/release-report.md`
- `knowledge/runs/run-0a837879-9eff-433e-a564-967be5ae509a/reports/github-release-notes.md`

Active run phase results after attaching runtime/product evidence:

- `RuntimeVerification`: succeeded.
- `Validation`: succeeded with 81,776 covered obligations and zero blockers.
- `Bundle`: succeeded; run-local bundle checksum `2a4974a171249351`, compressed size `4758062` bytes.
- `PatcherIntegration`: succeeded.
- `ProductValidation`: succeeded with `blockerCount=0`.
- `Release`: blocked on `PHASE_NOT_IMPLEMENTED`.

Prepared GitHub publication command:

```sh
gh workflow run release.yml --ref knowledge/all-of-create-aeronautics-ccd83746388f873b -f knowledge_run_id=run-0a837879-9eff-433e-a564-967be5ae509a -f pack_id=all-of-create-aeronautics -f fingerprint=ccd83746388f873b -f report_artifact_path=knowledge/runs/run-0a837879-9eff-433e-a564-967be5ae509a/reports/release-report.json
```

No GitHub publication approval was recorded, and no GitHub workflow was dispatched. `release prepare-github` reported `publicationApproved=false` with missing approval `approval required for GitHubReleasePublication`.

## Automated Verification

Completed successfully:

```sh
cargo run -p mpb-knowledge --bin mpb-knowledge -- validate-source knowledge/packs/all-of-create-aeronautics/source
cargo run -p mpb-knowledge --bin mpb-knowledge -- inspect-bundle knowledge/packs/all-of-create-aeronautics/bundle/knowledge-index.json
cargo test -p mpb-assets --test patcher
cargo test -p mpb-assets knowledge_bundle
cargo test -p mpb-knowledge --test worker_runtime --test experiments --test product_validation
cargo test --workspace
pnpm test
pnpm build
tools/build-minecraft-mod-container.sh
```

`tools/build-minecraft-mod-container.sh` initially exposed build-script/tooling issues before passing:

- `MPB_GRADLE=gradle` was rejected by `build.sh` because the script only accepted executable paths. The script now resolves command names through `command -v`.
- A single `gradle:8.14.3-jdk21` image cannot build all loader targets: ForgeGradle's Minecraft 1.20.1 setup requires JDK 17, while Fabric Loom requires running Gradle on JDK 21. The container wrapper now runs Fabric and NeoForge in the JDK 21 Gradle image and Forge in the JDK 17 Gradle image with a shared Gradle cache.
- The Gradle images do not provide `xxd` or `node`; `build.sh` now falls back to `od`/`fold` when refreshing embedded hex assets.

The passing containerized build refreshed:

- `crates/mpb-assets/src/mpb_mod_fabric_jar.hex`
- `crates/mpb-assets/src/mpb_mod_forge_jar.hex`
- `crates/mpb-assets/src/mpb_mod_neoforge_jar.hex`

## 2026-07-15 Working Tree Recovery

A fresh production container build during working-tree recovery exposed a stale hard-coded fixture
fingerprint in `MpbKnowledgeRuntimeTest`. The test now reads `exactFingerprint` from the installed
fixture bundle before writing its patch manifest, so regenerating the fixture cannot silently make
the Java runtime validation stale.

Tracked mod archives now disable file timestamps and use reproducible file ordering. The `xxd`,
Node, and `od`/`fold` encoders also emit the same 128-byte row width. Two consecutive supported
container builds produced identical SHA-256 values:

- Fabric: `fb757a0d1f4cb69bf0d39068badea8d24f5e269d4edb41b792198a7e64d408e9`
- Forge: `287dcf481266f8d6322a23beb5efc1c1407ecba27961deda0c97ad791b3ede98`
- NeoForge: `e5a0a36df6aec4f946516fcd93e8cff2a2f2976b1c47feab32657babdb25f43e`
