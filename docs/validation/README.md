# Validation Documentation

This directory is the source of truth for human-reviewed release checklists and concise current
validation summaries. Product and architecture contracts belong in their owning documents; raw
logs, generated reports, and superseded run evidence belong under `knowledge/runs/`, CI artifacts,
or other ignored artifact paths.

- [Patcher and runtime release checklist](patcher-runtime-release-checklist.md) covers the desktop patcher and embedded Fabric, Forge, and NeoForge runtimes.
- [First-party knowledge release checklist](first-party-knowledge-release-checklist.md) covers every curated knowledge pack and release package that contains one.
- [All of Create - Aeronautics summary](all-of-create-aeronautics.md) records the latest accepted validation state for the bundled first-party pack.

Checklists define repeatable gates; summaries record only the latest accepted outcome and point to
exact generated evidence. Do not append command transcripts, red/green implementation history,
working-tree recovery notes, or superseded runs here.
