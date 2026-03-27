export const DEFAULT_STORY_HEIGHT_FT = 10;
export const DEFAULT_SITE_SETBACK_FT = 5;

const GEOMETRY_EPSILON = 1e-6;
const HALF_PLANE_EPSILON = 1e-4;

export type Point2Ft = {
  xFt: number;
  yFt: number;
};

export type PolygonBoundsFt = {
  minXFt: number;
  minYFt: number;
  maxXFt: number;
  maxYFt: number;
  widthFt: number;
  depthFt: number;
};

export type Level = {
  id: string;
  name: string;
  elevationFt: number;
  heightFt: number;
};

export type Space = {
  id: string;
  levelId: string;
  name: string;
  footprint: Point2Ft[];
};

export type SitePlan = {
  levelId: string;
  boundary: Point2Ft[];
  edgeSetbacksFt: number[];
};

export type SiteEdge = {
  index: number;
  start: Point2Ft;
  end: Point2Ft;
  midpoint: Point2Ft;
  lengthFt: number;
  setbackFt: number;
};

export type DerivedFootprintResult =
  | { footprint: Point2Ft[]; error: null }
  | { footprint: null; error: string };

export type ProjectDoc = {
  id: string;
  name: string;
  defaultStoryHeightFt: number;
  levels: Level[];
  spaces: Space[];
  sitePlan?: SitePlan | null;
};

export type AutoGenerateLevelsInput = {
  storiesBelowGrade: number;
  storiesOnGrade: number;
  storyHeightFt: number;
};

export type LevelMutationResult = {
  doc: ProjectDoc;
  activeLevelId: string;
};

type GeneratedLevelSpec = {
  name: string;
  elevationFt: number;
  heightFt: number;
};

function getPositiveFeetOrFallback(value: number, fallback: number): number {
  return Number.isFinite(value) && value > 0 ? value : fallback;
}

function getStoryCount(value: number): number {
  if (!Number.isFinite(value)) {
    return 0;
  }

  return Math.max(0, Math.floor(value));
}

function createLevelIdFactory(levels: Level[]): () => string {
  const usedIds = new Set(levels.map((level) => level.id));
  let nextId = levels.reduce((highestId, level) => {
    const match = /^level-(\d+)$/.exec(level.id);
    return match ? Math.max(highestId, Number(match[1])) : highestId;
  }, 0) + 1;

  return () => {
    while (usedIds.has(`level-${nextId}`)) {
      nextId += 1;
    }

    const id = `level-${nextId}`;
    usedIds.add(id);
    nextId += 1;
    return id;
  };
}

function getNextManualLevelName(levels: Level[]): string {
  const usedNames = new Set(levels.map((level) => level.name));
  let nextIndex = 1;

  while (usedNames.has(`Level ${nextIndex}`)) {
    nextIndex += 1;
  }

  return `Level ${nextIndex}`;
}

function swapItems<T>(items: T[], leftIndex: number, rightIndex: number): T[] {
  const nextItems = [...items];
  const leftItem = nextItems[leftIndex];

  nextItems[leftIndex] = nextItems[rightIndex];
  nextItems[rightIndex] = leftItem;

  return nextItems;
}

function buildGeneratedLevelSpecs(input: AutoGenerateLevelsInput): GeneratedLevelSpec[] {
  const storiesBelowGrade = getStoryCount(input.storiesBelowGrade);
  const storiesOnGrade = getStoryCount(input.storiesOnGrade);
  const storyHeightFt = getPositiveFeetOrFallback(input.storyHeightFt, DEFAULT_STORY_HEIGHT_FT);
  const levels: GeneratedLevelSpec[] = [];

  for (let story = storiesBelowGrade; story >= 1; story -= 1) {
    levels.push({
      name: `Basement ${story}`,
      elevationFt: -story * storyHeightFt,
      heightFt: storyHeightFt
    });
  }

  for (let story = 1; story <= storiesOnGrade; story += 1) {
    levels.push({
      name: `Level ${story}`,
      elevationFt: (story - 1) * storyHeightFt,
      heightFt: storyHeightFt
    });
  }

  if (levels.length === 0) {
    levels.push({
      name: "Level 1",
      elevationFt: 0,
      heightFt: storyHeightFt
    });
  }

  return levels;
}

