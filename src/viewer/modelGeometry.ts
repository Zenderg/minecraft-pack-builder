import * as THREE from "three";

import type { FaceTexturePaths, RenderModelElement } from "../renderViewer";

export type ModelFaceName = "east" | "west" | "up" | "down" | "south" | "north";

export const MODEL_FACE_ORDER: ModelFaceName[] = [
  "east",
  "west",
  "up",
  "down",
  "south",
  "north",
];

const FACE_DEFINITIONS: Record<
  ModelFaceName,
  {
    normal: [number, number, number];
    corners: Array<(bounds: ElementBounds) => [number, number, number]>;
  }
> = {
  east: {
    normal: [1, 0, 0],
    corners: [
      ({ x1, y0, z1 }) => [x1, y0, z1],
      ({ x1, y0, z0 }) => [x1, y0, z0],
      ({ x1, y1, z0 }) => [x1, y1, z0],
      ({ x1, y1, z1 }) => [x1, y1, z1],
    ],
  },
  west: {
    normal: [-1, 0, 0],
    corners: [
      ({ x0, y0, z0 }) => [x0, y0, z0],
      ({ x0, y0, z1 }) => [x0, y0, z1],
      ({ x0, y1, z1 }) => [x0, y1, z1],
      ({ x0, y1, z0 }) => [x0, y1, z0],
    ],
  },
  up: {
    normal: [0, 1, 0],
    corners: [
      ({ x0, y1, z1 }) => [x0, y1, z1],
      ({ x1, y1, z1 }) => [x1, y1, z1],
      ({ x1, y1, z0 }) => [x1, y1, z0],
      ({ x0, y1, z0 }) => [x0, y1, z0],
    ],
  },
  down: {
    normal: [0, -1, 0],
    corners: [
      ({ x0, y0, z0 }) => [x0, y0, z0],
      ({ x1, y0, z0 }) => [x1, y0, z0],
      ({ x1, y0, z1 }) => [x1, y0, z1],
      ({ x0, y0, z1 }) => [x0, y0, z1],
    ],
  },
  south: {
    normal: [0, 0, 1],
    corners: [
      ({ x0, y0, z1 }) => [x0, y0, z1],
      ({ x1, y0, z1 }) => [x1, y0, z1],
      ({ x1, y1, z1 }) => [x1, y1, z1],
      ({ x0, y1, z1 }) => [x0, y1, z1],
    ],
  },
  north: {
    normal: [0, 0, -1],
    corners: [
      ({ x1, y0, z0 }) => [x1, y0, z0],
      ({ x0, y0, z0 }) => [x0, y0, z0],
      ({ x0, y1, z0 }) => [x0, y1, z0],
      ({ x1, y1, z0 }) => [x1, y1, z0],
    ],
  },
};

type ElementBounds = {
  x0: number;
  x1: number;
  y0: number;
  y1: number;
  z0: number;
  z1: number;
};

export function getModelElementFaces(element: RenderModelElement): ModelFaceName[] {
  const declaredFaces = MODEL_FACE_ORDER.filter((face) =>
    Object.prototype.hasOwnProperty.call(element.faceTexturePaths, face),
  );
  return declaredFaces.length ? declaredFaces : MODEL_FACE_ORDER;
}

export function completeBlockFaceTexturePaths(
  paths: FaceTexturePaths | null | undefined,
  texturePath: string | null | undefined,
): FaceTexturePaths {
  return Object.fromEntries(
    MODEL_FACE_ORDER.map((face) => [face, paths?.[face] ?? texturePath ?? null]),
  ) as FaceTexturePaths;
}

export function createModelElementGeometry(element: RenderModelElement): THREE.BufferGeometry {
  const geometry = new THREE.BufferGeometry();
  const faces = getModelElementFaces(element);
  const center = elementCenter(element);
  const bounds = elementBounds(element, center);
  const positions: number[] = [];
  const normals: number[] = [];
  const uvs: number[] = [];
  const indices: number[] = [];

  faces.forEach((face, materialIndex) => {
    const definition = FACE_DEFINITIONS[face];
    const vertexOffset = positions.length / 3;
    for (const corner of definition.corners) {
      positions.push(...corner(bounds));
      normals.push(...definition.normal);
    }
    uvs.push(...textureUvsForFace(element.faceUvs?.[face]));
    indices.push(vertexOffset, vertexOffset + 1, vertexOffset + 2);
    indices.push(vertexOffset, vertexOffset + 2, vertexOffset + 3);
    geometry.addGroup(materialIndex * 6, 6, materialIndex);
  });

  geometry.setAttribute("position", new THREE.Float32BufferAttribute(positions, 3));
  geometry.setAttribute("normal", new THREE.Float32BufferAttribute(normals, 3));
  geometry.setAttribute("uv", new THREE.Float32BufferAttribute(uvs, 2));
  geometry.setIndex(indices);
  applyElementRotation(geometry, element, center);
  applyModelRotation(geometry, element, center);
  geometry.computeBoundingBox();
  geometry.computeBoundingSphere();
  return geometry;
}

