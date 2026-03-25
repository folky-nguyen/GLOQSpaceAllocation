import {
  getSpaceBoundsFt,
  triangulateFootprint,
  type ProjectDoc,
  type Point2Ft
} from "./project-doc";
import type { Selection } from "./ui-store";

export type SpacePrismEmphasis = "normal" | "active-level" | "selected";
export type ThreeDVisibilityMode = "active-floor-only" | "all-levels";

export type SceneVertex = {
  position: [number, number, number];
  color: [number, number, number, number];
};

export type SpacePrismRenderItem = {
  id: string;
  levelId: string;
  name: string;
  emphasis: SpacePrismEmphasis;
};

export type SceneExtents = {
  minXFt: number;
  minYFt: number;
  minZFt: number;
  maxXFt: number;
  maxYFt: number;
  maxZFt: number;
};

export type SpaceScenePayload = {
  items: SpacePrismRenderItem[];
  vertices: SceneVertex[];
  edgeVertices: SceneVertex[];
  extents: SceneExtents;
  hasVisibleItems: boolean;
};

export type OrbitCamera = {
  targetXFt: number;
  targetYFt: number;
  targetZFt: number;
  distanceFt: number;
  yawDeg: number;
  pitchDeg: number;
};

export type CameraFrame = {
  eye: [number, number, number];
  forward: [number, number, number];
  right: [number, number, number];
  up: [number, number, number];
};

const DEFAULT_CAMERA_DISTANCE_FT = 60;
const DEFAULT_YAW_DEG = -35;
const DEFAULT_PITCH_DEG = 35;
const MIN_CAMERA_DISTANCE_FT = 8;
const DEFAULT_SCENE_EXTENTS: SceneExtents = {
  minXFt: 0,
  minYFt: 0,
  minZFt: 0,
  maxXFt: 0,
  maxYFt: 0,
  maxZFt: 0
};

function clamp(value: number, min: number, max: number): number {
  return Math.min(max, Math.max(min, value));
}

function degreesToRadians(value: number): number {
  return value * (Math.PI / 180);
}

function subtractVectors(left: [number, number, number], right: [number, number, number]): [number, number, number] {
  return [left[0] - right[0], left[1] - right[1], left[2] - right[2]];
}

function crossVectors(left: [number, number, number], right: [number, number, number]): [number, number, number] {
  return [
    left[1] * right[2] - left[2] * right[1],
    left[2] * right[0] - left[0] * right[2],
    left[0] * right[1] - left[1] * right[0]
  ];
}

function dotVectors(left: [number, number, number], right: [number, number, number]): number {
  return left[0] * right[0] + left[1] * right[1] + left[2] * right[2];
}

function normalizeVector(vector: [number, number, number]): [number, number, number] {
  const length = Math.hypot(vector[0], vector[1], vector[2]);

  if (length <= Number.EPSILON) {
    return [0, 0, 0];
  }

  return [vector[0] / length, vector[1] / length, vector[2] / length];
}

function multiplyMatrices(left: number[], right: number[]): number[] {
  const result = new Array<number>(16).fill(0);

  for (let column = 0; column < 4; column += 1) {
    for (let row = 0; row < 4; row += 1) {
      result[column * 4 + row] =
        left[row] * right[column * 4] +
        left[4 + row] * right[column * 4 + 1] +
        left[8 + row] * right[column * 4 + 2] +
        left[12 + row] * right[column * 4 + 3];
    }
  }

  return result;
}

function getPerspectiveProjectionMatrix(aspectRatio: number, nearFt: number, farFt: number): number[] {
  const safeAspect = Number.isFinite(aspectRatio) && aspectRatio > 0 ? aspectRatio : 1;
  const safeNear = Math.max(nearFt, 0.01);
  const safeFar = Math.max(farFt, safeNear + 1);
  const f = 1 / Math.tan(degreesToRadians(45) / 2);
  const inverseRange = 1 / (safeNear - safeFar);

  return [
    f / safeAspect, 0, 0, 0,
    0, f, 0, 0,
    0, 0, safeFar * inverseRange, -1,
    0, 0, safeFar * safeNear * inverseRange, 0
  ];
}

function getLookAtMatrix(eye: [number, number, number], target: [number, number, number]): number[] {
  const worldUp: [number, number, number] = [0, 0, 1];
  const zAxis = normalizeVector(subtractVectors(eye, target));
  const xAxis = normalizeVector(crossVectors(worldUp, zAxis));
  const yAxis = normalizeVector(crossVectors(zAxis, xAxis));

  return [
    xAxis[0], yAxis[0], zAxis[0], 0,
    xAxis[1], yAxis[1], zAxis[1], 0,
    xAxis[2], yAxis[2], zAxis[2], 0,
    -dotVectors(xAxis, eye), -dotVectors(yAxis, eye), -dotVectors(zAxis, eye), 1
  ];
}

