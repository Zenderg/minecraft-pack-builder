import {
  AlertTriangle,
  ArrowLeft,
  ArrowRight,
  Bot,
  Box,
  ChevronDown,
  CheckCircle2,
  ClipboardList,
  Database,
  EyeOff,
  FolderOpen,
  Info,
  KeyRound,
  Layers3,
  Languages,
  Loader2,
  MoreHorizontal,
  PackagePlus,
  Pencil,
  PlugZap,
  Plus,
  RefreshCcw,
  Settings,
  ShieldCheck,
  Trash2,
  X,
} from "lucide-react";
import { useCallback, useEffect, useLayoutEffect, useReducer, useRef, useState } from "react";

import { getInitialLanguage, languages, type Language, translate } from "./i18n";
import { ImportWizardWorkspace } from "./importWizard";
import {
  clampSidebarWidth,
  compactLibraryNodeGap,
  createEmptyLibraryDraft,
  getLoaderIconKind,
  getActiveLibrarySelection,
  getInitialExpandedModpackIds,
  getLibraryDialogContent,
  getModpackMenuPlacement,
  getNextOpenModpackMenuId,
  getNextSelectionAfterSchemeDelete,
  sidebarWidthLimits,
  shouldShowSeedFixtureAction,
  toggleExpandedModpack,
  type LoaderIconKind,
  type LibraryModpack,
  type LibraryScheme,
  type LibrarySelection,
} from "./library";
import {
  canFinishOnboardingWithKey,
  createInitialAppFlow,
  getCurseForgeKeyButtonState,
  getCurseForgeKeyInputCheckResult,
  onboardingReducer,
  shouldShowExistingKeyNotice,
  type CurseForgeKeyCheckResult,
  type CurseForgeKeyState,
  type SettingsSection,
} from "./onboarding";
import {
  cancelCurseForgeImport,
  checkCurseForgeApiKey,
  createScheme,
  deleteImportedModpack,
  deleteScheme,
  discoverAppPaths,
  getCurseForgeKeyStatus,
  listLibrary,
  listenToModpackImportProgress,
  listenToModpackImportStatus,
  openAppDataFolder,
  renameImportedModpack,
  renameScheme,
  retryModpackImport,
  saveCurseForgeApiKey,
  seedLocalLibraryFixture,
  type AppDataPaths,
  type CurseForgeCredentialStatus,
  type ImportProgress,
} from "./tauri";
import { type StageOptionId } from "./renderViewer";
import { ViewerWorkspace, type ViewerSelection, type ViewerToolContext } from "./ViewerWorkspace";
import "./styles.css";

const onboardingStorageKey = "mpb.onboardingComplete";

type LibraryDialog =
  | { kind: "createScheme"; modpackId: number; name: string; dimensions: [number, number, number] }
  | { kind: "renameScheme"; scheme: LibraryScheme; name: string }
  | { kind: "renameModpack"; modpack: LibraryModpack; name: string }
  | { kind: "infoModpack"; modpack: LibraryModpack }
  | { kind: "deleteScheme"; scheme: LibraryScheme }
  | { kind: "deleteModpack"; modpack: LibraryModpack };

