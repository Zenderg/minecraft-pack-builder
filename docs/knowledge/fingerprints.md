# Exact Modpack Fingerprints

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

Each file input is recorded as a canonical document entry with role, sorted normalized path, byte length, and checksum. Unsupported or partially collected deterministic inputs must produce blocking extraction diagnostics; they are not soft warnings.

The patcher/runtime integration must treat fingerprint mismatch as unsupported curated knowledge. The base MPB mod may still be installable, but the knowledge bundle must not be installed or marked active for mismatched instances.