function getEmphasis(selection: Selection, activeLevelId: string | null, spaceId: string, levelId: string): SpacePrismEmphasis {
  if (selection?.kind === "space" && selection.id === spaceId) {
    return "selected";
  }

  if (selection?.kind === "space-set" && selection.ids.includes(spaceId)) {
    return "selected";
  }

  return levelId === activeLevelId ? "active-level" : "normal";
}

function scaleColor(color: [number, number, number, number], factor: number): [number, number, number, number] {
  return [
    Math.min(color[0] * factor, 1),
    Math.min(color[1] * factor, 1),
    Math.min(color[2] * factor, 1),
    color[3]
  ];
}

function getBaseColor(emphasis: SpacePrismEmphasis): [number, number, number, number] {
  if (emphasis === "selected") {
    return [0.97, 0.7, 0.3, 1];
  }

  if (emphasis === "active-level") {
    return [0.42, 0.72, 0.96, 1];
  }

  return [0.48, 0.58, 0.7, 1];
}

function getEdgeColor(emphasis: SpacePrismEmphasis): [number, number, number, number] {
  if (emphasis === "selected") {
    return [1, 0.9, 0.52, 1];
  }

  if (emphasis === "active-level") {
    return [0.9, 0.97, 1, 1];
  }

  return [0.76, 0.84, 0.93, 1];
}

function pushTriangle(
  vertices: SceneVertex[],
  a: [number, number, number],
  b: [number, number, number],
  c: [number, number, number],
  color: [number, number, number, number]
): void {
  vertices.push({ position: a, color });
  vertices.push({ position: b, color });
  vertices.push({ position: c, color });
}

function pushLine(
  vertices: SceneVertex[],
  start: [number, number, number],
  end: [number, number, number],
  color: [number, number, number, number]
): void {
  vertices.push({ position: start, color });
  vertices.push({ position: end, color });
}

function toPosition(point: Point2Ft, zFt: number): [number, number, number] {
  return [point.xFt, point.yFt, zFt];
}

function appendPolygonPrismVertices(
  vertices: SceneVertex[],
  footprint: Point2Ft[],
  minZFt: number,
  sizeZFt: number,
  emphasis: SpacePrismEmphasis
): void {
  const topZFt = minZFt + sizeZFt;
  const triangles = triangulateFootprint(footprint);
  const baseColor = getBaseColor(emphasis);
  const topColor = scaleColor(baseColor, 1.08);
  const bottomColor = scaleColor(baseColor, 0.52);

  for (const triangle of triangles) {
    pushTriangle(
      vertices,
      toPosition(triangle[0], topZFt),
      toPosition(triangle[1], topZFt),
      toPosition(triangle[2], topZFt),
      topColor
    );
    pushTriangle(
      vertices,
      toPosition(triangle[2], minZFt),
      toPosition(triangle[1], minZFt),
      toPosition(triangle[0], minZFt),
      bottomColor
    );
  }

  for (let index = 0; index < footprint.length; index += 1) {
    const current = footprint[index];
    const next = footprint[(index + 1) % footprint.length];
    const sideColor = scaleColor(baseColor, 0.78 + (index % 3) * 0.08);

    pushTriangle(
      vertices,
      toPosition(current, minZFt),
      toPosition(current, topZFt),
      toPosition(next, topZFt),
      sideColor
    );
    pushTriangle(
      vertices,
      toPosition(current, minZFt),
      toPosition(next, topZFt),
      toPosition(next, minZFt),
      sideColor
    );
  }
}

function appendPolygonPrismEdges(
  edgeVertices: SceneVertex[],
  footprint: Point2Ft[],
  minZFt: number,
  sizeZFt: number,
  emphasis: SpacePrismEmphasis
): void {
  const topZFt = minZFt + sizeZFt;
  const edgeColor = getEdgeColor(emphasis);

  for (let index = 0; index < footprint.length; index += 1) {
    const current = footprint[index];
    const next = footprint[(index + 1) % footprint.length];
    const currentBottom = toPosition(current, minZFt);
    const currentTop = toPosition(current, topZFt);

    pushLine(edgeVertices, currentBottom, toPosition(next, minZFt), edgeColor);
    pushLine(edgeVertices, currentTop, toPosition(next, topZFt), edgeColor);
    pushLine(edgeVertices, currentBottom, currentTop, edgeColor);
  }
}

function shouldIncludeSpace(
  visibilityMode: ThreeDVisibilityMode,
  activeLevelId: string | null,
  spaceLevelId: string
): boolean {
  if (visibilityMode === "all-levels") {
    return true;
  }

  if (!activeLevelId) {
    return true;
  }

  return spaceLevelId === activeLevelId;
}

