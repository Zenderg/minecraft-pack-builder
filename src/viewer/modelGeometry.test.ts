import { describe, expect, it } from "vitest";

import { createModelElementGeometry } from "./modelGeometry";
import type { RenderModelElement } from "../renderViewer";

describe("model element geometry", () => {
  it("builds only faces declared by a Minecraft model element", () => {
    const element: RenderModelElement = {
      from: [7, 0, 0],
      to: [9, 16, 16],
      faceTexturePaths: {
        east: "/tmp/torch.png",
        west: "/tmp/torch.png",
      },
    };

    const geometry = createModelElementGeometry(element);

    expect(geometry.groups).toHaveLength(2);
    expect(geometry.index?.count).toBe(12);
    expect(geometry.attributes.position.count).toBe(8);
  });

  it("keeps model element rotation metadata on generated geometry", () => {
    const element: RenderModelElement = {
      from: [7, 0, 0],
      to: [9, 16, 16],
      rotation: {
        origin: [8, 8, 8],
        axis: "y",
        angle: 45,
        rescale: false,
      },
      faceTexturePaths: {
        east: "/tmp/torch.png",
        west: "/tmp/torch.png",
      },
    };

    const geometry = createModelElementGeometry(element);

    expect(geometry.userData.modelElementRotation).toEqual(element.rotation);
  });

  it("uses Minecraft face UVs instead of stretching the whole texture", () => {
    const element: RenderModelElement = {
      from: [7, 0, 0],
      to: [9, 16, 16],
      faceTexturePaths: {
        east: "/tmp/torch.png",
      },
      faceUvs: {
        east: [7, 6, 9, 16],
      },
    };

    const geometry = createModelElementGeometry(element);
    const uvs = Array.from(geometry.attributes.uv.array);

    expect(uvs).toEqual([
      7 / 16,
      0,
      9 / 16,
      0,
      9 / 16,
      10 / 16,
      7 / 16,
      10 / 16,
    ]);
  });

  it("applies blockstate model rotation around the block center", () => {
    const element: RenderModelElement = {
      from: [7, 3, 0],
      to: [9, 13, 2],
      modelRotation: {
        x: 0,
        y: 90,
        uvLock: true,
      },
      faceTexturePaths: {
        north: "/tmp/torch.png",
      },
    };

    const geometry = createModelElementGeometry(element);
    const box = geometry.boundingBox;

    expect(box?.min.x).toBeCloseTo(-0.5);
    expect(box?.max.x).toBeCloseTo(-0.5);
    expect(box?.min.z).toBeCloseTo(0.375);
    expect(box?.max.z).toBeCloseTo(0.5);
  });
});
