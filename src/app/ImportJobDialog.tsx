import { AlertTriangle, CheckCircle2, Loader2, RefreshCcw, Trash2, X } from "lucide-react";
import { useLayoutEffect, useRef } from "react";

import type { LibraryModpack } from "../library";
import type { ImportProgress } from "../tauri";
import type { Translator } from "./types";

export function ImportJobDialog({
  logs,
  modpack,
  onCancel,
  onClose,
  onDelete,
  onRetry,
  progress,
  stage,
  t,
}: {
  logs: string[];
  modpack: LibraryModpack;
  onCancel: () => void;
  onClose: () => void;
  onDelete: () => void;
  onRetry: () => void;
  progress: ImportProgress | null;
  stage: string;
  t: Translator;
}) {
  const progressValue = getImportJobProgressValue(stage, modpack.importStatus, progress);
  const stages = getImportJobStages(stage, modpack.importStatus);
  const canDelete = modpack.importStatus === "failed";
  const logViewportRef = useRef<HTMLDivElement | null>(null);
  const shouldStickLogToBottomRef = useRef(true);
  const visibleLogLines = logs.length > 0 ? logs : [modpack.importMessage ?? t("import.noLogYet")];

  useLayoutEffect(() => {
    const viewport = logViewportRef.current;
    if (!viewport || !shouldStickLogToBottomRef.current) {
      return;
    }
    viewport.scrollTop = viewport.scrollHeight;
  }, [visibleLogLines.length, visibleLogLines[visibleLogLines.length - 1]]);

  function handleLogScroll(event: React.UIEvent<HTMLDivElement>) {
    const viewport = event.currentTarget;
    const distanceFromBottom = viewport.scrollHeight - viewport.scrollTop - viewport.clientHeight;
    shouldStickLogToBottomRef.current = distanceFromBottom < 24;
  }

  return (
    <div className="modal-backdrop" role="presentation">
      <section className="import-job-modal" aria-label={t("import.processingTitle")} role="dialog">
        <header className="settings-modal-header">
          <div>
            <h2>{t("import.processingTitle")}</h2>
            <div className="import-job-meta" aria-label="Selected release metadata">
              <span>{modpack.versionName}</span>
              <span>{modpack.minecraftVersion ?? t("library.unknown")}</span>
              <span>{modpack.loader ?? t("library.unknown")}</span>
            </div>
          </div>
          <button aria-label={t("settings.close")} className="icon-action" onClick={onClose} type="button">
            <X size={18} />
          </button>
        </header>

        <div className="import-job-body">
          <section className="import-job-summary">
            <div className="import-job-progress">
              <div>
                <span>{t("import.progress")}</span>
                <strong>{progressValue}%</strong>
              </div>
              <progress max={100} value={progressValue} />
            </div>
          </section>

          <section className="import-job-stages" aria-label={t("import.stages")}>
            {stages.map((item) => (
              <div className={`import-job-stage ${item.state}`} key={item.key}>
                {item.state === "active" ? (
                  <Loader2 className="status-spinner" size={15} />
                ) : item.state === "done" ? (
                  <CheckCircle2 size={15} />
                ) : item.state === "failed" ? (
                  <AlertTriangle size={15} />
                ) : (
                  <span className="stage-dot" />
                )}
                <span>{t(item.label)}</span>
              </div>
            ))}
          </section>

          <section className="import-job-log" aria-label={t("import.liveLog")}>
            <div className="import-log-lines" onScroll={handleLogScroll} ref={logViewportRef}>
              {visibleLogLines.map((line, index) => (
                <code key={`${line}-${index}`}>{line}</code>
              ))}
            </div>
          </section>
        </div>

        <div className="dialog-actions">
          {canDelete && (
            <button className="secondary-action compact danger" onClick={onDelete} type="button">
              <Trash2 size={16} />
              {t("library.deleteModpack")}
            </button>
          )}
          <span className="dialog-actions-spacer" />
          {modpack.importStatus === "importing" && (
            <button className="secondary-action compact danger" onClick={onCancel} type="button">
              <X size={16} />
              {t("import.cancel")}
            </button>
          )}
          {modpack.importStatus === "failed" && (
            <button className="primary-action compact" onClick={onRetry} type="button">
              <RefreshCcw size={16} />
              {t("import.retry")}
            </button>
          )}
        </div>
      </section>
    </div>
  );
}

function getImportJobProgressValue(
  stage: string,
  status: LibraryModpack["importStatus"],
  progress: ImportProgress | null,
): number {
  if (status === "imported") {
    return 100;
  }

  if (progress?.progressPercent !== null && progress?.progressPercent !== undefined) {
    return Math.min(99, Math.max(0, progress.progressPercent));
  }

  if (stage === "parse" || stage === "failed") {
    return 30;
  }

  if (stage === "download") {
    if (!progress?.totalBytes || progress.totalBytes <= 0) {
      return 20;
    }
    const downloadRatio = Math.min(1, Math.max(0, progress.bytesDownloaded / progress.totalBytes));
    return Math.round(10 + downloadRatio * 20);
  }

  return 5;
}

export function importJobStageFromMessage(modpack: LibraryModpack): string {
  const message = modpack.importMessage?.toLowerCase() ?? "";
  if (modpack.importStatus === "failed") {
    return "failed";
  }
  if (message.includes("pars")) {
    return "parse";
  }
  if (message.includes("download")) {
    return "download";
  }
  if (modpack.importStatus === "imported") {
    return "done";
  }
  return "queued";
}

function getImportJobStages(stage: string, status: LibraryModpack["importStatus"]) {
  const order = ["queued", "download", "parse", "done"];
  const activeIndex = stage === "failed" ? 2 : Math.max(0, order.indexOf(stage));
  return [
    { key: "queued", label: "import.stage.queued" as const },
    { key: "download", label: "import.stage.download" as const },
    { key: "parse", label: "import.stage.parse" as const },
    { key: "done", label: "import.stage.done" as const },
  ].map((item, index) => {
    if (status === "failed" && item.key === "parse") {
      return { ...item, state: "failed" as const };
    }
    if (status === "imported" || index < activeIndex) {
      return { ...item, state: "done" as const };
    }
    if (index === activeIndex) {
      return { ...item, state: "active" as const };
    }
    return { ...item, state: "pending" as const };
  });
}