function getPreferredGeneratedLevelName(levels: GeneratedLevelSpec[]): string {
  return levels.find((level) => level.name === "Level 1")?.name
    ?? levels.at(-1)?.name
    ?? "Level 1";
}

function pointsMatch(left: Point2Ft, right: Point2Ft): boolean {
  return Math.abs(left.xFt - right.xFt) <= GEOMETRY_EPSILON
    && Math.abs(left.yFt - right.yFt) <= GEOMETRY_EPSILON;
}

function clampSiteSetbackFt(value: number): number {
  if (!Number.isFinite(value)) {
    return DEFAULT_SITE_SETBACK_FT;
  }

  return Math.max(0, value);
}

function subtractPoint(left: Point2Ft, right: Point2Ft): Point2Ft {
  return {
    xFt: left.xFt - right.xFt,
    yFt: left.yFt - right.yFt
  };
}

function addScaledPoint(point: Point2Ft, direction: Point2Ft, scale: number): Point2Ft {
  return {
    xFt: point.xFt + direction.xFt * scale,
    yFt: point.yFt + direction.yFt * scale
  };
}

function dotProduct(left: Point2Ft, right: Point2Ft): number {
  return left.xFt * right.xFt + left.yFt * right.yFt;
}

function crossVector(left: Point2Ft, right: Point2Ft): number {
  return left.xFt * right.yFt - left.yFt * right.xFt;
}

function intersectLines(
  firstPoint: Point2Ft,
  firstDirection: Point2Ft,
  secondPoint: Point2Ft,
  secondDirection: Point2Ft
): Point2Ft | null {
  const denominator = crossVector(firstDirection, secondDirection);

  if (Math.abs(denominator) <= GEOMETRY_EPSILON) {
    return null;
  }

  const delta = subtractPoint(secondPoint, firstPoint);
  const distance = crossVector(delta, secondDirection) / denominator;
  return addScaledPoint(firstPoint, firstDirection, distance);
}

export function createRectangleFootprint(xFt: number, yFt: number, widthFt: number, depthFt: number): Point2Ft[] {
  return [
    { xFt, yFt },
    { xFt: xFt + widthFt, yFt },
    { xFt: xFt + widthFt, yFt: yFt + depthFt },
    { xFt, yFt: yFt + depthFt }
  ];
}

export function normalizeFootprint(footprint: Point2Ft[]): Point2Ft[] {
  const normalizedPoints: Point2Ft[] = [];

  for (const point of footprint) {
    if (!Number.isFinite(point.xFt) || !Number.isFinite(point.yFt)) {
      continue;
    }

    if (normalizedPoints.length > 0 && pointsMatch(normalizedPoints[normalizedPoints.length - 1], point)) {
      continue;
    }

    normalizedPoints.push({ xFt: point.xFt, yFt: point.yFt });
  }

  if (normalizedPoints.length >= 2 && pointsMatch(normalizedPoints[0], normalizedPoints[normalizedPoints.length - 1])) {
    normalizedPoints.pop();
  }

  return normalizedPoints;
}

export function getPolygonSignedAreaSqFt(footprint: Point2Ft[]): number {
  const normalizedPoints = normalizeFootprint(footprint);

  if (normalizedPoints.length < 3) {
    return 0;
  }

  let doubleArea = 0;

  for (let index = 0; index < normalizedPoints.length; index += 1) {
    const current = normalizedPoints[index];
    const next = normalizedPoints[(index + 1) % normalizedPoints.length];
    doubleArea += current.xFt * next.yFt - next.xFt * current.yFt;
  }

  return doubleArea / 2;
}

export function getPolygonAreaSqFt(footprint: Point2Ft[]): number {
  return Math.abs(getPolygonSignedAreaSqFt(footprint));
}

