import { describe, expect, it } from "vitest";

import {
  getDefaultStageId,
  getRenderSceneMaterials,
  getRenderSceneMetrics,
  getStageOptions,
  getVisibleRenderBlocks,
  type RenderScene,
} from "./renderViewer";

const renderSceneFixture: RenderScene = {
  schemeId: 10,
  schemeName: "Render Fixture",
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
      texturePath: "/tmp/stone_bricks.png",
    },
    {
      coordinate: [1, 0, 0],
      blockId: "minecraft:stone_bricks",
      stageId: 1,
      color: "#9aa39e",
      texturePath: "/tmp/stone_bricks.png",
    },
    {
      coordinate: [2, 0, 0],
      blockId: "minecraft:stone_bricks",
      stageId: 1,
      color: "#9aa39e",
      texturePath: "/tmp/stone_bricks.png",
    },
    { coordinate: [1, 1, 0], blockId: "thermal:machine_frame", stageId: 2, color: "#d3a44e" },
    { coordinate: [2, 1, 0], blockId: "thermal:machine_frame", stageId: 2, color: "#d3a44e" },
    { coordinate: [3, 0, 0], blockId: "create:andesite_casing", stageId: 2, color: "#6bb48f" },
    { coordinate: [3, 1, 0], blockId: "minecraft:glass", stageId: 2, color: "#9bd8ff", alpha: 0.58 },
    { coordinate: [4, 0, 1], blockId: "create:andesite_casing", stageId: null, color: "#6bb48f" },
    { coordinate: [4, 1, 1], blockId: "minecraft:glass", stageId: null, color: "#9bd8ff", alpha: 0.58 },
  ],
  chunks: [
    { coordinate: [0, 0, 0], blockCount: 7, faceCount: 34 },
    { coordinate: [1, 0, 0], blockCount: 2, faceCount: 12 },
  ],
  largeSchemeThreshold: 4096,
};

describe("render viewer scene helpers", () => {
  it("uses cumulative construction stages and exposes Unassigned separately", () => {
    const scene = renderSceneFixture;
    const stages = getStageOptions(scene);

    expect(stages.map((stage) => stage.id)).toEqual(["stage:1", "stage:2", "unassigned"]);
    expect(getDefaultStageId(scene)).toBe("stage:2");
    expect(getVisibleRenderBlocks(scene, "stage:1").map((block) => block.coordinate)).toEqual([
      [0, 0, 0],
      [1, 0, 0],
      [2, 0, 0],
    ]);
    expect(getVisibleRenderBlocks(scene, "stage:2")).toHaveLength(7);
    expect(getVisibleRenderBlocks(scene, "unassigned").map((block) => block.stageId)).toEqual([
      null,
      null,
    ]);
  });

  it("reports viewer metrics for the selected stage", () => {
    const metrics = getRenderSceneMetrics(renderSceneFixture, "stage:2");

    expect(metrics.visibleBlocks).toBe(7);
    expect(metrics.totalBlocks).toBe(9);
    expect(metrics.dimensions).toBe("8 x 5 x 8");
    expect(metrics.chunkCount).toBeGreaterThan(0);
  });

  it("uses enriched registry materials with stack counts only when max stack size is known", () => {
    const sceneWithMaterials: RenderScene = {
      ...renderSceneFixture,
      materials: [
        {
          blockId: "minecraft:stone",
          displayName: "Stone",
          count: 65,
          itemId: "minecraft:stone",
          maxStackSize: 64,
          stackCount: 2,
          texturePath: "/tmp/stone.png",
        },
        {
          blockId: "create:andesite_casing",
          displayName: "Andesite Casing",
          count: 3,
          itemId: "create:andesite_casing",
          maxStackSize: null,
          stackCount: null,
          texturePath: null,
        },
      ],
    };

    expect(getRenderSceneMaterials(sceneWithMaterials)).toEqual(sceneWithMaterials.materials);
  });
});
