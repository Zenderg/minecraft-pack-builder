export type StageOptionId = `stage:${number}` | "unassigned";

export type RenderStage = {
  id: number;
  name: string;
  order: number;
};

export type RenderBlock = {
  coordinate: [number, number, number];
  blockId: string;
  stageId: number | null;
  color: string;
  alpha?: number;
  texturePath?: string | null;
  faceTexturePaths?: FaceTexturePaths | null;
  modelElements?: RenderModelElement[] | null;
};

export type FaceTexturePaths = {
  north?: string | null;
  south?: string | null;
  east?: string | null;
  west?: string | null;
  up?: string | null;
  down?: string | null;
};

export type RenderModelElement = {
  from: [number, number, number];
  to: [number, number, number];
  rotation?: RenderModelElementRotation | null;
  modelRotation?: RenderModelRotation | null;
  faceTexturePaths: FaceTexturePaths;
  faceUvs?: FaceUvs | null;
};

export type RenderModelElementRotation = {
  origin: [number, number, number];
  axis: "x" | "y" | "z";
  angle: number;
  rescale: boolean;
};

export type RenderModelRotation = {
  x: number;
  y: number;
  uvLock: boolean;
};

export type FaceUvs = Partial<Record<keyof FaceTexturePaths, [number, number, number, number] | null>>;

export type RenderChunkSummary = {
  coordinate: [number, number, number];
  blockCount: number;
  faceCount: number;
};

export type RenderScene = {
  schemeId: number;
  schemeName: string;
  dimensions: [number, number, number];
  stages: RenderStage[];
  blocks: RenderBlock[];
  materials?: RenderMaterialLine[];
  chunks: RenderChunkSummary[];
  largeSchemeThreshold: number;
};

export type StageOption = {
  id: StageOptionId;
  label: string;
  order: number | null;
};

export type RenderSceneMetrics = {
  dimensions: string;
  visibleBlocks: number;
  totalBlocks: number;
  chunkCount: number;
  faceCount: number;
  isLargeScheme: boolean;
};

export type RenderMaterialLine = {
  blockId: string;
  displayName?: string;
  itemId?: string | null;
  maxStackSize?: number | null;
  stackCount?: number | null;
  texturePath?: string | null;
  count: number;
};

export function getStageOptions(scene: RenderScene): StageOption[] {
  return [
    ...scene.stages
      .slice()
      .sort((left, right) => left.order - right.order)
      .map((stage) => ({
        id: `stage:${stage.id}` as const,
        label: stage.name,
        order: stage.order,
      })),
    { id: "unassigned" as const, label: "Unassigned", order: null },
  ];
}

export function getDefaultStageId(scene: RenderScene): StageOptionId {
  const lastStage = scene.stages
    .slice()
    .sort((left, right) => left.order - right.order)
    .at(-1);
  return lastStage ? `stage:${lastStage.id}` : "unassigned";
}

export function getVisibleRenderBlocks(scene: RenderScene, selectedStageId: StageOptionId): RenderBlock[] {
  if (selectedStageId === "unassigned") {
    return scene.blocks.filter((block) => block.stageId === null);
  }

  const selectedId = Number(selectedStageId.replace("stage:", ""));
  const selectedOrder =
    scene.stages.find((stage) => stage.id === selectedId)?.order ?? Number.MAX_SAFE_INTEGER;
  const visibleStageIds = new Set(
    scene.stages.filter((stage) => stage.order <= selectedOrder).map((stage) => stage.id),
  );
  return scene.blocks.filter((block) => block.stageId !== null && visibleStageIds.has(block.stageId));
}

export function getRenderSceneMetrics(
  scene: RenderScene,
  selectedStageId: StageOptionId,
): RenderSceneMetrics {
  const visibleBlocks = getVisibleRenderBlocks(scene, selectedStageId);
  return {
    dimensions: scene.dimensions.join(" x "),
    visibleBlocks: visibleBlocks.length,
    totalBlocks: scene.blocks.length,
    chunkCount: scene.chunks.length,
    faceCount: scene.chunks.reduce((total, chunk) => total + chunk.faceCount, 0),
    isLargeScheme: scene.blocks.length >= scene.largeSchemeThreshold,
  };
}

export function getRenderSceneMaterials(scene: RenderScene): RenderMaterialLine[] {
  if (scene.materials) {
    return scene.materials;
  }

  const counts = new Map<string, number>();
  for (const block of scene.blocks) {
    counts.set(block.blockId, (counts.get(block.blockId) ?? 0) + 1);
  }
  return [...counts.entries()]
    .sort(([left], [right]) => left.localeCompare(right))
    .map(([blockId, count]) => ({ blockId, count }));
}