export function getPolygonBoundsFt(footprint: Point2Ft[]): PolygonBoundsFt {
  const normalizedPoints = normalizeFootprint(footprint);

  if (normalizedPoints.length === 0) {
    return {
      minXFt: 0,
      minYFt: 0,
      maxXFt: 0,
      maxYFt: 0,
      widthFt: 0,
      depthFt: 0
    };
  }

  const bounds = normalizedPoints.reduce(
    (currentBounds, point) => ({
      minXFt: Math.min(currentBounds.minXFt, point.xFt),
      minYFt: Math.min(currentBounds.minYFt, point.yFt),
      maxXFt: Math.max(currentBounds.maxXFt, point.xFt),
      maxYFt: Math.max(currentBounds.maxYFt, point.yFt)
    }),
    {
      minXFt: Number.POSITIVE_INFINITY,
      minYFt: Number.POSITIVE_INFINITY,
      maxXFt: Number.NEGATIVE_INFINITY,
      maxYFt: Number.NEGATIVE_INFINITY
    }
  );

  return {
    ...bounds,
    widthFt: bounds.maxXFt - bounds.minXFt,
    depthFt: bounds.maxYFt - bounds.minYFt
  };
}

export function getPolygonCentroidFt(footprint: Point2Ft[]): Point2Ft {
  const normalizedPoints = normalizeFootprint(footprint);
  const signedArea = getPolygonSignedAreaSqFt(normalizedPoints);

  if (normalizedPoints.length < 3 || Math.abs(signedArea) <= GEOMETRY_EPSILON) {
    const bounds = getPolygonBoundsFt(normalizedPoints);
    return {
      xFt: bounds.minXFt + bounds.widthFt / 2,
      yFt: bounds.minYFt + bounds.depthFt / 2
    };
  }

  let centroidXTimesSixArea = 0;
  let centroidYTimesSixArea = 0;

  for (let index = 0; index < normalizedPoints.length; index += 1) {
    const current = normalizedPoints[index];
    const next = normalizedPoints[(index + 1) % normalizedPoints.length];
    const cross = current.xFt * next.yFt - next.xFt * current.yFt;
    centroidXTimesSixArea += (current.xFt + next.xFt) * cross;
    centroidYTimesSixArea += (current.yFt + next.yFt) * cross;
  }

  return {
    xFt: centroidXTimesSixArea / (6 * signedArea),
    yFt: centroidYTimesSixArea / (6 * signedArea)
  };
}

export function createDefaultSiteEdgeSetbacksFt(edgeCount: number): number[] {
  return Array.from({ length: Math.max(0, edgeCount) }, () => DEFAULT_SITE_SETBACK_FT);
}

export function repairSitePlanGeometry(sitePlan: SitePlan | null | undefined): SitePlan | null {
  if (!sitePlan) {
    return null;
  }

  const boundary = normalizeFootprint(sitePlan.boundary);

  if (boundary.length < 3) {
    return null;
  }

  const fallbackSetbacks = createDefaultSiteEdgeSetbacksFt(boundary.length);

  return {
    levelId: typeof sitePlan.levelId === "string" ? sitePlan.levelId : "",
    boundary,
    edgeSetbacksFt: fallbackSetbacks.map((fallbackValue, index) => {
      const nextValue = sitePlan.edgeSetbacksFt[index];
      return Number.isFinite(nextValue) ? Math.max(0, nextValue) : fallbackValue;
    })
  };
}

export function normalizeSitePlan(sitePlan: SitePlan | null | undefined, levels: Level[]): SitePlan | null {
  const repairedSitePlan = repairSitePlanGeometry(sitePlan);

  if (!repairedSitePlan) {
    return null;
  }

  const fallbackLevelId = levels[0]?.id ?? "";
  const levelId = repairedSitePlan.levelId && levels.some((level) => level.id === repairedSitePlan.levelId)
    ? repairedSitePlan.levelId
    : fallbackLevelId;

  if (!levelId) {
    return null;
  }

  return {
    ...repairedSitePlan,
    levelId
  };
}

export function repairProjectDoc(doc: ProjectDoc): ProjectDoc {
  return {
    ...doc,
    sitePlan: normalizeSitePlan(doc.sitePlan, doc.levels)
  };
}

