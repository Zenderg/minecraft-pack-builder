# First-Party Knowledge Release Checklist

Use this checklist before shipping any first-party curated knowledge pack or release package that contains one.

## Pack Identity

- [ ] The target Prism instance path, modpack identity, modpack version, Minecraft version, loader family, loader version, knowledge schema version, builder version, and lab tooling version are recorded in the pack validation note.
- [ ] The exact fingerprint was computed from the unpatched target inputs, excluding MPB-managed files that the patcher itself installs.
- [ ] The pack manifest target fingerprint and computed fingerprint match exactly.
- [ ] The patcher compatibility metadata points to the same pack id, schema version, and exact fingerprint as the generated bundle.

## Source Validation

- [ ] `cargo run -p mpb-knowledge --bin mpb-knowledge -- validate-source <pack>/source` passes.
- [ ] The validation report has zero uncovered entities.
- [ ] There are no placeholders, template variables, conflict markers, `unknown`, `todo`, `stub`, or `inferred_only` values.
- [ ] Behavioral claims are linked to accepted runtime observation evidence for the exact fingerprint.
- [ ] Worker/model outputs are recorded only as drafts or decisions; no worker-only output is trusted without deterministic extraction or accepted runtime evidence.
- [ ] Mechanic overlays, traits, relationships, recipes, dependency chains, evidence links, and manifest metadata are complete.

## Runtime Bundle

- [ ] `cargo run -p mpb-knowledge --bin mpb-knowledge -- build-bundle <pack>/source <pack>/bundle` regenerates `knowledge-index.json` from validated source.
- [ ] `cargo run -p mpb-knowledge --bin mpb-knowledge -- inspect-bundle <pack>/bundle/knowledge-index.json` reports the expected pack id, schema version, entity count, and evidence count.
- [ ] Bundle indexes cover entity lookup by id, localized name, tag, use case, mechanic, interface, recipe/dependency graph, mechanic details, evidence, and claims.
- [ ] The runtime bundle contains no raw lab logs, screenshots, saves, notebooks, local worker traces, or generation-only tooling artifacts.
- [ ] Any compressed or embedded bundle artifact checksums match the uncompressed runtime `knowledge-index.json` that the patcher installs.

## Patcher Behavior

- [ ] Patcher install writes the managed mod jar and matching knowledge bundle under `<instance>/mpb/knowledge/<pack-id>/`.
- [ ] Repair restores missing or changed managed knowledge files when the exact fingerprint still matches.
- [ ] Update replaces stale managed knowledge metadata and files.
- [ ] Unpatch removes managed knowledge files and leaves `<instance>/mpb/schemes` intact unless `delete_schemes` is explicitly true.
- [ ] Unsupported fingerprint behavior is verified: the base MPB mod may install when compatible, but mismatched first-party knowledge is not installed and is reported unavailable.
- [ ] Conflict behavior is verified for unmanaged files at managed knowledge paths.

## Minecraft Runtime And MCP

- [ ] The Java runtime rejects missing, malformed, checksum-mismatched, or metadata-mismatched bundles.
- [ ] `mpb_knowledge_status` reports active pack id, exact fingerprint, and schema version when a matching pack is installed.
- [ ] `mpb_search_entities`, `mpb_get_entity_card`, `mpb_get_recipe_graph`, `mpb_get_mechanic_details`, and `mpb_get_evidence` return read-only responses from the installed bundle.
- [ ] With no exact active pack, `mpb_knowledge_status` reports unsupported/unavailable and all other knowledge tools return a clear unsupported response.
- [ ] MCP tool catalog schemas include all knowledge tools and keep them separate from scheme mutation tools.
- [ ] MPB Manager prompt text instructs agents to query curated knowledge when a pack is active.
- [ ] MPB Manager prompt text instructs agents not to claim curated modpack support when no exact pack is active.

## Automated Release Commands

- [ ] `cargo test -p mpb-knowledge`
- [ ] `cargo test -p mpb-assets`
- [ ] `cargo test --workspace`
- [ ] `pnpm test`
- [ ] `pnpm build`
- [ ] `mods/mpb-minecraft-mod/build.sh` with the local Gradle/JDK configuration documented in `mods/mpb-minecraft-mod/README.md`

## Real Prism Client Smoke

- [ ] Build or open the release MPB Patcher, not only the browser/Vite frontend.
- [ ] Patch the target Prism instance through the desktop patcher.
- [ ] Start Minecraft from PrismLauncher.
- [ ] Open MPB Manager with `/mpb`, a loader config entry, or the assigned keybinding.
- [ ] Confirm the displayed MCP endpoint and prompt match the knowledge availability state.
- [ ] Connect an external MCP client or scripted JSON-RPC probe to the running `/mcp` endpoint.
- [ ] Run `mpb_knowledge_status`.
- [ ] Run at least one representative entity search, entity card lookup, recipe/dependency graph lookup, mechanic lookup, and evidence lookup.
- [ ] Record the endpoint, commands/probes, results, and any unavailable manual steps in the pack-specific validation note under `docs/validation/`.

Browser-only validation is not a release substitute for this product. If desktop or Minecraft validation cannot be run on the current machine, stop release packaging and record the exact unavailable steps, environment limitation, and required follow-up owner in `docs/validation/`.
