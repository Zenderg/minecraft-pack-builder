# Phase 3 Validation: Local Library, SQLite Storage, And Scheme Records

## Artifact

- Rendered workspace screenshot: `/Users/koshmarus/Projects/minecraft-pack-builder/docs/validation/media/phase-3-library.png`
- Desktop/dev entry point: `pnpm dev`, then open `http://127.0.0.1:5173/`
- SQLite app data file in the desktop app: `<app data dir>/library.sqlite3`

## What Changed

- The local library is backed by SQLite migrations and Rust repository methods.
- Imported modpacks and schemes are shown as a two-level sidebar tree.
- Imported modpack rows expand/collapse when clicking the modpack name.
- The left sidebar is resizable between fixed minimum and maximum widths.
- Imported modpack actions are grouped behind a `...` menu with information, rename, and delete actions.
- The modpack action menu closes when clicking elsewhere, and scheme rows are selectable across the full row.
- Scheme create, rename, and delete flows use autosaved backend operations.
- Imported modpack rename and delete flows are wired through the same SQLite repository.
- Duplicate imported modpack names receive numeric suffixes, for example `AOC - 1.0.0 (2)`.
- A seeded local library fixture is available for phase validation.

## Manual Validation

1. Launch the app or dev server.
2. Skip onboarding if it appears.
3. Confirm the sidebar shows imported modpacks with schemes nested underneath.
4. Click a modpack name and confirm its schemes collapse/expand.
5. Drag the sidebar divider and confirm the sidebar resizes within reasonable limits.
6. Use the `+` action on a modpack, enter a scheme name, and confirm that the scheme appears.
7. Use the `...` action on a modpack, then click elsewhere and confirm the menu closes.
8. Use the `...` action on a modpack and open information, rename, and delete flows.
9. Click empty space inside a scheme row and confirm the scheme still selects.
10. Use the pencil/delete actions on a scheme and confirm the item updates or disappears.
11. Quit and relaunch the desktop app, then confirm persisted names/deletions remain.
12. Use the seeded fixture or duplicate fixture import to confirm numeric suffixes are visible.

## Checklist

- [x] Sidebar shows modpacks and schemes as a two-level tree.
- [x] Sidebar library tree is unframed inside the main sidebar to reduce nested panel weight.
- [x] Imported modpacks collapse/expand by clicking the modpack name.
- [x] Left sidebar can be resized between fixed min/max widths.
- [x] Imported modpack row actions are grouped into `+` and `...`, with information in a modal.
- [x] Imported modpack action menu closes on outside click.
- [x] Scheme rows can be selected by clicking the full row, not only the text/icon cluster.
- [x] Imported modpack rows use compact vertical spacing.
- [x] A scheme belongs to exactly one imported modpack.
- [x] SQLite migrations include imported modpacks, schemes, dimensions, stages, change requests, settings metadata, and import status.
- [x] Scheme create, rename, and delete operations autosave through Rust storage.
- [x] Imported modpack rename/delete operations autosave through Rust storage.
- [x] Deleting an imported modpack cascades schemes and returns the local cache path for removal.
- [x] Duplicate imported modpack names become distinct with numeric suffixes.
- [x] Rendered browser smoke test created `QA Library Tower` through the in-app modal without new console warnings or errors.

## Engineering Validation

- `cargo test --workspace`
- `pnpm test`
- `pnpm build`
