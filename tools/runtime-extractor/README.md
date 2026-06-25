# MPB Runtime Extractor

Small loader-specific Java mods embedded into `mpb-assets` as hex-encoded jars.

The extractor runs inside an app-owned Prism/Minecraft runtime and writes the JSON file named by:

```text
-Dmpb.runtimeOutput=/path/to/report.json
```

It is intentionally reflection-heavy so the shared runtime dumper can survive Minecraft and loader API drift. Loader entrypoint classes are compiled against local stubs and the stubs are not packaged into the final jars.

Rebuild all embedded jars from the repository root:

```bash
bash tools/runtime-extractor/build.sh
```

The report currently includes:

- authoritative runtime item max stack sizes;
- `approximation` runtime render asset records for block states when the server runtime can derive bounded, non-full-cube static shapes.

Shape-derived render assets are a fallback for cases where static JSON models do not provide a useful render payload. The server-side runtime cannot access every client-only renderer. Blocks whose visual fidelity depends on client-baked models, block entity renderers, animation, custom client renderers, or dynamic world context still need a future client-baked extractor path.
