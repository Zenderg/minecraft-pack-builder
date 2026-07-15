# Runtime Bundle Format

This document is the source of truth for the generated `knowledge-index.json` runtime bundle schema
and validation invariants. Source-pack authoring belongs in the [knowledge overview](README.md),
runtime loading behavior in the [Minecraft mod documentation](../../mods/mpb-minecraft-mod/README.md),
and release sequencing in the [pipeline operator guide](autonomous-release-pipeline.md).

Runtime bundles are generated JSON files named `knowledge-index.json`. They are read-only runtime artifacts: no raw lab logs, model traces, notebooks, or generation tooling are present in the bundle.

Top-level fields:

- `manifest`: pack id/version, exact fingerprint, schema version, builder/lab versions, validation command, validation timestamp, and coverage summary.
- `indexes`: compact read-only lookup indexes for runtime MCP tools.
- `checksums`: checksums for source files used to build the bundle and the generated bundle payload.

Required indexes:

- `entitiesById`
- `entitiesByLocalizedName`
- `entitiesByTag`
- `entitiesByUseCase`
- `entitiesByMechanic`
- `entitiesByInterface`
- `recipeGraphs`
- `mechanicDetails`
- `evidenceById`
- `claimsByEntityId`

Bundles are generated only after `validate_source_pack` succeeds. Query gaps, uncovered entities, incomplete overlays, incomplete dependency chains, placeholders, trusted worker output, missing manifest metadata, fingerprint mismatches, and behavioral claims without runtime evidence are release-blocking validation errors.
