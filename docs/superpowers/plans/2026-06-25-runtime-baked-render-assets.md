# Runtime Baked Render Assets Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make the schematic viewer consume runtime-derived baked render assets when a loader extractor provides them, while preserving the existing static JSON model fallback.

**Architecture:** Prism asset indexing produces a registry report with static block metadata and optional runtime render assets. Tauri render-scene selects the best available render asset per blockstate and sends neutral baked elements to the viewer. Three.js keeps rendering local meshes; it does not become a Minecraft renderer.

**Current extractor fidelity:** The embedded Forge, NeoForge, and Fabric extractor jars now emit runtime item stack sizes and server-side shape-derived `renderAssets` with `fidelity: "approximation"` / `source: "minecraft-runtime-shape"` when bounded non-full-cube shapes can be read from the loaded runtime. Exact client-baked mod visuals remain a separate future extractor path.

**Tech Stack:** Rust workspace crates (`mpb-assets`, `src-tauri`), Serde JSON registry reports, React/TypeScript render DTOs, Three.js.

---

### Task 1: Decompose Asset Index Runtime Contracts

**Files:**
- Create: `crates/mpb-assets/src/asset_index/runtime.rs`
- Create: `crates/mpb-assets/src/asset_index/registry_file.rs`
- Modify: `crates/mpb-assets/src/asset_index.rs`

- [x] **Step 1: Move runtime report parsing and launcher code into `asset_index/runtime.rs`.**

Keep the public surface inside `asset_index.rs` small:

```rust
mod asset_index;
```

Runtime module owns cached runtime report parsing, stack sizes, and baked render assets.

- [x] **Step 2: Move compact registry serialization into `asset_index/registry_file.rs`.**

The registry file writer must keep omitting heavy static `modelElements` arrays from the top-level block file, while allowing runtime render assets to be serialized.

### Task 2: Add Baked Render Asset Schema

**Files:**
- Modify: `crates/mpb-assets/src/asset_index.rs`
- Modify: `crates/mpb-assets/src/asset_index/runtime.rs`
- Modify: `crates/mpb-assets/tests/prism_asset_index.rs`

- [x] **Step 1: Write a failing test for cached runtime baked assets.**

The test writes `fingerprint-aoc-content-aoc-runtime.json` with:

```json
{
  "status": "ready",
  "items": [{ "itemId": "thermal:machine_frame", "maxStackSize": 16 }],
  "blocks": [{
    "identifier": "thermal:machine_frame",
    "renderAssets": [{
      "fidelity": "runtimeBaked",
      "source": "minecraft-runtime",
      "elements": [{
        "from": [0, 0, 0],
        "to": [16, 16, 16],
        "faceTexturePaths": { "north": "/tmp/runtime-front.png" },
        "faceUvs": { "north": [0, 0, 16, 16] }
      }]
    }]
  }]
}
```

Expected: the block report exposes one runtime render asset and still merges stack size.

- [x] **Step 2: Implement schema types.**

Add serializable/deserializable types:

```rust
pub struct BakedRenderAssetSample {
    pub fidelity: String,
    pub source: String,
    pub condition: Option<BlockstateModelCondition>,
    pub model: Option<String>,
    pub elements: Vec<ModelElementSample>,
}
```

- [x] **Step 3: Merge runtime render assets by block id.**

Runtime assets win only when present. Static metadata remains available for names, materials, states, and fallback render data.

### Task 3: Send Runtime Assets To Viewer

**Files:**
- Modify: `src-tauri/src/render_scene.rs`
- Modify: `src-tauri/tests/render_scene.rs`
- Modify: `src/renderViewer.ts`

- [x] **Step 1: Write a failing Tauri render-scene test.**

Given registry block metadata with `renderAssets`, `render_scene_from_scheme_with_registry_report` should choose runtime elements for a matching block before static `modelVariants`.

- [x] **Step 2: Add render DTO fields.**

Expose `renderFidelity` and `renderSource` on `RenderBlockDto` / `RenderBlock`, and keep `modelElements` as the neutral geometry payload.

- [x] **Step 3: Keep variant matching semantics.**

For multipart assets, include all matching runtime assets. For non-multipart assets, use the first matching runtime asset.

- [x] **Step 4: Keep static models ahead of runtime shape approximations.**

`exact`/`runtimeBaked` assets can replace static model variants. Shape-derived `approximation` assets are used only when the static model path has no render payload.

### Task 4: Render Runtime-Baked Geometry In Three.js

**Files:**
- Modify: `src/viewer/ThreeSchemeViewer.tsx`
- Modify: `src/viewer/modelGeometry.ts`
- Modify: `src/viewer/modelGeometry.test.ts`

- [x] **Step 1: Write a failing geometry/render grouping test.**

Runtime-baked elements should group by texture, UV, geometry, and fidelity without losing hover mapping.

- [x] **Step 2: Reuse existing model element geometry.**

Runtime-baked assets use the same neutral `RenderModelElement` shape, so the viewer does not need Minecraft-specific logic.

- [x] **Step 3: Include fidelity in material keys.**

This prevents static approximations and runtime-baked geometry from being accidentally merged in diagnostics-sensitive render paths.

### Task 5: Document Durable Product Contract

**Files:**
- Modify: `docs/validation/registry-and-prism-handoff.md`

- [x] **Step 1: Record the runtime-baked viewer contract.**

Document that production fidelity comes from loader runtime extraction, not hand-written TypeScript/Rust Minecraft model emulation.

### Task 6: Implement Bundled Runtime Extractor Render Assets

**Files:**
- Create: `tools/runtime-extractor/**`
- Modify: `crates/mpb-assets/src/runtime_extractor_jar.hex`
- Modify: `crates/mpb-assets/src/runtime_extractor_forge_jar.hex`
- Modify: `crates/mpb-assets/src/runtime_extractor_fabric_jar.hex`
- Modify: `crates/mpb-assets/tests/runtime_extractor_contract.rs`

- [x] **Step 1: Add source-built loader extractor jars.**

Forge, NeoForge, and Fabric entrypoints call a shared reflection-heavy runtime dumper and write JSON to `-Dmpb.runtimeOutput`.

- [x] **Step 2: Emit bounded runtime shape render assets.**

The dumper writes `blocks[].renderAssets` for non-full-cube static voxel shapes that can be read safely from the server runtime.

- [x] **Step 3: Rebuild embedded hex jar artifacts.**

Run `bash tools/runtime-extractor/build.sh` after Java source changes.

- [x] **Step 4: Verify jar contract.**

The contract test decodes embedded jars and checks loader metadata, entrypoint classes, shared runtime dumper, and render asset strings.

### Task 7: Decompose Static Asset Indexing

**Files:**
- Create: `crates/mpb-assets/src/asset_index/static_assets.rs`
- Modify: `crates/mpb-assets/src/asset_index.rs`

- [x] **Step 1: Move static asset collection into `asset_index/static_assets.rs`.**

The top-level asset index file now orchestrates registry building while static model/blockstate/texture parsing lives in its own module.

### Task 8: Verification

**Files:**
- Test only

- [x] **Step 1: Run Rust tests.**

Run:

```bash
cargo test --workspace
```

- [x] **Step 2: Run frontend tests.**

Run:

```bash
pnpm test
```

- [x] **Step 3: Run frontend build.**

Run:

```bash
pnpm build
```