export function getProjectSitePlan(doc: ProjectDoc): SitePlan | null {
  return normalizeSitePlan(doc.sitePlan, doc.levels);
}

export function getSitePlanEdges(sitePlan: SitePlan | null | undefined): SiteEdge[] {
  const repairedSitePlan = repairSitePlanGeometry(sitePlan);

  if (!repairedSitePlan) {
    return [];
  }

  return repairedSitePlan.boundary.map((start, index) => {
    const end = repairedSitePlan.boundary[(index + 1) % repairedSitePlan.boundary.length];
    const direction = subtractPoint(end, start);

    return {
      index,
      start,
      end,
      midpoint: {
        xFt: start.xFt + direction.xFt / 2,
        yFt: start.yFt + direction.yFt / 2
      },
      lengthFt: Math.hypot(direction.xFt, direction.yFt),
      setbackFt: repairedSitePlan.edgeSetbacksFt[index] ?? DEFAULT_SITE_SETBACK_FT
    };
  });
}

export function deriveSitePlanFootprint(sitePlan: SitePlan | null | undefined): DerivedFootprintResult {
  const repairedSitePlan = repairSitePlanGeometry(sitePlan);

  if (!repairedSitePlan) {
    return {
      footprint: null,
      error: "Site boundary must include at least 3 valid points."
    };
  }

  const signedArea = getPolygonSignedAreaSqFt(repairedSitePlan.boundary);

  if (Math.abs(signedArea) <= GEOMETRY_EPSILON) {
    return {
      footprint: null,
      error: "Site boundary must enclose a valid area."
    };
  }

  const orientationFactor = signedArea >= 0 ? 1 : -1;
  const offsetEdges = repairedSitePlan.boundary.map((start, index) => {
    const end = repairedSitePlan.boundary[(index + 1) % repairedSitePlan.boundary.length];
    const direction = subtractPoint(end, start);
    const lengthFt = Math.hypot(direction.xFt, direction.yFt);

    if (lengthFt <= GEOMETRY_EPSILON) {
      return null;
    }

    const inwardNormal: Point2Ft = {
      xFt: orientationFactor * (-direction.yFt / lengthFt),
      yFt: orientationFactor * (direction.xFt / lengthFt)
    };
    const setbackFt = repairedSitePlan.edgeSetbacksFt[index] ?? DEFAULT_SITE_SETBACK_FT;

    return {
      start,
      direction,
      inwardNormal,
      setbackFt,
      offsetStart: addScaledPoint(start, inwardNormal, setbackFt)
    };
  });

  if (offsetEdges.some((edge) => !edge)) {
    return {
      footprint: null,
      error: "Site boundary cannot contain a zero-length edge."
    };
  }

  const resolvedEdges = offsetEdges.flatMap((edge) => edge ? [edge] : []);
  const footprint = resolvedEdges.map((currentEdge, index) => {
    const previousEdge = resolvedEdges[(index - 1 + resolvedEdges.length) % resolvedEdges.length];
    return intersectLines(previousEdge.offsetStart, previousEdge.direction, currentEdge.offsetStart, currentEdge.direction);
  });

  if (footprint.some((point) => !point || !Number.isFinite(point.xFt) || !Number.isFinite(point.yFt))) {
    return {
      footprint: null,
      error: "Setbacks must resolve to a valid building footprint."
    };
  }

  const normalizedFootprint = normalizeFootprint(footprint.flatMap((point) => point ? [point] : []));

  if (normalizedFootprint.length < 3 || getPolygonAreaSqFt(normalizedFootprint) <= GEOMETRY_EPSILON) {
    return {
      footprint: null,
      error: "Setbacks collapse the building footprint."
    };
  }

  const isInsideAllInsetHalfPlanes = normalizedFootprint.every((point) => (
    resolvedEdges.every((edge) => {
      const insetDistance = dotProduct(subtractPoint(point, edge.start), edge.inwardNormal);
      return insetDistance >= edge.setbackFt - HALF_PLANE_EPSILON;
    })
  ));

  if (!isInsideAllInsetHalfPlanes) {
    return {
      footprint: null,
      error: "Setbacks exceed the available site depth."
    };
  }

  return {
    footprint: normalizedFootprint,
    error: null
  };
}

