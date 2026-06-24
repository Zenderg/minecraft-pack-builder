import { describe, expect, it } from "vitest";

import { translate } from "../i18n";
import { getAgentDisplay } from "./settingsControls";

describe("getAgentDisplay", () => {
  const t = (key: Parameters<typeof translate>[1]) => translate("en", key);

  it("keeps connected sidebar status compact and green-toned", () => {
    const display = getAgentDisplay(
      {
        serverRunning: true,
        transport: "streamable-http",
        endpoint: "http://127.0.0.1:7777/mcp",
        protocolVersion: "2025-06-18",
        activeClient: "codex-mcp-client",
        toolCount: 19,
      },
      t,
    );

    expect(display.compact).toBe("AI connected");
    expect(display.compact).not.toContain("codex-mcp-client");
    expect(display.status).toBe("AI connected");
    expect(display.tone).toBe("connected");
  });

  it("keeps disconnected and waiting states warning-toned", () => {
    expect(getAgentDisplay(null, t).tone).toBe("warning");
    expect(
      getAgentDisplay(
        {
          serverRunning: true,
          transport: "streamable-http",
          endpoint: "http://127.0.0.1:7777/mcp",
          protocolVersion: "2025-06-18",
          activeClient: null,
          toolCount: 19,
        },
        t,
      ).tone,
    ).toBe("warning");
  });
});
