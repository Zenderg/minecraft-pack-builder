import {
  AlertTriangle,
  ArrowLeft,
  ArrowRight,
  Bot,
  Box,
  CheckCircle2,
  ChevronDown,
  Database,
  EyeOff,
  FolderOpen,
  Globe2,
  KeyRound,
  Layers3,
  Languages,
  PackagePlus,
  PlugZap,
  RefreshCcw,
  Settings,
  ShieldCheck,
  X,
} from "lucide-react";
import { useEffect, useReducer, useState } from "react";

import { getInitialLanguage, languages, type Language, translate } from "./i18n";
import {
  canFinishOnboardingWithKey,
  createInitialAppFlow,
  getCurseForgeKeyCheckResult,
  onboardingReducer,
  type CurseForgeKeyCheckResult,
  type CurseForgeKeyState,
  type SettingsSection,
} from "./onboarding";
import {
  discoverAppPaths,
  getCurseForgeKeyStatus,
  openAppDataFolder,
  saveCurseForgeApiKey,
  type AppDataPaths,
  type CurseForgeCredentialStatus,
} from "./tauri";
import "./styles.css";

const sampleMaterials = [
  { name: "minecraft:stone_bricks", count: 284 },
  { name: "thermal:machine_frame", count: 24 },
  { name: "create:andesite_casing", count: 48 },
];

