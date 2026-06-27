# Minecraft Pack Builder: Modded Knowledge Lab Direction

Date: 2026-06-27

## 1. Purpose

This document records the product and architecture conclusions from the modded-Minecraft feasibility discussion. It is intentionally product-facing and non-personal: the direction is framed around a selected target modpack, not around any individual user's private play context.

The original MPB idea assumed a layer between an external AI agent and a Minecraft player, where the agent could design build schemes for a specific modded Minecraft environment. The hard part is not placing modded blocks in a scheme. The hard part is understanding what modded blocks do, which structures are mechanically valid, which alternatives exist, and which rules are changed by a concrete modpack.

The current conclusion is that MPB should not promise universal mod understanding from a generic LLM. A more honest and potentially viable direction is a knowledge-production pipeline: extract facts from a concrete modpack, run controlled experiments in a real Minecraft runtime, store evidence-backed knowledge, and let an orchestrating agent use that knowledge when generating schemes.

## 2. Decisions

### 2.1 Universal Mod Understanding Is Not A Product Contract

MPB should not claim that an agent can reliably infer correct mechanics for arbitrary mods from block registries, recipes, assets, or raw Java sources.

Registries and block states can tell an agent which blocks exist and which state properties are valid. They do not reliably explain multiblock validation rules, internal volumes, controller placement, energy or fluid network behavior, block mass, mod-specific physics, server-side constraints, or modpack-specific config changes.

Raw source or decompiled Java is closer to the truth, but it is still not a practical product contract for an LLM. The runtime behavior may depend on tick logic, events, mixins, loader APIs, capabilities, configs, datapacks, integration mods, block entity state, GUI settings, and item interactions. A model may infer useful patterns from code, but MPB must not treat that as a guarantee.

### 2.2 The Immediate Scope Is One Selected Target Modpack

The knowledge pipeline is not initially intended to process hundreds of modpacks. The practical target is one selected modpack, or a small number of closely managed target environments, where deeper curated knowledge has value.

This matters because a one-modpack target allows an offline knowledge-production process:

- index the modpack;
- run local experiments;
- record observations;
- review and correct generated notes;
- build a reusable knowledge pack;
- ship or consume that knowledge later during scheme generation.

The user-facing flow should not be "launch a model on any modpack and wait while it discovers everything." The more credible flow is "MPB uses prepared knowledge packs and can optionally run bounded local checks for the currently selected environment."

### 2.3 Knowledge Packs Are The Product Asset

The central durable artifact should be a versioned mod knowledge pack, not a hidden prompt, a pile of unstructured notes, or a model weight file alone.

Knowledge packs should contain evidence-backed facts tied to:

- mod id;
- mod version;
- Minecraft version;
- loader;
- relevant config or datapack context;
- source type;
- evidence id;
- confidence level.

Knowledge should distinguish between at least these confidence levels:

- `runtime_verified`: confirmed by a controlled lab experiment in a real modded runtime.
- `extracted`: derived from structured assets, recipes, tags, configs, or runtime registries.
- `documented`: taken from guidebook/wiki/tooltip/manual content.
- `inferred`: model-generated or heuristic and not yet independently verified.
- `rejected`: tested or reviewed and found false for this environment.

Only `runtime_verified`, `extracted`, and reviewed `documented` claims should be treated as strong scheme-generation context.

## 3. Rejected Directions

### 3.1 Built-In Universal Mod Adapters

Hand-written adapters for many individual mods are rejected as the default strategy. They create the appearance of universal mod support while actually encoding narrow, maintenance-heavy exceptions. They may still exist later for especially important mechanisms, but they should not be the foundation of the product direction.

### 3.2 Generic "Read All Java And Understand The Mod" Flow

Giving an agent raw Java or decompiled mod sources through MCP is rejected as a reliable foundation. It may help with investigation, but it is too expensive, too version-sensitive, and too easy to over-trust.

### 3.3 Training A New Model From Scratch

Training a custom neural model from scratch is rejected. The project does not start with enough data, compute, evaluation infrastructure, or proof that model weights are the primary bottleneck.

