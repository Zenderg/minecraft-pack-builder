# Architecture

This document is the source of truth for repository-wide runtime boundaries, data ownership, and
code ownership. User-visible behavior belongs in the [product documents](../product/README.md),
component build procedures in module READMEs, and knowledge schemas and operations under
[`docs/knowledge`](../knowledge/README.md).

## System Boundary

MPB is split into a desktop patcher, an instance-local Minecraft runtime, and developer-only
knowledge-production tooling:

1. The Tauri patcher discovers PrismLauncher roots and instances, evaluates loader and Minecraft
   compatibility, and applies, repairs, or removes MPB-managed files.
2. The patcher installs an exact bundled Fabric, Forge, or NeoForge runtime artifact. It installs a
   curated knowledge bundle only when the instance matches that bundle's exact fingerprint.
3. The client-only Minecraft mod owns the live MCP server, MPB Manager, schemes, guide rendering,
   and import/export inside the real Minecraft client.
4. The developer-only knowledge pipeline extracts deterministic facts, records accepted runtime
   evidence, validates complete coverage, builds read-only bundles, and prepares release evidence.

PrismLauncher remains the owner of Minecraft instances. MPB manages only the files recorded in its
patch manifest and preserves user schemes unless deletion is explicitly requested.

## Code Ownership

- `src/` owns the React patcher UI; it does not host the Minecraft MCP runtime or render Minecraft.
- `src-tauri/` owns the desktop shell and the command adapter around the Rust patcher backend.
- `crates/mpb-assets/` owns Prism discovery, compatibility evaluation, patch operations, managed-file integrity, and embedded runtime and knowledge artifacts.
- `mods/mpb-minecraft-mod/` owns the common Java runtime and thin Fabric, Forge, and NeoForge entrypoints. Its [README](../../mods/mpb-minecraft-mod/README.md) defines runtime and build invariants.
- `crates/mpb-knowledge/` owns exact fingerprints, source validation, bundle generation and inspection, resumable release orchestration, evidence gates, and report generation.
- `mods/mpb-knowledge-lab/` owns the isolated developer-side experiment command contract and must never be installed by the patcher.
- `crates/mpb-core/`, `crates/mpb-storage/`, and `crates/mpb-export/` own reusable Rust scheme semantics, instance-file persistence, and schematic serialization. The Java client runtime remains the live in-game owner.

## Data Ownership

Runtime state is scoped to the selected Prism instance under `<instance>/mpb/`: configuration,
schemes, cache, patch manifest, and installed knowledge bundles. Reviewable knowledge-pack sources
live under `knowledge/packs/<pack-id>/source/`; generated run databases, reports, model artifacts,
clones, and raw lab evidence follow the retention rules in [`knowledge/README.md`](../../knowledge/README.md).

The patch manifest is the authority for managed-file ownership and integrity. A knowledge manifest
and bundle are trusted only for their exact fingerprint and schema metadata; mismatch deactivates
curated knowledge rather than falling back to inferred compatibility.