export function App() {
  const [language, setLanguage] = useState<Language>(() => getInitialLanguage());
  const [flow, dispatch] = useReducer(
    onboardingReducer,
    null,
    () =>
      createInitialAppFlow({
        onboardingComplete: localStorage.getItem(onboardingStorageKey) === "true",
      }),
  );
  const [paths, setPaths] = useState<AppDataPaths | null>(null);
  const [keyStatus, setKeyStatus] = useState<CurseForgeCredentialStatus | null>(null);
  const [apiKeyInput, setApiKeyInput] = useState("");
  const [keyCheckResult, setKeyCheckResult] = useState<CurseForgeKeyCheckResult>("idle");
  const [keyCheckMessage, setKeyCheckMessage] = useState("");
  const [isSavingKey, setIsSavingKey] = useState(false);
  const [diagnosticsMessage, setDiagnosticsMessage] = useState("");
  const [library, setLibrary] = useState<LibraryModpack[]>([]);
  const [librarySelection, setLibrarySelection] = useState<LibrarySelection | null>(null);
  const [libraryMessage, setLibraryMessage] = useState("");
  const [libraryDialog, setLibraryDialog] = useState<LibraryDialog | null>(null);
  const [importJobModpackId, setImportJobModpackId] = useState<number | null>(null);
  const [importProgressByModpack, setImportProgressByModpack] = useState<Record<number, ImportProgress>>({});
  const [importLogsByModpack, setImportLogsByModpack] = useState<Record<number, string[]>>({});
  const [importStageByModpack, setImportStageByModpack] = useState<Record<number, string>>({});
  const [expandedModpackIds, setExpandedModpackIds] = useState<Set<number>>(new Set());
  const [sidebarWidth, setSidebarWidth] = useState<number>(sidebarWidthLimits.default);
  const [viewerSelection, setViewerSelection] = useState<ViewerSelection>(null);
  const [viewerStageId, setViewerStageId] = useState<StageOptionId | null>(null);
  const [viewerToolContext, setViewerToolContext] = useState<ViewerToolContext | null>(null);
  const handleViewerSelectionChange = useCallback((selection: ViewerSelection) => {
    setViewerSelection(selection);
  }, []);
  const handleViewerStageChange = useCallback((stageId: StageOptionId | null) => {
    setViewerStageId(stageId);
  }, []);
  const handleViewerToolContextChange = useCallback((context: ViewerToolContext | null) => {
    setViewerToolContext(context);
  }, []);

  const t = (key: Parameters<typeof translate>[1]) => translate(language, key);
  const selectedModpack =
    library.find((modpack) => modpack.id === librarySelection?.modpackId) ?? null;
  const selectedScheme =
    selectedModpack?.schemes.find((scheme) => scheme.id === librarySelection?.schemeId) ?? null;
  const importJobModpack =
    library.find((modpack) => modpack.id === importJobModpackId) ?? null;

  useEffect(() => {
    discoverAppPaths()
      .then(setPaths)
      .catch((error: unknown) => setDiagnosticsMessage(String(error)));
  }, []);

  useEffect(() => {
    getCurseForgeKeyStatus()
      .then((status) => {
        setKeyStatus(status);
        dispatch({ type: "setCurseForgeKeyState", state: status.state });
      })
      .catch((error: unknown) => {
        setKeyStatus({
          state: "unavailable",
          backend: "OS secure credential storage",
          message: String(error),
          apiKey: null,
        });
        dispatch({ type: "keyUnavailable" });
      });
  }, []);

  useEffect(() => {
    refreshLibrary();
  }, []);

  function appendImportLog(modpackId: number, message: string) {
    const trimmed = message.trim();
    if (!trimmed) {
      return;
    }
    const timestamp = new Date().toLocaleTimeString();
    setImportLogsByModpack((current) => {
      const previous = current[modpackId] ?? [];
      const nextLine = `${timestamp} ${trimmed}`;
      if (previous[previous.length - 1] === nextLine) {
        return current;
      }
      return {
        ...current,
        [modpackId]: [...previous.slice(-80), nextLine],
      };
    });
  }

  function clearImportJobHistory(modpackId: number) {
    setImportLogsByModpack((current) => {
      if (!(modpackId in current)) {
        return current;
      }
      const next = { ...current };
      delete next[modpackId];
      return next;
    });
    setImportProgressByModpack((current) => {
      if (!(modpackId in current)) {
        return current;
      }
      const next = { ...current };
      delete next[modpackId];
      return next;
    });
  }

  useEffect(() => {
    let unlisten: (() => void) | null = null;
    listenToModpackImportStatus((event) => {
      applyLibrary(event.library, { modpackId: event.modpackId, schemeId: -1 });
      setImportStageByModpack((current) => ({ ...current, [event.modpackId]: event.stage }));
      if (event.message) {
        appendImportLog(event.modpackId, event.message);
      }
    })
      .then((nextUnlisten) => {
        unlisten = nextUnlisten;
      })
      .catch((error: unknown) => setLibraryMessage(String(error)));

    return () => {
      unlisten?.();
    };
  }, []);

  useEffect(() => {
    let unlisten: (() => void) | null = null;
    listenToModpackImportProgress((event) => {
      setImportProgressByModpack((current) => ({ ...current, [event.modpackId]: event }));
    })
      .then((nextUnlisten) => {
        unlisten = nextUnlisten;
      })
      .catch((error: unknown) => setLibraryMessage(String(error)));

    return () => {
      unlisten?.();
    };
  }, []);

  function applyLibrary(nextLibrary: LibraryModpack[], requestedSelection = librarySelection) {
    const previousLibrary = library;
    setLibrary(nextLibrary);
    setLibrarySelection(getActiveLibrarySelection(nextLibrary, requestedSelection));
    setExpandedModpackIds((current) => {
      if (current.size === 0) {
        return getInitialExpandedModpackIds(nextLibrary);
      }

      const nextModpackIds = new Set(nextLibrary.map((modpack) => modpack.id));
      const previousModpackIds = new Set(previousLibrary.map((modpack) => modpack.id));
      const nextExpanded = new Set([...current].filter((id) => nextModpackIds.has(id)));
      for (const modpack of nextLibrary) {
        if (!previousModpackIds.has(modpack.id)) {
          nextExpanded.add(modpack.id);
        }
      }
      return nextExpanded;
    });
  }

  function handleToggleModpack(modpackId: number) {
    setExpandedModpackIds((current) => toggleExpandedModpack(current, modpackId));
  }

  function handleSidebarResizePointerDown(event: React.PointerEvent<HTMLButtonElement>) {
    event.preventDefault();
    const handlePointerMove = (moveEvent: PointerEvent) => {
      setSidebarWidth(clampSidebarWidth(moveEvent.clientX));
    };
    const handlePointerUp = () => {
      window.removeEventListener("pointermove", handlePointerMove);
      window.removeEventListener("pointerup", handlePointerUp);
    };

    window.addEventListener("pointermove", handlePointerMove);
    window.addEventListener("pointerup", handlePointerUp);
  }

  async function refreshLibrary() {
    try {
      const nextLibrary = await listLibrary();
      applyLibrary(nextLibrary, librarySelection);
      setLibraryMessage("");
    } catch (error) {
      setLibraryMessage(String(error));
    }
  }

  async function handleSeedLibraryFixture() {
    try {
      const nextLibrary = await seedLocalLibraryFixture();
      applyLibrary(nextLibrary, null);
      setLibraryMessage(t("library.fixtureLoaded"));
    } catch (error) {
      setLibraryMessage(String(error));
    }
  }

  function handleCreateScheme(modpackId: number) {
    const draft = createEmptyLibraryDraft(modpackId);
    setLibraryDialog({ kind: "createScheme", ...draft });
  }

  function handleRenameScheme(scheme: LibraryScheme) {
    setLibraryDialog({ kind: "renameScheme", scheme, name: scheme.name });
  }

  function handleDeleteScheme(scheme: LibraryScheme) {
    setLibraryDialog({ kind: "deleteScheme", scheme });
  }

  function handleRenameModpack(modpack: LibraryModpack) {
    setLibraryDialog({ kind: "renameModpack", modpack, name: modpack.localName });
  }

  function handleShowModpackInfo(modpack: LibraryModpack) {
    setLibraryDialog({ kind: "infoModpack", modpack });
  }

  function handleDeleteModpack(modpack: LibraryModpack) {
    setLibraryDialog({ kind: "deleteModpack", modpack });
  }

  function handleLibraryDialogNameChange(name: string) {
    setLibraryDialog((dialog) => {
      if (
        !dialog ||
        dialog.kind === "deleteScheme" ||
        dialog.kind === "deleteModpack" ||
        dialog.kind === "infoModpack"
      ) {
        return dialog;
      }
      return { ...dialog, name };
    });
  }

  async function handleConfirmLibraryDialog() {
    if (!libraryDialog) {
      return;
    }

    if (libraryDialog.kind === "infoModpack") {
      setLibraryDialog(null);
      return;
    }

    try {
      if (libraryDialog.kind === "createScheme") {
        const name = libraryDialog.name.trim() || "New scheme";
        const nextLibrary = await createScheme(
          libraryDialog.modpackId,
          name,
          libraryDialog.dimensions,
        );
        const createdScheme = nextLibrary
          .find((modpack) => modpack.id === libraryDialog.modpackId)
          ?.schemes.find((scheme) => scheme.name === name);
        applyLibrary(
          nextLibrary,
          createdScheme
            ? { modpackId: libraryDialog.modpackId, schemeId: createdScheme.id }
            : librarySelection,
        );
      }

      if (libraryDialog.kind === "renameScheme") {
        const name = libraryDialog.name.trim();
        if (!name) {
          return;
        }
        const nextLibrary = await renameScheme(libraryDialog.scheme.id, name);
        applyLibrary(nextLibrary, {
          modpackId: libraryDialog.scheme.modpackId,
          schemeId: libraryDialog.scheme.id,
        });
      }

      if (libraryDialog.kind === "deleteScheme") {
        const nextSelection = getNextSelectionAfterSchemeDelete(library, {
          modpackId: libraryDialog.scheme.modpackId,
          schemeId: libraryDialog.scheme.id,
        });
        const nextLibrary = await deleteScheme(libraryDialog.scheme.id);
        applyLibrary(nextLibrary, nextSelection);
      }

      if (libraryDialog.kind === "renameModpack") {
        const name = libraryDialog.name.trim();
        if (!name) {
          return;
        }
        const nextLibrary = await renameImportedModpack(libraryDialog.modpack.id, name);
        applyLibrary(nextLibrary, librarySelection);
      }

      if (libraryDialog.kind === "deleteModpack") {
        const nextLibrary = await deleteImportedModpack(libraryDialog.modpack.id);
        applyLibrary(nextLibrary, null);
      }

      setLibraryDialog(null);
      setLibraryMessage(t("library.autosaved"));
    } catch (error) {
      setLibraryMessage(String(error));
    }
  }

  function completeOnboarding() {
    localStorage.setItem(onboardingStorageKey, "true");
    dispatch({ type: "finishOnboarding" });
  }

  function skipOnboarding() {
    localStorage.setItem(onboardingStorageKey, "true");
    dispatch({ type: "skipOnboarding" });
  }

  function restartOnboarding() {
    localStorage.removeItem(onboardingStorageKey);
    setKeyCheckResult("idle");
    setKeyCheckMessage("");
    dispatch({ type: "restartOnboarding" });
  }

  function handleAddModpack() {
    dispatch({ type: "startAddModpack" });
  }

  async function handleCheckAndSaveKey() {
    const inputCheckResult = getCurseForgeKeyInputCheckResult(apiKeyInput);
    if (inputCheckResult === "empty") {
      setKeyCheckResult("empty");
      setKeyCheckMessage("");
      return;
    }

    setIsSavingKey(true);
    setKeyCheckResult("checking");
    setKeyCheckMessage("");
    try {
      await checkCurseForgeApiKey(apiKeyInput);
      const status = await saveCurseForgeApiKey(apiKeyInput);
      setKeyStatus(status);
      setApiKeyInput("");
      if (status.state === "saved") {
        setKeyCheckResult("valid");
        dispatch({ type: "keySaved" });
      } else {
        setKeyCheckResult("invalid");
        setKeyCheckMessage(status.message ?? t("settings.keyCheckInvalid"));
        dispatch({ type: "keyUnavailable" });
      }
    } catch (error) {
      setKeyCheckResult("invalid");
      setKeyCheckMessage(String(error));
    } finally {
      setIsSavingKey(false);
    }
  }

  function handleUpdateKeyInput(value: string) {
    setApiKeyInput(value);
    setKeyCheckResult("idle");
    setKeyCheckMessage("");
  }

  function handleCheckKey() {
    void handleCheckAndSaveKey();
  }

  async function handleOpenDataFolder() {
    try {
      const nextPaths = await openAppDataFolder();
      if (nextPaths) {
        setPaths(nextPaths);
        setDiagnosticsMessage(nextPaths.appDataDir);
      } else {
        setDiagnosticsMessage(t("settings.desktopOnly"));
      }
    } catch (error) {
      setDiagnosticsMessage(String(error));
    }
  }

  if (flow.screen === "onboarding") {
    return (
      <OnboardingScreen
        apiKeyInput={apiKeyInput}
        isSavingKey={isSavingKey}
        keyState={flow.curseForgeKey}
        keyStatus={keyStatus}
        keyCheckResult={keyCheckResult}
        keyCheckMessage={keyCheckMessage}
        keyNotice={flow.keyNotice}
        language={language}
        onBack={() => dispatch({ type: "previousOnboardingStep" })}
        onCheckKey={handleCheckKey}
        onFinish={completeOnboarding}
        onLanguageChange={setLanguage}
        onNextAi={() => dispatch({ type: "setOnboardingStep", step: "curseforge" })}
        onNextLanguage={() => dispatch({ type: "setOnboardingStep", step: "ai" })}
        onSkip={skipOnboarding}
        onUpdateKey={handleUpdateKeyInput}
        step={flow.onboardingStep}
        t={t}
      />
    );
  }

  return (
    <main
      className="app-shell antialiased"
      style={{ "--sidebar-width": `${sidebarWidth}px` } as React.CSSProperties}
    >
      <aside className="sidebar" aria-label={t("workspace.library")}>
        <div className="brand">
          <div className="brand-mark">
            <Box size={18} />
          </div>
          <div>
            <h1>{t("app.title")}</h1>
            <span className="brand-status">
              <Bot size={12} />
              {t("status.aiDisconnected")}
            </span>
          </div>
        </div>

        <button className="primary-action" onClick={handleAddModpack} type="button">
          <PackagePlus size={17} />
          <span>{t("workspace.addModpack")}</span>
        </button>

        <section className="library-panel">
          <div className="panel-title">
            <span>{t("workspace.library")}</span>
          </div>
          <LibraryTree
            canSeedFixture={shouldShowSeedFixtureAction(library, import.meta.env.DEV)}
            expandedModpackIds={expandedModpackIds}
            library={library}
            onCreateScheme={handleCreateScheme}
            onDeleteModpack={handleDeleteModpack}
            onDeleteScheme={handleDeleteScheme}
            onRenameModpack={handleRenameModpack}
            onRenameScheme={handleRenameScheme}
            onSelect={setLibrarySelection}
            onSeed={handleSeedLibraryFixture}
            onShowImportJob={(modpack) => setImportJobModpackId(modpack.id)}
            onShowModpackInfo={handleShowModpackInfo}
            onToggleModpack={handleToggleModpack}
            selected={librarySelection}
            t={t}
          />
          {libraryMessage && <p className="library-message">{libraryMessage}</p>}
        </section>

        <button
          className="settings-link"
          onClick={() => dispatch({ type: "openSettings", section: "ai" })}
          type="button"
        >
          <Settings size={17} />
          <span>{t("workspace.settings")}</span>
        </button>
      </aside>
      <button
        aria-label={t("library.resizeSidebar")}
        className="sidebar-resize-handle"
        onPointerDown={handleSidebarResizePointerDown}
        type="button"
      />

      <section className="workspace">
        <div className="content-grid">
          <ViewerWorkspace
            modpack={selectedModpack}
            onSelectionChange={handleViewerSelectionChange}
            onStageChange={handleViewerStageChange}
            onToolContextChange={handleViewerToolContextChange}
            scheme={selectedScheme}
            selectedStageId={viewerStageId}
            t={t}
          />

          <RightToolPanel
            onStageChange={setViewerStageId}
            selection={viewerSelection}
            t={t}
            toolContext={viewerToolContext}
          />
        </div>
      </section>
      {flow.settingsModalOpen && (
        <SettingsModal
          apiKeyInput={apiKeyInput}
          diagnosticsMessage={diagnosticsMessage}
          isSavingKey={isSavingKey}
          keyCheckResult={keyCheckResult}
          keyCheckMessage={keyCheckMessage}
          keyNotice={flow.keyNotice}
          keyState={flow.curseForgeKey}
          keyStatus={keyStatus}
          language={language}
          onCheckKey={handleCheckKey}
          onClose={() => dispatch({ type: "closeSettings" })}
          onLanguageChange={setLanguage}
          onOpenDataFolder={handleOpenDataFolder}
          onRestartOnboarding={restartOnboarding}
          onSectionChange={(section) => dispatch({ type: "openSettings", section })}
          onUpdateKey={handleUpdateKeyInput}
          paths={paths}
          section={flow.settingsSection}
          t={t}
        />
      )}
      {flow.importModalOpen && (
        <div className="modal-backdrop" role="presentation">
          <ImportWizardWorkspace
            library={library}
            onClose={() => dispatch({ type: "closeImportWizard" })}
            onImported={(nextLibrary, modpackId) => {
              applyLibrary(nextLibrary, { modpackId, schemeId: -1 });
              setImportStageByModpack((current) => ({ ...current, [modpackId]: "queued" }));
              appendImportLog(modpackId, t("import.addStarted"));
              setLibraryMessage(t("import.addStarted"));
            }}
            t={t}
          />
        </div>
      )}
      {libraryDialog && (
        <LibraryActionDialog
          dialog={libraryDialog}
          onCancel={() => setLibraryDialog(null)}
          onConfirm={handleConfirmLibraryDialog}
          onNameChange={handleLibraryDialogNameChange}
          t={t}
        />
      )}
      {importJobModpack && (
        <ImportJobDialog
          logs={importLogsByModpack[importJobModpack.id] ?? []}
          modpack={importJobModpack}
          onCancel={async () => {
            try {
              await cancelCurseForgeImport();
              appendImportLog(importJobModpack.id, t("import.cancelRequested"));
            } catch (error) {
              appendImportLog(importJobModpack.id, String(error));
            }
          }}
          onClose={() => setImportJobModpackId(null)}
          onDelete={() => {
            setImportJobModpackId(null);
            handleDeleteModpack(importJobModpack);
          }}
          onRetry={async () => {
            clearImportJobHistory(importJobModpack.id);
            try {
              const nextLibrary = await retryModpackImport(importJobModpack.id);
              applyLibrary(nextLibrary, { modpackId: importJobModpack.id, schemeId: -1 });
              setImportStageByModpack((current) => ({
                ...current,
                [importJobModpack.id]: "queued",
              }));
              appendImportLog(importJobModpack.id, t("import.retryQueued"));
            } catch (error) {
              appendImportLog(importJobModpack.id, String(error));
            }
          }}
          progress={importProgressByModpack[importJobModpack.id] ?? null}
          stage={importStageByModpack[importJobModpack.id] ?? importJobStageFromMessage(importJobModpack)}
          t={t}
        />
      )}
    </main>
  );
}