function crossProduct(origin: Point2Ft, left: Point2Ft, right: Point2Ft): number {
  return (left.xFt - origin.xFt) * (right.yFt - origin.yFt)
    - (left.yFt - origin.yFt) * (right.xFt - origin.xFt);
}

function isPointInsideTriangle(point: Point2Ft, a: Point2Ft, b: Point2Ft, c: Point2Ft): boolean {
  const area1 = crossProduct(point, a, b);
  const area2 = crossProduct(point, b, c);
  const area3 = crossProduct(point, c, a);
  const hasNegative = area1 < -GEOMETRY_EPSILON || area2 < -GEOMETRY_EPSILON || area3 < -GEOMETRY_EPSILON;
  const hasPositive = area1 > GEOMETRY_EPSILON || area2 > GEOMETRY_EPSILON || area3 > GEOMETRY_EPSILON;

  return !(hasNegative && hasPositive);
}

export function triangulateFootprint(footprint: Point2Ft[]): Point2Ft[][] {
  const normalizedPoints = normalizeFootprint(footprint);

  if (normalizedPoints.length < 3) {
    return [];
  }

  const orientedPoints = getPolygonSignedAreaSqFt(normalizedPoints) >= 0
    ? normalizedPoints
    : [...normalizedPoints].reverse();
  const remainingIndices = orientedPoints.map((_, index) => index);
  const triangles: Point2Ft[][] = [];
  let safetyCounter = 0;

  while (remainingIndices.length > 3 && safetyCounter < orientedPoints.length * orientedPoints.length) {
    let earFound = false;

    for (let index = 0; index < remainingIndices.length; index += 1) {
      const previousIndex = remainingIndices[(index - 1 + remainingIndices.length) % remainingIndices.length];
      const currentIndex = remainingIndices[index];
      const nextIndex = remainingIndices[(index + 1) % remainingIndices.length];
      const previousPoint = orientedPoints[previousIndex];
      const currentPoint = orientedPoints[currentIndex];
      const nextPoint = orientedPoints[nextIndex];

      if (crossProduct(previousPoint, currentPoint, nextPoint) <= GEOMETRY_EPSILON) {
        continue;
      }

      const containsAnotherPoint = remainingIndices.some((candidateIndex) => {
        if (candidateIndex === previousIndex || candidateIndex === currentIndex || candidateIndex === nextIndex) {
          return false;
        }

        return isPointInsideTriangle(orientedPoints[candidateIndex], previousPoint, currentPoint, nextPoint);
      });

      if (containsAnotherPoint) {
        continue;
      }

      triangles.push([previousPoint, currentPoint, nextPoint]);
      remainingIndices.splice(index, 1);
      earFound = true;
      break;
    }

    if (!earFound) {
      break;
    }

    safetyCounter += 1;
  }

  if (remainingIndices.length === 3) {
    triangles.push(remainingIndices.map((index) => orientedPoints[index]));
  }

  if (triangles.length === 0 && orientedPoints.length >= 3) {
    for (let index = 1; index < orientedPoints.length - 1; index += 1) {
      triangles.push([orientedPoints[0], orientedPoints[index], orientedPoints[index + 1]]);
    }
  }

  return triangles;
}

export function getSpaceBoundsFt(space: Space): PolygonBoundsFt {
  return getPolygonBoundsFt(space.footprint);
}

export function getSpaceLabelPointFt(space: Space): Point2Ft {
  return getPolygonCentroidFt(space.footprint);
}

