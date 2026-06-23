export type AppScreen = "onboarding" | "workspace" | "settings" | "importWizard";
export type OnboardingStep = "language" | "ai" | "curseforge";
export type SettingsSection = "ai" | "curseforge" | "language" | "data";
export type CurseForgeKeyState = "missing" | "saved" | "unavailable";
export type KeyNotice = "idle" | "missing" | "saved" | "replaced" | "unavailable";

export type AppFlowState = {
  screen: AppScreen;
  onboardingStep: OnboardingStep;
  curseForgeKey: CurseForgeKeyState;
  settingsSection: SettingsSection;
  keyNotice: KeyNotice;
  settingsModalOpen: boolean;
};

export type AppFlowAction =
  | { type: "setScreen"; screen: AppScreen }
  | { type: "setOnboardingStep"; step: OnboardingStep }
  | { type: "previousOnboardingStep" }
  | { type: "openSettings"; section: SettingsSection }
  | { type: "closeSettings" }
  | { type: "restartOnboarding" }
  | { type: "skipOnboarding" }
  | { type: "finishOnboarding" }
  | { type: "setCurseForgeKeyState"; state: CurseForgeKeyState }
  | { type: "keySaved" }
  | { type: "keyUnavailable" }
  | { type: "startAddModpack" };

export function createInitialAppFlow(options: { onboardingComplete: boolean }): AppFlowState {
  return {
    screen: options.onboardingComplete ? "workspace" : "onboarding",
    onboardingStep: "language",
    curseForgeKey: "missing",
    settingsSection: "curseforge",
    keyNotice: "idle",
    settingsModalOpen: false,
  };
}

export function getAddModpackTarget(curseForgeKey: CurseForgeKeyState): {
  screen: AppScreen;
  settingsSection: SettingsSection;
  settingsModalOpen: boolean;
} {
  if (curseForgeKey !== "saved") {
    return { screen: "workspace", settingsSection: "curseforge", settingsModalOpen: true };
  }

  return { screen: "importWizard", settingsSection: "curseforge", settingsModalOpen: false };
}

export function getPreviousOnboardingStep(step: OnboardingStep): OnboardingStep {
  if (step === "curseforge") {
    return "ai";
  }
  if (step === "ai") {
    return "language";
  }
  return "language";
}

export type CurseForgeKeyCheckResult = "empty" | "formatReady";

export function getCurseForgeKeyCheckResult(apiKey: string): CurseForgeKeyCheckResult {
  return apiKey.trim().length === 0 ? "empty" : "formatReady";
}

export function canFinishOnboardingWithKey(curseForgeKey: CurseForgeKeyState): boolean {
  return curseForgeKey === "saved";
}

export function onboardingReducer(state: AppFlowState, action: AppFlowAction): AppFlowState {
  switch (action.type) {
    case "setScreen":
      return { ...state, screen: action.screen };
    case "setOnboardingStep":
      return { ...state, onboardingStep: action.step };
    case "previousOnboardingStep":
      return { ...state, onboardingStep: getPreviousOnboardingStep(state.onboardingStep) };
    case "openSettings":
      return {
        ...state,
        screen: state.screen === "onboarding" ? "workspace" : state.screen,
        settingsModalOpen: true,
        settingsSection: action.section,
      };
    case "closeSettings":
      return { ...state, settingsModalOpen: false };
    case "restartOnboarding":
      return {
        ...state,
        screen: "onboarding",
        onboardingStep: "language",
        settingsModalOpen: false,
      };
    case "skipOnboarding":
    case "finishOnboarding":
      return { ...state, screen: "workspace", settingsModalOpen: false };
    case "setCurseForgeKeyState":
      return {
        ...state,
        curseForgeKey: action.state,
        keyNotice: noticeForKeyState(action.state, state.keyNotice),
      };
    case "keySaved":
      return {
        ...state,
        curseForgeKey: "saved",
        keyNotice: state.curseForgeKey === "saved" ? "replaced" : "saved",
      };
    case "keyUnavailable":
      return { ...state, curseForgeKey: "unavailable", keyNotice: "unavailable" };
    case "startAddModpack": {
      const target = getAddModpackTarget(state.curseForgeKey);
      return {
        ...state,
        screen: target.screen,
        settingsSection: target.settingsSection,
        settingsModalOpen: target.settingsModalOpen,
        keyNotice: target.settingsModalOpen ? "missing" : state.keyNotice,
      };
    }
    default:
      return state;
  }
}

function noticeForKeyState(state: CurseForgeKeyState, current: KeyNotice): KeyNotice {
  if (current === "saved" || current === "replaced") {
    return current;
  }
  if (state === "missing") {
    return "missing";
  }
  if (state === "unavailable") {
    return "unavailable";
  }
  return "idle";
}
