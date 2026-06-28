# Phase 2 Validation: Onboarding, Settings, And Secure CurseForge Key Storage

## Openable Artifacts

- Local web preview: run `pnpm dev`, then open `http://127.0.0.1:5173/`.
- Desktop preview: run `pnpm tauri dev` and use the same flow in the Tauri window.
- Onboarding screenshot: `docs/validation/media/phase-2-onboarding.png`.

## What Changed

- First launch now opens onboarding with language selection, AI integration instructions, and CurseForge API key setup.
- Onboarding keeps `Skip` on the left and step navigation on the right to reduce accidental exits.
- Onboarding supports going back to previous steps.
- AI integration onboarding includes an English-only prompt the user can paste into their external agent.
- Settings includes an action to show onboarding again after it has been skipped or completed.
- Skipping onboarding enters the workspace without requiring a key.
- Settings now opens as a modal over the workspace and has screens for AI integration, CurseForge API key management, language, and data folders.
- CurseForge API keys are sent to the Rust backend and saved through OS secure credential storage where available.
- The key form uses one `Check key` action. During phase 2 this checks local input and saves the key through secure storage; it does not call CurseForge until the import API work in phase 5.
- The onboarding `Finish` action is enabled only when a CurseForge key already exists.
- Saved keys are represented only as status. The key value is not shown again after saving.
- Starting Add modpack without a saved key opens the Settings modal on the CurseForge API key screen.

## Manual Validation Checklist

- [x] Onboarding explains why the CurseForge API key is needed.
- [x] Skip stays visually separated on the left while Back/Next/Finish stay on the right.
- [x] Each non-first onboarding step can return to the previous step.
- [x] AI integration includes a pasteable English prompt for the user's external agent and the prompt text does not change with UI language.
- [x] Onboarding can be reopened from Settings after being skipped.
- [x] Onboarding can be skipped and the empty workspace remains reachable.
- [x] Language can be switched during onboarding and later in Settings.
- [x] Settings opens as a modal and includes AI integration, CurseForge API key, language, and data folder screens.
- [x] Add modpack redirects to the CurseForge key settings modal when no key is saved.
- [x] CurseForge key setup has one `Check key` action; local/fake phase-2 validation saves the key and says online validation comes later.
- [x] The CurseForge onboarding `Finish` button is disabled until a saved key exists, and existing saved keys are clearly reported.
- [x] The key input is password-style and is cleared after save attempts.
- [x] The UI has clear missing, saved, replaced, and secure-storage-unavailable states.
- [x] Data folder access remains available from Settings.

## Engineering Validation

- `pnpm test`
- `pnpm build`
- `cargo test --workspace`

## Notes

- Browser screenshot capture used system headless Chrome because the in-app browser tab crashed on localhost in this environment and the Playwright bundled Chromium binary is not installed.
- Secure storage unit tests validate the app contract and no-plaintext fallback behavior. Manual OS Keychain/Credential Manager/Secret Service save verification should be run in the desktop app on each target platform.
