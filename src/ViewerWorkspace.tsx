import { Cuboid, Loader2, MousePointer2 } from "lucide-react";
import { useEffect, useMemo, useRef, useState } from "react";
import { convertFileSrc } from "@tauri-apps/api/core";
import type { InstancedMesh, Material, Texture } from "three";

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
  type RenderBlock,
  type RenderModelElement,
  type RenderScene,
  type RenderSceneMetrics,
  type StageOption,
  type StageOptionId,
} from "./renderViewer";
import { getSchemeRenderScene } from "./tauri";

type Translator = (key: Parameters<typeof translate>[1]) => string;

type HoveredBlock = {
  coordinate: [number, number, number];
  blockId: string;
} | null;

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

type ThreeRuntime = {
  rebuildBlocks: () => void;
};

type RenderBlockInstance = {
  block: RenderBlock;
  element: RenderModelElement;
};

function elementsForBlock(block: RenderBlock): RenderModelElement[] {
  if (block.modelElements?.length) {
    return block.modelElements;
  }
  return [
    {
      from: [0, 0, 0],
      to: [16, 16, 16],
      faceTexturePaths: block.faceTexturePaths ?? {},
    },
  ];
}

function parseModelVector(value: string): [number, number, number] {
  const parts = value.split(",").map(Number);
  return [
    Number.isFinite(parts[0]) ? parts[0] : 0,
    Number.isFinite(parts[1]) ? parts[1] : 0,
    Number.isFinite(parts[2]) ? parts[2] : 0,
  ];
}

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