export function setSiteEdgeSetback(doc: ProjectDoc, edgeIndex: number, setbackFt: number): ProjectDoc {
  const repairedSitePlan = getProjectSitePlan(doc);

  if (!repairedSitePlan || !Number.isInteger(edgeIndex) || edgeIndex < 0 || edgeIndex >= repairedSitePlan.boundary.length) {
    return repairProjectDoc(doc);
  }

  const edgeSetbacksFt = repairedSitePlan.edgeSetbacksFt.map((currentSetbackFt, index) => (
    index === edgeIndex ? clampSiteSetbackFt(setbackFt) : currentSetbackFt
  ));

  return repairProjectDoc({
    ...doc,
    sitePlan: {
      ...repairedSitePlan,
      edgeSetbacksFt
    }
  });
}

export function createStarterProjectDoc(): ProjectDoc {
  const levelId = "level-1";

  return repairProjectDoc({
    id: "project-starter",
    name: "GLOQ Tower",
    defaultStoryHeightFt: DEFAULT_STORY_HEIGHT_FT,
    levels: [
      {
        id: levelId,
        name: "Level 1",
        elevationFt: 0,
        heightFt: DEFAULT_STORY_HEIGHT_FT
      }
    ],
    spaces: [
      {
        id: "space-lobby",
        levelId,
        name: "Lobby",
        footprint: [
          { xFt: 0, yFt: 2 },
          { xFt: 14, yFt: 0 },
          { xFt: 18, yFt: 10 },
          { xFt: 12, yFt: 15 },
          { xFt: 0, yFt: 13 }
        ]
      },
      {
        id: "space-conference",
        levelId,
        name: "Conference",
        footprint: [
          { xFt: 20, yFt: 1 },
          { xFt: 34, yFt: 0 },
          { xFt: 36, yFt: 8 },
          { xFt: 31, yFt: 14 },
          { xFt: 22, yFt: 12 }
        ]
      },
      {
        id: "space-open-office",
        levelId,
        name: "Open Office",
        footprint: [
          { xFt: 1, yFt: 18 },
          { xFt: 11, yFt: 16 },
          { xFt: 24, yFt: 18 },
          { xFt: 23, yFt: 32 },
          { xFt: 15, yFt: 35 },
          { xFt: 2, yFt: 30 }
        ]
      }
    ],
    sitePlan: null
  });
}

export function getLevelById(doc: ProjectDoc, levelId: string): Level | null {
  return doc.levels.find((level) => level.id === levelId) ?? null;
}

export function getValidActiveLevelId(doc: ProjectDoc, activeLevelId: string | null | undefined): string {
  if (activeLevelId && doc.levels.some((level) => level.id === activeLevelId)) {
    return activeLevelId;
  }

  return doc.levels[0]?.id ?? "";
}

export function getLevelSpaces(doc: ProjectDoc, levelId: string): Space[] {
  return doc.spaces.filter((space) => space.levelId === levelId);
}

export function getSpaceAreaSqFt(space: Space): number {
  return getPolygonAreaSqFt(space.footprint);
}

export function createLevel(doc: ProjectDoc, activeLevelId: string): LevelMutationResult {
  const insertAfterId = getValidActiveLevelId(doc, activeLevelId);
  const insertAfterIndex = Math.max(0, doc.levels.findIndex((level) => level.id === insertAfterId));
  const insertAfterLevel = doc.levels[insertAfterIndex] ?? doc.levels.at(-1);

  if (!insertAfterLevel) {
    return {
      doc,
      activeLevelId: ""
    };
  }

  const nextId = createLevelIdFactory(doc.levels);
  const heightFt = getPositiveFeetOrFallback(doc.defaultStoryHeightFt, DEFAULT_STORY_HEIGHT_FT);
  const level: Level = {
    id: nextId(),
    name: getNextManualLevelName(doc.levels),
    elevationFt: insertAfterLevel.elevationFt + heightFt,
    heightFt
  };

  return {
    doc: repairProjectDoc({
      ...doc,
      levels: [
        ...doc.levels.slice(0, insertAfterIndex + 1),
        level,
        ...doc.levels.slice(insertAfterIndex + 1)
      ]
    }),
    activeLevelId: level.id
  };
}

