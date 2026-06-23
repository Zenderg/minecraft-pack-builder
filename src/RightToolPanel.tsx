import { ChevronDown, Database, Layers3 } from "lucide-react";
import { useState } from "react";

import { translate } from "./i18n";
import { type StageOptionId } from "./renderViewer";
import { type ViewerToolContext } from "./ViewerWorkspace";

type Translator = (key: Parameters<typeof translate>[1]) => string;

export function RightToolPanel({
  onStageChange,
  t,
  toolContext,
}: {
  onStageChange: (stageId: StageOptionId) => void;
  t: Translator;
  toolContext: ViewerToolContext | null;
}) {
  const [openSections, setOpenSections] = useState({
    materials: false,
    stages: true,
  });
  const materialTotal = toolContext?.materials.reduce((total, line) => total + line.count, 0) ?? 0;

  function toggleSection(section: keyof typeof openSections) {
    setOpenSections((current) => ({ ...current, [section]: !current[section] }));
  }

  return (
    <aside className="right-rail tools-sidebar" aria-label={t("tools.sidebar")}>
      <div className="tool-summary" aria-label="Render metrics">
        <span>
          {toolContext
            ? `${toolContext.metrics.visibleBlocks} / ${toolContext.metrics.totalBlocks}`
            : "--"}
        </span>
        <span>{toolContext ? `${toolContext.metrics.chunkCount} chunks` : "-- chunks"}</span>
        <span>{toolContext ? `${toolContext.metrics.faceCount} faces` : "-- faces"}</span>
      </div>
      <div className="tool-tree">
        <section className={openSections.stages ? "tool-node expanded" : "tool-node"}>
          <div className="tree-item tool-row">
            <button className="tree-label tool-label" onClick={() => toggleSection("stages")} type="button">
              <Layers3 size={16} />
              {t("tools.stages")}
            </button>
            <strong>
              {toolContext
                ? `${toolContext.metrics.visibleBlocks} / ${toolContext.metrics.totalBlocks}`
                : "--"}
            </strong>
            <ChevronDown className={openSections.stages ? "open" : ""} size={16} />
          </div>
          {openSections.stages && (
            <div className="tool-children">
              <div className="tool-panel-stage-list">
                {toolContext ? (
                  toolContext.stageOptions.map((stage) => (
                    <button
                      className={stage.id === toolContext.selectedStageId ? "active" : ""}
                      key={stage.id}
                      onClick={() => onStageChange(stage.id)}
                      type="button"
                    >
                      <span>{stage.label}</span>
                      <strong>{stage.order ?? t("tools.unassigned")}</strong>
                    </button>
                  ))
                ) : (
                  <div className="empty-list">{t("tools.openScheme")}</div>
                )}
              </div>
            </div>
          )}
        </section>

        <section className={openSections.materials ? "tool-node expanded" : "tool-node"}>
          <div className="tree-item tool-row">
            <button className="tree-label tool-label" onClick={() => toggleSection("materials")} type="button">
              <Database size={16} />
              {t("tools.materials")}
            </button>
            <strong>{t("materials.total")}: {materialTotal}</strong>
            <ChevronDown className={openSections.materials ? "open" : ""} size={16} />
          </div>
          {openSections.materials && (
            <div className="tool-children">
              {toolContext ? (
                <ul className="materials-list">
                  {toolContext.materials.map((material) => (
                    <li key={material.blockId}>
                      <span>{material.blockId}</span>
                      <strong>{material.count}</strong>
                    </li>
                  ))}
                </ul>
              ) : (
                <div className="empty-list">{t("tools.openScheme")}</div>
              )}
            </div>
          )}
        </section>
      </div>
    </aside>
  );
}
