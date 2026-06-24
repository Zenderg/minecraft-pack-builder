import { convertFileSrc } from "@tauri-apps/api/core";
import { useEffect, useRef, type CSSProperties } from "react";
import type { InstancedMesh, Material, Texture } from "three";

import type { RenderBlock, RenderModelElement } from "../renderViewer";
import {
  completeBlockFaceTexturePaths,
  createModelElementGeometry,
  getModelElementCenter,
  getModelElementFaces,
} from "./modelGeometry";

export type HoveredBlock = {
  coordinate: [number, number, number];
  blockId: string;
} | null;

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
      faceTexturePaths: completeBlockFaceTexturePaths(block.faceTexturePaths, block.texturePath),
    },
  ];
}

function modelElementKey(element: RenderModelElement): string {
  const rotation = element.rotation
    ? [
        element.rotation.origin.join(","),
        element.rotation.axis,
        element.rotation.angle,
        element.rotation.rescale,
      ].join(":")
    : "";
  const modelRotation = element.modelRotation
    ? [
        element.modelRotation.x,
        element.modelRotation.y,
        element.modelRotation.uvLock,
      ].join(":")
    : "";
  return [
    element.from.join(","),
    element.to.join(","),
    rotation,
    modelRotation,
    ...getModelElementFaces(element).flatMap((face) => [
      face,
      element.faceTexturePaths[face] ?? "",
    ]),
  ].join("\u001e");
}

export function ThreeSchemeViewer({
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
            const key = [
              modelElementKey(element),
              block.color,
              block.alpha ?? 1,
            ].join(materialKeySeparator);
            byMaterial.set(key, [...(byMaterial.get(key) ?? []), { block, element }]);
          }
        }

        for (const [key, materialInstances] of byMaterial) {
          const [_elementKey, color, alphaValue] = key.split(materialKeySeparator);
          const alpha = Number(alphaValue);
          const element = materialInstances[0].element;
          const geometry = createModelElementGeometry(element);
          const material = getModelElementFaces(element).map((face) => {
            const texturePath = element.faceTexturePaths[face];
            const texture = texturePath ? textureForPath(texturePath) : null;
            return new THREE.MeshStandardMaterial({
              color: texture ? "#ffffff" : color,
              map: texture,
              opacity: alpha,
              transparent: texture ? true : alpha < 1,
              alphaTest: texture ? 0.35 : 0,
              side: THREE.DoubleSide,
              roughness: 0.88,
              metalness: 0.04,
            });
          });
          const mesh = new THREE.InstancedMesh(geometry, material, materialInstances.length);
          const matrix = new THREE.Matrix4();
          materialInstances.forEach(({ block, element }, index) => {
            const [centerX, centerY, centerZ] = getModelElementCenter(element);
            matrix.makeTranslation(
              block.coordinate[0] + centerX,
              block.coordinate[1] + centerY,
              block.coordinate[2] + centerZ,
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
                } as CSSProperties
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