export function deleteLevel(doc: ProjectDoc, levelId: string, activeLevelId: string): LevelMutationResult {
  if (doc.levels.length <= 1) {
    return {
      doc,
      activeLevelId: getValidActiveLevelId(doc, activeLevelId)
    };
  }

  const deleteIndex = doc.levels.findIndex((level) => level.id === levelId);

  if (deleteIndex === -1) {
    return {
      doc,
      activeLevelId: getValidActiveLevelId(doc, activeLevelId)
    };
  }

  const levels = doc.levels.filter((level) => level.id !== levelId);
  const nextDoc: ProjectDoc = repairProjectDoc({
    ...doc,
    levels,
    spaces: doc.spaces.filter((space) => space.levelId !== levelId)
  });
  const fallbackLevel = levels[Math.min(deleteIndex, levels.length - 1)] ?? levels[0];

  return {
    doc: nextDoc,
    activeLevelId: activeLevelId === levelId
      ? fallbackLevel.id
      : getValidActiveLevelId(nextDoc, activeLevelId)
  };
}

export function renameLevel(doc: ProjectDoc, levelId: string, name: string): ProjectDoc {
  const trimmedName = name.trim();

  if (!trimmedName) {
    return doc;
  }

  return repairProjectDoc({
    ...doc,
    levels: doc.levels.map((level) => (
      level.id === levelId
        ? { ...level, name: trimmedName }
        : level
    ))
  });
}

export function moveLevel(doc: ProjectDoc, levelId: string, direction: "up" | "down"): ProjectDoc {
  const index = doc.levels.findIndex((level) => level.id === levelId);

  if (index === -1) {
    return doc;
  }

  const nextIndex = direction === "up" ? index - 1 : index + 1;

  if (nextIndex < 0 || nextIndex >= doc.levels.length) {
    return doc;
  }

  return repairProjectDoc({
    ...doc,
    levels: swapItems(doc.levels, index, nextIndex)
  });
}

export function setLevelElevation(doc: ProjectDoc, levelId: string, elevationFt: number): ProjectDoc {
  if (!Number.isFinite(elevationFt)) {
    return doc;
  }

  return repairProjectDoc({
    ...doc,
    levels: doc.levels.map((level) => (
      level.id === levelId
        ? { ...level, elevationFt }
        : level
    ))
  });
}

export function setDefaultStoryHeight(doc: ProjectDoc, heightFt: number): ProjectDoc {
  if (!Number.isFinite(heightFt) || heightFt <= 0) {
    return doc;
  }

  return repairProjectDoc({
    ...doc,
    defaultStoryHeightFt: heightFt
  });
}

export function autoGenerateLevels(doc: ProjectDoc, input: AutoGenerateLevelsInput): LevelMutationResult {
  const levelsToGenerate = buildGeneratedLevelSpecs(input);
  const storyHeightFt = levelsToGenerate[0]?.heightFt ?? DEFAULT_STORY_HEIGHT_FT;
  const existingLevelsByName = new Map<string, Level[]>();

  for (const level of doc.levels) {
    const matchingLevels = existingLevelsByName.get(level.name) ?? [];
    matchingLevels.push(level);
    existingLevelsByName.set(level.name, matchingLevels);
  }

  const nextId = createLevelIdFactory(doc.levels);
  const levels = levelsToGenerate.map((levelSpec) => {
    const reusableLevel = existingLevelsByName.get(levelSpec.name)?.shift();

    return {
      id: reusableLevel?.id ?? nextId(),
      name: levelSpec.name,
      elevationFt: levelSpec.elevationFt,
      heightFt: levelSpec.heightFt
    };
  });
  const keptLevelIds = new Set(levels.map((level) => level.id));
  const nextDoc: ProjectDoc = repairProjectDoc({
    ...doc,
    defaultStoryHeightFt: storyHeightFt,
    levels,
    spaces: doc.spaces.filter((space) => keptLevelIds.has(space.levelId))
  });
  const preferredLevelName = getPreferredGeneratedLevelName(levelsToGenerate);
  const preferredLevel = levels.find((level) => level.name === preferredLevelName) ?? levels[0];

  return {
    doc: nextDoc,
    activeLevelId: preferredLevel.id
  };
}
