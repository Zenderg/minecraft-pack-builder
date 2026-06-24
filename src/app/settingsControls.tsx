import type React from "react";

import type { Language } from "../i18n";
import type { AgentStatus } from "../tauri";
import type { Translator } from "./types";

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
    "Use those tools for PrismLauncher instances, scheme edits, validation, selections, materials, and export.",
    "Do not read or write the app data files directly unless I explicitly ask.",
    `Respond to me in the same language as the Minecraft Pack Builder interface: ${interfaceLanguage}.`,
  ].join("\n");
}

export function getAgentDisplay(agentStatus: AgentStatus | null, t: Translator) {
  if (!agentStatus?.serverRunning) {
    return {
      compact: t("status.aiDisconnected"),
      status: t("status.aiDisconnected"),
      tone: "warning",
    };
  }

  if (agentStatus.activeClient) {
    return {
      compact: t("status.aiConnected"),
      status: t("status.aiConnected"),
      tone: "connected",
    };
  }

  return {
    compact: t("status.aiServerRunning"),
    status: t("status.aiServerRunning"),
    tone: "warning",
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
