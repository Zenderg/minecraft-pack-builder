import {
  Bot,
  Box,
  PackagePlus,
  Settings,
} from "lucide-react";
import { useCallback, useEffect, useReducer, useState } from "react";

import { getInitialLanguage, type Language, translate } from "./i18n";
import { formatBackendError } from "./backendErrors";
import { ImportWizardWorkspace } from "./importWizard";
import {
  clampSidebarWidth,
  createEmptyLibraryDraft,
  getActiveLibrarySelection,
  getInitialExpandedModpackIds,
  getNextSelectionAfterSchemeDelete,
  sidebarWidthLimits,
  toggleExpandedModpack,
  type LibraryModpack,
  type LibraryScheme,
  type LibrarySelection,
  type SchemeDimensions,
} from "./library";
import {
  createInitialAppFlow,
  getCurseForgeKeyInputCheckResult,
  onboardingReducer,
  type CurseForgeKeyCheckResult,
} from "./onboarding";
import {
  cancelCurseForgeImport,
  checkCurseForgeApiKey,
  checkForUpdates,
  createScheme,
  deleteImportedModpack,
  deleteScheme,
  discoverAppPaths,
  exportScheme,
  getCurseForgeKeyStatus,
  getAiIntegrationStatus,
  listLibrary,
  listenToAgentEvents,
  listenToModpackImportProgress,
  listenToModpackImportStatus,
  openAppDataFolder,
  renameImportedModpack,
  renameScheme,
  retryModpackImport,
  saveCurseForgeApiKey,
  type AppDataPaths,
  type AgentStatus,
  type CurseForgeCredentialStatus,
  type ImportProgress,
  type UpdateCheckResult,
} from "./tauri";
import { chooseExportDestination, type ExportFormat } from "./exportDialog";
import { type StageOptionId } from "./renderViewer";
import { RightToolPanel } from "./RightToolPanel";
import { ViewerWorkspace, type ViewerToolContext } from "./ViewerWorkspace";
import { ImportJobDialog, importJobStageFromMessage } from "./app/ImportJobDialog";
import { ExportSchemeDialog } from "./app/ExportSchemeDialog";
import { LibraryActionDialog } from "./app/LibraryActionDialog";
import { LibraryTree } from "./app/LibraryTree";
import { OnboardingScreen } from "./app/OnboardingScreen";
import { SettingsModal } from "./app/SettingsModal";
import { getAgentDisplay } from "./app/settingsControls";
import type { ExportDialog, LibraryDialog } from "./app/types";
import "./styles.css";
import "./styles/appShell.css";
import "./styles/library.css";
import "./styles/viewer.css";
import "./styles/onboarding.css";
import "./styles/importJob.css";
import "./styles/settings.css";

