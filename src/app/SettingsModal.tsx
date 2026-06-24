import { EyeOff, FolderOpen, KeyRound, Languages, PlugZap, RefreshCcw, X } from "lucide-react";

import { languages, type Language } from "../i18n";
import type {
  CurseForgeKeyCheckResult,
  CurseForgeKeyState,
  SettingsSection,
} from "../onboarding";
import type { AgentStatus, AppDataPaths, CurseForgeCredentialStatus } from "../tauri";
import { getAgentDisplay, KeyForm, PromptBlock, SettingsPane, StatusRows } from "./settingsControls";
import type { Translator } from "./types";

export function SettingsModal(props: {
  agentStatus: AgentStatus | null;
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
  const agentDisplay = getAgentDisplay(props.agentStatus, t);
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
              <PromptBlock endpoint={props.agentStatus?.endpoint ?? null} language={props.language} t={t} />
              <StatusRows
                rows={[
                  [t("settings.status"), agentDisplay.status],
                  [t("settings.activeClient"), props.agentStatus?.activeClient ?? t("settings.noActiveClient")],
                  [t("settings.protocol"), props.agentStatus?.protocolVersion ?? "MCP"],
                  [t("settings.tools"), props.agentStatus ? String(props.agentStatus.toolCount) : "0"],
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
