# Phase 4 Validation

## Openable Artifacts

- JSON validation report: `docs/validation/phase-4-domain-demo-report.json`.
- Desktop app diagnostics report: in the Tauri app, the `generate_domain_demo_report` command writes `phase-4-domain-demo-report.json` into the diagnostics folder.

The temporary in-app demo panel used during implementation was removed before push so the main workspace remains focused on real library, viewer, materials, and review surfaces. Phase 4 remains validated through the JSON report, backend diagnostics command, and automated domain tests.

## User Checklist

- [x] The JSON demo scheme report shows dimensions, stage count, block count, and material count.
- [x] Invalid operations are shown as rejected actions in the validation report.
- [x] The materials list reflects valid operations after placement and bulk changes.
- [x] `Unassigned` blocks are included in final block/material counts.
- [x] The JSON report is readable without opening developer tools or a running app.

## Engineering Checklist

- [x] Rust unit tests cover placing, bulk setting, bounds validation, resize rejection, stage visibility, material generation, and structured validation errors.
- [x] Bulk operations validate before mutation and keep the scheme unchanged on invalid input.
- [x] Synthetic block registry fixtures are available through `BlockRegistry::synthetic_fixture`.
- [x] Tauri writes the demo JSON report into diagnostics.
- [x] Frontend tests cover demo summary and `Unassigned` material presentation.

## Verification Commands

- `cargo test -p mpb-core`
- `cargo test -p app-tauri domain_demo_report`
- `pnpm test src/phase4Demo.test.ts`
- `pnpm build`
