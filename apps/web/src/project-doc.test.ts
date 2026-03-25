import { describe, expect, it } from "vitest";
import {
  autoGenerateLevels,
  deriveSitePlanFootprint,
  createRectangleFootprint,
  createLevel,
  createStarterProjectDoc,
  deleteLevel,
  getPolygonBoundsFt,
  getSpaceAreaSqFt,
  moveLevel,
  repairProjectDoc,
  renameLevel,
  setSiteEdgeSetback,
  setDefaultStoryHeight,
  type ProjectDoc
} from "./project-doc";
import { MIXED_CASES } from "./test-cases";

function createDoc(): ProjectDoc {
  return createStarterProjectDoc();
}

function isPointOnSegment(
  point: { xFt: number; yFt: number },
  start: { xFt: number; yFt: number },
  end: { xFt: number; yFt: number }
): boolean {
  const cross = (end.xFt - start.xFt) * (point.yFt - start.yFt)
    - (end.yFt - start.yFt) * (point.xFt - start.xFt);

  if (Math.abs(cross) > 1e-6) {
    return false;
  }

  const dot = (point.xFt - start.xFt) * (end.xFt - start.xFt)
    + (point.yFt - start.yFt) * (end.yFt - start.yFt);

  if (dot < 0) {
    return false;
  }

  const lengthSquared = (end.xFt - start.xFt) ** 2 + (end.yFt - start.yFt) ** 2;
  return dot <= lengthSquared + 1e-6;
}

function isPointInsidePolygon(
  point: { xFt: number; yFt: number },
  polygon: Array<{ xFt: number; yFt: number }>
): boolean {
  for (let index = 0; index < polygon.length; index += 1) {
    const start = polygon[index];
    const end = polygon[(index + 1) % polygon.length];

    if (isPointOnSegment(point, start, end)) {
      return true;
    }
  }

  let inside = false;

  for (let index = 0, previousIndex = polygon.length - 1; index < polygon.length; previousIndex = index, index += 1) {
    const current = polygon[index];
    const previous = polygon[previousIndex];
    const crossesRay = (current.yFt > point.yFt) !== (previous.yFt > point.yFt);

    if (!crossesRay) {
      continue;
    }

    const xAtPointY = ((previous.xFt - current.xFt) * (point.yFt - current.yFt)) / (previous.yFt - current.yFt)
      + current.xFt;

    if (point.xFt < xAtPointY) {
      inside = !inside;
    }
  }

  return inside;
}