const onboardingStorageKey = "mpb.onboardingComplete";

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
  const [keyCheckResult, setKeyCheckResult] = useState<CurseForgeKeyCheckResult | "idle">("idle");
  const [isSavingKey, setIsSavingKey] = useState(false);
  const [diagnosticsMessage, setDiagnosticsMessage] = useState("");

  const t = (key: Parameters<typeof translate>[1]) => translate(language, key);

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
    dispatch({ type: "restartOnboarding" });
  }

  function handleAddModpack() {
    dispatch({ type: "startAddModpack" });
  }

  async function handleCheckAndSaveKey() {
    const checkResult = getCurseForgeKeyCheckResult(apiKeyInput);
    setKeyCheckResult(checkResult);
    if (checkResult === "empty") {
      return;
    }

    setIsSavingKey(true);
    try {
      const status = await saveCurseForgeApiKey(apiKeyInput);
      setKeyStatus(status);
      setApiKeyInput("");
      if (status.state === "saved") {
        setKeyCheckResult("formatReady");
        dispatch({ type: "keySaved" });
      } else {
        setKeyCheckResult("idle");
        dispatch({ type: "keyUnavailable" });
      }
    } catch (error) {
      setKeyStatus({
        state: "unavailable",
        backend: keyStatus?.backend ?? "OS secure credential storage",
        message: String(error),
        apiKey: null,
      });
      dispatch({ type: "keyUnavailable" });
    } finally {
      setIsSavingKey(false);
    }
  }

  function handleUpdateKeyInput(value: string) {
    setApiKeyInput(value);
    setKeyCheckResult("idle");
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
    <main className="app-shell antialiased">
      <aside className="sidebar" aria-label={t("workspace.library")}>
        <div className="brand">
          <div className="brand-mark">
            <Box size={18} />
          </div>
          <div>
            <h1>{t("app.title")}</h1>
          </div>
        </div>

        <button className="primary-action" onClick={handleAddModpack} type="button">
          <PackagePlus size={17} />
          <span>{t("workspace.addModpack")}</span>
        </button>

        <section className="library-panel">
          <div className="panel-title">
            <span>{t("workspace.library")}</span>
            <ChevronDown size={16} />
          </div>
          <div className="tree-item active">
            <Database size={15} />
            <span>{t("library.modpack")}</span>
          </div>
          <div className="tree-item nested selected">
            <Layers3 size={15} />
            <span>{t("library.scheme")}</span>
          </div>
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

      <section className="workspace">
        <header className="status-strip">
          <div className="status-group">
            <span className="status-pill warning">
              <Bot size={15} />
              {t("status.aiDisconnected")}
            </span>
          </div>
          <div className="language-switch" aria-label={t("settings.language")}>
            <Globe2 size={16} />
            {languages.map((option) => (
              <button
                className={option === language ? "active" : ""}
                key={option}
                onClick={() => setLanguage(option)}
                type="button"
              >
                {option.toUpperCase()}
              </button>
            ))}
          </div>
        </header>

        <div className="content-grid">
          {flow.screen === "importWizard" ? (
            <ImportReadyWorkspace t={t} />
          ) : (
            <ViewerWorkspace t={t} />
          )}

          <aside className="right-rail" aria-label={t("workspace.review")}>
            <section className="tool-panel">
              <div className="section-heading">
                <span>{t("workspace.review")}</span>
                <strong>{t("review.pending")}: 0</strong>
              </div>
              <div className="selection-box">
                <span>{t("review.selection")}</span>
                <code>x: --, y: --, z: --</code>
              </div>
              <div className="empty-list">{t("review.changeRequests")}</div>
            </section>

            <section className="tool-panel">
              <div className="section-heading">
                <span>{t("workspace.materials")}</span>
                <strong>{t("materials.total")}: 356</strong>
              </div>
              <ul className="materials-list">
                {sampleMaterials.map((material) => (
                  <li key={material.name}>
                    <span>{material.name}</span>
                    <strong>{material.count}</strong>
                  </li>
                ))}
              </ul>
            </section>

          </aside>
        </div>
      </section>
      {flow.settingsModalOpen && (
        <SettingsModal
          apiKeyInput={apiKeyInput}
          diagnosticsMessage={diagnosticsMessage}
          isSavingKey={isSavingKey}
          keyCheckResult={keyCheckResult}
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
    </main>
  );
}

type Translator = (key: Parameters<typeof translate>[1]) => string;

function OnboardingScreen(props: {
  apiKeyInput: string;
  isSavingKey: boolean;
  keyState: CurseForgeKeyState;
  keyStatus: CurseForgeCredentialStatus | null;
  keyCheckResult: CurseForgeKeyCheckResult | "idle";
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

function ViewerWorkspace({ t }: { t: Translator }) {
  return (
    <section className="viewer-region" aria-label={t("workspace.viewer")}>
      <div className="section-heading">
        <span>{t("workspace.viewer")}</span>
        <strong>64 x 64 x 64</strong>
      </div>
      <div className="viewer-canvas">
        <div className="grid-floor" />
        <div className="block-stack stack-a" />
        <div className="block-stack stack-b" />
        <div className="block-stack stack-c" />
        <div className="viewer-empty">
          <h2>{t("viewer.emptyTitle")}</h2>
          <p>{t("viewer.emptyBody")}</p>
        </div>
      </div>
    </section>
  );
}

function ImportReadyWorkspace({ t }: { t: Translator }) {
  return (
    <section className="viewer-region settings-workspace" aria-label={t("workspace.addModpack")}>
      <div className="section-heading">
        <span>{t("workspace.addModpack")}</span>
        <strong>{t("settings.curseforgeKey")}</strong>
      </div>
      <div className="empty-state-panel">
        <PackagePlus size={34} />
        <h2>{t("import.readyTitle")}</h2>
        <p>{t("import.readyBody")}</p>
      </div>
    </section>
  );
}

function SettingsModal(props: {
  apiKeyInput: string;
  diagnosticsMessage: string;
  isSavingKey: boolean;
  keyCheckResult: CurseForgeKeyCheckResult | "idle";
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
  keyCheckResult: CurseForgeKeyCheckResult | "idle";
  keyState: CurseForgeKeyState;
  keyStatus: CurseForgeCredentialStatus | null;
  onCheckKey: () => void;
  onUpdateKey: (value: string) => void;
  t: Translator;
}) {
  const { t } = props;
  return (
    <div className="key-form">
      <div className={`key-status ${props.keyState}`}>
        {props.keyState === "saved" ? <ShieldCheck size={18} /> : <AlertTriangle size={18} />}
        <div>
          <strong>{keyLabel(t, props.keyState, "idle")}</strong>
          <span>
            {t("settings.backend")}: {props.keyStatus?.backend ?? "OS secure credential storage"}
          </span>
          {props.keyStatus?.message && <span>{props.keyStatus.message}</span>}
        </div>
      </div>
      {props.keyState === "saved" && <p className="key-check-message formatReady">{t("settings.existingKey")}</p>}
      <div className="secret-input-row">
        <input
          autoComplete="off"
          onChange={(event) => props.onUpdateKey(event.currentTarget.value)}
          placeholder={t("settings.keyPlaceholder")}
          type="password"
          value={props.apiKeyInput}
        />
        <button
          className="secondary-action compact"
          disabled={props.isSavingKey}
          onClick={props.onCheckKey}
          type="button"
        >
          <CheckCircle2 size={16} />
          {t("settings.checkKey")}
        </button>
      </div>
      {props.keyCheckResult !== "idle" && (
        <p className={`key-check-message ${props.keyCheckResult}`}>
          {props.keyCheckResult === "empty" ? t("settings.keyCheckEmpty") : t("settings.keyCheckReady")}
          <span>{t("settings.keyCheckNote")}</span>
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