type Translator = (key: Parameters<typeof translate>[1]) => string;

function OnboardingScreen(props: {
  apiKeyInput: string;
  isSavingKey: boolean;
  keyState: CurseForgeKeyState;
  keyStatus: CurseForgeCredentialStatus | null;
  keyCheckResult: CurseForgeKeyCheckResult;
  keyCheckMessage: string;
  keyNotice: "idle" | "missing" | "saved" | "replaced" | "unavailable";
  language: Language;
  onBack: () => void;
  onCheckKey: () => void;
  onFinish: () => void;
  onLanguageChange: (language: Language) => void;
  onNextAi: () => void;
  onNextLanguage: () => void;
  onSkip: () => void;
  onUpdateKey: (value: string) => void;
  step: "language" | "ai" | "curseforge";
  t: Translator;
}) {
  const { t } = props;
  return (
    <main className="onboarding-shell">
      <section className="onboarding-panel" aria-label={t("onboarding.title")}>
        <div className="brand onboarding-brand">
          <div className="brand-mark">
            <Box size={18} />
          </div>
          <div>
            <h1>{t("app.title")}</h1>
            <span>{t("onboarding.title")}</span>
          </div>
        </div>

        {props.step === "language" && (
          <div className="onboarding-step">
            <StepIcon>
              <Languages size={22} />
            </StepIcon>
            <h2>{t("onboarding.languageTitle")}</h2>
            <p>{t("onboarding.languageBody")}</p>
            <div className="choice-row">
              {languages.map((option) => (
                <button
                  className={option === props.language ? "choice-button active" : "choice-button"}
                  key={option}
                  onClick={() => props.onLanguageChange(option)}
                  type="button"
                >
                  {option.toUpperCase()}
                </button>
              ))}
            </div>
            <div className="onboarding-actions split">
              <button className="ghost-action" onClick={props.onSkip} type="button">
                {t("onboarding.skip")}
              </button>
              <div className="nav-actions">
                <button className="primary-action compact" onClick={props.onNextLanguage} type="button">
                  {t("onboarding.next")}
                  <ArrowRight size={16} />
                </button>
              </div>
            </div>
          </div>
        )}

        {props.step === "ai" && (
          <div className="onboarding-step">
            <StepIcon>
              <PlugZap size={22} />
            </StepIcon>
            <h2>{t("onboarding.aiTitle")}</h2>
            <p>{t("onboarding.aiBody")}</p>
            <PromptBlock t={t} />
            <StatusRows
              rows={[
                [t("settings.status"), t("status.aiDisconnected")],
                [t("settings.activeClient"), t("settings.noActiveClient")],
              ]}
            />
            <div className="onboarding-actions split">
              <button className="ghost-action" onClick={props.onSkip} type="button">
                {t("onboarding.skip")}
              </button>
              <div className="nav-actions">
                <button className="secondary-action compact" onClick={props.onBack} type="button">
                  <ArrowLeft size={16} />
                  {t("onboarding.back")}
                </button>
                <button className="primary-action compact" onClick={props.onNextAi} type="button">
                  {t("onboarding.next")}
                  <ArrowRight size={16} />
                </button>
              </div>
            </div>
          </div>
        )}

        {props.step === "curseforge" && (
          <div className="onboarding-step">
            <StepIcon>
              <KeyRound size={22} />
            </StepIcon>
            <h2>{t("onboarding.keyTitle")}</h2>
            <p>{t("onboarding.keyBody")}</p>
            <KeyForm
              apiKeyInput={props.apiKeyInput}
              isSavingKey={props.isSavingKey}
              keyCheckResult={props.keyCheckResult}
              keyCheckMessage={props.keyCheckMessage}
              keyNotice={props.keyNotice}
              keyState={props.keyState}
              keyStatus={props.keyStatus}
              onCheckKey={props.onCheckKey}
              onUpdateKey={props.onUpdateKey}
              t={t}
            />
            <div className="onboarding-actions split">
              <button className="ghost-action" onClick={props.onFinish} type="button">
                {t("onboarding.skip")}
              </button>
              <div className="nav-actions">
                <button className="secondary-action compact" onClick={props.onBack} type="button">
                  <ArrowLeft size={16} />
                  {t("onboarding.back")}
                </button>
                <button
                  className="primary-action compact"
                  disabled={!canFinishOnboardingWithKey(props.keyState)}
                  onClick={props.onFinish}
                  type="button"
                >
                  {t("onboarding.finish")}
                </button>
              </div>
            </div>
          </div>
        )}
      </section>
    </main>
  );
}