describe("project-doc level mutations", () => {
  it("creates a level above the active level using the default story height", () => {
    const starterDoc = setDefaultStoryHeight(createDoc(), 12);
    const result = createLevel(starterDoc, starterDoc.levels[0].id);
    const createdLevel = result.doc.levels[1];

    expect(result.doc.levels).toHaveLength(2);
    expect(result.activeLevelId).toBe(createdLevel.id);
    expect(createdLevel.name).toBe("Level 2");
    expect(createdLevel.elevationFt).toBe(12);
    expect(createdLevel.heightFt).toBe(12);
  });

  it("deletes a level, removes dependent spaces, and picks the adjacent active level", () => {
    const starterDoc = createDoc();
    const addedLevel = createLevel(starterDoc, starterDoc.levels[0].id);
    const docWithExtraSpace: ProjectDoc = {
      ...addedLevel.doc,
      spaces: [
        ...addedLevel.doc.spaces,
        {
          id: "space-level-2",
          levelId: addedLevel.activeLevelId,
          name: "Level 2 Office",
          footprint: createRectangleFootprint(0, 0, 10, 12)
        }
      ]
    };
    const result = deleteLevel(docWithExtraSpace, addedLevel.activeLevelId, addedLevel.activeLevelId);

    expect(result.doc.levels).toHaveLength(1);
    expect(result.doc.spaces.every((space) => space.levelId !== addedLevel.activeLevelId)).toBe(true);
    expect(result.activeLevelId).toBe(starterDoc.levels[0].id);
  });

  it("reorders levels without changing the level identities", () => {
    const starterDoc = autoGenerateLevels(createDoc(), {
      storiesBelowGrade: 1,
      storiesOnGrade: 2,
      storyHeightFt: 10
    }).doc;
    const originalIds = starterDoc.levels.map((level) => level.id);
    const movedDoc = moveLevel(starterDoc, starterDoc.levels[1].id, "down");

    expect(movedDoc.levels.map((level) => level.name)).toEqual(["Basement 1", "Level 2", "Level 1"]);
    expect(movedDoc.levels.map((level) => level.id).sort()).toEqual([...originalIds].sort());
  });

  it("auto-generate preserves spaces for levels whose generated names survive", () => {
    const starterDoc = renameLevel(createDoc(), "level-1", "Level 2");
    const docWithSpace: ProjectDoc = {
      ...starterDoc,
      spaces: [
        ...starterDoc.spaces,
        {
          id: "space-reused-level",
          levelId: "level-1",
          name: "Stacked Office",
          footprint: createRectangleFootprint(2, 2, 12, 14)
        }
      ]
    };
    const result = autoGenerateLevels(docWithSpace, {
      storiesBelowGrade: 0,
      storiesOnGrade: 3,
      storyHeightFt: 10
    });

    expect(result.doc.levels.map((level) => level.name)).toEqual(["Level 1", "Level 2", "Level 3"]);
    expect(result.doc.levels[1].id).toBe("level-1");
    expect(result.doc.spaces.some((space) => space.levelId === "level-1")).toBe(true);
  });

  it("auto-generate discards spaces on levels that no longer exist", () => {
    const starterDoc = createDoc();
    const basementDoc = autoGenerateLevels(starterDoc, {
      storiesBelowGrade: 1,
      storiesOnGrade: 1,
      storyHeightFt: 10
    }).doc;
    const basementLevel = basementDoc.levels.find((level) => level.name === "Basement 1");
    const docWithBasementSpace: ProjectDoc = {
      ...basementDoc,
      spaces: [
        ...basementDoc.spaces,
        {
          id: "space-basement-storage",
          levelId: basementLevel?.id ?? "",
          name: "Storage",
          footprint: createRectangleFootprint(1, 1, 8, 10)
        }
      ]
    };
    const result = autoGenerateLevels(docWithBasementSpace, {
      storiesBelowGrade: 0,
      storiesOnGrade: 1,
      storyHeightFt: 10
    });

    expect(result.doc.levels.map((level) => level.name)).toEqual(["Level 1"]);
    expect(result.doc.spaces.some((space) => space.name === "Storage")).toBe(false);
    expect(result.activeLevelId).toBe(result.doc.levels[0].id);
  });

  it("computes polygon area for non-rectangular spaces", () => {
    const doc = createDoc();
    const polygonSpace = {
      id: "space-polygon",
      levelId: doc.levels[0].id,
      name: "Apartment",
      footprint: [
        { xFt: 0, yFt: 0 },
        { xFt: 10, yFt: 0 },
        { xFt: 12, yFt: 6 },
        { xFt: 4, yFt: 12 },
        { xFt: 0, yFt: 8 }
      ]
    };

    expect(getSpaceAreaSqFt(polygonSpace)).toBe(106);
  });

  it("repairs incomplete site setback arrays and invalid site host levels", () => {
    const repairedDoc = repairProjectDoc({
      id: "project-site-repair",
      name: "Site Repair",
      defaultStoryHeightFt: 10,
      levels: [
        {
          id: "level-1",
          name: "Level 1",
          elevationFt: 0,
          heightFt: 10
        }
      ],
      spaces: [],
      sitePlan: {
        levelId: "missing-level",
        boundary: createRectangleFootprint(0, 0, 60, 40),
        edgeSetbacksFt: [8, Number.NaN]
      }
    });

    expect(repairedDoc.sitePlan?.levelId).toBe("level-1");
    expect(repairedDoc.sitePlan?.edgeSetbacksFt).toEqual([8, 5, 5, 5]);
  });

  it("derives an inset building footprint from clockwise site polygons", () => {
    const result = deriveSitePlanFootprint({
      levelId: "level-1",
      boundary: [
        { xFt: 0, yFt: 0 },
        { xFt: 0, yFt: 30 },
        { xFt: 40, yFt: 30 },
        { xFt: 40, yFt: 0 }
      ],
      edgeSetbacksFt: [5, 5, 5, 5]
    });

    expect(result.error).toBeNull();
    expect(result.footprint).not.toBeNull();

    const bounds = getPolygonBoundsFt(result.footprint ?? []);
    expect(bounds.minXFt).toBeCloseTo(5);
    expect(bounds.minYFt).toBeCloseTo(5);
    expect(bounds.widthFt).toBeCloseTo(30);
    expect(bounds.depthFt).toBeCloseTo(20);
  });

  it("returns an invalid footprint state when setbacks exceed the available lot depth", () => {
    const result = deriveSitePlanFootprint({
      levelId: "level-1",
      boundary: createRectangleFootprint(0, 0, 10, 10),
      edgeSetbacksFt: [6, 6, 6, 6]
    });

    expect(result.footprint).toBeNull();
    expect(result.error).not.toBeNull();
  });

  it("updates one site-edge setback while keeping the rest unchanged", () => {
    const starterDoc = repairProjectDoc({
      ...createDoc(),
      sitePlan: {
        levelId: "level-1",
        boundary: createRectangleFootprint(-10, -10, 80, 60),
        edgeSetbacksFt: [5, 5, 5, 5]
      }
    });
    const nextDoc = setSiteEdgeSetback(starterDoc, 2, 12.5);

    expect(nextDoc.sitePlan?.edgeSetbacksFt).toEqual([5, 5, 12.5, 5]);
  });

  it("keeps every mixed case host-level space point inside the default site footprint", () => {
    for (const sampleCase of MIXED_CASES) {
      const sitePlan = sampleCase.doc.sitePlan;

      expect(sitePlan, `${sampleCase.id} should include a site plan`).not.toBeNull();

      if (!sitePlan) {
        continue;
      }

      const result = deriveSitePlanFootprint(sitePlan);

      expect(result.error, `${sampleCase.id} should derive a default footprint`).toBeNull();
      expect(result.footprint, `${sampleCase.id} should derive a default footprint`).not.toBeNull();

      const footprint = result.footprint ?? [];
      const offenders = sampleCase.doc.spaces
        .filter((space) => space.levelId === sitePlan.levelId)
        .flatMap((space) => (
          space.footprint
            .filter((point) => !isPointInsidePolygon(point, footprint))
            .map((point) => `${space.id}@${point.xFt},${point.yFt}`)
        ));

      expect(offenders, `${sampleCase.id} has points outside the default footprint`).toEqual([]);
    }
  });
});
