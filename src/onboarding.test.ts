import { describe, expect, it } from "vitest";

import {
  canFinishOnboardingWithKey,
  createInitialAppFlow,
  getCurseForgeKeyInputCheckResult,
  getAddModpackTarget,
  getPreviousOnboardingStep,
  getCurseForgeKeyButtonState,
  isCurseForgeKeyCheckBusy,
  onboardingReducer,
  shouldShowExistingKeyNotice,
  type CurseForgeKeyState,
} from "./onboarding";

describe("phase 2 onboarding flow", () => {
  it("starts in onboarding until the first launch flow is completed or skipped", () => {
    expect(createInitialAppFlow({ onboardingComplete: false }).screen).toBe("onboarding");
    expect(createInitialAppFlow({ onboardingComplete: true }).screen).toBe("workspace");
  });

  it("can restart onboarding from settings after it was completed", () => {
    const state = onboardingReducer(
      {
        screen: "workspace",
        onboardingStep: "curseforge",
        curseForgeKey: "saved",
        settingsSection: "ai",
        keyNotice: "saved",
        settingsModalOpen: true,
        importModalOpen: false,
      },
      { type: "restartOnboarding" },
    );

    expect(state.screen).toBe("onboarding");
    expect(state.onboardingStep).toBe("language");
    expect(state.settingsModalOpen).toBe(false);
  });

  it("allows skipping onboarding into the workspace without a CurseForge key", () => {
    const state = onboardingReducer(
      {
        screen: "onboarding",
        onboardingStep: "curseforge",
        curseForgeKey: "missing",
        settingsSection: "curseforge",
        keyNotice: "idle",
        settingsModalOpen: false,
        importModalOpen: false,
      },
      { type: "skipOnboarding" },
    );

    expect(state.screen).toBe("workspace");
    expect(state.curseForgeKey).toBe("missing");
  });

  it("allows returning to previous onboarding steps without leaving onboarding", () => {
    expect(getPreviousOnboardingStep("ai")).toBe("language");
    expect(getPreviousOnboardingStep("curseforge")).toBe("ai");
    expect(getPreviousOnboardingStep("language")).toBe("language");

    const state = onboardingReducer(
      {
        screen: "onboarding",
        onboardingStep: "curseforge",
        curseForgeKey: "missing",
        settingsSection: "curseforge",
        keyNotice: "idle",
        settingsModalOpen: false,
        importModalOpen: false,
      },
      { type: "previousOnboardingStep" },
    );

    expect(state.screen).toBe("onboarding");
    expect(state.onboardingStep).toBe("ai");
  });

  it("routes add-modpack to settings when the CurseForge key is missing or unavailable", () => {
    const missing: CurseForgeKeyState = "missing";
    const unavailable: CurseForgeKeyState = "unavailable";
    const saved: CurseForgeKeyState = "saved";

    expect(getAddModpackTarget(missing)).toEqual({
      screen: "workspace",
      settingsSection: "curseforge",
      settingsModalOpen: true,
      importModalOpen: false,
    });
    expect(getAddModpackTarget(unavailable)).toEqual({
      screen: "workspace",
      settingsSection: "curseforge",
      settingsModalOpen: true,
      importModalOpen: false,
    });
    expect(getAddModpackTarget(saved)).toEqual({
      screen: "workspace",
      settingsSection: "curseforge",
      settingsModalOpen: false,
      importModalOpen: true,
    });
  });

  it("distinguishes first save from replacement without exposing the key value", () => {
    const saved = onboardingReducer(
      {
        screen: "settings",
        onboardingStep: "curseforge",
        curseForgeKey: "missing",
        settingsSection: "curseforge",
        keyNotice: "idle",
        settingsModalOpen: true,
        importModalOpen: false,
      },
      { type: "keySaved" },
    );

    expect(saved.curseForgeKey).toBe("saved");
    expect(saved.keyNotice).toBe("saved");
    expect(JSON.stringify(saved)).not.toContain("secret-token");

    const replaced = onboardingReducer(saved, { type: "keySaved" });

    expect(replaced.curseForgeKey).toBe("saved");
    expect(replaced.keyNotice).toBe("replaced");
  });

  it("prechecks CurseForge key input before the online phase 5 validation", () => {
    expect(getCurseForgeKeyInputCheckResult("")).toBe("empty");
    expect(getCurseForgeKeyInputCheckResult("  abc123  ")).toBe("ready");
  });

  it("treats only the active key check as a busy state", () => {
    expect(isCurseForgeKeyCheckBusy("checking")).toBe(true);
    expect(isCurseForgeKeyCheckBusy("idle")).toBe(false);
    expect(isCurseForgeKeyCheckBusy("valid")).toBe(false);
    expect(isCurseForgeKeyCheckBusy("invalid")).toBe(false);
  });

  it("keeps the key check button visibly loading while disabled", () => {
    expect(getCurseForgeKeyButtonState("checking", true)).toEqual({
      disabled: true,
      loading: true,
    });
    expect(getCurseForgeKeyButtonState("valid", false)).toEqual({
      disabled: false,
      loading: false,
    });
  });

  it("shows the existing-key notice only before replacing or checking a key", () => {
    expect(shouldShowExistingKeyNotice("saved", "", "idle")).toBe(true);
    expect(shouldShowExistingKeyNotice("saved", "new-key", "idle")).toBe(false);
    expect(shouldShowExistingKeyNotice("saved", "", "valid")).toBe(false);
    expect(shouldShowExistingKeyNotice("missing", "", "idle")).toBe(false);
  });

  it("allows finishing the CurseForge onboarding step only after a key exists", () => {
    expect(canFinishOnboardingWithKey("missing")).toBe(false);
    expect(canFinishOnboardingWithKey("unavailable")).toBe(false);
    expect(canFinishOnboardingWithKey("saved")).toBe(true);
  });
});
