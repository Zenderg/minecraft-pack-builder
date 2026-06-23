# Phase 7 Validation: Render Preparation And Three.js 3D Viewer

Date: 2026-06-24

## Artifact

- Browser/dev viewer: `http://127.0.0.1:5173/`
- Production build output: `dist/index.html`
- Backend demo scene command: `get_scheme_render_scene`
- Rust render crate: `crates/mpb-render`
- Frontend viewer modules:
  - `src/renderViewer.ts`
  - `src/ViewerWorkspace.tsx`

## What To Open

1. Run `pnpm dev`.
2. Open `http://127.0.0.1:5173/`.
3. Skip onboarding if needed.
4. Select `AOC - 1.0.0` -> `Starter Factory` in the sidebar.

The viewer should show stage controls, a 3D canvas, render metrics, and selection coordinates after clicking a block.

## User Checklist

- [x] A scheme opens into a real viewer surface instead of the phase 1-6 decorative placeholder.
- [x] Stage controls are visible for `Stage 1`, `Stage 2`, and `Unassigned`.
- [x] `Stage 1` shows the stage 1 cumulative subset.
- [x] `Stage 2` shows stages 1 and 2.
- [x] `Unassigned` is available as its own v1 display group.
- [x] Viewer metrics show visible blocks, total blocks, chunk count, and face count.
- [x] The viewer resizes through `ResizeObserver`.
- [x] Clicking rendered blocks reports coordinates through the selection overlay/right rail when WebGL is available.
- [x] A non-WebGL fallback preview remains visible for test and constrained environments.
- [x] Empty, loading, error, and large-scheme UI states exist.

## Engineering Checklist

- [x] `mpb-render` prepares render chunks and compact mesh buffers.
- [x] Adjacent opaque internal faces are skipped.
- [x] Picking metadata is emitted per generated face.
- [x] Cumulative stage visibility is covered by tests.
- [x] Optional future-stage translucency is covered by tests.
- [x] Tauri exposes `get_scheme_render_scene` for the desktop viewer.
- [x] Frontend helper tests cover stage filtering and metrics.
- [x] React viewer test covers stage controls and selected-stage counts.

## Verification Commands

Passed:

```bash
cargo test --workspace
pnpm test
pnpm build
curl -s -D - http://127.0.0.1:5173/
```

`pnpm build` emits a Vite chunk-size warning for the dynamically imported Three.js bundle. The build succeeds.

## Screenshot Note

The validation contract asks for UI screenshots under `docs/validation/media/`. In this Codex thread, the Browser plugin was not available and the repository did not have Playwright/Puppeteer installed, so I could not capture an automated screenshot without adding a browser automation dependency that is not otherwise part of the app.

Manual screenshot target: `docs/validation/media/phase-7-viewer.png`.
