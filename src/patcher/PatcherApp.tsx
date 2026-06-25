import { open } from "@tauri-apps/plugin-dialog";
import { CheckCircle2, FolderOpen, RefreshCw, ShieldAlert, Wrench, XCircle } from "lucide-react";
import { useEffect, useMemo, useReducer, useState } from "react";

import { formatBackendError } from "../backendErrors";
import { getInitialLanguage, type Language } from "../i18n";
import {
  discoverPrismLauncherRoots,
  listPatcherInstances,
  patchPrismInstance,
  removePrismInstancePatch,
} from "../tauri";
import {
  createInitialPatcherState,
  getNextStepText,
  getPatchStatusAction,
  patcherReducer,
  type PatchStatus,
  type PatcherInstance,
} from "./patcherState";
import "./patcher.css";

const copy = {
  en: {
    title: "MPB Patcher",
    subtitle: "Patch PrismLauncher instances for the MPB Minecraft mod.",
    chooseRoot: "Choose Launcher Root",
    refresh: "Refresh",
    detectedRoot: "Launcher Root",
    instances: "Instances",
    details: "Patch details",
    noInstances: "No PrismLauncher instances found.",
    minecraft: "Minecraft",
    loader: "Loader",
    path: "Path",
    reason: "Reason",
    next: "Next step",
    remove: "Remove patch",
    deleteSchemes: "Delete MPB schemes from this instance",
    apply: "Apply patch",
    update: "Update patch",
    repair: "Repair patch",
    removing: "Removing patch...",
    working: "Patching...",
    desktopOnly: "Choose a PrismLauncher Launcher Root in the desktop app.",
  },
  ru: {
    title: "MPB Patcher",
    subtitle: "Патчит инстансы PrismLauncher для Minecraft-мода MPB.",
    chooseRoot: "Выбрать Launcher Root",
    refresh: "Обновить",
    detectedRoot: "Launcher Root",
    instances: "Инстансы",
    details: "Детали патча",
    noInstances: "Инстансы PrismLauncher не найдены.",
    minecraft: "Minecraft",
    loader: "Лоадер",
    path: "Путь",
    reason: "Причина",
    next: "Следующий шаг",
    remove: "Удалить патч",
    deleteSchemes: "Удалить MPB-схемы из этого инстанса",
    apply: "Применить патч",
    update: "Обновить патч",
    repair: "Восстановить патч",
    removing: "Удаляю патч...",
    working: "Патчу...",
    desktopOnly: "Выберите PrismLauncher Launcher Root в desktop-приложении.",
  },
} satisfies Record<Language, Record<string, string>>;

