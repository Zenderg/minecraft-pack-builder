# Exact Modpack Fingerprints

This document is the source of truth for curated-knowledge fingerprint inputs, normalization,
deliberate exclusions, and mismatch behavior. Per-pack values belong in each pack's
`source/manifest.json`; release commands belong in the [pipeline operator guide](autonomous-release-pipeline.md).

Curated knowledge uses exact fingerprint matching. There are no version ranges, no user override, and no "close enough" trusted mode.

`mpb-knowledge` fingerprints include:

- Modpack identity and version from Prism metadata when available.
- Minecraft version.
- Loader family and loader version.
- Full relevant file inputs from `mods`, `config`, `datapacks`, `kubejs`, `scripts`, `resourcepacks`, `texturepacks`, and `shaderpacks`.
- `instance.cfg` and `mmc-pack.json`.
- Knowledge schema version.
- Builder version.
- Lab tooling version.

Each file input is recorded as a canonical document entry with role, sorted normalized path, byte length, and checksum. `mmc-pack.json` is canonicalized to sorted component `uid` and `version` pairs before hashing, so Prism cache-only fields such as `cachedName`, `cachedVersion`, and `cachedVolatile` do not change the fingerprint. Unsupported or partially collected deterministic inputs must produce blocking extraction diagnostics; they are not soft warnings.

The patch-target fingerprint deliberately excludes the MPB-managed
`mods/mpb-minecraft-mod.jar`. Installing or repairing MPB must not invalidate the exact knowledge
match it just applied. Runtime bundle schema metadata remains separate from the builder, lab, and
fingerprint-schema salt values used to compute a target fingerprint.

The patcher/runtime integration must treat fingerprint mismatch as unsupported curated knowledge. The base MPB mod may still be installable, but the knowledge bundle must not be installed or marked active for mismatched instances.
