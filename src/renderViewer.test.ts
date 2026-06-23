import { describe, expect, it } from "vitest";

import {
  browserRenderSceneFixture,
  getDefaultStageId,
  getRenderSceneMetrics,
  getStageOptions,
  getVisibleRenderBlocks,
} from "./renderViewer";

describe("render viewer scene helpers", () => {
  it("uses cumulative construction stages and exposes Unassigned separately", () => {
    const scene = browserRenderSceneFixture;
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
    const metrics = getRenderSceneMetrics(browserRenderSceneFixture, "stage:2");

    expect(metrics.visibleBlocks).toBe(7);
    expect(metrics.totalBlocks).toBe(9);
    expect(metrics.dimensions).toBe("8 x 5 x 8");
    expect(metrics.chunkCount).toBeGreaterThan(0);
  });
});