function textureUvsForFace(uv: [number, number, number, number] | null | undefined): number[] {
  if (!uv) {
    return [0, 0, 1, 0, 1, 1, 0, 1];
  }
  const [u0, v0, u1, v1] = uv;
  const left = u0 / 16;
  const right = u1 / 16;
  const top = 1 - v0 / 16;
  const bottom = 1 - v1 / 16;
  return [left, bottom, right, bottom, right, top, left, top];
}

export function getModelElementCenter(element: RenderModelElement): [number, number, number] {
  return [
    (element.from[0] + element.to[0]) / 32,
    (element.from[1] + element.to[1]) / 32,
    (element.from[2] + element.to[2]) / 32,
  ];
}

function elementCenter(element: RenderModelElement): THREE.Vector3 {
  return new THREE.Vector3(...getModelElementCenter(element));
}

function elementBounds(element: RenderModelElement, center: THREE.Vector3): ElementBounds {
  return {
    x0: element.from[0] / 16 - center.x,
    x1: element.to[0] / 16 - center.x,
    y0: element.from[1] / 16 - center.y,
    y1: element.to[1] / 16 - center.y,
    z0: element.from[2] / 16 - center.z,
    z1: element.to[2] / 16 - center.z,
  };
}

function applyElementRotation(
  geometry: THREE.BufferGeometry,
  element: RenderModelElement,
  center: THREE.Vector3,
) {
  const rotation = element.rotation;
  if (!rotation) {
    return;
  }

  const origin = new THREE.Vector3(
    rotation.origin[0] / 16 - center.x,
    rotation.origin[1] / 16 - center.y,
    rotation.origin[2] / 16 - center.z,
  );
  const axis = new THREE.Vector3(
    rotation.axis === "x" ? 1 : 0,
    rotation.axis === "y" ? 1 : 0,
    rotation.axis === "z" ? 1 : 0,
  );
  const matrix = new THREE.Matrix4()
    .makeTranslation(origin.x, origin.y, origin.z)
    .multiply(new THREE.Matrix4().makeRotationAxis(axis, THREE.MathUtils.degToRad(rotation.angle)))
    .multiply(rescaleMatrix(rotation.axis, rotation.angle, rotation.rescale))
    .multiply(new THREE.Matrix4().makeTranslation(-origin.x, -origin.y, -origin.z));

  geometry.applyMatrix4(matrix);
  geometry.userData.modelElementRotation = rotation;
}

function applyModelRotation(
  geometry: THREE.BufferGeometry,
  element: RenderModelElement,
  center: THREE.Vector3,
) {
  const rotation = element.modelRotation;
  if (!rotation) {
    return;
  }

  const origin = new THREE.Vector3(0.5 - center.x, 0.5 - center.y, 0.5 - center.z);
  const matrix = new THREE.Matrix4()
    .makeTranslation(origin.x, origin.y, origin.z)
    .multiply(new THREE.Matrix4().makeRotationY(THREE.MathUtils.degToRad(rotation.y)))
    .multiply(new THREE.Matrix4().makeRotationX(THREE.MathUtils.degToRad(rotation.x)))
    .multiply(new THREE.Matrix4().makeTranslation(-origin.x, -origin.y, -origin.z));

  geometry.applyMatrix4(matrix);
  geometry.userData.modelRotation = rotation;
}

function rescaleMatrix(
  axis: "x" | "y" | "z",
  angle: number,
  shouldRescale: boolean,
): THREE.Matrix4 {
  if (!shouldRescale) {
    return new THREE.Matrix4();
  }
  const radians = THREE.MathUtils.degToRad(Math.abs(angle));
  const scale = 1 / Math.max(Math.cos(radians), 0.0001);
  return new THREE.Matrix4().makeScale(
    axis === "x" ? 1 : scale,
    axis === "y" ? 1 : scale,
    axis === "z" ? 1 : scale,
  );
}
