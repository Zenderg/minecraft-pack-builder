# Minecraft Pack Builder: First-Party Modded Knowledge Packs

This document is the source of truth for the product-level support and trust contract of
first-party modded knowledge packs. Exact formats, fingerprint computation, worker policy, runtime
bundle layout, and release operations belong in the focused [`docs/knowledge`](../knowledge/README.md)
documents; patcher and runtime behavior belongs in the [core product contract](patcher-and-minecraft-mod.md).

## 1. Purpose

This document turns the earlier modded-Minecraft knowledge-lab direction into a production-grade product and architecture contract.

Minecraft Pack Builder should let an external AI agent design build schemes for a supported modded Minecraft environment without pretending that a generic model can understand arbitrary mods on the fly. The supported path is first-party curated knowledge packs: MPB developers build, test, verify, and bundle evidence-backed knowledge for exact target modpack fingerprints.

The reference target for this design is `All of Create - Aeronautics`. The architecture should support other curated target modpacks later, but each production knowledge pack is tied to an exact modpack fingerprint.

## 2. Product Contract

MPB does not ship a partial knowledge system as trusted product behavior. If a first-party knowledge pack is bundled into a release, external agents may treat it as the trusted source of truth for its exact supported modpack fingerprint.

Production-ready means:

- every discovered relevant modpack entity is represented;
- every discovered block, item, recipe, fluid, entity, tag, config/datapack/script/resource/data-pack input, mechanic, and relationship that can affect schemes is covered;
- every block and build-relevant entity has behavior, interfaces, use cases, limitations, dependencies, and evidence;
- every mod-added property that affects building or mechanical use is recorded as a mechanic trait or overlay;
- no shipped trusted pack contains `unknown`, `todo`, `stub`, `inferred_only`, unresolved coverage, or placeholder knowledge;
- any unresolved block, item, mechanic, overlay, or dependency blocks the production-ready status of the entire pack.

The end-user flow stays simple:

1. The user downloads one MPB patcher file.
2. The user selects a PrismLauncher instance.
3. The patcher installs the managed MPB Minecraft mod and the bundled first-party knowledge pack when the instance matches the pack fingerprint.
4. The user starts Minecraft, opens MPB Manager, copies the MCP prompt/endpoint, and connects an external AI agent.
5. The external agent uses MPB MCP tools to create and edit schemes and to query the bundled knowledge.

End users do not run local model inference, fine-tuning, lab tooling, knowledge generation, or knowledge validation.

## 3. Supported Fingerprint Model

Knowledge-enabled mode requires an exact fingerprint match. For `All of Create - Aeronautics`, the fingerprint includes at least:

- modpack identity and version;
- Minecraft version;
- loader and loader version;
- full mod list and versions;
- configs;
- datapacks;
- KubeJS/CraftTweaker or equivalent scripts;
- resource/data packs that can affect registries, recipes, tags, models, or mechanics;
- MPB knowledge schema version;
- MPB knowledge-builder and lab-tooling version.

One patcher release supports one exact production knowledge fingerprint per target pack. Version ranges, "close enough" matching, and user override are not trusted production behavior.

If the selected Prism instance does not match the bundled knowledge fingerprint:

- the patcher may still install or repair the base MPB mod if the loader/Minecraft version is otherwise supported;
- knowledge tools are disabled or reported unavailable;
- the copied agent prompt must clearly say that curated modpack knowledge is unsupported for this instance;
- MPB must not allow an external agent to use mismatched first-party knowledge.

## 4. Knowledge Scope

The knowledge pack covers the full modpack graph, not only blocks. Blocks are the central object for MPB schemes, but useful scheme generation also depends on items, recipes, fluids, entities, tags, configs, and mechanic relationships.

The pack includes:

- block ids, block states, item forms, localized names where useful, tags, and registry metadata;
- recipe and processing chains, including inputs, outputs, catalysts, required machines, fuels, fluids, energy, and intermediate dependencies;
- block and item behavior in the world;
- interfaces such as inventory, fluid, energy, kinetic, redstone, GUI, collision, contraption, vessel, multiblock, recipe-machine, and entity interaction channels;
- use cases: when an entity is useful for scheme generation;
- avoid cases: when an entity is a bad choice or not useful for a requested function;
- hard requirements and constraints;
- compact evidence summaries;
- relationships between entities and mechanics.

