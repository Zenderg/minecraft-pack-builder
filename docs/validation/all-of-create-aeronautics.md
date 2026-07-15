# All of Create - Aeronautics Validation Summary

This document is the source of truth for the latest accepted human-reviewed validation status of the
current All of Create - Aeronautics first-party knowledge-pack fingerprint. It contains only the
latest accepted gate status and links to owning contracts and exact run evidence; superseded
fingerprints, exploratory commands, raw logs, and local absolute paths do not belong here.

## Accepted Target

- Pack id: `all-of-create-aeronautics`
- Modpack version: `v2.0`
- Minecraft: `1.21.1`
- Loader: `NeoForge 21.1.233`
- Knowledge schema: `mpb-knowledge-v1`
- Exact patch-target fingerprint: `b16b2b58a198088e`
- Accepted run: `run-964c7049-1e6f-4c8d-9d99-7a955d68de04`

Machine-readable identity and coverage are owned by the
[pack sources](../../knowledge/packs/all-of-create-aeronautics/README.md). Fingerprint normalization
and exclusions are owned by the [fingerprint contract](../knowledge/fingerprints.md).

## Latest Accepted Result

The accepted run passed `Extraction`, `Drafting`, `ExperimentPlanning`, `AdapterExpansion`,
`RuntimeVerification`, `Validation`, `Bundle`, `PatcherIntegration`, and `ProductValidation` with
zero product blockers. It used `Qwen2.5-Coder-1.5B-Instruct` as the recorded local worker model and
accepted real cloned-Prism runtime evidence plus live MCP probes for knowledge status, entity search,
entity card, recipe graph, mechanic details, and evidence lookup.

Exact generated evidence remains local under:

- `knowledge/runs/run-964c7049-1e6f-4c8d-9d99-7a955d68de04/reports/mcp-live-probe-2026-06-29-qwen.json`
- `knowledge/runs/run-964c7049-1e6f-4c8d-9d99-7a955d68de04/reports/release-report.json`

The orchestrator's `Release` phase remains blocked with `PHASE_NOT_IMPLEMENTED`. GitHub publication
was prepared but not dispatched because no `GitHubReleasePublication` approval was recorded.
Therefore this summary proves validation through `ProductValidation`; it does not claim that a
GitHub release was published.

Every future release must rerun the
[first-party knowledge checklist](first-party-knowledge-release-checklist.md) and replace this
summary with the latest accepted result. Pipeline behavior is defined by the
[requirements](../knowledge/release-pipeline-requirements.md) and
[operator guide](../knowledge/autonomous-release-pipeline.md).
