# Project Documentation

This index is the source of truth for documentation ownership and navigation. It explains where
durable product, architecture, subsystem, and validation information belongs; end-user setup stays
in the [project README](../README.md), and repository-wide agent rules stay in [`AGENTS.md`](../AGENTS.md).

## Product

- [Product documentation](product/README.md) owns user-visible behavior and support boundaries.
- [Patcher and Minecraft mod contract](product/patcher-and-minecraft-mod.md) defines the core product.
- [First-party knowledge-pack contract](product/first-party-knowledge-packs.md) defines curated support and trust guarantees.

## Architecture And Subsystems

- [Architecture overview](architecture/README.md) maps runtime boundaries and code ownership.
- [Curated knowledge documentation](knowledge/README.md) owns knowledge formats, fingerprints, worker policy, release-pipeline requirements, and operator procedures.
- [Minecraft runtime module](../mods/mpb-minecraft-mod/README.md) owns the client mod contract and build.
- [Knowledge Lab module](../mods/mpb-knowledge-lab/README.md) owns the developer-only lab boundary and command contract.
- [Knowledge artifact tree](../knowledge/README.md) owns tracking and retention rules for pack sources and local generated artifacts.

## Validation

[Validation documentation](validation/README.md) contains reusable release checklists and concise
current validation summaries. Raw command output, generated reports, and implementation-session
logs do not belong in committed documentation.