function LibraryTree(props: {
  canSeedFixture: boolean;
  expandedModpackIds: Set<number>;
  library: LibraryModpack[];
  onCreateScheme: (modpackId: number) => void;
  onDeleteModpack: (modpack: LibraryModpack) => void;
  onDeleteScheme: (scheme: LibraryScheme) => void;
  onRenameModpack: (modpack: LibraryModpack) => void;
  onRenameScheme: (scheme: LibraryScheme) => void;
  onSelect: (selection: LibrarySelection) => void;
  onSeed: () => void;
  onShowImportJob: (modpack: LibraryModpack) => void;
  onShowModpackInfo: (modpack: LibraryModpack) => void;
  onToggleModpack: (modpackId: number) => void;
  selected: LibrarySelection | null;
  t: Translator;
}) {
  const { t } = props;
  const [openModpackMenu, setOpenModpackMenu] = useState<{
    id: number;
    left: number;
    top: number;
  } | null>(null);
  useEffect(() => {
    if (openModpackMenu === null) {
      return;
    }

    const closeMenu = () => {
      setOpenModpackMenu(null);
    };

    window.addEventListener("pointerdown", closeMenu);
    return () => window.removeEventListener("pointerdown", closeMenu);
  }, [openModpackMenu]);

  if (props.library.length === 0) {
    return (
      <div className="library-empty-state">
        <p>{t("library.empty")}</p>
        {props.canSeedFixture && (
          <button className="secondary-action compact" onClick={props.onSeed} type="button">
            <Database size={15} />
            {t("library.loadFixture")}
          </button>
        )}
      </div>
    );
  }

  return (
    <div
      className="library-tree"
      style={{ "--library-node-gap": `${compactLibraryNodeGap}px` } as React.CSSProperties}
    >
      {props.library.map((modpack) => (
        <div
          className={props.expandedModpackIds.has(modpack.id) ? "library-node expanded" : "library-node"}
          key={modpack.id}
        >
          <div className="tree-item modpack-row">
            <button
              className="tree-label modpack-label"
              onClick={() => {
                if (modpack.importStatus === "imported") {
                  props.onToggleModpack(modpack.id);
                } else {
                  props.onShowImportJob(modpack);
                }
              }}
              type="button"
            >
              <LoaderIcon kind={getLoaderIconKind(modpack.loader)} />
              <span title={modpack.localName}>{modpack.localName}</span>
            </button>
            {modpack.importStatus === "imported" ? (
              <div className="tree-actions">
                <button
                  aria-label={t("library.createScheme")}
                  className="icon-action small"
                  onClick={() => props.onCreateScheme(modpack.id)}
                  type="button"
                >
                  <Plus size={14} />
                </button>
                <button
                  aria-label={t("library.modpackActions")}
                  className="icon-action small"
                  onClick={(event) => {
                    event.stopPropagation();
                    const placement = getModpackMenuPlacement(event.currentTarget.getBoundingClientRect(), {
                      width: window.innerWidth,
                      height: window.innerHeight,
                    });
                    setOpenModpackMenu((current) => {
                      const nextId = getNextOpenModpackMenuId(current?.id ?? null, modpack.id, "menuButton");
                      return nextId === null ? null : { id: nextId, ...placement };
                    });
                  }}
                  type="button"
                >
                  <MoreHorizontal size={15} />
                </button>
              </div>
            ) : (
              <ImportStatusIndicator status={modpack.importStatus} t={t} />
            )}
            {openModpackMenu?.id === modpack.id && (
              <div
                className="modpack-menu"
                onPointerDown={(event) => event.stopPropagation()}
                role="menu"
                style={
                  {
                    "--modpack-menu-left": `${openModpackMenu.left}px`,
                    "--modpack-menu-top": `${openModpackMenu.top}px`,
                  } as React.CSSProperties
                }
              >
                <button
                  onClick={() => {
                    setOpenModpackMenu(null);
                    props.onShowModpackInfo(modpack);
                  }}
                  role="menuitem"
                  type="button"
                >
                  <Info size={14} />
                  {t("library.information")}
                </button>
                <button
                  onClick={() => {
                    setOpenModpackMenu(null);
                    props.onRenameModpack(modpack);
                  }}
                  role="menuitem"
                  type="button"
                >
                  <Pencil size={14} />
                  {t("library.renameModpack")}
                </button>
                <button
                  className="danger"
                  onClick={() => {
                    setOpenModpackMenu(null);
                    props.onDeleteModpack(modpack);
                  }}
                  role="menuitem"
                  type="button"
                >
                  <Trash2 size={14} />
                  {t("library.deleteModpack")}
                </button>
              </div>
            )}
          </div>
          {modpack.importStatus === "imported" &&
            props.expandedModpackIds.has(modpack.id) &&
            (modpack.schemes.length === 0 ? (
              <div className="tree-item nested empty-scheme-row">{t("library.noSchemes")}</div>
            ) : (
              modpack.schemes.map((scheme) => (
              <div
                className={
                  props.selected?.schemeId === scheme.id
                    ? "tree-item nested selected scheme-row"
                    : "tree-item nested scheme-row"
                }
                key={scheme.id}
              >
                <button
                  className="tree-label scheme-label"
                  onClick={() => props.onSelect({ modpackId: modpack.id, schemeId: scheme.id })}
                  type="button"
                >
                  <Layers3 size={15} />
                  <span title={scheme.name}>{scheme.name}</span>
                </button>
                <div className="tree-actions">
                  <button
                    aria-label={t("library.renameScheme")}
                    className="icon-action small"
                    onClick={() => props.onRenameScheme(scheme)}
                    type="button"
                  >
                    <Pencil size={14} />
                  </button>
                  <button
                    aria-label={t("library.deleteScheme")}
                    className="icon-action small danger"
                    onClick={() => props.onDeleteScheme(scheme)}
                    type="button"
                  >
                    <Trash2 size={14} />
                  </button>
                </div>
              </div>
              ))
            ))}
        </div>
      ))}
    </div>
  );
}

