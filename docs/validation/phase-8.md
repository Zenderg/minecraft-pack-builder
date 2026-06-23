# Phase 8 Validation: Viewer Selection And Materials

Date: 2026-06-24

Status: scoped to viewer inspection and materials.

## Decision Note

Phase 8 covers viewer inspection around the existing 3D workspace. It does not add an in-app note or status workflow for selected areas; users discuss edits with the external AI client.

The phase should be resumed only after a separate UX/design pass defines the intended behavior for:

- block hover information;
- optional block or area selection;
- camera behavior;
- how selection state should interact with AI tooling.

## Current Accepted Scope

- Stage controls remain available in the right tools rail.
- Materials remain available in the right tools rail.
- Hovering a rendered block may show transient block information without changing the camera.

## Out Of Scope

- No separate in-app note list or status workflow is exposed for selected areas.
- No click-to-select workflow is considered accepted until the UX contract is approved.
- No camera movement should happen as a result of inspecting a block.

## Validation Checklist

- [x] The app does not expose a separate note/status section in the right tools rail.
- [x] The app does not expose selected-area note controls.
- [x] Materials remain visible for the current scheme.
- [x] The right tools rail is bounded so expanded sections scroll inside the sidebar.
- [ ] Selection UX has an approved contract.
- [ ] Selection UX has accepted manual desktop validation.

## Verification Commands

Run after implementation changes:

```bash
cargo test --workspace
pnpm test
pnpm build
```
