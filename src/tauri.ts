import { invoke } from "@tauri-apps/api/core";

import type { PatchAction, PatcherInstance } from "./patcher/patcherState";

export type PrismRootValidation = {
  rootPath: string;
  valid: boolean;
  message: string;
  instanceCount: number;
};

export type PatcherOperationResult = {
  status: string;
  steps: Array<{
    label: string;
    status: string;
  }>;
};

function isTauriRuntime(): boolean {
  return "__TAURI_INTERNALS__" in window;
}

let browserPatcherInstances: PatcherInstance[] = [
  {
    instanceId: "browser-demo",
    displayName: "Local Prism instance",
    instancePath: "/PrismLauncher/instances/Local Prism instance",
    minecraftDir: "/PrismLauncher/instances/Local Prism instance/.minecraft",
    minecraftVersion: "1.20.1",
    loader: "Fabric",
    loaderVersion: "0.16.9",
    patchStatus: "notPatched",
    patchReason: null,
    knowledgeStatus: "unavailable",
    knowledgePackId: null,
    knowledgeReason: "Browser preview does not evaluate Prism fingerprints.",
  },
];

export async function discoverPrismLauncherRoots(): Promise<PrismRootValidation[]> {
  if (!isTauriRuntime()) {
    return [
      {
        rootPath: "/PrismLauncher",
        valid: true,
        message: "Browser preview uses a simulated PrismLauncher root.",
        instanceCount: browserPatcherInstances.length,
      },
    ];
  }

  return invoke<PrismRootValidation[]>("discover_prism_launcher_roots");
}

export async function validatePrismLauncherRoot(rootPath: string): Promise<PrismRootValidation> {
  if (!isTauriRuntime()) {
    return {
      rootPath,
      valid: true,
      message: "Browser preview uses a simulated PrismLauncher root.",
      instanceCount: browserPatcherInstances.length,
    };
  }

  return invoke<PrismRootValidation>("validate_prism_launcher_root", { rootPath });
}

export async function listPatcherInstances(rootPath: string): Promise<PatcherInstance[]> {
  if (!isTauriRuntime()) {
    return structuredClone(browserPatcherInstances);
  }

  return invoke<PatcherInstance[]>("list_patcher_instances", { rootPath });
}

export async function patchPrismInstance(
  instancePath: string,
  action: PatchAction,
): Promise<PatcherOperationResult> {
  if (!isTauriRuntime()) {
    browserPatcherInstances = browserPatcherInstances.map((instance) =>
      instance.instancePath === instancePath
        ? { ...instance, patchStatus: "patched", patchReason: null }
        : instance,
    );
    return {
      status: "patched",
      steps: [
        { label: "Prepared instance mpb folders", status: "done" },
        { label: "Installed MPB Minecraft mod", status: "done" },
        { label: "Wrote MPB patch manifest", status: "done" },
      ],
    };
  }

  return invoke<PatcherOperationResult>("patch_prism_instance", { instancePath, action });
}

export async function removePrismInstancePatch(
  instancePath: string,
  deleteSchemes: boolean,
): Promise<PatcherOperationResult> {
  if (!isTauriRuntime()) {
    browserPatcherInstances = browserPatcherInstances.map((instance) =>
      instance.instancePath === instancePath
        ? { ...instance, patchStatus: "notPatched", patchReason: null }
        : instance,
    );
    return {
      status: "notPatched",
      steps: [{ label: deleteSchemes ? "Removed MPB data" : "Removed MPB patch", status: "done" }],
    };
  }

  return invoke<PatcherOperationResult>("remove_prism_instance_patch", {
    instancePath,
    deleteSchemes,
  });
}
