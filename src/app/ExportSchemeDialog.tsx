import { Download, FileOutput, X } from "lucide-react";

import { type ExportFormat } from "../exportDialog";
import type { ExportDialog, Translator } from "./types";

const exportFormats: Array<{ format: ExportFormat; labelKey: "export.format.schem" | "export.format.litematic" }> = [
  { format: "schem", labelKey: "export.format.schem" },
  { format: "litematic", labelKey: "export.format.litematic" },
];

export function ExportSchemeDialog(props: {
  dialog: ExportDialog;
  onCancel: () => void;
  onChoosePath: () => void;
  onConfirm: () => void;
  onFormatChange: (format: ExportFormat) => void;
  t: Translator;
}) {
  const { dialog, t } = props;

  return (
    <div className="modal-backdrop" role="presentation">
      <section className="settings-modal library-dialog export-dialog" aria-label={t("export.title")} role="dialog">
        <header className="settings-modal-header">
          <div>
            <h2>{t("export.title")}</h2>
            <span>{dialog.scheme.name}</span>
          </div>
          <button aria-label={t("library.cancel")} className="icon-action" onClick={props.onCancel} type="button">
            <X size={18} />
          </button>
        </header>

        <div className="export-dialog-body">
          <fieldset className="export-format-field">
            <legend>{t("export.format")}</legend>
            <div className="export-format-options">
              {exportFormats.map((item) => (
                <button
                  className={dialog.format === item.format ? "active" : ""}
                  key={item.format}
                  onClick={() => props.onFormatChange(item.format)}
                  type="button"
                >
                  <FileOutput size={15} />
                  {t(item.labelKey)}
                </button>
              ))}
            </div>
          </fieldset>

          <label className="library-dialog-field">
            <span>{t("export.destination")}</span>
            <div className="export-path-row">
              <input
                readOnly
                placeholder={t("export.destinationPlaceholder")}
                value={dialog.destinationPath}
              />
              <button className="secondary-action compact" onClick={props.onChoosePath} type="button">
                {t("export.chooseDestination")}
              </button>
            </div>
          </label>
        </div>

        <div className="dialog-actions">
          <button className="secondary-action compact" onClick={props.onCancel} type="button">
            {t("library.cancel")}
          </button>
          <button
            className={dialog.isExporting ? "primary-action compact loading" : "primary-action compact"}
            disabled={!dialog.destinationPath || dialog.isExporting}
            onClick={props.onConfirm}
            type="button"
          >
            <Download size={15} />
            {dialog.isExporting ? t("export.exporting") : t("export.confirm")}
          </button>
        </div>
      </section>
    </div>
  );
}
