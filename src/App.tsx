import {
  Bot,
  Box,
  CheckCircle2,
  ChevronDown,
  Database,
  FolderOpen,
  Globe2,
  Layers3,
  PackagePlus,
  Settings,
} from "lucide-react";
import { useEffect, useState } from "react";

import { getInitialLanguage, languages, type Language, translate } from "./i18n";
import { discoverAppPaths, openAppDataFolder, type AppDataPaths } from "./tauri";
import "./styles.css";

const sampleMaterials = [
  { name: "minecraft:stone_bricks", count: 284 },
  { name: "thermal:machine_frame", count: 24 },
  { name: "create:andesite_casing", count: 48 },
];

export function App() {
  const [language, setLanguage] = useState<Language>(() => getInitialLanguage());
  const [paths, setPaths] = useState<AppDataPaths | null>(null);
  const [diagnosticsMessage, setDiagnosticsMessage] = useState("");

  const t = (key: Parameters<typeof translate>[1]) => translate(language, key);

  useEffect(() => {
    discoverAppPaths()
      .then(setPaths)
      .catch((error: unknown) => setDiagnosticsMessage(String(error)));
  }, []);

  async function handleOpenDataFolder() {
    try {
      const nextPaths = await openAppDataFolder();
      if (nextPaths) {
        setPaths(nextPaths);
        setDiagnosticsMessage(nextPaths.appDataDir);
      } else {
        setDiagnosticsMessage(t("settings.desktopOnly"));
      }
    } catch (error) {
      setDiagnosticsMessage(String(error));
    }
  }

  return (
    <main className="app-shell antialiased">
      <aside className="sidebar" aria-label={t("workspace.library")}>
        <div className="brand">
          <div className="brand-mark">
            <Box size={18} />
          </div>
          <div>
            <h1>{t("app.title")}</h1>
            <span>{t("status.localOnly")}</span>
          </div>
        </div>

        <button className="primary-action" type="button">
          <PackagePlus size={17} />
          <span>{t("workspace.addModpack")}</span>
        </button>

        <section className="library-panel">
          <div className="panel-title">
            <span>{t("workspace.library")}</span>
            <ChevronDown size={16} />
          </div>
          <div className="tree-item active">
            <Database size={15} />
            <span>{t("library.modpack")}</span>
          </div>
          <div className="tree-item nested selected">
            <Layers3 size={15} />
            <span>{t("library.scheme")}</span>
          </div>
        </section>

        <button className="settings-link" type="button">
          <Settings size={17} />
          <span>{t("workspace.settings")}</span>
        </button>
      </aside>

      <section className="workspace">
        <header className="status-strip">
          <div className="status-group">
            <span className="status-pill muted">
              <CheckCircle2 size={15} />
              {t("status.localOnly")}
            </span>
            <span className="status-pill warning">
              <Bot size={15} />
              {t("status.aiDisconnected")}
            </span>
          </div>
          <div className="language-switch" aria-label={t("settings.language")}>
            <Globe2 size={16} />
            {languages.map((option) => (
              <button
                className={option === language ? "active" : ""}
                key={option}
                onClick={() => setLanguage(option)}
                type="button"
              >
                {option.toUpperCase()}
              </button>
            ))}
          </div>
        </header>

        <div className="content-grid">
          <section className="viewer-region" aria-label={t("workspace.viewer")}>
            <div className="section-heading">
              <span>{t("workspace.viewer")}</span>
              <strong>64 x 64 x 64</strong>
            </div>
            <div className="viewer-canvas">
              <div className="grid-floor" />
              <div className="block-stack stack-a" />
              <div className="block-stack stack-b" />
              <div className="block-stack stack-c" />
              <div className="viewer-empty">
                <h2>{t("viewer.emptyTitle")}</h2>
                <p>{t("viewer.emptyBody")}</p>
              </div>
            </div>
          </section>

          <aside className="right-rail" aria-label={t("workspace.review")}>
            <section className="tool-panel">
              <div className="section-heading">
                <span>{t("workspace.review")}</span>
                <strong>{t("review.pending")}: 0</strong>
              </div>
              <div className="selection-box">
                <span>{t("review.selection")}</span>
                <code>x: --, y: --, z: --</code>
              </div>
              <div className="empty-list">{t("review.changeRequests")}</div>
            </section>

            <section className="tool-panel">
              <div className="section-heading">
                <span>{t("workspace.materials")}</span>
                <strong>{t("materials.total")}: 356</strong>
              </div>
              <ul className="materials-list">
                {sampleMaterials.map((material) => (
                  <li key={material.name}>
                    <span>{material.name}</span>
                    <strong>{material.count}</strong>
                  </li>
                ))}
              </ul>
            </section>

            <section className="tool-panel settings-panel">
              <div className="section-heading">
                <span>{t("workspace.settings")}</span>
                <strong>{t("settings.diagnostics")}</strong>
              </div>
              <button className="secondary-action" onClick={handleOpenDataFolder} type="button">
                <FolderOpen size={16} />
                {t("settings.openDataFolder")}
              </button>
              <p className="path-label">
                {paths?.appDataDir ?? diagnosticsMessage ?? t("settings.appData")}
              </p>
            </section>
          </aside>
        </div>
      </section>
    </main>
  );
}
