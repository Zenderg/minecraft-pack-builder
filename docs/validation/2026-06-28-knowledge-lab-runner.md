# Knowledge Lab Runner Validation Notes

Date: 2026-06-28

The developer-only knowledge lab lives under `mods/mpb-knowledge-lab/` and is intentionally separate
from patcher-managed MPB runtime artifacts. `apply_mpb_patch` must not install it.

Canonical validation target:

- local PrismLauncher client instance;
- exact target modpack fingerprint;
- disposable world or isolated lab area;
- JDK 17 and Gradle available locally.

Dedicated-server/headless operation is outside the current production contract. Raw logs, snapshots,
local notebooks, and worker traces are developer artifacts and stay under ignored paths such as
`knowledge/lab-artifacts/`.

Local scaffold build command:

```bash
gradle -p mods/mpb-knowledge-lab --no-daemon build
```

Current local validation result:

- JDK available: `java version "21.0.1"`;
- `gradle` was not present in `PATH`;
- `MPB_GRADLE` was not set;
- Java command-contract sources compiled with
  `javac --release 17 -encoding UTF-8 -d /private/tmp/mpb-knowledge-lab-classes mods/mpb-knowledge-lab/common/src/main/java/com/mpb/lab/*.java`.