export function PatcherApp() {
  const [language] = useState<Language>(() => getInitialLanguage());
  const [state, dispatch] = useReducer(patcherReducer, undefined, createInitialPatcherState);
  const [deleteSchemes, setDeleteSchemes] = useState(false);
  const t = copy[language];

  const selectedInstance = useMemo(
    () =>
      state.instances.find((instance) => instance.instancePath === state.selectedInstancePath) ??
      null,
    [state.instances, state.selectedInstancePath],
  );

  useEffect(() => {
    void detectRoot();
  }, []);

  async function detectRoot() {
    dispatch({ type: "loading" });
    try {
      const roots = await discoverPrismLauncherRoots();
      const root = roots.find((candidate) => candidate.valid);
      if (!root) {
        dispatch({ type: "loaded", rootPath: null, instances: [], message: t.desktopOnly });
        return;
      }
      await loadRoot(root.rootPath);
    } catch (error) {
      dispatch({ type: "failed", message: formatBackendError(error) });
    }
  }

  async function loadRoot(rootPath: string) {
    dispatch({ type: "loading", rootPath });
    try {
      const instances = await listPatcherInstances(rootPath);
      dispatch({ type: "loaded", rootPath, instances });
    } catch (error) {
      dispatch({ type: "failed", message: formatBackendError(error) });
    }
  }

  async function chooseRoot() {
    const selected = await open({ directory: true, multiple: false });
    if (typeof selected === "string") {
      await loadRoot(selected);
    }
  }

  async function runPatch(instance: PatcherInstance) {
    const action = getPatchStatusAction(instance.patchStatus);
    if (!action) {
      return;
    }
    dispatch({ type: "operationStarted", instancePath: instance.instancePath, action: action.action });
    try {
      await patchPrismInstance(instance.instancePath, action.action);
      const instances = state.rootPath ? await listPatcherInstances(state.rootPath) : [];
      dispatch({
        type: "operationCompleted",
        instances,
        message: "",
      });
    } catch (error) {
      dispatch({ type: "failed", message: formatBackendError(error) });
    }
  }

  async function removePatch(instance: PatcherInstance) {
    dispatch({ type: "operationStarted", instancePath: instance.instancePath, action: "remove" });
    try {
      await removePrismInstancePatch(instance.instancePath, deleteSchemes);
      const instances = state.rootPath ? await listPatcherInstances(state.rootPath) : [];
      dispatch({
        type: "operationCompleted",
        instances,
        message: deleteSchemes ? "MPB patch and schemes removed." : "MPB patch removed.",
      });
    } catch (error) {
      dispatch({ type: "failed", message: formatBackendError(error) });
    }
  }

  return (
    <main className="patcher-shell">
      <header className="patcher-header">
        <div>
          <h1>{t.title}</h1>
          <p>{t.subtitle}</p>
        </div>
        <div className="patcher-actions">
          <button type="button" className="patcher-button secondary" onClick={chooseRoot}>
            <FolderOpen size={16} aria-hidden="true" />
            {t.chooseRoot}
          </button>
          <button
            type="button"
            className="patcher-button secondary"
            onClick={() => (state.rootPath ? loadRoot(state.rootPath) : detectRoot())}
            disabled={state.loading}
          >
            <RefreshCw size={16} aria-hidden="true" />
            {t.refresh}
          </button>
        </div>
      </header>

      <section className="patcher-root">
        <span>{t.detectedRoot}</span>
        <strong>{state.rootPath ?? "Not selected"}</strong>
      </section>

      <div className="patcher-grid">
        <section className="patcher-list" aria-label={t.instances}>
          <h2>{t.instances}</h2>
          {state.instances.length === 0 ? (
            <p className="patcher-empty">{state.loading ? "Loading..." : t.noInstances}</p>
          ) : (
            state.instances.map((instance) => (
              <button
                type="button"
                key={instance.instancePath}
                className={
                  instance.instancePath === selectedInstance?.instancePath
                    ? "patcher-instance selected"
                    : "patcher-instance"
                }
                onClick={() =>
                  dispatch({ type: "selectInstance", instancePath: instance.instancePath })
                }
              >
                <span className="patcher-instance-name">{instance.displayName}</span>
                <span className={`patcher-status ${instance.patchStatus}`}>
                  {statusIcon(instance.patchStatus)}
                  {statusLabel(instance.patchStatus, language)}
                </span>
                <span className="patcher-meta">
                  {instance.minecraftVersion ?? "Minecraft ?"} · {instance.loader ?? "Unknown"}
                </span>
              </button>
            ))
          )}
        </section>

        <section className="patcher-detail" aria-label={t.details}>
          {selectedInstance ? (
            <InstanceDetails
              instance={selectedInstance}
              language={language}
              deleteSchemes={deleteSchemes}
              setDeleteSchemes={setDeleteSchemes}
              busy={state.operation?.instancePath === selectedInstance.instancePath}
              onPatch={() => runPatch(selectedInstance)}
              onRemove={() => removePatch(selectedInstance)}
            />
          ) : (
            <p className="patcher-empty">{t.noInstances}</p>
          )}
        </section>
      </div>

      {state.message ? <aside className="patcher-message">{state.message}</aside> : null}
    </main>
  );
}