function ImportStatusIndicator({
  status,
  t,
}: {
  status: LibraryModpack["importStatus"];
  t: Translator;
}) {
  if (status === "importing") {
    return (
      <span className="import-status-indicator importing" title={t("import.state.importing")}>
        <Loader2 className="status-spinner" size={15} />
      </span>
    );
  }
  if (status === "failed") {
    return (
      <span className="import-status-indicator failed" title={t("import.state.failed")}>
        <AlertTriangle size={15} />
      </span>
    );
  }
  return null;
}

function LoaderIcon({ kind }: { kind: LoaderIconKind }) {
  const label = {
    forge: "F",
    neoforge: "NF",
    fabric: "Fb",
    quilt: "Q",
    generic: "MC",
  }[kind];

  return (
    <span aria-hidden="true" className={`loader-icon ${kind}`}>
      {label}
    </span>
  );
}

function RightToolPanel({
  onStageChange,
  selection,
  t,
  toolContext,
}: {
  onStageChange: (stageId: StageOptionId) => void;
  selection: ViewerSelection;
  t: Translator;
  toolContext: ViewerToolContext | null;
}) {
  const [openSections, setOpenSections] = useState({
    materials: false,
    review: false,
    stages: true,
  });
  const materialTotal = toolContext?.materials.reduce((total, line) => total + line.count, 0) ?? 0;

  function toggleSection(section: keyof typeof openSections) {
    setOpenSections((current) => ({ ...current, [section]: !current[section] }));
  }

  return (
    <aside className="right-rail tools-sidebar" aria-label={t("tools.sidebar")}>
      <div className="tool-summary" aria-label="Render metrics">
        <span>
          {toolContext
            ? `${toolContext.metrics.visibleBlocks} / ${toolContext.metrics.totalBlocks}`
            : "--"}
        </span>
        <span>{toolContext ? `${toolContext.metrics.chunkCount} chunks` : "-- chunks"}</span>
        <span>{toolContext ? `${toolContext.metrics.faceCount} faces` : "-- faces"}</span>
      </div>
      <div className="tool-tree">
        <section className={openSections.stages ? "tool-node expanded" : "tool-node"}>
          <div className="tree-item tool-row">
            <button className="tree-label tool-label" onClick={() => toggleSection("stages")} type="button">
              <Layers3 size={16} />
              {t("tools.stages")}
            </button>
            <strong>
              {toolContext
                ? `${toolContext.metrics.visibleBlocks} / ${toolContext.metrics.totalBlocks}`
                : "--"}
            </strong>
            <ChevronDown className={openSections.stages ? "open" : ""} size={16} />
          </div>
          {openSections.stages && (
            <div className="tool-children">
              <div className="tool-panel-stage-list">
                {toolContext ? (
                  toolContext.stageOptions.map((stage) => (
                    <button
                      className={stage.id === toolContext.selectedStageId ? "active" : ""}
                      key={stage.id}
                      onClick={() => onStageChange(stage.id)}
                      type="button"
                    >
                      <span>{stage.label}</span>
                      <strong>{stage.order ?? t("tools.unassigned")}</strong>
                    </button>
                  ))
                ) : (
                  <div className="empty-list">{t("tools.openScheme")}</div>
                )}
              </div>
            </div>
          )}
        </section>

        <section className={openSections.review ? "tool-node expanded" : "tool-node"}>
          <div className="tree-item tool-row">
            <button className="tree-label tool-label" onClick={() => toggleSection("review")} type="button">
              <ClipboardList size={16} />
              {t("tools.review")}
            </button>
            <strong>{t("review.pending")}: 0</strong>
            <ChevronDown className={openSections.review ? "open" : ""} size={16} />
          </div>
          {openSections.review && (
            <div className="tool-children">
              <div className="selection-box">
                <span>{t("review.selection")}</span>
                <code>
                  {selection
                    ? `x: ${selection.coordinate[0]}, y: ${selection.coordinate[1]}, z: ${selection.coordinate[2]}`
                    : "x: --, y: --, z: --"}
                </code>
                {selection && <span>{selection.blockId}</span>}
              </div>
              <div className="empty-list">{t("review.changeRequests")}</div>
            </div>
          )}
        </section>

        <section className={openSections.materials ? "tool-node expanded" : "tool-node"}>
          <div className="tree-item tool-row">
            <button className="tree-label tool-label" onClick={() => toggleSection("materials")} type="button">
              <Database size={16} />
              {t("tools.materials")}
            </button>
            <strong>{t("materials.total")}: {materialTotal}</strong>
            <ChevronDown className={openSections.materials ? "open" : ""} size={16} />
          </div>
          {openSections.materials && (
            <div className="tool-children">
              {toolContext ? (
                <ul className="materials-list">
                  {toolContext.materials.map((material) => (
                    <li key={material.blockId}>
                      <span>{material.blockId}</span>
                      <strong>{material.count}</strong>
                    </li>
                  ))}
                </ul>
              ) : (
                <div className="empty-list">{t("tools.openScheme")}</div>
              )}
            </div>
          )}
        </section>
      </div>
    </aside>
  );
}

