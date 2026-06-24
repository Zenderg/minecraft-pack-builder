import { X } from "lucide-react";

import { getLibraryDialogContent, type LibraryModpack } from "../library";
import type { LibraryDialog, Translator } from "./types";

export function LibraryActionDialog(props: {
  dialog: LibraryDialog;
  onCancel: () => void;
  onConfirm: () => void;
  onNameChange: (name: string) => void;
  t: Translator;
}) {
  const { dialog, t } = props;
  const isDelete = dialog.kind === "deleteScheme" || dialog.kind === "deleteModpack";
  const isInfo = dialog.kind === "infoModpack";
  const content = getLibraryDialogContent(dialog.kind);
  const title = t(content.titleKey);

  return (
    <div className="modal-backdrop" role="presentation">
      <section
        className={`settings-modal library-dialog ${content.tone}`}
        aria-label={title}
        role="dialog"
      >
        <header className="settings-modal-header">
          <h2>{title}</h2>
          <button
            aria-label={t("library.cancel")}
            className="icon-action"
            onClick={props.onCancel}
            type="button"
          >
            <X size={18} />
          </button>
        </header>
        {content.bodyKey && (
          <div className="library-dialog-copy">
            <p>{t(content.bodyKey)}</p>
          </div>
        )}
        {isInfo && <ModpackInfoRows modpack={dialog.modpack} t={t} />}
        {!isDelete && !isInfo && (
          <label className="library-dialog-field">
            <span>{content.fieldKey ? t(content.fieldKey) : t("library.nameLabel")}</span>
            <input
              autoFocus
              onChange={(event) => props.onNameChange(event.currentTarget.value)}
              value={dialog.name}
            />
          </label>
        )}
        <div className="dialog-actions">
          <button className="secondary-action compact" onClick={props.onCancel} type="button">
            {isInfo ? t("library.close") : t("library.cancel")}
          </button>
          {!isInfo && (
            <button
              className={isDelete ? "secondary-action compact danger" : "primary-action compact"}
              onClick={props.onConfirm}
              type="button"
            >
              {t("library.confirm")}
            </button>
          )}
        </div>
      </section>
    </div>
  );
}

function ModpackInfoRows({ modpack, t }: { modpack: LibraryModpack; t: Translator }) {
  const rows: Array<[string, string]> = [
    [t("library.localName"), modpack.localName],
    [t("library.releaseVersion"), modpack.versionName],
    [t("library.minecraftVersion"), modpack.minecraftVersion ?? t("library.unknown")],
    [t("library.loader"), modpack.loader ?? t("library.unknown")],
    [t("library.sourceUrl"), modpack.sourceUrl ?? t("library.unknown")],
    [t("library.importStatus"), modpack.importStatus],
    [t("library.importMessage"), modpack.importMessage ?? t("library.unknown")],
    [t("library.schemeCount"), String(modpack.schemes.length)],
  ];

  return (
    <dl className="modpack-info-list">
      {rows.map(([label, value]) => (
        <div key={label}>
          <dt>{label}</dt>
          <dd>{value}</dd>
        </div>
      ))}
    </dl>
  );
}