function InstanceDetails({
  instance,
  language,
  deleteSchemes,
  setDeleteSchemes,
  busy,
  onPatch,
  onRemove,
}: {
  instance: PatcherInstance;
  language: Language;
  deleteSchemes: boolean;
  setDeleteSchemes: (value: boolean) => void;
  busy: boolean;
  onPatch: () => void;
  onRemove: () => void;
}) {
  const t = copy[language];
  const action = getPatchStatusAction(instance.patchStatus);
  const actionLabel =
    action?.labelKey === "patcher.apply"
      ? t.apply
      : action?.labelKey === "patcher.update"
        ? t.update
        : t.repair;
  const canRemovePatch = instance.patchStatus === "patched" || instance.patchStatus === "needsRepair";

  return (
    <>
      <div className="patcher-detail-heading">
        <div>
          <h2>{instance.displayName}</h2>
          <span className={`patcher-status ${instance.patchStatus}`}>
            {statusIcon(instance.patchStatus)}
            {statusLabel(instance.patchStatus, language)}
          </span>
        </div>
      </div>

      <dl className="patcher-facts">
        <div>
          <dt>{t.minecraft}</dt>
          <dd>{instance.minecraftVersion ?? "Unknown"}</dd>
        </div>
        <div>
          <dt>{t.loader}</dt>
          <dd>
            {instance.loader ?? "Unknown"}
            {instance.loaderVersion ? ` ${instance.loaderVersion}` : ""}
          </dd>
        </div>
        <div>
          <dt>{t.path}</dt>
          <dd>{instance.instancePath}</dd>
        </div>
        {instance.patchReason ? (
          <div>
            <dt>{t.reason}</dt>
            <dd>{instance.patchReason}</dd>
          </div>
        ) : null}
      </dl>

      {action ? (
        <div className="patcher-detail-actions">
          <button type="button" className="patcher-button primary" onClick={onPatch} disabled={busy}>
            <Wrench size={16} aria-hidden="true" />
            {busy ? t.working : actionLabel}
          </button>
        </div>
      ) : null}

      {instance.patchStatus === "patched" ? (
        <section className="patcher-next">
          <h3>{t.next}</h3>
          <p>{getNextStepText(language)}</p>
        </section>
      ) : null}

      {canRemovePatch ? (
        <div className="patcher-detail-actions patcher-danger-zone">
          <label className="patcher-checkbox">
            <input
              type="checkbox"
              checked={deleteSchemes}
              onChange={(event) => setDeleteSchemes(event.currentTarget.checked)}
            />
            {t.deleteSchemes}
          </label>
          <button type="button" className="patcher-button danger" onClick={onRemove} disabled={busy}>
            <XCircle size={16} aria-hidden="true" />
            {busy ? t.removing : t.remove}
          </button>
        </div>
      ) : null}
    </>
  );
}

function statusIcon(status: PatchStatus) {
  if (status === "patched") {
    return <CheckCircle2 size={15} aria-hidden="true" />;
  }
  if (status === "conflict" || status === "unsupported" || status === "instanceRunning") {
    return <ShieldAlert size={15} aria-hidden="true" />;
  }
  return <Wrench size={15} aria-hidden="true" />;
}

function statusLabel(status: PatchStatus, language: Language): string {
  const labels: Record<Language, Record<PatchStatus, string>> = {
    en: {
      notPatched: "Not patched",
      patched: "Patched",
      needsUpdate: "Needs update",
      needsRepair: "Needs repair",
      conflict: "Conflict",
      unsupported: "Unsupported",
      instanceRunning: "Instance running",
    },
    ru: {
      notPatched: "Не пропатчен",
      patched: "Пропатчен",
      needsUpdate: "Нужно обновить",
      needsRepair: "Нужно восстановить",
      conflict: "Конфликт",
      unsupported: "Не поддерживается",
      instanceRunning: "Инстанс запущен",
    },
  };
  return labels[language][status];
}
