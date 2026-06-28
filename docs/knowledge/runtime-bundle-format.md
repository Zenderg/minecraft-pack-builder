# Runtime Bundle Format

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
