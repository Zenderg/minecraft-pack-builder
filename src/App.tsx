import { open } from "@tauri-apps/plugin-dialog";
import { Bot, Box, Settings } from "lucide-react";
import { useCallback, useEffect, useReducer, useState } from "react";

import { formatBackendError } from "./backendErrors";
import { chooseExportDestination, type ExportFormat } from "./exportDialog";
import { getInitialLanguage, type Language, translate } from "./i18n";
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
import { createInitialAppFlow, onboardingReducer, type PrismRootState } from "./onboarding";
import { RightToolPanel } from "./RightToolPanel";
import { type StageOptionId } from "./renderViewer";
import {
  checkForUpdates,
  createScheme,
  deleteScheme,
  discoverAppPaths,
  discoverPrismLauncherRoots,
  exportScheme,
  getAiIntegrationStatus,
  confirmPrismInstanceRelink,
  listLibrary,
  listPrismRelinkCandidates,
  listenToAgentEvents,
  listenToLibraryChanged,
  openAppDataFolder,
  renameScheme,
  selectPrismLauncherRoot,
  type AgentStatus,
  type AppDataPaths,
  type PrismRelinkCandidate,
  type PrismRootValidation,
  type UpdateCheckResult,
} from "./tauri";
import { ExportSchemeDialog } from "./app/ExportSchemeDialog";
import { LibraryActionDialog } from "./app/LibraryActionDialog";
import { LibraryTree } from "./app/LibraryTree";
import { OnboardingScreen } from "./app/OnboardingScreen";
import { SettingsModal } from "./app/SettingsModal";
import { getAgentDisplay } from "./app/settingsControls";
import type { ExportDialog, LibraryDialog } from "./app/types";
import { ViewerWorkspace, type ViewerToolContext } from "./ViewerWorkspace";
import "./styles.css";
import "./styles/appShell.css";
import "./styles/library.css";
import "./styles/viewer.css";
import "./styles/onboarding.css";
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
  const [agentStatus, setAgentStatus] = useState<AgentStatus | null>(null);
  const [automaticUpdateChecks, setAutomaticUpdateChecks] = useState(
    () => localStorage.getItem(automaticUpdateChecksStorageKey) !== "false",
  );
  const [updateCheck, setUpdateCheck] = useState<UpdateCheckResult | null>(null);
  const [updateCheckBusy, setUpdateCheckBusy] = useState(false);
  const [diagnosticsMessage, setDiagnosticsMessage] = useState("");
  const [prismValidation, setPrismValidation] = useState<PrismRootValidation | null>(null);
  const [library, setLibrary] = useState<LibraryModpack[]>([]);
  const [librarySelection, setLibrarySelection] = useState<LibrarySelection | null>(null);
  const [libraryMessage, setLibraryMessage] = useState("");
  const [libraryDialog, setLibraryDialog] = useState<LibraryDialog | null>(null);
  const [exportDialog, setExportDialog] = useState<ExportDialog | null>(null);
  const [relinkCandidate, setRelinkCandidate] = useState<PrismRelinkCandidate | null>(null);
  const [dismissedRelinks, setDismissedRelinks] = useState<Set<string>>(new Set());
  const [expandedModpackIds, setExpandedModpackIds] = useState<Set<number>>(new Set());
  const [sidebarWidth, setSidebarWidth] = useState<number>(sidebarWidthLimits.default);
  const [viewerStageId, setViewerStageId] = useState<StageOptionId | null>(null);
  const [viewerRevision, setViewerRevision] = useState(0);
  const [viewerToolContext, setViewerToolContext] = useState<ViewerToolContext | null>(null);

  const t = (key: Parameters<typeof translate>[1]) => translate(language, key);
  const selectedModpack =
    library.find((modpack) => modpack.id === librarySelection?.modpackId) ?? null;
  const selectedScheme =
    selectedModpack?.schemes.find((scheme) => scheme.id === librarySelection?.schemeId) ?? null;
  const agentDisplay = getAgentDisplay(agentStatus, t);

  const handleViewerStageChange = useCallback((stageId: StageOptionId | null) => {
    setViewerStageId(stageId);
  }, []);
  const handleViewerToolContextChange = useCallback((context: ViewerToolContext | null) => {
    setViewerToolContext(context);
  }, []);

  useEffect(() => {
    if (selectedScheme && selectedModpack?.status === "ready") {
      return;
    }
    setViewerStageId(null);
    setViewerToolContext(null);
  }, [selectedScheme, selectedModpack?.status]);

  useEffect(() => {
    discoverAppPaths()
      .then(setPaths)
      .catch((error: unknown) => setDiagnosticsMessage(formatBackendError(error)));
  }, []);

  useEffect(() => {
    discoverPrismLauncherRoots()
      .then(async (roots) => {
        const validRoot = roots.find((root) => root.valid);
        if (!validRoot) {
          dispatch({ type: "setPrismRootState", state: "invalid" });
          return;
        }
        const selection = await selectPrismLauncherRoot(validRoot.rootPath);
        setPrismValidation(selection.validation);
        applyLibrary(selection.library, librarySelection);
        showRelinkCandidate(selection.relinkCandidates);
        dispatch({ type: "setPrismRootState", state: "valid" });
      })
      .catch((error: unknown) => {
        setLibraryMessage(formatBackendError(error));
        dispatch({ type: "setPrismRootState", state: "invalid" });
      });
  }, []);

  useEffect(() => {
    void refreshLibrary();
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

  useEffect(() => {
    let unlisten: (() => void) | null = null;
    listenToLibraryChanged((event) => {
      applyLibrary(event.library, librarySelection);
    })
      .then((nextUnlisten) => {
        unlisten = nextUnlisten;
      })
      .catch((error: unknown) => setLibraryMessage(formatBackendError(error)));

    return () => {
      unlisten?.();
    };
  }, [librarySelection]);

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
      const candidates = await listPrismRelinkCandidates();
      showRelinkCandidate(candidates);
      setLibraryMessage("");
    } catch (error) {
      setLibraryMessage(formatBackendError(error));
    }
  }

  async function handleChoosePrismRoot() {
    try {
      const selected = await open({ directory: true, multiple: false });
      if (typeof selected !== "string") {
        return;
      }
      const selection = await selectPrismLauncherRoot(selected);
      setPrismValidation(selection.validation);
      dispatch({
        type: "setPrismRootState",
        state: selection.validation.valid ? "valid" : "invalid",
      });
      applyLibrary(selection.library, librarySelection);
      showRelinkCandidate(selection.relinkCandidates);
      setLibraryMessage(selection.validation.message);
    } catch (error) {
      setLibraryMessage(formatBackendError(error));
      dispatch({ type: "setPrismRootState", state: "invalid" });
    }
  }

  function showRelinkCandidate(candidates: PrismRelinkCandidate[]) {
    const candidate = candidates.find(
      (item) => !dismissedRelinks.has(item.discoveredIdentityFingerprint),
    );
    setRelinkCandidate((current) => current ?? candidate ?? null);
  }

  async function handleConfirmRelink() {
    if (!relinkCandidate) {
      return;
    }
    try {
      const nextLibrary = await confirmPrismInstanceRelink(
        relinkCandidate.existingId,
        relinkCandidate.discoveredIdentityFingerprint,
      );
      applyLibrary(nextLibrary, librarySelection);
      setDismissedRelinks((current) => new Set(current).add(relinkCandidate.discoveredIdentityFingerprint));
      setRelinkCandidate(null);
      setLibraryMessage(t("library.autosaved"));
    } catch (error) {
      setLibraryMessage(formatBackendError(error));
    }
  }

  function handleSkipRelink() {
    if (!relinkCandidate) {
      return;
    }
    setDismissedRelinks((current) => new Set(current).add(relinkCandidate.discoveredIdentityFingerprint));
    setRelinkCandidate(null);
  }

  function handleCreateScheme(modpackId: number) {
    const modpack = library.find((item) => item.id === modpackId);
    if (modpack?.status !== "ready") {
      setLibraryMessage(t("library.instanceNotReady"));
      return;
    }
    const draft = createEmptyLibraryDraft(modpackId);
    setLibraryDialog({ kind: "createScheme", ...draft });
  }

  function handleRenameScheme(scheme: LibraryScheme) {
    setLibraryDialog({ kind: "renameScheme", scheme, name: scheme.name });
  }

  function handleDeleteScheme(scheme: LibraryScheme) {
    setLibraryDialog({ kind: "deleteScheme", scheme });
  }

  function handleShowModpackInfo(modpack: LibraryModpack) {
    setLibraryDialog({ kind: "infoModpack", modpack });
  }

  function handleLibraryDialogNameChange(name: string) {
    setLibraryDialog((dialog) => {
      if (!dialog || dialog.kind === "deleteScheme" || dialog.kind === "infoModpack") {
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
    dispatch({ type: "restartOnboarding" });
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
        language={language}
        onBack={() => dispatch({ type: "previousOnboardingStep" })}
        onChoosePrismRoot={handleChoosePrismRoot}
        onFinish={completeOnboarding}
        onLanguageChange={setLanguage}
        onNextAi={() => dispatch({ type: "setOnboardingStep", step: "prism" })}
        onNextLanguage={() => dispatch({ type: "setOnboardingStep", step: "ai" })}
        onSkip={skipOnboarding}
        prismRoot={flow.prismRoot as PrismRootState}
        prismValidation={prismValidation}
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

        <section className="library-panel">
          <div className="panel-title">
            <span>{t("workspace.library")}</span>
          </div>
          <LibraryTree
            expandedModpackIds={expandedModpackIds}
            library={library}
            onCreateScheme={handleCreateScheme}
            onDeleteScheme={handleDeleteScheme}
            onExportScheme={handleOpenExportDialog}
            onRenameScheme={handleRenameScheme}
            onSelect={setLibrarySelection}
            onShowModpackInfo={handleShowModpackInfo}
            onToggleModpack={handleToggleModpack}
            selected={librarySelection}
            t={t}
          />
          {libraryMessage && <p className="library-message">{libraryMessage}</p>}
        </section>

        <button
          className="settings-link"
          onClick={() => dispatch({ type: "openSettings", section: "prism" })}
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
        {selectedScheme && selectedModpack?.status === "ready" && (
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
        {selectedScheme && selectedModpack?.status !== "ready" && (
          <div className="viewer-empty-state">
            <h2>{t("viewer.blockedTitle")}</h2>
            <p>{selectedModpack?.statusMessage ?? t("viewer.blockedBody")}</p>
          </div>
        )}
      </section>
      {flow.settingsModalOpen && (
        <SettingsModal
          agentStatus={agentStatus}
          automaticUpdateChecks={automaticUpdateChecks}
          diagnosticsMessage={diagnosticsMessage}
          language={language}
          onCheckUpdates={() => void handleCheckUpdates()}
          onChoosePrismRoot={handleChoosePrismRoot}
          onClose={() => dispatch({ type: "closeSettings" })}
          onLanguageChange={setLanguage}
          onOpenDataFolder={handleOpenDataFolder}
          onRestartOnboarding={restartOnboarding}
          onSectionChange={(section) => dispatch({ type: "openSettings", section })}
          onToggleAutomaticUpdateChecks={handleToggleAutomaticUpdateChecks}
          paths={paths}
          prismValidation={prismValidation}
          section={flow.settingsSection}
          t={t}
          updateCheck={updateCheck}
          updateCheckBusy={updateCheckBusy}
        />
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
      {relinkCandidate && (
        <div className="modal-backdrop" role="presentation">
          <section className="settings-modal relink-modal" aria-label={t("library.relinkTitle")} role="dialog">
            <header className="settings-modal-header">
              <div>
                <h2>{t("library.relinkTitle")}</h2>
                <span>{t("library.relinkBody")}</span>
              </div>
            </header>
            <dl className="status-rows">
              <div>
                <dt>{t("library.relinkExisting")}</dt>
                <dd>
                  {relinkCandidate.existingDisplayName}
                  <br />
                  {relinkCandidate.existingInstancePath}
                </dd>
              </div>
              <div>
                <dt>{t("library.relinkDiscovered")}</dt>
                <dd>
                  {relinkCandidate.discoveredDisplayName}
                  <br />
                  {relinkCandidate.discoveredInstancePath}
                </dd>
              </div>
              <div>
                <dt>{t("library.minecraftVersion")}</dt>
                <dd>{relinkCandidate.minecraftVersion ?? t("library.unknown")}</dd>
              </div>
              <div>
                <dt>{t("library.loader")}</dt>
                <dd>{relinkCandidate.loader ?? t("library.unknown")}</dd>
              </div>
            </dl>
            <footer className="dialog-actions">
              <button className="secondary-action compact" onClick={handleSkipRelink} type="button">
                {t("library.relinkSkip")}
              </button>
              <button className="primary-action compact" onClick={() => void handleConfirmRelink()} type="button">
                {t("library.relinkConfirm")}
              </button>
            </footer>
          </section>
        </div>
      )}
    </main>
  );
}