function ImportJobDialog({
  logs,
  modpack,
  onCancel,
  onClose,
  onDelete,
  onRetry,
  progress,
  stage,
  t,
}: {
  logs: string[];
  modpack: LibraryModpack;
  onCancel: () => void;
  onClose: () => void;
  onDelete: () => void;
  onRetry: () => void;
  progress: ImportProgress | null;
  stage: string;
  t: Translator;
}) {
  const progressValue = getImportJobProgressValue(stage, modpack.importStatus, progress);
  const stages = getImportJobStages(stage, modpack.importStatus);
  const canDelete = modpack.importStatus === "failed";
  const logViewportRef = useRef<HTMLDivElement | null>(null);
  const shouldStickLogToBottomRef = useRef(true);
  const visibleLogLines = logs.length > 0 ? logs : [modpack.importMessage ?? t("import.noLogYet")];

  useLayoutEffect(() => {
    const viewport = logViewportRef.current;
    if (!viewport || !shouldStickLogToBottomRef.current) {
      return;
    }
    viewport.scrollTop = viewport.scrollHeight;
  }, [visibleLogLines.length, visibleLogLines[visibleLogLines.length - 1]]);

  function handleLogScroll(event: React.UIEvent<HTMLDivElement>) {
    const viewport = event.currentTarget;
    const distanceFromBottom = viewport.scrollHeight - viewport.scrollTop - viewport.clientHeight;
    shouldStickLogToBottomRef.current = distanceFromBottom < 24;
  }

  return (
    <div className="modal-backdrop" role="presentation">
      <section className="import-job-modal" aria-label={t("import.processingTitle")} role="dialog">
        <header className="settings-modal-header">
          <div>
            <h2>{t("import.processingTitle")}</h2>
            <div className="import-job-meta" aria-label="Selected release metadata">
              <span>{modpack.versionName}</span>
              <span>{modpack.minecraftVersion ?? t("library.unknown")}</span>
              <span>{modpack.loader ?? t("library.unknown")}</span>
            </div>
          </div>
          <button aria-label={t("settings.close")} className="icon-action" onClick={onClose} type="button">
            <X size={18} />
          </button>
        </header>

        <div className="import-job-body">
          <section className="import-job-summary">
            <div className="import-job-progress">
              <div>
                <span>{t("import.progress")}</span>
                <strong>{progressValue}%</strong>
              </div>
              <progress max={100} value={progressValue} />
            </div>
          </section>

          <section className="import-job-stages" aria-label={t("import.stages")}>
            {stages.map((item) => (
              <div className={`import-job-stage ${item.state}`} key={item.key}>
                {item.state === "active" ? (
                  <Loader2 className="status-spinner" size={15} />
                ) : item.state === "done" ? (
                  <CheckCircle2 size={15} />
                ) : item.state === "failed" ? (
                  <AlertTriangle size={15} />
                ) : (
                  <span className="stage-dot" />
                )}
                <span>{t(item.label)}</span>
              </div>
            ))}
          </section>

          <section className="import-job-log" aria-label={t("import.liveLog")}>
            <div className="import-log-lines" onScroll={handleLogScroll} ref={logViewportRef}>
              {visibleLogLines.map((line, index) => (
                <code key={`${line}-${index}`}>{line}</code>
              ))}
            </div>
          </section>
        </div>

        <div className="dialog-actions">
          {canDelete && (
            <button className="secondary-action compact danger" onClick={onDelete} type="button">
              <Trash2 size={16} />
              {t("library.deleteModpack")}
            </button>
          )}
          <span className="dialog-actions-spacer" />
          {modpack.importStatus === "importing" && (
            <button className="secondary-action compact danger" onClick={onCancel} type="button">
              <X size={16} />
              {t("import.cancel")}
            </button>
          )}
          {modpack.importStatus === "failed" && (
            <button className="primary-action compact" onClick={onRetry} type="button">
              <RefreshCcw size={16} />
              {t("import.retry")}
            </button>
          )}
        </div>
      </section>
    </div>
  );
}

