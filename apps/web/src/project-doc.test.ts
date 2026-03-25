import { describe, expect, it } from "vitest";
import {
  autoGenerateLevels,
  createLevel,
  createStarterProjectDoc,
  deleteLevel,
  moveLevel
} from "./project-doc";

describe("level mutations", () => {
  it("creates a new level above the active level using the project default height", () => {
    const project = createStarterProjectDoc();
    const result = createLevel(project, project.levels[0].id);
    const createdLevel = result.doc.levels[1];

    expect(result.activeLevelId).toBe(createdLevel.id);
    expect(createdLevel.name).toBe("Level 2");
    expect(createdLevel.elevationFt).toBe(10);
    expect(createdLevel.heightFt).toBe(project.defaultStoryHeightFt);
  });

  it("deletes dependent spaces and shifts the active level to an adjacent survivor", () => {
    const project = createStarterProjectDoc();
    const levelTwo = {
      id: "level-02",
      name: "Level 2",
      elevationFt: 10,
      heightFt: 10
    };
    const nextProject = {
      ...project,
      levels: [...project.levels, levelTwo],
      spaces: [
        ...project.spaces,
        {
          id: "space-office",
          levelId: levelTwo.id,
          name: "Office",
          xFt: 0,
          yFt: 0,
          widthFt: 12,
          depthFt: 14
        }
      ]
    };

    const result = deleteLevel(nextProject, levelTwo.id, levelTwo.id);

    expect(result.activeLevelId).toBe(project.levels[0].id);
    expect(result.doc.levels).toHaveLength(1);
    expect(result.doc.spaces.every((space) => space.levelId !== levelTwo.id)).toBe(true);
  });

  it("reorders levels without changing their ids", () => {
    const project = createStarterProjectDoc();
    const nextProject = {
      ...project,
      levels: [
        ...project.levels,
        {
          id: "level-02",
          name: "Level 2",
          elevationFt: 10,
          heightFt: 10
        }
      ]
    };

    const movedProject = moveLevel(nextProject, "level-02", "up");

    expect(movedProject.levels.map((level) => level.id)).toEqual(["level-02", "level-01"]);
  });
});

describe("autoGenerateLevels", () => {
  it("preserves spaces on reused named levels", () => {
    const project = createStarterProjectDoc();
    const levelTwo = {
      id: "level-02",
      name: "Level 2",
      elevationFt: 10,
      heightFt: 10
    };
    const nextProject = {
      ...project,
      levels: [...project.levels, levelTwo],
      spaces: [
        ...project.spaces,
        {
          id: "space-office",
          levelId: levelTwo.id,
          name: "Office",
          xFt: 10,
          yFt: 10,
          widthFt: 15,
          depthFt: 15
        }
      ]
    };

    const result = autoGenerateLevels(nextProject, {
      storiesBelowGrade: 1,
      storiesOnGrade: 3,
      storyHeightFt: 10
    });

    expect(result.doc.levels.map((level) => level.name)).toEqual([
      "Basement 1",
      "Level 1",
      "Level 2",
      "Level 3"
    ]);
    expect(result.doc.levels[1].id).toBe(project.levels[0].id);
    expect(result.activeLevelId).toBe(project.levels[0].id);
    expect(result.doc.spaces.some((space) => space.id === "space-lobby")).toBe(true);
    expect(result.doc.spaces.some((space) => space.id === "space-office")).toBe(true);
    expect(result.doc.defaultStoryHeightFt).toBe(10);
  });

  it("removes spaces that belong to discarded levels", () => {
    const project = createStarterProjectDoc();
    const nextProject = {
      ...project,
      levels: [
        ...project.levels,
        {
          id: "level-02",
          name: "Level 2",
          elevationFt: 10,
          heightFt: 10
        }
      ],
      spaces: [
        ...project.spaces,
        {
          id: "space-office",
          levelId: "level-02",
          name: "Office",
          xFt: 10,
          yFt: 10,
          widthFt: 15,
          depthFt: 15
        }
      ]
    };

    const result = autoGenerateLevels(nextProject, {
      storiesBelowGrade: 1,
      storiesOnGrade: 1,
      storyHeightFt: 10
    });

    expect(result.doc.levels.map((level) => level.name)).toEqual(["Basement 1", "Level 1"]);
    expect(result.doc.spaces.some((space) => space.id === "space-lobby")).toBe(true);
    expect(result.doc.spaces.some((space) => space.id === "space-office")).toBe(false);
  });
});
