# Phase 12 Validation: Cross-Platform Packaging And Update Flow

## Openable Artifacts

- Tauri bundler config: `src-tauri/tauri.conf.json`
- Release workflow: `.github/workflows/release.yml`
- Updater metadata template: `docs/validation/phase-12-latest.template.json`
- Update settings screen: Settings -> Updates in the desktop app
- macOS bundle command: `pnpm tauri build --target universal-apple-darwin`
- Local debug macOS app from validation: `target/debug/bundle/macos/Minecraft Pack Builder.app`
- Local debug DMG from validation: `target/debug/bundle/dmg/Minecraft Pack Builder_0.1.0_aarch64.dmg`
- Local debug updater archive/signature from validation:
  `target/debug/bundle/macos/Minecraft Pack Builder.app.tar.gz` and
  `target/debug/bundle/macos/Minecraft Pack Builder.app.tar.gz.sig`

## What Changed

- Tauri bundler now targets macOS app/dmg, Windows MSI/NSIS, and Linux AppImage/deb/rpm artifacts explicitly.
- Tauri updater artifacts are enabled and configured to read static `latest.json` metadata from GitHub Releases.
- The desktop backend exposes a non-throwing `check_for_updates` command through the Tauri updater plugin.
- Settings now includes an Updates screen with an automatic-check preference and manual `Check for updates` action.
- GitHub Actions has a release matrix for macOS, Windows, and Linux bundle builds.
- Public distribution requirements for updater signing, Apple signing, and notarization are documented in this validation note.

## User Validation Checklist

- [ ] Open Settings -> Updates and confirm the automatic update checks toggle is visible.
- [ ] Click `Check for updates` and confirm the result reports either the current version, an available version, or a non-blocking network/configuration error.
- [ ] Confirm the app remains usable after an update check failure.
- [ ] Run a macOS package build and open the generated `.app` outside the dev environment.
- [ ] Confirm release artifacts are grouped by platform in the GitHub Release draft.

## Engineering Validation

- `pnpm test src/App.viewer.test.tsx src/phase12Packaging.test.ts`
- `pnpm build`
- `cargo test --workspace`
- `pnpm tauri build --target universal-apple-darwin`
- Local validation also ran `TAURI_SIGNING_PRIVATE_KEY=... TAURI_SIGNING_PRIVATE_KEY_PASSWORD="" pnpm tauri build --debug` and produced the macOS app, DMG, updater archive, and updater signature listed above.

## Free Unsigned Distribution Notes

- The updater `pubkey` in `src-tauri/tauri.conf.json` must match the private key stored in CI as `TAURI_SIGNING_PRIVATE_KEY`.
- Release `latest.json` must be generated from the signed updater artifacts, not copied verbatim from the template.
- This project currently uses the free unsigned macOS distribution path. Do not require Apple Developer Program credentials for the release workflow.
- macOS users may see Gatekeeper warnings for unsigned/unnotarized builds and can open the app with right click -> Open or System Settings -> Privacy & Security -> Open Anyway.
- Windows builds are currently unsigned. Users may see SmartScreen warnings.
- Linux AppImage/deb/rpm artifacts should be smoke-launched on a clean Linux desktop.
- Do not introduce paid signing/notarization requirements unless the user explicitly changes this distribution policy.
