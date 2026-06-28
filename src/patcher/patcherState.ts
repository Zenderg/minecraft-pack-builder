import { textForLanguage, type Language } from "../i18n";

export type PatchStatus =
  | "notPatched"
  | "patched"
  | "needsUpdate"
  | "needsRepair"
  | "conflict"
  | "unsupported"
  | "instanceRunning";

export type PatchAction = "apply" | "update" | "repair";
export type KnowledgeStatus = "available" | "installed" | "unavailable";

export type PatcherInstance = {
  instanceId: string;
  displayName: string;
  instancePath: string;
  minecraftDir: string;
  minecraftVersion: string | null;
  loader: string | null;
  loaderVersion: string | null;
  patchStatus: PatchStatus;
  patchReason: string | null;
  knowledgeStatus: KnowledgeStatus;
  knowledgePackId: string | null;
  knowledgeReason: string | null;
};

export type PatcherOperation = {
  busy: boolean;
  instancePath: string;
  action: PatchAction | "remove";
};

export type PatcherState = {
  rootPath: string | null;
  instances: PatcherInstance[];
  selectedInstancePath: string | null;
  loading: boolean;
  message: string;
  operation: PatcherOperation | null;
};

export type PatcherAction =
  | { type: "loading"; rootPath?: string | null }
  | { type: "loaded"; rootPath: string | null; instances: PatcherInstance[]; message?: string }
  | { type: "failed"; message: string }
  | { type: "selectInstance"; instancePath: string }
  | { type: "operationStarted"; instancePath: string; action: PatchAction | "remove" }
  | { type: "operationCompleted"; message: string; instances: PatcherInstance[] };

export function createInitialPatcherState(): PatcherState {
  return {
    rootPath: null,
    instances: [],
    selectedInstancePath: null,
    loading: true,
    message: "",
    operation: null,
  };
}

export function patcherReducer(state: PatcherState, action: PatcherAction): PatcherState {
  switch (action.type) {
    case "loading":
      return {
        ...state,
        rootPath: action.rootPath ?? state.rootPath,
        loading: true,
        message: "",
      };
    case "loaded": {
      const selectedStillExists = action.instances.some(
        (instance) => instance.instancePath === state.selectedInstancePath,
      );
      return {
        ...state,
        rootPath: action.rootPath,
        instances: action.instances,
        selectedInstancePath: selectedStillExists
          ? state.selectedInstancePath
          : action.instances[0]?.instancePath ?? null,
        loading: false,
        message: action.message ?? "",
      };
    }
    case "failed":
      return {
        ...state,
        loading: false,
        message: action.message,
        operation: null,
      };
    case "selectInstance":
      return {
        ...state,
        selectedInstancePath: action.instancePath,
      };
    case "operationStarted":
      return {
        ...state,
        operation: {
          busy: true,
          instancePath: action.instancePath,
          action: action.action,
        },
        message: "",
      };
    case "operationCompleted":
      return {
        ...state,
        instances: action.instances,
        operation: null,
        message: action.message,
      };
  }
}

export function getPatchStatusAction(
  status: PatchStatus,
): { action: PatchAction; labelKey: PatcherLabelKey } | null {
  switch (status) {
    case "notPatched":
      return { action: "apply", labelKey: "patcher.apply" };
    case "needsUpdate":
      return { action: "update", labelKey: "patcher.update" };
    case "needsRepair":
      return { action: "repair", labelKey: "patcher.repair" };
    default:
      return null;
  }
}

export type PatcherLabelKey =
  | "patcher.apply"
  | "patcher.update"
  | "patcher.repair"
  | "patcher.remove";

export function getNextStepText(
  language: Language,
  knowledgeStatus: KnowledgeStatus = "unavailable",
): string {
  const hasKnowledge = knowledgeStatus === "available" || knowledgeStatus === "installed";
  const knowledgeText = hasKnowledge
    ? textForLanguage(language, {
        en: "curated knowledge is available for this instance",
        ru: "кураторская база знаний доступна для этого инстанса",
      })
    : textForLanguage(language, {
        en: "curated modpack knowledge is unsupported for this instance",
        ru: "кураторская база знаний не поддерживается для этого инстанса",
      });
  return textForLanguage(language, {
    en: `Done. Next, start the instance in PrismLauncher, open MPB Manager in Minecraft with /mpb or assigned keybindings, then copy the agent prompt from MPB Manager; ${knowledgeText}.`,
    ru: `Готово. Теперь запусти инстанс в PrismLauncher, открой MPB Manager в Minecraft через /mpb или назначенные клавиши, затем скопируй prompt для агента из MPB Manager; ${knowledgeText}.`,
  });
}
