export type AppScreen = "onboarding" | "workspace" | "settings";
export type OnboardingStep = "language" | "ai" | "prism";
export type SettingsSection = "ai" | "prism" | "language" | "data" | "updates";
export type PrismRootState = "unknown" | "valid" | "invalid";

export type AppFlowState = {
  screen: AppScreen;
  onboardingStep: OnboardingStep;
  prismRoot: PrismRootState;
  settingsSection: SettingsSection;
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
  | { type: "setPrismRootState"; state: PrismRootState };

export function createInitialAppFlow(options: { onboardingComplete: boolean }): AppFlowState {
  return {
    screen: options.onboardingComplete ? "workspace" : "onboarding",
    onboardingStep: "language",
    prismRoot: "unknown",
    settingsSection: "prism",
    settingsModalOpen: false,
  };
}

export function getPreviousOnboardingStep(step: OnboardingStep): OnboardingStep {
  if (step === "prism") {
    return "ai";
  }
  if (step === "ai") {
    return "language";
  }
  return "language";
}

export function canFinishOnboardingWithPrism(prismRoot: PrismRootState): boolean {
  return prismRoot === "valid";
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
    case "setPrismRootState":
      return { ...state, prismRoot: action.state };
    default:
      return state;
  }
}
