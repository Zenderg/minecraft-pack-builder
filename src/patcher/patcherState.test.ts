import { describe, expect, it } from "vitest";

import {
  getNextStepText,
  getPatchStatusAction,
  patcherReducer,
  type PatcherState,
} from "./patcherState";

describe("patcher state", () => {
  it("maps patch statuses to the allowed primary action without guessing unsupported cases", () => {
    expect(getPatchStatusAction("notPatched")).toEqual({ action: "apply", labelKey: "patcher.apply" });
    expect(getPatchStatusAction("needsUpdate")).toEqual({ action: "update", labelKey: "patcher.update" });
    expect(getPatchStatusAction("needsRepair")).toEqual({ action: "repair", labelKey: "patcher.repair" });
    expect(getPatchStatusAction("patched")).toBeNull();
    expect(getPatchStatusAction("unsupported")).toBeNull();
    expect(getPatchStatusAction("conflict")).toBeNull();
    expect(getPatchStatusAction("instanceRunning")).toBeNull();
  });

  it("returns localized next steps after patch success", () => {
    expect(getNextStepText("en", "installed")).toContain("curated knowledge is available");
    expect(getNextStepText("en", "unavailable")).toContain("curated modpack knowledge is unsupported");
    expect(getNextStepText("en", "available")).toContain("curated knowledge is available");
    expect(getNextStepText("en", "installed")).toContain("/mpb");
    expect(getNextStepText("ru", "installed")).toContain("кураторская база знаний доступна");
    expect(getNextStepText("ru", "unavailable")).toContain("кураторская база знаний не поддерживается");
    expect(getNextStepText("ru", "available")).toContain("кураторская база знаний доступна");
    expect(getNextStepText("ru", "installed")).toContain("/mpb");
  });

  it("tracks operation progress and refreshes instances after completion", () => {
    const initial: PatcherState = {
      rootPath: "/Prism",
      instances: [],
      selectedInstancePath: null,
      loading: false,
      message: "",
      operation: null,
    };

    const started = patcherReducer(initial, {
      type: "operationStarted",
      instancePath: "/Prism/instances/Pack",
      action: "apply",
    });
    expect(started.operation?.busy).toBe(true);

    const completed = patcherReducer(started, {
      type: "operationCompleted",
      message: "Patched",
      instances: [
        {
          instanceId: "Pack",
          displayName: "Pack",
          instancePath: "/Prism/instances/Pack",
          minecraftDir: "/Prism/instances/Pack/.minecraft",
          minecraftVersion: "1.20.1",
          loader: "Fabric",
          loaderVersion: "0.16.9",
          patchStatus: "patched",
          patchReason: null,
          knowledgeStatus: "installed",
          knowledgePackId: "fixture-minimal",
          knowledgeReason: null,
        },
      ],
    });

    expect(completed.operation).toBeNull();
    expect(completed.instances[0].patchStatus).toBe("patched");
    expect(completed.instances[0].knowledgeStatus).toBe("installed");
    expect(completed.message).toBe("Patched");
  });
});