const onboardingStorageKey = "mpb.onboardingComplete";
const automaticUpdateChecksStorageKey = "mpb.autoUpdateChecks";

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
  const [agentStatus, setAgentStatus] = useState<AgentStatus | null>(null);
  const [automaticUpdateChecks, setAutomaticUpdateChecks] = useState(
    () => localStorage.getItem(automaticUpdateChecksStorageKey) !== "false",
  );
  const [updateCheck, setUpdateCheck] = useState<UpdateCheckResult | null>(null);
  const [updateCheckBusy, setUpdateCheckBusy] = useState(false);
  const [apiKeyInput, setApiKeyInput] = useState("");
  const [keyCheckResult, setKeyCheckResult] = useState<CurseForgeKeyCheckResult>("idle");
  const [keyCheckMessage, setKeyCheckMessage] = useState("");
  const [isSavingKey, setIsSavingKey] = useState(false);
  const [diagnosticsMessage, setDiagnosticsMessage] = useState("");
  const [library, setLibrary] = useState<LibraryModpack[]>([]);
  const [librarySelection, setLibrarySelection] = useState<LibrarySelection | null>(null);
  const [libraryMessage, setLibraryMessage] = useState("");
  const [libraryDialog, setLibraryDialog] = useState<LibraryDialog | null>(null);
  const [exportDialog, setExportDialog] = useState<ExportDialog | null>(null);
  const [importJobModpackId, setImportJobModpackId] = useState<number | null>(null);
  const [importProgressByModpack, setImportProgressByModpack] = useState<Record<number, ImportProgress>>({});
  const [importLogsByModpack, setImportLogsByModpack] = useState<Record<number, string[]>>({});
  const [importStageByModpack, setImportStageByModpack] = useState<Record<number, string>>({});
  const [expandedModpackIds, setExpandedModpackIds] = useState<Set<number>>(new Set());
  const [sidebarWidth, setSidebarWidth] = useState<number>(sidebarWidthLimits.default);
  const [viewerStageId, setViewerStageId] = useState<StageOptionId | null>(null);
  const [viewerRevision, setViewerRevision] = useState(0);
  const [viewerToolContext, setViewerToolContext] = useState<ViewerToolContext | null>(null);
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
  const agentDisplay = getAgentDisplay(agentStatus, t);

  useEffect(() => {
    if (selectedScheme) {
      return;
    }
    setViewerStageId(null);
    setViewerToolContext(null);
  }, [selectedScheme]);

  useEffect(() => {
    discoverAppPaths()
      .then(setPaths)
      .catch((error: unknown) => setDiagnosticsMessage(formatBackendError(error)));
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
          message: formatBackendError(error),
          apiKey: null,
        });
        dispatch({ type: "keyUnavailable" });
      });
  }, []);

  useEffect(() => {
    refreshLibrary();
  }, []);

  useEffect(() => {
    if (!automaticUpdateChecks) {
      return;
    }
    void handleCheckUpdates({ silent: true });
  }, []);

  useEffect(() => {
    let active = true;
    let intervalId: number | null = null;
    const refreshAgentStatus = () => {
      getAiIntegrationStatus()
        .then((status) => {
          if (active) {
            setAgentStatus(status);
          }
        })
        .catch(() => {
          if (active) {
            setAgentStatus(null);
          }
        });
    };

    refreshAgentStatus();
    intervalId = window.setInterval(refreshAgentStatus, 2500);
    return () => {
      active = false;
      if (intervalId !== null) {
        window.clearInterval(intervalId);
      }
    };
  }, []);

  useEffect(() => {
    let unlisten: (() => void) | null = null;
    listenToAgentEvents((event) => {
      void getAiIntegrationStatus().then(setAgentStatus).catch(() => setAgentStatus(null));
      if ("libraryChanged" in event) {
        void refreshLibrary();
      }
      if ("schemeChanged" in event) {
        setViewerRevision((revision) => revision + 1);
        setLibraryMessage(t("library.autosaved"));
      }
    })
      .then((nextUnlisten) => {
        unlisten = nextUnlisten;
      })
      .catch((error: unknown) => setLibraryMessage(formatBackendError(error)));

    return () => {
      unlisten?.();
    };
  }, [language]);

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
      .catch((error: unknown) => setLibraryMessage(formatBackendError(error)));

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
      .catch((error: unknown) => setLibraryMessage(formatBackendError(error)));

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
      setLibraryMessage(formatBackendError(error));
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

  function handleLibraryDialogDimensionsChange(dimensions: SchemeDimensions) {
    setLibraryDialog((dialog) => {
      if (!dialog || dialog.kind !== "createScheme") {
        return dialog;
      }
      return { ...dialog, dimensions };
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
      setLibraryMessage(formatBackendError(error));
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
      setKeyCheckMessage(formatBackendError(error));
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
      setDiagnosticsMessage(formatBackendError(error));
    }
  }

  async function handleCheckUpdates(options: { silent?: boolean } = {}) {
    setUpdateCheckBusy(true);
    try {
      const result = await checkForUpdates();
      setUpdateCheck(result);
      if (!options.silent && result.status === "available") {
        setLibraryMessage(t("settings.updateAvailable").replace("{version}", result.latestVersion ?? ""));
      }
    } catch (error) {
      setUpdateCheck({
        status: "failed",
        currentVersion: "0.1.0",
        latestVersion: null,
        notes: null,
        date: null,
        errorMessage: formatBackendError(error),
      });
    } finally {
      setUpdateCheckBusy(false);
    }
  }

  function handleToggleAutomaticUpdateChecks(enabled: boolean) {
    setAutomaticUpdateChecks(enabled);
    localStorage.setItem(automaticUpdateChecksStorageKey, enabled ? "true" : "false");
  }

  function handleOpenExportDialog(scheme: LibraryScheme) {
    setExportDialog({
      scheme,
      format: "schem",
      destinationPath: "",
      isExporting: false,
    });
  }

  function handleExportFormatChange(format: ExportFormat) {
    setExportDialog((dialog) =>
      dialog ? { ...dialog, format, destinationPath: "" } : dialog,
    );
  }

  async function handleChooseExportDestination() {
    if (!exportDialog) {
      return;
    }

    const defaultFileName = `${exportDialog.scheme.name}.${exportDialog.format}`;
    try {
      const destinationPath = await chooseExportDestination({
        defaultFileName,
        format: exportDialog.format,
      });
      if (!destinationPath) {
        return;
      }
      setExportDialog((dialog) => (dialog ? { ...dialog, destinationPath } : dialog));
    } catch (error) {
      setLibraryMessage(`${t("export.failed")}: ${formatBackendError(error)}`);
    }
  }

  async function handleConfirmExport() {
    if (!exportDialog || !exportDialog.destinationPath || exportDialog.isExporting) {
      return;
    }

    const { scheme, format, destinationPath } = exportDialog;
    setExportDialog({ ...exportDialog, isExporting: true });
    try {
      const artifact = await exportScheme(scheme.id, format, destinationPath);
      setLibraryMessage(t("export.success").replace("{path}", artifact.path));
      setExportDialog(null);
    } catch (error) {
      setLibraryMessage(`${t("export.failed")}: ${formatBackendError(error)}`);
      setExportDialog((dialog) => (dialog ? { ...dialog, isExporting: false } : dialog));
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
            <span className={`brand-status ${agentDisplay.tone}`}>
              <Bot size={12} />
              {agentDisplay.compact}
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
            expandedModpackIds={expandedModpackIds}
            library={library}
            onCreateScheme={handleCreateScheme}
            onDeleteModpack={handleDeleteModpack}
            onDeleteScheme={handleDeleteScheme}
            onExportScheme={handleOpenExportDialog}
            onRenameModpack={handleRenameModpack}
            onRenameScheme={handleRenameScheme}
            onSelect={setLibrarySelection}
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
        {selectedScheme && (
          <div className="content-grid">
            <ViewerWorkspace
              modpack={selectedModpack}
              onStageChange={handleViewerStageChange}
              onToolContextChange={handleViewerToolContextChange}
              revision={viewerRevision}
              scheme={selectedScheme}
              selectedStageId={viewerStageId}
              t={t}
            />

            <RightToolPanel
              onStageChange={setViewerStageId}
              t={t}
              toolContext={viewerToolContext}
            />
          </div>
        )}
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
          agentStatus={agentStatus}
          automaticUpdateChecks={automaticUpdateChecks}
          onCheckKey={handleCheckKey}
          onCheckUpdates={() => void handleCheckUpdates()}
          onClose={() => dispatch({ type: "closeSettings" })}
          onLanguageChange={setLanguage}
          onOpenDataFolder={handleOpenDataFolder}
          onRestartOnboarding={restartOnboarding}
          onSectionChange={(section) => dispatch({ type: "openSettings", section })}
          onToggleAutomaticUpdateChecks={handleToggleAutomaticUpdateChecks}
          onUpdateKey={handleUpdateKeyInput}
          paths={paths}
          section={flow.settingsSection}
          t={t}
          updateCheck={updateCheck}
          updateCheckBusy={updateCheckBusy}
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
          onDimensionsChange={handleLibraryDialogDimensionsChange}
          onNameChange={handleLibraryDialogNameChange}
          t={t}
        />
      )}
      {exportDialog && (
        <ExportSchemeDialog
          dialog={exportDialog}
          onCancel={() => setExportDialog(null)}
          onChoosePath={handleChooseExportDestination}
          onConfirm={handleConfirmExport}
          onFormatChange={handleExportFormatChange}
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
              appendImportLog(importJobModpack.id, formatBackendError(error));
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
              appendImportLog(importJobModpack.id, formatBackendError(error));
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