function ThreeSchemeViewer({
  blocks,
  dimensions,
  hoveredBlock,
  onHoverBlock,
}: {
  blocks: RenderBlock[];
  dimensions: [number, number, number];
  hoveredBlock: HoveredBlock;
  onHoverBlock: (block: HoveredBlock) => void;
}) {
  const canvasRef = useRef<HTMLCanvasElement | null>(null);
  const viewportRef = useRef<HTMLDivElement | null>(null);
  const blocksRef = useRef(blocks);
  const onHoverBlockRef = useRef(onHoverBlock);
  const runtimeRef = useRef<ThreeRuntime | null>(null);

  useEffect(() => {
    blocksRef.current = blocks;
    runtimeRef.current?.rebuildBlocks();
  }, [blocks]);

  useEffect(() => {
    onHoverBlockRef.current = onHoverBlock;
  }, [onHoverBlock]);

  useEffect(() => {
    const canvas = canvasRef.current;
    const viewport = viewportRef.current;
    if (!canvas || !viewport || !("WebGL2RenderingContext" in window)) {
      return;
    }
    const liveCanvas = canvas;
    const liveViewport = viewport;

    let disposed = false;
    let cleanup: (() => void) | null = null;

    async function mountThree() {
      const [THREE, controlsModule] = await Promise.all([
        import("three"),
        import("three/examples/jsm/controls/OrbitControls.js"),
      ]);
      if (disposed) {
        return;
      }

      const renderer = new THREE.WebGLRenderer({ antialias: true, alpha: true, canvas: liveCanvas });
      renderer.setPixelRatio(Math.min(window.devicePixelRatio, 2));
      const scene = new THREE.Scene();
      scene.background = new THREE.Color("#0e1314");
      const camera = new THREE.PerspectiveCamera(45, 1, 0.1, 1000);
      camera.position.set(dimensions[0] * 1.1, dimensions[1] * 1.2 + 5, dimensions[2] * 1.45);

      const controls = new controlsModule.OrbitControls(camera, liveCanvas);
      controls.enableDamping = true;
      controls.target.set(dimensions[0] / 2, dimensions[1] / 3, dimensions[2] / 2);

      scene.add(new THREE.AmbientLight("#ffffff", 1.9));
      const directional = new THREE.DirectionalLight("#dfffe9", 2.8);
      directional.position.set(8, 14, 10);
      scene.add(directional);

      const grid = new THREE.GridHelper(
        Math.max(dimensions[0], dimensions[2]) + 2,
        Math.max(dimensions[0], dimensions[2]) + 2,
        "#36534a",
        "#263532",
      );
      grid.position.set(dimensions[0] / 2, -0.01, dimensions[2] / 2);
      scene.add(grid);

      const raycaster = new THREE.Raycaster();
      const pointer = new THREE.Vector2();
      const interactiveMeshes: Array<{ mesh: InstancedMesh; blocks: RenderBlock[] }> = [];
      const textureLoader = new THREE.TextureLoader();
      const materialKeySeparator = "\u001f";
      const textureCache = new Map<string, Texture>();

      function textureForPath(path: string): Texture {
        const cached = textureCache.get(path);
        if (cached) {
          return cached;
        }
        const texture = textureLoader.load(
          convertFileSrc(path),
          (loaded) => {
            loaded.needsUpdate = true;
          },
          undefined,
          () => {
            textureCache.delete(path);
          },
        );
        texture.colorSpace = THREE.SRGBColorSpace;
        texture.magFilter = THREE.NearestFilter;
        texture.minFilter = THREE.NearestMipmapNearestFilter;
        texture.generateMipmaps = true;
        textureCache.set(path, texture);
        return texture;
      }

      function rebuildBlocks() {
        for (const item of interactiveMeshes) {
          item.mesh.geometry.dispose();
          disposeMaterial(item.mesh.material);
          scene.remove(item.mesh);
        }
        interactiveMeshes.length = 0;

        const byMaterial = new Map<string, RenderBlockInstance[]>();
        for (const block of blocksRef.current) {
          for (const element of elementsForBlock(block)) {
            const faceTextures = element.faceTexturePaths ?? block.faceTexturePaths;
            const key = [
              element.from.join(","),
              element.to.join(","),
              faceTextures?.east ?? block.texturePath ?? "",
              faceTextures?.west ?? block.texturePath ?? "",
              faceTextures?.up ?? block.texturePath ?? "",
              faceTextures?.down ?? block.texturePath ?? "",
              faceTextures?.south ?? block.texturePath ?? "",
              faceTextures?.north ?? block.texturePath ?? "",
              block.color,
              block.alpha ?? 1,
            ].join(materialKeySeparator);
            byMaterial.set(key, [...(byMaterial.get(key) ?? []), { block, element }]);
          }
        }

        for (const [key, materialInstances] of byMaterial) {
          const [
            fromValue,
            toValue,
            eastTexturePath,
            westTexturePath,
            upTexturePath,
            downTexturePath,
            southTexturePath,
            northTexturePath,
            color,
            alphaValue,
          ] = key.split(materialKeySeparator);
          const from = parseModelVector(fromValue);
          const to = parseModelVector(toValue);
          const alpha = Number(alphaValue);
          const texturePaths = [
            eastTexturePath,
            westTexturePath,
            upTexturePath,
            downTexturePath,
            southTexturePath,
            northTexturePath,
          ];
          const geometry = new THREE.BoxGeometry(
            Math.max(0.001, (to[0] - from[0]) / 16),
            Math.max(0.001, (to[1] - from[1]) / 16),
            Math.max(0.001, (to[2] - from[2]) / 16),
          );
          const material = texturePaths.some(Boolean)
            ? texturePaths.map((texturePath) => {
                const texture = texturePath ? textureForPath(texturePath) : null;
                return new THREE.MeshStandardMaterial({
                  color: texture ? "#ffffff" : color,
                  map: texture,
                  opacity: alpha,
                  transparent: true,
                  alphaTest: texture ? 0.35 : 0,
                  side: THREE.DoubleSide,
                  roughness: 0.88,
                  metalness: 0.04,
                });
              })
            : new THREE.MeshStandardMaterial({
                color,
                opacity: alpha,
                transparent: alpha < 1,
                roughness: 0.88,
                metalness: 0.04,
              });
          const mesh = new THREE.InstancedMesh(geometry, material, materialInstances.length);
          const matrix = new THREE.Matrix4();
          materialInstances.forEach(({ block, element }, index) => {
            const elementCenter = [
              (element.from[0] + element.to[0]) / 32,
              (element.from[1] + element.to[1]) / 32,
              (element.from[2] + element.to[2]) / 32,
            ] as const;
            matrix.makeTranslation(
              block.coordinate[0] + elementCenter[0],
              block.coordinate[1] + elementCenter[1],
              block.coordinate[2] + elementCenter[2],
            );
            mesh.setMatrixAt(index, matrix);
          });
          mesh.instanceMatrix.needsUpdate = true;
          scene.add(mesh);
          interactiveMeshes.push({
            mesh,
            blocks: materialInstances.map((instance) => instance.block),
          });
        }
      }

      function resize() {
        const rect = liveViewport.getBoundingClientRect();
        const width = Math.max(1, rect.width);
        const height = Math.max(1, rect.height);
        renderer.setSize(width, height, false);
        camera.aspect = width / height;
        camera.updateProjectionMatrix();
      }

      function pickBlock(event: PointerEvent): RenderBlock | null {
        const rect = liveCanvas.getBoundingClientRect();
        pointer.x = ((event.clientX - rect.left) / rect.width) * 2 - 1;
        pointer.y = -((event.clientY - rect.top) / rect.height) * 2 + 1;
        raycaster.setFromCamera(pointer, camera);
        const hits = raycaster.intersectObjects(interactiveMeshes.map((item) => item.mesh));
        const hit = hits[0];
        if (!hit || hit.instanceId === undefined) {
          return null;
        }
        const item = interactiveMeshes.find((entry) => entry.mesh === hit.object);
        return item?.blocks[hit.instanceId] ?? null;
      }

      function handlePointerMove(event: PointerEvent) {
        const block = pickBlock(event);
        onHoverBlockRef.current(
          block
            ? {
                blockId: block.blockId,
                coordinate: block.coordinate,
              }
            : null,
        );
      }

      function handlePointerLeave() {
        onHoverBlockRef.current(null);
      }

      const resizeObserver = new ResizeObserver(resize);
      resizeObserver.observe(liveViewport);
      liveCanvas.addEventListener("pointermove", handlePointerMove);
      liveCanvas.addEventListener("pointerleave", handlePointerLeave);
      rebuildBlocks();
      resize();
      runtimeRef.current = { rebuildBlocks };

      let frame = 0;
      function animate() {
        if (disposed) {
          return;
        }
        controls.update();
        renderer.render(scene, camera);
        frame = window.requestAnimationFrame(animate);
      }
      animate();

      cleanup = () => {
        if (runtimeRef.current?.rebuildBlocks === rebuildBlocks) {
          runtimeRef.current = null;
        }
        window.cancelAnimationFrame(frame);
        resizeObserver.disconnect();
        liveCanvas.removeEventListener("pointermove", handlePointerMove);
        liveCanvas.removeEventListener("pointerleave", handlePointerLeave);
        controls.dispose();
        for (const item of interactiveMeshes) {
          item.mesh.geometry.dispose();
          disposeMaterial(item.mesh.material);
        }
        for (const texture of textureCache.values()) {
          texture.dispose();
        }
        textureCache.clear();
        renderer.dispose();
      };
    }

    void mountThree();

    return () => {
      disposed = true;
      cleanup?.();
    };
  }, [dimensions]);

  return (
    <div className="viewer-canvas three-viewer" ref={viewportRef}>
      <canvas aria-label="3D scheme canvas" className="viewer-three-canvas" ref={canvasRef} />
      {!("WebGL2RenderingContext" in window) && (
        <div className="viewer-css-preview" aria-hidden="true">
          {blocks.slice(0, 24).map((block) => (
            <span
              key={`${block.coordinate.join(":")}:${block.blockId}`}
              style={
                {
                  "--block-x": block.coordinate[0],
                  "--block-y": block.coordinate[1],
                  "--block-z": block.coordinate[2],
                  "--block-color": block.color,
                  "--block-alpha": block.alpha ?? 1,
                } as React.CSSProperties
              }
            />
          ))}
        </div>
      )}
      {hoveredBlock && (
        <div className="block-hover-overlay">
          <span>{hoveredBlock.blockId}</span>
          <code>{hoveredBlock.coordinate.join(", ")}</code>
        </div>
      )}
    </div>
  );
}

function disposeMaterial(material: Material | Material[]) {
  if (Array.isArray(material)) {
    material.forEach((item) => item.dispose());
    return;
  }
  material.dispose();
}

function getValidStageId(scene: RenderScene, selectedStageId: StageOptionId | null): StageOptionId {
  const options = getStageOptions(scene);
  if (selectedStageId && options.some((option) => option.id === selectedStageId)) {
    return selectedStageId;
  }
  return getDefaultStageId(scene);
}