### 3.4 A Player Bot In A Real User World

A bot that joins a normal user world or server as a real player is rejected as the default path. It is too invasive and can affect shared gameplay state. If bot-like control is researched later, it should target isolated disposable environments.

### 3.5 Screen-Controlled Minecraft Automation

Driving a real Minecraft client through screen, mouse, and keyboard automation is rejected as a core architecture. It is slow, fragile, difficult to reproduce, and poorly suited to exact modded-state inspection.

## 4. Proposed Architecture

### 4.1 Orchestrator Plus Local Workers

The strongest working model is a two-layer intelligence system:

```text
large external orchestrator
  -> local knowledge system
      -> deterministic extractors
      -> small local model workers
      -> instrumented Minecraft lab runner
      -> evidence-backed knowledge repository
  -> MPB scheme generation and confidence report
```

The orchestrator remains responsible for high-level planning, trade-off reasoning, user communication, and final scheme design. It should not read an entire large modpack directly. It should query the local knowledge system, request experiments, and consume compact evidence-backed facts.

Small local models should act as cheap workers, not as the main designer. They can process many small tasks that would be too expensive to send repeatedly to a large model.

### 4.2 Small Model Worker Tasks

Candidate worker tasks:

- classify block roles from block id, namespace, lang keys, tags, recipes, and tooltip text;
- convert guidebook/manual/tooltip text into structured claims;
- summarize lab experiment logs into compact observation records;
- propose likely next experiments from a hypothesis and current evidence;
- detect conflicts between a new claim and existing knowledge;
- convert high-level construction steps into strict MPB tool-operation JSON;
- validate that generated JSON matches a schema before it reaches the Minecraft runtime.

These tasks are narrow enough to fine-tune with LoRA later. They do not require the small model to become a general Minecraft engineer.

### 4.3 Suggested Model Strategy

Do not train a model from scratch. Start with an open-weight small model and use it without fine-tuning first. Record inputs, outputs, corrections, and experiment outcomes. Fine-tune only after the project has a real dataset.

Plausible starting candidates:

- `Qwen2.5-Coder-1.5B-Instruct` for structured JSON, tool calls, code-like DSL, logs, and scheme operations.
- `Qwen3-1.7B` or `Qwen3-4B` for slightly broader reasoning worker tasks.
- `SmolLM2-1.7B-Instruct` as a lightweight on-device alternative.

The preferred first worker candidate is `Qwen2.5-Coder-1.5B-Instruct`, because the early tasks are mostly structured transformation and validation, not open-ended scheme design.

Fine-tuning should use LoRA or another parameter-efficient method. Fine-tuning is not done per modpack as "the model learns this modpack." Instead, knowledge extraction and lab results create training examples over time. Fine-tuning is periodic and task-specific.

### 4.4 Knowledge Builder Flow

The knowledge builder for a selected modpack should work roughly like this:

1. Discover installed mods, versions, loader, Minecraft version, configs, datapacks, and resource data.
2. Extract registry ids, block states, recipes, tags, language strings, tooltips where available, and guidebook-like content where accessible.
3. Run deterministic indexing and initial heuristics.
4. Use local worker models to draft candidate claims and classify blocks.
5. Select a bounded set of high-value hypotheses for lab testing.
6. Run experiments in an isolated Minecraft lab environment.
7. Record raw observations and compact summaries.
8. Mark claims as verified, rejected, extracted, documented, or inferred.
9. Store the resulting knowledge pack in a versioned repository.
10. Make only compact, relevant facts available to the scheme-generation orchestrator.

## 5. Minecraft Lab Direction

### 5.1 MCP Is Transport, Not Magic

MCP is acceptable as the controlled tool interface. It does not make Minecraft automation work by itself. The useful behavior must be implemented behind MCP by MPB tooling.

The lab should expose high-level tools instead of raw world dumps:

- prepare or reset an isolated lab area;
- place a small structure;
- set block states where valid;
- use an item on a block;
- run a bounded number of ticks;
- inspect selected blocks and block entities;
- measure changes in inventories, fluids, energy, or other observable state where APIs allow it;
- compare before/after snapshots;
- record an observation.

