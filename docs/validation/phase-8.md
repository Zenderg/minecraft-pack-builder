# Phase 8 Validation: Deferred

Date: 2026-06-24

Status: deferred, not implemented, not accepted as complete.

## Decision Note

Phase 8 remains in the main implementation plan, but the review workflow is intentionally not implemented in the current codebase.

An initial experimental slice for block/area selection and change requests was removed because the interaction model was not good enough to keep as product behavior. The phase should be resumed only after a separate UX/design pass defines the intended behavior for:

- block hover information;
- optional block or area selection;
- change request creation;
- camera behavior;
- how review state should interact with AI tooling.

## Current Accepted Scope

- Stage controls remain available in the right tools rail.
- Materials remain available in the right tools rail.
- Hovering a rendered block may show transient block information without changing the camera.

## Not Accepted

- No Review section is exposed in the UI.
- No change request creation, listing, focusing, or resolving flow is exposed.
- No click-to-select review workflow is considered accepted.
- No camera movement should happen as a result of inspecting a block.

## Validation Checklist

- [x] The app does not expose a `Review` section in the right tools rail.
- [x] The app does not expose change request controls.
- [x] Materials remain visible for the current scheme.
- [x] The right tools rail is bounded so expanded sections scroll inside the sidebar.
- [ ] Phase 8 has an approved UX contract.
- [ ] Phase 8 has accepted manual desktop validation.

## Verification Commands

Run after implementation changes:

```bash
cargo test --workspace
pnpm test
pnpm build
```
