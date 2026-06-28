# MPB Knowledge Lab

This tree is a developer-only lab runner scaffold for first-party modded knowledge production.
It is intentionally separate from `mods/mpb-minecraft-mod`, is not referenced by patcher artifacts,
and must not be installed by `apply_mpb_patch`.

The canonical target is a local PrismLauncher client instance with the exact target modpack and a
disposable world or isolated lab area. Dedicated-server and headless operation are outside the
production contract for first-party knowledge packs.

## Command Contract

The lab exposes batch-first operations for release readiness:

- prepare or reset an isolated lab area;
- place structures;
- set block states;
- use items on blocks;
- run bounded ticks;
- inspect block entities, inventories, fluids, energy, kinetic, vessel, or other observable state
  where loader/mod APIs make that possible;
- compare before and after snapshots;
- record compact structured observations.

The batch suite must fail on uncovered entities, failed experiments, unresolved mechanics, stale
fingerprints, placeholder artifacts, or invalid runtime bundles. Raw logs, snapshots, notebooks, and
worker traces belong under `knowledge/lab-artifacts/` and are ignored by git.

## Local Build

When a local Gradle and JDK 17 toolchain are available:

```bash
gradle -p mods/mpb-knowledge-lab --no-daemon build
```

This scaffold compiles the lab command contract as plain Java. Loader-specific Minecraft adapters
should be added behind this contract when a target modpack experiment requires runtime APIs.
