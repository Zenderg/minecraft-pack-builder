import { describe, expect, it } from "vitest";

import {
  canFinishOnboardingWithPrism,
  createInitialAppFlow,
  getPreviousOnboardingStep,
  onboardingReducer,
} from "./onboarding";

describe("Prism onboarding flow", () => {
  it("starts in onboarding until the first launch flow is completed or skipped", () => {
    expect(createInitialAppFlow({ onboardingComplete: false }).screen).toBe("onboarding");
    expect(createInitialAppFlow({ onboardingComplete: true }).screen).toBe("workspace");
  });

  it("can restart onboarding from settings after it was completed", () => {
    const state = onboardingReducer(
      {
        screen: "workspace",
        onboardingStep: "prism",
        prismRoot: "valid",
        settingsSection: "ai",
        settingsModalOpen: true,
      },
      { type: "restartOnboarding" },
    );

    expect(state.screen).toBe("onboarding");
    expect(state.onboardingStep).toBe("language");
    expect(state.settingsModalOpen).toBe(false);
  });

  it("allows skipping onboarding into the workspace without a Prism root", () => {
    const state = onboardingReducer(
      {
        screen: "onboarding",
        onboardingStep: "prism",
        prismRoot: "unknown",
        settingsSection: "prism",
        settingsModalOpen: false,
      },
      { type: "skipOnboarding" },
    );

    expect(state.screen).toBe("workspace");
    expect(state.prismRoot).toBe("unknown");
  });

  it("allows returning to previous onboarding steps without leaving onboarding", () => {
    expect(getPreviousOnboardingStep("ai")).toBe("language");
    expect(getPreviousOnboardingStep("prism")).toBe("ai");
    expect(getPreviousOnboardingStep("language")).toBe("language");

    const state = onboardingReducer(
      {
        screen: "onboarding",
        onboardingStep: "prism",
        prismRoot: "unknown",
        settingsSection: "prism",
        settingsModalOpen: false,
      },
      { type: "previousOnboardingStep" },
    );

    expect(state.screen).toBe("onboarding");
    expect(state.onboardingStep).toBe("ai");
  });

  it("tracks Prism root validity without storing any secret values", () => {
    const state = onboardingReducer(
      {
        screen: "settings",
        onboardingStep: "prism",
        prismRoot: "unknown",
        settingsSection: "prism",
        settingsModalOpen: true,
      },
      { type: "setPrismRootState", state: "valid" },
    );

    expect(state.prismRoot).toBe("valid");
    expect(JSON.stringify(state)).not.toContain("api-key");
  });

  it("allows finishing the Prism onboarding step only after a valid root exists", () => {
    expect(canFinishOnboardingWithPrism("unknown")).toBe(false);
    expect(canFinishOnboardingWithPrism("invalid")).toBe(false);
    expect(canFinishOnboardingWithPrism("valid")).toBe(true);
  });
});
