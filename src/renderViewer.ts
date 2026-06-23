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
};

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
  count: number;
};

export const browserRenderSceneFixture: RenderScene = {
  schemeId: 10,
  schemeName: "Starter Factory",
  dimensions: [8, 5, 8],
  stages: [
    { id: 1, name: "Stage 1", order: 1 },
    { id: 2, name: "Stage 2", order: 2 },
  ],
  blocks: [
    {
      coordinate: [0, 0, 0],
      blockId: "minecraft:stone_bricks",
      stageId: 1,
      color: "#9aa39e",
    },
    {
      coordinate: [1, 0, 0],
      blockId: "minecraft:stone_bricks",
      stageId: 1,
      color: "#9aa39e",
    },
    {
      coordinate: [2, 0, 0],
      blockId: "minecraft:stone_bricks",
      stageId: 1,
      color: "#9aa39e",
    },
    {
      coordinate: [1, 1, 0],
      blockId: "thermal:machine_frame",
      stageId: 2,
      color: "#d3a44e",
    },
    {
      coordinate: [2, 1, 0],
      blockId: "thermal:machine_frame",
      stageId: 2,
      color: "#d3a44e",
    },
    {
      coordinate: [3, 0, 0],
      blockId: "create:andesite_casing",
      stageId: 2,
      color: "#6bb48f",
    },
    {
      coordinate: [3, 1, 0],
      blockId: "minecraft:glass",
      stageId: 2,
      color: "#9bd8ff",
      alpha: 0.58,
    },
    {
      coordinate: [4, 0, 1],
      blockId: "create:andesite_casing",
      stageId: null,
      color: "#6bb48f",
    },
    {
      coordinate: [4, 1, 1],
      blockId: "minecraft:glass",
      stageId: null,
      color: "#9bd8ff",
      alpha: 0.58,
    },
  ],
  chunks: [
    { coordinate: [0, 0, 0], blockCount: 7, faceCount: 34 },
    { coordinate: [1, 0, 0], blockCount: 2, faceCount: 12 },
  ],
  largeSchemeThreshold: 4096,
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
  const counts = new Map<string, number>();
  for (const block of scene.blocks) {
    counts.set(block.blockId, (counts.get(block.blockId) ?? 0) + 1);
  }
  return [...counts.entries()]
    .sort(([left], [right]) => left.localeCompare(right))
    .map(([blockId, count]) => ({ blockId, count }));
}
