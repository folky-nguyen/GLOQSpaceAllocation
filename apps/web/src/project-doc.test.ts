import { describe, expect, it } from "vitest";
import {
  autoGenerateLevels,
  createLevel,
  createStarterProjectDoc,
  deleteLevel,
  moveLevel,
  renameLevel,
  setDefaultStoryHeight,
  type ProjectDoc
} from "./project-doc";

function createDoc(): ProjectDoc {
  return createStarterProjectDoc();
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
          xFt: 0,
          yFt: 0,
          widthFt: 10,
          depthFt: 12
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
          xFt: 2,
          yFt: 2,
          widthFt: 12,
          depthFt: 14
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
          xFt: 1,
          yFt: 1,
          widthFt: 8,
          depthFt: 10
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
});