function getImportJobProgressValue(
  stage: string,
  status: LibraryModpack["importStatus"],
  progress: ImportProgress | null,
): number {
  if (status === "imported") {
    return 100;
  }

  if (progress?.progressPercent !== null && progress?.progressPercent !== undefined) {
    return Math.min(99, Math.max(0, progress.progressPercent));
  }

  if (stage === "parse" || stage === "failed") {
    return 30;
  }

  if (stage === "download") {
    if (!progress?.totalBytes || progress.totalBytes <= 0) {
      return 20;
    }
    const downloadRatio = Math.min(1, Math.max(0, progress.bytesDownloaded / progress.totalBytes));
    return Math.round(10 + downloadRatio * 20);
  }

  return 5;
}

function importJobStageFromMessage(modpack: LibraryModpack): string {
  const message = modpack.importMessage?.toLowerCase() ?? "";
  if (modpack.importStatus === "failed") {
    return "failed";
  }
  if (message.includes("pars")) {
    return "parse";
  }
  if (message.includes("download")) {
    return "download";
  }
  if (modpack.importStatus === "imported") {
    return "done";
  }
  return "queued";
}

function getImportJobStages(stage: string, status: LibraryModpack["importStatus"]) {
  const order = ["queued", "download", "parse", "done"];
  const activeIndex = stage === "failed" ? 2 : Math.max(0, order.indexOf(stage));
  return [
    { key: "queued", label: "import.stage.queued" as const },
    { key: "download", label: "import.stage.download" as const },
    { key: "parse", label: "import.stage.parse" as const },
    { key: "done", label: "import.stage.done" as const },
  ].map((item, index) => {
    if (status === "failed" && item.key === "parse") {
      return { ...item, state: "failed" as const };
    }
    if (status === "imported" || index < activeIndex) {
      return { ...item, state: "done" as const };
    }
    if (index === activeIndex) {
      return { ...item, state: "active" as const };
    }
    return { ...item, state: "pending" as const };
  });
}

function LibraryActionDialog(props: {
  dialog: LibraryDialog;
  onCancel: () => void;
  onConfirm: () => void;
  onNameChange: (name: string) => void;
  t: Translator;
}) {
  const { dialog, t } = props;
  const isDelete = dialog.kind === "deleteScheme" || dialog.kind === "deleteModpack";
  const isInfo = dialog.kind === "infoModpack";
  const content = getLibraryDialogContent(dialog.kind);
  const title = t(content.titleKey);

  return (
    <div className="modal-backdrop" role="presentation">
      <section
        className={`settings-modal library-dialog ${content.tone}`}
        aria-label={title}
        role="dialog"
      >
        <header className="settings-modal-header">
          <h2>{title}</h2>
          <button
            aria-label={t("library.cancel")}
            className="icon-action"
            onClick={props.onCancel}
            type="button"
          >
            <X size={18} />
          </button>
        </header>
        {content.bodyKey && (
          <div className="library-dialog-copy">
            <p>{t(content.bodyKey)}</p>
          </div>
        )}
        {isInfo && <ModpackInfoRows modpack={dialog.modpack} t={t} />}
        {!isDelete && !isInfo && (
          <label className="library-dialog-field">
            <span>{content.fieldKey ? t(content.fieldKey) : t("library.nameLabel")}</span>
            <input
              autoFocus
              onChange={(event) => props.onNameChange(event.currentTarget.value)}
              value={dialog.name}
            />
          </label>
        )}
        <div className="dialog-actions">
          <button className="secondary-action compact" onClick={props.onCancel} type="button">
            {isInfo ? t("library.close") : t("library.cancel")}
          </button>
          {!isInfo && (
            <button
              className={isDelete ? "secondary-action compact danger" : "primary-action compact"}
              onClick={props.onConfirm}
              type="button"
            >
              {t("library.confirm")}
            </button>
          )}
        </div>
      </section>
    </div>
  );
}

function ModpackInfoRows({ modpack, t }: { modpack: LibraryModpack; t: Translator }) {
  const rows: Array<[string, string]> = [
    [t("library.localName"), modpack.localName],
    [t("library.releaseVersion"), modpack.versionName],
    [t("library.minecraftVersion"), modpack.minecraftVersion ?? t("library.unknown")],
    [t("library.loader"), modpack.loader ?? t("library.unknown")],
    [t("library.sourceUrl"), modpack.sourceUrl ?? t("library.unknown")],
    [t("library.importStatus"), modpack.importStatus],
    [t("library.importMessage"), modpack.importMessage ?? t("library.unknown")],
    [t("library.schemeCount"), String(modpack.schemes.length)],
  ];

  return (
    <dl className="modpack-info-list">
      {rows.map(([label, value]) => (
        <div key={label}>
          <dt>{label}</dt>
          <dd>{value}</dd>
        </div>
      ))}
    </dl>
  );
}

