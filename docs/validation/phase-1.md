# Phase 1 Validation: Desktop Shell And Product Workspace

Date: 2026-06-23

## Artifact

- Desktop app dev command: `pnpm tauri dev`
- Frontend production artifact: `dist/index.html`
- Workspace screenshot: `docs/validation/media/phase-1-workspace.png`
- macOS app data folder: `/Users/koshmarus/Library/Application Support/com.mpb.minecraft-pack-builder`
- macOS diagnostics folder: `/Users/koshmarus/Library/Application Support/com.mpb.minecraft-pack-builder/diagnostics`

## What To Open

1. Run `pnpm tauri dev` from the repository root.
2. Inspect the launched macOS desktop window titled `Minecraft Pack Builder`.
3. Open settings in the right rail and use `Open data folder`.
4. Compare the visible workspace with `docs/validation/media/phase-1-workspace.png`.

## User Checklist

- [x] The app launches as a macOS desktop app through Tauri dev.
- [x] The main workspace shape is visible immediately: sidebar, viewer, right review/materials rail, top status strip, settings, and AI connection indicator.
- [x] The UI is dark themed.
- [x] English and Russian are both available through the visible language switch.
- [x] The app discovers and creates its local app data and diagnostics folders.
- [x] The settings surface includes an action for opening the local app data folder.

## Engineering Checklist

- [x] `cargo test --workspace`
- [x] `pnpm test`
- [x] `pnpm build`
- [x] Tauri dev launch on macOS

## Notes

- macOS `screencapture` was unavailable in this sandboxed session with `could not create image from display`, so the validation screenshot was captured from the same Vite surface with headless Chrome while Tauri dev was running.
- The Tauri runtime created both the app data folder and the diagnostics folder during launch.
