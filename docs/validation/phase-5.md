# Phase 5 Validation

## Openable Artifacts

- Browser/Vite preview: run `pnpm dev`, open `http://127.0.0.1:5173/`, click `Add modpack`.
- Rendered wizard screenshot: `docs/validation/media/phase-5-import-wizard.png`.
- Desktop app import path: in the Tauri desktop app, save and validate a CurseForge API key, open `Add modpack`, type a modpack name such as `All of Create - Aeronautics` or `AOC`, select the intended modpack result, choose a release, then download it.
- Downloaded archive location after desktop import: the app data folder under `modpacks/<slug>-<fileId>/archives/<fileName>`.

## User Checklist

- [x] Add Modpack opens as a modal instead of replacing the scheme viewer workspace.
- [x] The wizard searches CurseForge modpacks by debounced name input and shows selectable project results.
- [x] Project results include the CurseForge project logo when available.
- [x] Non-modpack CurseForge URLs are rejected by backend URL parsing.
- [x] Release discovery is performed through Rust backend code.
- [x] Releases appear as one list with Minecraft version and loader filters.
- [x] The latest release is selected by default.
- [x] If filters hide the previous selection, the selection moves to the latest visible release.
- [x] Download progress is surfaced to the UI.
- [x] Cancellation is represented as a failed user-visible state with no imported modpack id.
- [x] Successful import records a fixed local modpack snapshot in the sidebar/library.
- [x] CurseForge API key checks show visible loading and validate against CurseForge before saving.
- [x] The same backend import command can be called by future AI tools without direct filesystem access.

## Engineering Checklist

- [x] Rust URL parsing tests cover valid modpack pages and non-modpack rejection.
- [x] Mocked CurseForge gateway tests cover release discovery, latest release selection, filters, download progress, and cancellation cleanup.
- [x] Frontend state tests cover default selection, filters, cancellation state, and selection changes after filtering.
- [x] CurseForge project search uses the documented search endpoint with PrismLauncher-compatible default parameters: `index=0`, `pageSize=25`, `sortField=1`, `sortOrder=desc`.
- [x] Secure credential storage has macOS Keychain, Linux Secret Service, and Windows Credential Manager implementations.
- [x] Browser rendered QA covers Add Modpack, release discovery, filtering, import result, sidebar update, and console health.

## Verification Commands

- `cargo test --workspace`
- `pnpm test`
- `pnpm build`

## Browser QA Evidence

- URL: `http://127.0.0.1:5173/`
- Viewport: default in-app browser viewport, 1280 x 720.
- Flow tested: workspace -> Add modpack -> search by modpack name -> select a project -> filter releases to Minecraft `1.20.1` and loader `Forge` -> Download selected.
- Observed result: release list narrowed to `AOC 1.1.0`; import status showed `/browser-demo/modpacks/aoc-200/aoc-1.1.0.zip`; sidebar showed `AOC - AOC 1.1.0`.
- Console result: no new `warn` or `error` logs during the final verified interaction.

## Notes

- The automated browser preview uses a local browser fixture for CurseForge releases and archive paths because Vite browser mode cannot access OS secure credential storage.
- The real CurseForge API and archive download path are implemented in Rust through `mpb-assets` and Tauri commands. Manual desktop validation requires a saved CurseForge API key and network access.
- Secure storage read/write is implemented through macOS Keychain, Linux Secret Service, and Windows Credential Manager. A packaged-app smoke test on an actual Windows environment remains part of the cross-platform packaging phase.