Player progression is out of scope. MPB does not infer whether the player is early-game, mid-game, late-game, or quest-gated. Instead, the pack records recipes and dependencies so an external agent can respond correctly when a user supplies constraints such as "I do not have brass" or "avoid precision mechanisms." The agent may then select alternatives or honestly report that the requested function cannot be built without required resources.

## 5. Mechanic Traits And Overlays

Modded mechanics often add important properties that are not visible in ordinary block registries. The knowledge pack treats these as first-class mechanic traits and overlays.

A mechanic trait is a build-relevant property attached to an entity. Examples:

- mass;
- buoyancy;
- drag;
- stress impact;
- kinetic input/output role;
- valid vessel material;
- contraption assembly behavior;
- redstone role;
- recipe-machine role;
- fluid or inventory capacity;
- stability modifier;
- required tool or activation item;
- "breaks when assembled" or "cannot move on contraptions."

A mechanic overlay is a modpack-specific system that applies traits or relationships across many entities. Examples:

- Aeronautics vessel physics;
- Create kinetic networks;
- Create contraption assembly;
- processing and logistics graphs;
- custom recipe or progression scripts;
- modpack-specific config changes.

Release gates must verify not only that every block has a card, but also that every discovered overlay has complete coverage for all applicable entities. If a mod assigns mass to blocks, the production pack must know the mass behavior for all applicable blocks, not only the obvious Aeronautics blocks.

## 6. Evidence Model

Knowledge is stored as evidence-backed claims. A claim is an atomic statement the system may use, such as:

- a block has a specific mechanic trait;
- an item is required by a recipe chain;
- two entities interact through a specific channel;
- a block is useful for a stated use case;
- a block is not useful for a stated use case;
- a runtime experiment confirmed a behavior;
- a runtime experiment rejected a hypothesis.

Trusted claims must be backed by one or more evidence summaries. Evidence summaries are compact, structured records, not raw logs. They are designed for agent consumption without flooding MCP responses.

Example shape:

```json
{
  "evidence_id": "aoca_create_press_depot_runtime_001",
  "method": "runtime_lab",
  "result": "create:mechanical_press pressed minecraft:iron_ingot on create:depot into create:iron_sheet while powered at 32 RPM",
  "observed_inputs": ["create:mechanical_press", "create:depot", "minecraft:iron_ingot"],
  "observed_outputs": ["create:iron_sheet"],
  "limits": ["single recipe path tested", "belt throughput not measured by this evidence"],
  "verified_for_fingerprint": "aoca:<exact-fingerprint>"
}
```

Raw lab artifacts, logs, snapshots, local notebooks, and worker traces are local developer artifacts. They are not committed or shipped by this product contract. The production pack contains trusted structured knowledge and compact evidence summaries only.

## 7. Interaction Coverage

The pack does not run meaningless pairwise tests for every entity combination. Interaction testing is hypothesis-driven and graph-driven.

The builder first discovers typed interaction channels and mechanic graphs. Examples:

- kinetic graph;
- recipe-processing graph;
- vessel/airship graph;
- inventory and logistics graph;
- fluid graph;
- redstone graph;
- multiblock graph;
- contraption membership graph;
- entity interaction graph.

Pairwise or multi-entity experiments run only when there is a structural reason to expect interaction: shared interface, shared recipe, shared tag, shared capability, mechanic overlay membership, documented requirement, config relationship, or previous evidence conflict.

This is still strict coverage. "No interaction expected" is itself a verified negative result based on typed probes and graph membership, not an excuse to ignore entities.

## 8. Developer-Side Knowledge Builder

Knowledge generation is developer-side infrastructure. It never becomes an end-user feature.

The developer workflow:

1. Select the exact target Prism/modpack instance.
2. Compute the target fingerprint.
3. Run deterministic extraction over registries, block states, recipes, tags, language data, configs, datapacks, scripts, resource/data packs, and accessible guidebook/tooltip/manual data.
4. Build the initial entity graph and mechanic overlay candidates.
5. Optionally use local model workers for draft classification, summarization, conflict detection, and experiment proposals.
6. Run dev-only lab experiments in a disposable local client world or isolated lab area.
7. Convert observations into structured evidence summaries.
8. Build source knowledge records.
9. Run strict local validation.
10. Build the compact runtime knowledge bundle.
11. Commit only the production knowledge sources and runtime bundle required for the patcher release.

Local model workers are allowed only as assistants. They may draft, classify, summarize, propose experiments, or detect inconsistencies. They are not a source of truth. Their output must either be backed by deterministic extraction/runtime evidence and pass gates, or be rejected.

Fine-tuning is not silently deferred or forgotten. Each production pack records one of these decisions:

- no fine-tuning used because worker outputs are not trusted directly and all trusted knowledge passed deterministic/runtime gates;
- fine-tuning used for a named worker task, with the model, dataset, evaluation threshold, and result recorded;
- fine-tuning required because current worker quality blocks the pack.

## 9. Dev-Only Lab Mod

The lab runner is a separate developer-only Minecraft mod/artifact. It is not installed by the user patcher and is not present in the normal MPB runtime.

The lab mod runs in a local client Prism instance with a disposable world or isolated lab area. Dedicated-server/headless operation is not part of the production contract. It may be researched later as an optimization, but the canonical lab target is the client environment where the target modpack actually runs.

The lab mod exposes high-level experiment operations for the builder, such as:

- prepare or reset a lab area;
- place structures;
- set valid block states;
- use items on blocks;
- run bounded ticks;
- inspect selected blocks, block entities, inventories, fluids, energy, kinetic state, vessel traits, or other observable modded state where APIs allow;
- compare before/after snapshots;
- record structured observations.

Lab experiments are batch-first for release readiness. Interactive exploration is allowed during development, but it is not a release gate. A production-ready pack requires a local command that runs the full coverage suite and fails on any uncovered entity, failed experiment, unresolved mechanic, stale fingerprint, placeholder, or invalid bundle.

## 10. Validation Gates

The local validation command is strict and blocking. It must fail the pack if any gate fails.

Required gates:

- **Fingerprint gate:** source records and runtime bundle match the exact target fingerprint.
- **Coverage gate:** every discovered entity and build-relevant relationship has a production card or record.
- **Trait/overlay gate:** every discovered mechanic overlay has complete coverage for applicable entities.
- **Experiment gate:** every behavioral or mechanical claim has runtime evidence unless it is strictly deterministic extracted data such as a recipe, tag, or registry property.
- **Recipe/dependency gate:** recipe and dependency chains are complete enough for an external agent to reason about user-supplied resource constraints.
- **No unresolved gate:** no shipped trusted source contains `unknown`, `todo`, `stub`, `inferred_only`, placeholder text, or unresolved conflict markers.
- **Worker gate:** model-worker outputs are not trusted unless converted into evidence-backed records that pass validation.
- **Bundle gate:** the runtime bundle can answer all required read-only MCP query types without falling back to raw logs or model inference.
- **README/manifest gate:** pack manifest records fingerprint, source version, schema version, builder/lab version, validation command, validation timestamp, and coverage summary.

These gates validate knowledge before release. The user runtime does not revalidate the knowledge pack. It trusts the bundled pack when the fingerprint matches.

## 11. Repository And Bundle Format

The source of truth should remain reviewable structured files in the repository. JSON or JSONL are preferred for source records because they are easy to diff, validate, and regenerate.

The shipped runtime artifact may be a compact indexed bundle optimized for Java-side read-only queries. It does not need to be SQLite if that complicates the Minecraft mod. Acceptable runtime bundle forms include:

- compact JSON indexes;
- compressed JSONL indexes;
- binary indexes generated from source records;
- another read-only format that the Minecraft mod can load safely.

The important contract is source/build separation:

- source records are human-reviewable and validation-friendly;
- runtime bundle is generated from source records;
- runtime bundle is read-only in the user instance;
- patcher manages the bundle as a checksummed file in the instance;
- raw local lab artifacts are not committed or shipped.