function SettingsModal(props: {
  apiKeyInput: string;
  diagnosticsMessage: string;
  isSavingKey: boolean;
  keyCheckResult: CurseForgeKeyCheckResult;
  keyCheckMessage: string;
  keyNotice: "idle" | "missing" | "saved" | "replaced" | "unavailable";
  keyState: CurseForgeKeyState;
  keyStatus: CurseForgeCredentialStatus | null;
  language: Language;
  onCheckKey: () => void;
  onClose: () => void;
  onLanguageChange: (language: Language) => void;
  onOpenDataFolder: () => void;
  onRestartOnboarding: () => void;
  onSectionChange: (section: SettingsSection) => void;
  onUpdateKey: (value: string) => void;
  paths: AppDataPaths | null;
  section: SettingsSection;
  t: Translator;
}) {
  const { t } = props;
  const sections: Array<[SettingsSection, string]> = [
    ["ai", t("settings.aiIntegration")],
    ["curseforge", t("settings.curseforgeKey")],
    ["language", t("settings.language")],
    ["data", t("settings.dataFolders")],
  ];

  return (
    <div className="modal-backdrop" role="presentation">
      <section className="settings-modal" aria-label={t("workspace.settings")} role="dialog">
        <header className="settings-modal-header">
          <div>
            <h2>{t("workspace.settings")}</h2>
            <span>{t("settings.status")}</span>
          </div>
          <button
            aria-label={t("settings.close")}
            className="icon-action"
            onClick={props.onClose}
            type="button"
          >
            <X size={18} />
          </button>
        </header>
      <div className="settings-layout">
        <nav className="settings-tabs" aria-label={t("workspace.settings")}>
          {sections.map(([section, label]) => (
            <button
              className={props.section === section ? "active" : ""}
              key={section}
              onClick={() => props.onSectionChange(section)}
              type="button"
            >
              {label}
            </button>
          ))}
        </nav>

        <div className="settings-content">
          {props.section === "ai" && (
            <SettingsPane icon={<PlugZap size={23} />} title={t("settings.aiIntegration")}>
              <p>{t("settings.aiInstructions")}</p>
              <PromptBlock t={t} />
              <StatusRows
                rows={[
                  [t("settings.status"), t("status.aiDisconnected")],
                  [t("settings.activeClient"), t("settings.noActiveClient")],
                  [t("settings.connection"), t("settings.desktopOnly")],
                ]}
              />
              <button className="secondary-action compact" onClick={props.onRestartOnboarding} type="button">
                <RefreshCcw size={16} />
                {t("settings.showOnboarding")}
              </button>
            </SettingsPane>
          )}

          {props.section === "curseforge" && (
            <SettingsPane icon={<KeyRound size={23} />} title={t("settings.curseforgeKey")}>
              <p>{t("onboarding.keyBody")}</p>
              <KeyForm
                apiKeyInput={props.apiKeyInput}
                isSavingKey={props.isSavingKey}
                keyCheckResult={props.keyCheckResult}
                keyCheckMessage={props.keyCheckMessage}
                keyNotice={props.keyNotice}
                keyState={props.keyState}
                keyStatus={props.keyStatus}
                onCheckKey={props.onCheckKey}
                onUpdateKey={props.onUpdateKey}
                t={t}
              />
              <p className="subtle-line">
                <EyeOff size={15} />
                {t("settings.keyStoredNotice")}
              </p>
            </SettingsPane>
          )}

          {props.section === "language" && (
            <SettingsPane icon={<Languages size={23} />} title={t("settings.language")}>
              <p>{t("onboarding.languageBody")}</p>
              <div className="choice-row">
                {languages.map((option) => (
                  <button
                    className={option === props.language ? "choice-button active" : "choice-button"}
                    key={option}
                    onClick={() => props.onLanguageChange(option)}
                    type="button"
                  >
                    {option.toUpperCase()}
                  </button>
                ))}
              </div>
            </SettingsPane>
          )}

          {props.section === "data" && (
            <SettingsPane icon={<FolderOpen size={23} />} title={t("settings.dataFolders")}>
              <button className="secondary-action compact" onClick={props.onOpenDataFolder} type="button">
                <FolderOpen size={16} />
                {t("settings.openDataFolder")}
              </button>
              <StatusRows
                rows={[
                  [t("settings.appData"), props.paths?.appDataDir ?? props.diagnosticsMessage],
                  [
                    t("settings.diagnosticsFolder"),
                    props.paths?.diagnosticsDir ?? props.diagnosticsMessage,
                  ],
                ]}
              />
            </SettingsPane>
          )}
        </div>
      </div>
      </section>
    </div>
  );
}

function KeyForm(props: {
  apiKeyInput: string;
  isSavingKey: boolean;
  keyCheckResult: CurseForgeKeyCheckResult;
  keyCheckMessage: string;
  keyNotice: "idle" | "missing" | "saved" | "replaced" | "unavailable";
  keyState: CurseForgeKeyState;
  keyStatus: CurseForgeCredentialStatus | null;
  onCheckKey: () => void;
  onUpdateKey: (value: string) => void;
  t: Translator;
}) {
  const { t } = props;
  const keyButtonState = getCurseForgeKeyButtonState(props.keyCheckResult, props.isSavingKey);
  return (
    <div className="key-form">
      <div className={`key-status ${props.keyState}`}>
        {props.keyState === "saved" ? <ShieldCheck size={18} /> : <AlertTriangle size={18} />}
        <div>
          <strong>{keyLabel(t, props.keyState, props.keyNotice)}</strong>
          <span>
            {t("settings.backend")}: {props.keyStatus?.backend ?? "OS secure credential storage"}
          </span>
          {props.keyStatus?.message && <span>{props.keyStatus.message}</span>}
        </div>
      </div>
      {shouldShowExistingKeyNotice(props.keyState, props.apiKeyInput, props.keyCheckResult) && (
        <p className="key-check-message valid">{t("settings.existingKey")}</p>
      )}
      <div className={keyButtonState.loading ? "secret-input-row checking" : "secret-input-row"}>
        <input
          aria-busy={keyButtonState.loading}
          autoComplete="off"
          onChange={(event) => props.onUpdateKey(event.currentTarget.value)}
          placeholder={t("settings.keyPlaceholder")}
          type="password"
          value={props.apiKeyInput}
        />
        <button
          aria-busy={keyButtonState.loading}
          className={keyButtonState.loading ? "secondary-action compact loading" : "secondary-action compact"}
          disabled={keyButtonState.disabled}
          onClick={props.onCheckKey}
          type="button"
        >
          {keyButtonState.loading ? <Loader2 className="button-spinner" size={16} /> : <CheckCircle2 size={16} />}
          {keyButtonState.loading ? t("settings.checkingKey") : t("settings.checkKey")}
        </button>
      </div>
      {props.keyCheckResult !== "idle" && (
        <p
          aria-live="polite"
          className={`key-check-message ${props.keyCheckResult}`}
          role={keyButtonState.loading ? "status" : undefined}
        >
          {keyButtonState.loading && <Loader2 className="status-spinner" size={16} />}
          {keyCheckMessage(t, props.keyCheckResult, props.keyCheckMessage)}
        </p>
      )}
    </div>
  );
}

function PromptBlock({ t }: { t: Translator }) {
  return (
    <div className="prompt-block">
      <span>{t("onboarding.aiPromptTitle")}</span>
      <code>{t("onboarding.aiPrompt")}</code>
    </div>
  );
}

function SettingsPane(props: { children: React.ReactNode; icon: React.ReactNode; title: string }) {
  return (
    <section className="settings-pane">
      <div className="settings-pane-title">
        <span>{props.icon}</span>
        <h2>{props.title}</h2>
      </div>
      {props.children}
    </section>
  );
}

function StatusRows({ rows }: { rows: Array<[string, string | undefined]> }) {
  return (
    <dl className="status-rows">
      {rows.map(([label, value]) => (
        <div key={label}>
          <dt>{label}</dt>
          <dd>{value || "--"}</dd>
        </div>
      ))}
    </dl>
  );
}

function StepIcon({ children }: { children: React.ReactNode }) {
  return <div className="step-icon">{children}</div>;
}

function keyCheckMessage(
  t: Translator,
  result: CurseForgeKeyCheckResult,
  detail: string,
): string {
  if (result === "empty") {
    return t("settings.keyCheckEmpty");
  }
  if (result === "checking") {
    return t("settings.keyCheckChecking");
  }
  if (result === "valid") {
    return t("settings.keyCheckValid");
  }
  if (result === "invalid") {
    return detail || t("settings.keyCheckInvalid");
  }
  return "";
}

function keyLabel(
  t: Translator,
  state: CurseForgeKeyState,
  notice: "idle" | "missing" | "saved" | "replaced" | "unavailable",
) {
  if (notice === "saved") {
    return t("settings.keySaved");
  }
  if (notice === "replaced") {
    return t("settings.keyReplaced");
  }
  if (state === "saved") {
    return t("settings.keySaved");
  }
  if (state === "unavailable") {
    return t("settings.keyUnavailable");
  }
  return t("settings.keyMissing");
}
