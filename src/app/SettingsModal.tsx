import { BellRing, FolderOpen, Languages, PlugZap, RefreshCcw, X } from "lucide-react";

import { languages, type Language } from "../i18n";
import type { SettingsSection } from "../onboarding";
import type { AgentStatus, AppDataPaths, PrismRootValidation, UpdateCheckResult } from "../tauri";
import { getAgentDisplay, PromptBlock, SettingsPane, StatusRows } from "./settingsControls";
import type { Translator } from "./types";

export function SettingsModal(props: {
  agentStatus: AgentStatus | null;
  automaticUpdateChecks: boolean;
  diagnosticsMessage: string;
  language: Language;
  onCheckUpdates: () => void;
  onChoosePrismRoot: () => void;
  onClose: () => void;
  onLanguageChange: (language: Language) => void;
  onOpenDataFolder: () => void;
  onRestartOnboarding: () => void;
  onSectionChange: (section: SettingsSection) => void;
  onToggleAutomaticUpdateChecks: (enabled: boolean) => void;
  paths: AppDataPaths | null;
  prismValidation: PrismRootValidation | null;
  section: SettingsSection;
  t: Translator;
  updateCheck: UpdateCheckResult | null;
  updateCheckBusy: boolean;
}) {
  const { t } = props;
  const agentDisplay = getAgentDisplay(props.agentStatus, t);
  const sections: Array<[SettingsSection, string]> = [
    ["ai", t("settings.aiIntegration")],
    ["prism", t("settings.prismLauncher")],
    ["language", t("settings.language")],
    ["data", t("settings.dataFolders")],
    ["updates", t("settings.updates")],
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

            {props.section === "prism" && (
              <SettingsPane icon={<FolderOpen size={23} />} title={t("settings.prismLauncher")}>
                <p>{t("onboarding.prismBody")}</p>
                <p className="subtle-line">{t("settings.prismFolderHint")}</p>
                <button className="secondary-action compact" onClick={props.onChoosePrismRoot} type="button">
                  <FolderOpen size={16} />
                  {t("settings.choosePrismRoot")}
                </button>
                <StatusRows
                  rows={[
                    [t("settings.prismRoot"), props.prismValidation?.rootPath ?? "--"],
                    [t("settings.status"), props.prismValidation?.message ?? t("settings.prismNotConfigured")],
                    [
                      t("settings.prismInstances"),
                      props.prismValidation ? String(props.prismValidation.instanceCount) : "--",
                    ],
                  ]}
                />
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

            {props.section === "updates" && (
              <SettingsPane icon={<BellRing size={23} />} title={t("settings.updates")}>
                <p>{t("settings.updateInstructions")}</p>
                <label className="settings-toggle-row">
                  <input
                    checked={props.automaticUpdateChecks}
                    onChange={(event) => props.onToggleAutomaticUpdateChecks(event.currentTarget.checked)}
                    type="checkbox"
                  />
                  <span>
                    <strong>{t("settings.automaticUpdates")}</strong>
                    <small>{t("settings.automaticUpdatesHelp")}</small>
                  </span>
                </label>
                <button
                  aria-busy={props.updateCheckBusy}
                  className={props.updateCheckBusy ? "secondary-action compact loading" : "secondary-action compact"}
                  disabled={props.updateCheckBusy}
                  onClick={props.onCheckUpdates}
                  type="button"
                >
                  <RefreshCcw className={props.updateCheckBusy ? "button-spinner" : undefined} size={16} />
                  {props.updateCheckBusy ? t("settings.checkingUpdates") : t("settings.checkUpdates")}
                </button>
                <StatusRows
                  rows={[
                    [t("settings.currentVersion"), props.updateCheck?.currentVersion ?? "0.1.0"],
                    [t("settings.latestVersion"), props.updateCheck?.latestVersion ?? t("settings.upToDate")],
                    [t("settings.updateStatus"), updateStatusText(props.updateCheck, t)],
                    [t("settings.releaseDate"), props.updateCheck?.date ?? "--"],
                  ]}
                />
                {props.updateCheck?.notes && (
                  <p className="key-check-message valid">{props.updateCheck.notes}</p>
                )}
                {props.updateCheck?.errorMessage && (
                  <p className="key-check-message invalid">{props.updateCheck.errorMessage}</p>
                )}
              </SettingsPane>
            )}
          </div>
        </div>
      </section>
    </div>
  );
}

function updateStatusText(updateCheck: UpdateCheckResult | null, t: Translator): string {
  if (!updateCheck) {
    return t("settings.updateNotChecked");
  }
  if (updateCheck.status === "available") {
    return t("settings.updateAvailable").replace("{version}", updateCheck.latestVersion ?? "");
  }
  if (updateCheck.status === "failed") {
    return t("settings.updateCheckFailed");
  }
  return t("settings.upToDate");
}
