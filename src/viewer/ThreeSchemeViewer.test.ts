import { describe, expect, it } from "vitest";

import type { RenderBlock, RenderModelElement } from "../renderViewer";
import { renderBlockInstanceKey } from "./ThreeSchemeViewer";

const element: RenderModelElement = {
  from: [0, 0, 0],
  to: [16, 16, 16],
  faceTexturePaths: {
    north: "/tmp/front.png",
  },
};

const baseBlock: RenderBlock = {
  coordinate: [0, 0, 0],
  blockId: "mod:machine",
  stageId: null,
  color: "#9aa39e",
};

describe("three scheme viewer instance grouping", () => {
  it("keeps runtime-baked and static render assets in separate instance groups", () => {
    const runtimeKey = renderBlockInstanceKey(
      {
        ...baseBlock,
        renderFidelity: "runtimeBaked",
        renderSource: "minecraft-runtime",
      },
      element,
    );
    const staticKey = renderBlockInstanceKey(
      {
        ...baseBlock,
        renderFidelity: "staticModel",
        renderSource: "mod:block/machine",
      },
      element,
    );

    expect(runtimeKey).not.toBe(staticKey);
  });
});