export function buildSpaceScenePayload(
  doc: ProjectDoc,
  input: { activeLevelId: string | null; selection: Selection; visibilityMode: ThreeDVisibilityMode }
): SpaceScenePayload {
  const levelsById = new Map(doc.levels.map((level) => [level.id, level]));
  const items: SpacePrismRenderItem[] = [];
  const vertices: SceneVertex[] = [];
  const edgeVertices: SceneVertex[] = [];
  let extents: SceneExtents | null = null;

  for (const space of doc.spaces) {
    const level = levelsById.get(space.levelId);

    if (!level || !shouldIncludeSpace(input.visibilityMode, input.activeLevelId, space.levelId)) {
      continue;
    }

    const emphasis = getEmphasis(input.selection, input.activeLevelId, space.id, space.levelId);
    const bounds = getSpaceBoundsFt(space);
    items.push({
      id: space.id,
      levelId: space.levelId,
      name: space.name,
      emphasis
    });
    appendPolygonPrismVertices(vertices, space.footprint, level.elevationFt, level.heightFt, emphasis);
    appendPolygonPrismEdges(edgeVertices, space.footprint, level.elevationFt, level.heightFt, emphasis);

    if (!extents) {
      extents = {
        minXFt: bounds.minXFt,
        minYFt: bounds.minYFt,
        minZFt: level.elevationFt,
        maxXFt: bounds.maxXFt,
        maxYFt: bounds.maxYFt,
        maxZFt: level.elevationFt + level.heightFt
      };
      continue;
    }

    extents = {
      minXFt: Math.min(extents.minXFt, bounds.minXFt),
      minYFt: Math.min(extents.minYFt, bounds.minYFt),
      minZFt: Math.min(extents.minZFt, level.elevationFt),
      maxXFt: Math.max(extents.maxXFt, bounds.maxXFt),
      maxYFt: Math.max(extents.maxYFt, bounds.maxYFt),
      maxZFt: Math.max(extents.maxZFt, level.elevationFt + level.heightFt)
    };
  }

  if (!extents || items.length === 0) {
    return {
      items: [],
      vertices: [],
      edgeVertices: [],
      extents: DEFAULT_SCENE_EXTENTS,
      hasVisibleItems: false
    };
  }

  return {
    items,
    vertices,
    edgeVertices,
    extents,
    hasVisibleItems: true
  };
}

export function getDefaultOrbitCamera(scene: SpaceScenePayload): OrbitCamera {
  if (!scene.hasVisibleItems) {
    return {
      targetXFt: 0,
      targetYFt: 0,
      targetZFt: 0,
      distanceFt: DEFAULT_CAMERA_DISTANCE_FT,
      yawDeg: DEFAULT_YAW_DEG,
      pitchDeg: DEFAULT_PITCH_DEG
    };
  }

  const widthFt = scene.extents.maxXFt - scene.extents.minXFt;
  const depthFt = scene.extents.maxYFt - scene.extents.minYFt;
  const heightFt = scene.extents.maxZFt - scene.extents.minZFt;
  const largestDimensionFt = Math.max(widthFt, depthFt, heightFt, 1);

  return {
    targetXFt: (scene.extents.minXFt + scene.extents.maxXFt) / 2,
    targetYFt: (scene.extents.minYFt + scene.extents.maxYFt) / 2,
    targetZFt: (scene.extents.minZFt + scene.extents.maxZFt) / 2,
    distanceFt: Math.max(largestDimensionFt * 2.25, MIN_CAMERA_DISTANCE_FT),
    yawDeg: DEFAULT_YAW_DEG,
    pitchDeg: DEFAULT_PITCH_DEG
  };
}

export function getOrbitCameraFrame(camera: OrbitCamera): CameraFrame {
  const yawRadians = degreesToRadians(camera.yawDeg);
  const pitchRadians = degreesToRadians(clamp(camera.pitchDeg, 10, 80));
  const safeDistanceFt = Math.max(camera.distanceFt, MIN_CAMERA_DISTANCE_FT);
  const cosPitch = Math.cos(pitchRadians);
  const eye: [number, number, number] = [
    camera.targetXFt + Math.cos(yawRadians) * cosPitch * safeDistanceFt,
    camera.targetYFt + Math.sin(yawRadians) * cosPitch * safeDistanceFt,
    camera.targetZFt + Math.sin(pitchRadians) * safeDistanceFt
  ];
  const target: [number, number, number] = [camera.targetXFt, camera.targetYFt, camera.targetZFt];
  const forward = normalizeVector(subtractVectors(target, eye));
  const worldUp: [number, number, number] = [0, 0, 1];
  const right = normalizeVector(crossVectors(forward, worldUp));
  const up = normalizeVector(crossVectors(right, forward));

  return { eye, forward, right, up };
}

export function getOrbitCameraViewProjectionMatrix(camera: OrbitCamera, aspectRatio: number): number[] {
  const frame = getOrbitCameraFrame(camera);
  const viewMatrix = getLookAtMatrix(frame.eye, [camera.targetXFt, camera.targetYFt, camera.targetZFt]);
  const farFt = Math.max(camera.distanceFt * 8, 200);
  const projectionMatrix = getPerspectiveProjectionMatrix(aspectRatio, 0.1, farFt);
  return multiplyMatrices(projectionMatrix, viewMatrix);
}
