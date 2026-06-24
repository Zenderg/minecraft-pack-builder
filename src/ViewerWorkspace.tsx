import { Cuboid, Loader2, MousePointer2 } from "lucide-react";
import { useEffect, useMemo, useState } from "react";

import { formatBackendError } from "./backendErrors";
import { translate } from "./i18n";
import type { LibraryModpack, LibraryScheme } from "./library";
import {
  getDefaultStageId,
  getRenderSceneMaterials,
  getRenderSceneMetrics,
  getStageOptions,
  getVisibleRenderBlocks,
  type RenderMaterialLine,
  type RenderScene,
  type RenderSceneMetrics,
  type StageOption,
  type StageOptionId,
} from "./renderViewer";
import { getSchemeRenderScene } from "./tauri";
import { ThreeSchemeViewer, type HoveredBlock } from "./viewer/ThreeSchemeViewer";

type Translator = (key: Parameters<typeof translate>[1]) => string;

type ViewerState =
  | { kind: "empty" }
  | { kind: "loading" }
  | { kind: "ready"; scene: RenderScene }
  | { kind: "error"; message: string };

export type ViewerToolContext = {
  materials: RenderMaterialLine[];
  metrics: RenderSceneMetrics;
  scene: RenderScene;
  selectedStageId: StageOptionId;
  stageOptions: StageOption[];
};

export function ViewerWorkspace({
  modpack,
  onStageChange,
  onToolContextChange,
  revision,
  scheme,
  selectedStageId,
  t,
}: {
  modpack: LibraryModpack | null;
  onStageChange: (stageId: StageOptionId | null) => void;
  onToolContextChange: (context: ViewerToolContext | null) => void;
  revision: number;
  scheme: LibraryScheme | null;
  selectedStageId: StageOptionId | null;
  t: Translator;
}) {
  const [viewerState, setViewerState] = useState<ViewerState>({ kind: "empty" });

  useEffect(() => {
    let active = true;
    onToolContextChange(null);

    if (!scheme) {
      setViewerState({ kind: "empty" });
      onStageChange(null);
      return () => {
        active = false;
      };
    }

    setViewerState({ kind: "loading" });
    getSchemeRenderScene(scheme.id)
      .then((scene) => {
        if (!active) {
          return;
        }
        onStageChange(getDefaultStageId(scene));
        setViewerState({ kind: "ready", scene });
      })
      .catch((error: unknown) => {
        if (active) {
          setViewerState({ kind: "error", message: formatBackendError(error) });
        }
      });

    return () => {
      active = false;
    };
  }, [onStageChange, onToolContextChange, revision, scheme]);

  const effectiveStageId =
    viewerState.kind === "ready"
      ? getValidStageId(viewerState.scene, selectedStageId)
      : "unassigned";

  useEffect(() => {
    if (viewerState.kind !== "ready") {
      onToolContextChange(null);
      return;
    }
    onToolContextChange({
      materials: getRenderSceneMaterials(viewerState.scene),
      metrics: getRenderSceneMetrics(viewerState.scene, effectiveStageId),
      scene: viewerState.scene,
      selectedStageId: effectiveStageId,
      stageOptions: getStageOptions(viewerState.scene),
    });
  }, [effectiveStageId, onToolContextChange, viewerState]);

  return (
    <section className="viewer-region" aria-label={t("workspace.viewer")}>
      {viewerState.kind === "ready" && scheme ? (
        <ReadyViewer
          modpack={modpack}
          scene={viewerState.scene}
          selectedStageId={effectiveStageId}
          t={t}
        />
      ) : (
        <ViewerStatus state={viewerState} scheme={scheme} t={t} />
      )}
    </section>
  );
}

function ReadyViewer({
  modpack,
  scene,
  selectedStageId,
  t,
}: {
  modpack: LibraryModpack | null;
  scene: RenderScene;
  selectedStageId: StageOptionId;
  t: Translator;
}) {
  const metrics = getRenderSceneMetrics(scene, selectedStageId);
  const visibleBlocks = useMemo(
    () => getVisibleRenderBlocks(scene, selectedStageId),
    [scene, selectedStageId],
  );
  const [hoveredBlock, setHoveredBlock] = useState<HoveredBlock>(null);

  useEffect(() => {
    setHoveredBlock(null);
  }, [selectedStageId]);

  return (
    <>
      <ThreeSchemeViewer
        blocks={visibleBlocks}
        dimensions={scene.dimensions}
        hoveredBlock={hoveredBlock}
        onHoverBlock={setHoveredBlock}
      />

      <div className="viewer-footer">
        <span className="viewer-footer-primary">
          <Cuboid size={14} />
          <span>{scene.schemeName}</span>
          <code>{metrics.dimensions}</code>
        </span>
        <span>{modpack?.displayName ?? t("workspace.library")}</span>
        {metrics.isLargeScheme && <strong>{t("viewer.largeScheme")}</strong>}
      </div>
    </>
  );
}

function ViewerStatus({
  scheme,
  state,
  t,
}: {
  scheme: LibraryScheme | null;
  state: ViewerState;
  t: Translator;
}) {
  return (
    <div className="viewer-canvas viewer-status-canvas">
      {state.kind === "loading" && (
        <div className="viewer-state-message">
          <Loader2 className="status-spinner" size={22} />
          <h2>{t("viewer.loadingTitle")}</h2>
          <p>{scheme?.name ?? t("workspace.viewer")}</p>
        </div>
      )}
      {state.kind === "error" && (
        <div className="viewer-state-message error">
          <h2>{t("viewer.errorTitle")}</h2>
          <p>{state.message}</p>
        </div>
      )}
      {state.kind === "empty" && (
        <div className="viewer-state-message">
          <MousePointer2 size={24} />
          <h2>{t("viewer.emptyTitle")}</h2>
          <p>{t("viewer.emptyBody")}</p>
        </div>
      )}
    </div>
  );
}

function getValidStageId(scene: RenderScene, selectedStageId: StageOptionId | null): StageOptionId {
  const options = getStageOptions(scene);
  if (selectedStageId && options.some((option) => option.id === selectedStageId)) {
    return selectedStageId;
  }
  return getDefaultStageId(scene);
}