The agent should receive compact signals, not full world snapshots or huge NBT dumps by default.

### 5.2 Instrumented Lab Mod Over Player Bot

The preferred lab architecture is an instrumented MPB Minecraft mod running inside the target modpack. The mod exposes lab operations through MCP or an internal lab API. It manipulates an isolated area or disposable world and returns structured observations.

This is preferred over a protocol bot because modded Minecraft behavior often depends on loader APIs, block entities, custom interactions, custom GUIs, and mod-specific networking. A protocol bot is useful research context, but it should not be assumed to work universally for modded engineering tasks.

### 5.3 Lab Results Are Evidence, Not Total Proof

Lab experiments increase confidence but do not prove all edge cases. A successful experiment over 200 ticks does not prove behavior under every server configuration, lag profile, chunk-loading state, upgrade item, or neighboring-mod interaction.

Generated schemes should therefore include a confidence report that distinguishes:

- mechanically checked behavior;
- known untested assumptions;
- unsupported or unknown mechanics;
- manual verification requirements.

## 6. User-Facing Product Shape

The end-user product should not expose "a model reading the entire modpack" as the main experience.

A more honest user-facing flow:

1. The user selects a patched Prism instance.
2. MPB detects whether a compatible knowledge pack exists for that modpack context.
3. If knowledge exists, the agent uses it during scheme design.
4. If knowledge is incomplete, MPB may run bounded local checks or explain that a mechanic is not verified.
5. The generated scheme includes an in-game ghost guide, materials, and a confidence/evidence report.

Example report categories:

- `Checked`: facts backed by runtime experiments or strong extracted data.
- `Likely`: documented or repeated inferred facts not yet runtime-verified.
- `Unknown`: areas where MPB lacks enough evidence.
- `Manual review`: places where the player should confirm behavior in the actual world/server.

The product should avoid implying that an unverified generated scheme is mechanically guaranteed.

## 7. Data And Evaluation

The first durable dataset should come from real pipeline usage:

- raw extracted mod metadata;
- worker prompts and outputs;
- corrections to worker outputs;
- lab experiment specs;
- lab observations;
- final accepted/rejected claims;
- generated scheme operations;
- user or reviewer corrections.

Evaluation should be task-specific:

- JSON schema validity for structured outputs;
- precision of extracted claims;
- false-positive rate for block role classification;
- usefulness of experiment proposals;
- agreement between summarized observations and raw lab logs;
- scheme-operation validity against MPB schema and block registry.

The project should not judge local worker quality by general chatbot quality.

## 8. Integration With Current MPB Pivot

This direction complements the current MPB patcher and Minecraft mod pivot.

Current MPB runtime priorities remain valid:

- Prism instance patching;
- client-only Minecraft mod;
- instance-local scheme files;
- MCP over Streamable HTTP;
- runtime block registry;
- in-game ghost guide;
- no server-side placement in a real user world.

The knowledge lab is a future layer on top of that runtime. It should not reintroduce the removed desktop 3D viewer as a core product. Minecraft remains the honest runtime and visual environment.

## 9. Open Questions

- What is the first target mechanic class for a knowledge-lab prototype?
- Which data format should represent knowledge packs: JSON, JSONL, SQLite, or a hybrid?
- Which claims require human review before they become `documented` or `runtime_verified`?
- How much of the lab runner can be generic before mod-specific APIs become necessary?
- Should knowledge packs live in the main repo, a separate repository, or downloadable package bundles?
- What is the smallest experiment that can disprove or validate the knowledge-lab approach?

## 10. Recommended Next Step

The next design task should define a narrow knowledge-lab prototype, not a full product rewrite.

Recommended prototype:

- one selected target modpack;
- one small worker model used without fine-tuning at first;
- one evidence-backed knowledge-pack format;
- deterministic extraction of registries, recipes, tags, lang strings, and configs;
- a minimal lab notebook;
- a small set of manually triggered experiments;
- a confidence report consumed by the existing scheme-generation workflow.

Fine-tuning should wait until the prototype has enough corrected examples to justify it.
