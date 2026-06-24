import { AlertTriangle, CheckCircle2, Loader2, ShieldCheck } from "lucide-react";
import type React from "react";

import type { Language } from "../i18n";
import {
  getCurseForgeKeyButtonState,
  shouldShowExistingKeyNotice,
  type CurseForgeKeyCheckResult,
  type CurseForgeKeyState,
} from "../onboarding";
import type { AgentStatus, CurseForgeCredentialStatus } from "../tauri";
import type { Translator } from "./types";

export function KeyForm(props: {
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

export function PromptBlock({
  endpoint,
  language,
  t,
}: {
  endpoint: string | null;
  language: Language;
  t: Translator;
}) {
  return (
    <div className="prompt-block">
      <span>{t("onboarding.aiPromptTitle")}</span>
      <code>{buildAgentHandoffPrompt(endpoint, language, t)}</code>
    </div>
  );
}

function buildAgentHandoffPrompt(endpoint: string | null, language: Language, t: Translator): string {
  const interfaceLanguage = language === "ru" ? "Russian" : "English";
  const endpointLine = endpoint ?? "[paste the Minecraft Pack Builder MCP endpoint from Settings]";
  return [
    "You are helping me use Minecraft Pack Builder.",
    "",
    "The app is running a local MCP server at:",
    endpointLine,
    "",
    'If you can configure MCP servers for this client, add this endpoint as "minecraft-pack-builder".',
    "If the client must be restarted or MCP config must be reloaded, tell me exactly what to do.",
    "After the MCP server is connected, verify that the Minecraft Pack Builder tools are available.",
    "Use those tools for modpack imports, scheme edits, validation, selections, materials, and export.",
    "Do not read or write the app data files directly unless I explicitly ask.",
    `Respond to me in the same language as the Minecraft Pack Builder interface: ${interfaceLanguage}.`,
  ].join("\n");
}

export function getAgentDisplay(agentStatus: AgentStatus | null, t: Translator) {
  if (!agentStatus?.serverRunning) {
    return {
      compact: t("status.aiDisconnected"),
      status: t("status.aiDisconnected"),
    };
  }

  if (agentStatus.activeClient) {
    return {
      compact: `${t("status.aiConnected")}: ${agentStatus.activeClient}`,
      status: t("status.aiConnected"),
    };
  }

  return {
    compact: t("status.aiServerRunning"),
    status: t("status.aiServerRunning"),
  };
}

export function SettingsPane(props: { children: React.ReactNode; icon: React.ReactNode; title: string }) {
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

export function StatusRows({ rows }: { rows: Array<[string, string | undefined]> }) {
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

export function StepIcon({ children }: { children: React.ReactNode }) {
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
