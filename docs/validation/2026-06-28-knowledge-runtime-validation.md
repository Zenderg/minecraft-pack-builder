# Knowledge Bundle Runtime Validation

Date: 2026-06-28

Scope: first-party fixture knowledge bundle installation through the patcher, Java runtime bundle loading, and read-only MCP knowledge tools.

Automated checks completed:

- `cargo test -p mpb-assets patcher` completed, but the `patcher` filter selected zero tests.
- `cargo test -p mpb-assets` passed and ran the patcher integration tests that cover matching knowledge install, mismatch fallback, repair, update, conflict, and unpatch behavior.
- `cargo test --workspace` passed.
- `pnpm test src/patcher/patcherState.test.ts` passed.
- `pnpm test` passed.
- `pnpm build` passed.
- Java common runtime plus tests compiled with `javac --release 17`.
- Java runtime test harness passed outside the sandbox because MCP compatibility tests bind loopback sockets.

Full Minecraft mod production build status:

- `gradle` was not available in the login shell `PATH`.
- A usable Gradle distribution was available from the local Gradle wrapper cache.
- `/usr/libexec/java_home -V` reported only JDK 21.0.1, but the production mod build completed successfully on that JDK.
- `mods/mpb-minecraft-mod/build.sh` passed when invoked with `MPB_GRADLE` pointing to the cached Gradle 8.14.3 binary and refreshed `crates/mpb-assets/src/mpb_mod_*_jar.hex`.

```bash
MPB_GRADLE=<local-gradle-8.14.3-bin>/gradle \
mods/mpb-minecraft-mod/build.sh
```