## 12. Patcher Integration

The MPB patcher remains the one-file user-facing installer. It manages the knowledge bundle the same way it manages MPB-owned mod artifacts.

The patcher installs managed files such as:

```text
<instance>/mpb/config.json
<instance>/mpb/schemes/*.mpb.json
<instance>/mpb/cache/
<instance>/mpb/knowledge/<pack-id>/
<instance>/mpb/patch-manifest.json
```

The patch manifest records knowledge bundle files, checksums, pack id, exact fingerprint, schema version, and compatibility metadata.

Patch statuses must account for knowledge files:

- `Patched`: MPB mod and matching knowledge bundle are present and unchanged.
- `Needs update`: bundled MPB mod or knowledge bundle differs from the patcher release.
- `Needs repair`: managed knowledge files are missing or modified.
- `Unsupported`: loader/Minecraft version is unsupported, or knowledge-enabled mode is requested for a nonmatching fingerprint.
- `Conflict`: unmanaged files conflict with the managed MPB mod or knowledge bundle path.

Unpatch removes managed knowledge files. It must not remove user schemes unless the user explicitly asks to delete schemes.

## 13. User Runtime And MCP Surface

The user MPB Minecraft mod hosts the MCP server as it does today. Knowledge query tools live in the Minecraft mod, not the Tauri patcher app and not a separate local service. This preserves the current flow: run Minecraft, open MPB Manager, copy endpoint/prompt, connect an external agent.

The knowledge MCP surface is read-only and minimal:

- search entities by id, name, tag, use case, mechanic, or interface;
- get an entity card;
- get recipe and dependency graph slices;
- get mechanic details and use-case records;
- get compact evidence summaries for specific claims or entity records;
- report the active knowledge pack id, fingerprint, schema version, and coverage summary.

The runtime surface does not include local model inference, fine-tuning, knowledge generation, lab experiments, or broad scheme proof. Basic existing scheme sanity checks may remain, but the knowledge system does not ask the user agent to revalidate trusted bundled knowledge.

The external AI agent is responsible for design reasoning. MPB supplies trusted facts and scheme-editing tools.

## 14. Agent Behavior Contract

The agent should use MPB as a grounded tool layer:

- query the knowledge pack instead of guessing mod mechanics;
- use recipe/dependency data when the user gives resource constraints;
- ask for evidence summaries only when explanation or confidence is needed;
- avoid requesting raw logs or whole-pack dumps;
- create and edit schemes through existing MPB scheme tools;
- honestly report impossibility when user constraints remove required resources or mechanics.

If knowledge tools are unavailable because the fingerprint does not match, the prompt must tell the agent not to claim curated modpack support.

## 15. Rejected Directions

These are rejected for this design:

- universal support for arbitrary modpacks from generic LLM inference;
- user-side local knowledge generation;
- user-side local model inference or fine-tuning;
- user-side lab experiments;
- shipping raw lab logs or snapshots as the user-facing knowledge source;
- hidden lab tools in the normal user MPB mod;
- version-range compatibility for trusted knowledge packs;
- treating unverified model-worker output as product knowledge;
- pairwise testing every entity against every other entity without a mechanic reason;
- requiring a separate desktop app or daemon during agent use.

## 16. Ownership Boundaries

The production contract is divided across focused subsystem ownership:

- source schema, fingerprinting, worker policy, runtime bundles, and release operations belong in [`docs/knowledge`](../knowledge/README.md);
- the dev-only lab and batch experiment command contract belongs in the [Knowledge Lab README](../../mods/mpb-knowledge-lab/README.md);
- Minecraft read-only knowledge tools and bundle loading belong in the [runtime README](../../mods/mpb-minecraft-mod/README.md);
- patcher installation, update, repair, and unpatch behavior belongs in the [core product contract](patcher-and-minecraft-mod.md);
- pack identity and source records belong under `knowledge/packs/<pack-id>/`;
- repeatable release gates and concise current results belong under [`docs/validation`](../validation/README.md).

Experimental local output must not be described as production-ready or shipped as trusted knowledge.
