import { describe, expect, it } from "vitest";
import type { Selection } from "./ui-store";
import {
  buildSpaceScenePayload,
  getDefaultOrbitCamera,
  type SpaceScenePayload
} from "./space-scene";
import {
  createRectangleFootprint,
  type ProjectDoc
} from "./project-doc";

function createSceneDoc(): ProjectDoc {
  return {
    id: "project-scene-test",
    name: "Scene Test",
    defaultStoryHeightFt: 10,
    levels: [
      {
        id: "level-1",
        name: "Level 1",
        elevationFt: 0,
        heightFt: 10
      },
      {
        id: "level-2",
        name: "Level 2",
        elevationFt: 12,
        heightFt: 11
      },
      {
        id: "level-b1",
        name: "Basement 1",
        elevationFt: -9,
        heightFt: 9
      }
    ],
    spaces: [
      {
        id: "space-l1-a",
        levelId: "level-1",
        name: "Lobby",
        footprint: createRectangleFootprint(0, 0, 20, 12)
      },
      {
        id: "space-l2-a",
        levelId: "level-2",
        name: "Office",
        footprint: createRectangleFootprint(24, 4, 16, 18)
      },
      {
        id: "space-b1-a",
        levelId: "level-b1",
        name: "Storage",
        footprint: createRectangleFootprint(-6, -4, 10, 8)
      }
    ]
  };
}

function buildScene(selection: Selection = null): SpaceScenePayload {
  return buildSpaceScenePayload(createSceneDoc(), {
    activeLevelId: "level-2",
    selection
  });
}

describe("buildSpaceScenePayload", () => {
  it("renders only the active level and precomputes mesh vertices", () => {
    const scene = buildScene();

    expect(scene.items.map((item) => item.id)).toEqual(["space-l2-a"]);
    expect(scene.vertices).toHaveLength(36);
  });

  it("keeps the active level elevation in the filtered scene", () => {
    const scene = buildScene();

    expect(scene.extents.minZFt).toBe(12);
    expect(scene.extents.maxZFt).toBe(23);
    expect(scene.vertices.some((vertex) => vertex.position[2] === 12)).toBe(true);
  });

  it("marks visible spaces on the active level when no specific space is selected", () => {
    const scene = buildScene();

    expect(scene.items.every((item) => item.levelId === "level-2")).toBe(true);
    expect(scene.items.every((item) => item.emphasis === "active-level")).toBe(true);
  });

  it("gives the selected space stronger emphasis than the rest of its level", () => {
    const scene = buildScene({ kind: "space", id: "space-l2-a" });
    const selected = scene.items.find((item) => item.id === "space-l2-a");

    expect(selected?.emphasis).toBe("selected");
    expect(scene.items).toHaveLength(1);
  });

  it("computes scene extents for the active level footprint only", () => {
    const scene = buildScene();

    expect(scene.extents).toEqual({
      minXFt: 24,
      minYFt: 4,
      minZFt: 12,
      maxXFt: 40,
      maxYFt: 22,
      maxZFt: 23
    });
  });

  it("returns a safe empty scene when no spaces are present", () => {
    const scene = buildSpaceScenePayload(
      {
        ...createSceneDoc(),
        spaces: []
      },
      {
        activeLevelId: "level-1",
        selection: null
      }
    );

    expect(scene.items).toEqual([]);
    expect(scene.vertices).toEqual([]);
    expect(scene.hasVisibleItems).toBe(false);
    expect(scene.extents).toEqual({
      minXFt: 0,
      minYFt: 0,
      minZFt: 0,
      maxXFt: 0,
      maxYFt: 0,
      maxZFt: 0
    });
  });
});

describe("getDefaultOrbitCamera", () => {
  it("fits the camera around the current scene extents", () => {
    const camera = getDefaultOrbitCamera(buildScene());

    expect(camera.targetXFt).toBe(32);
    expect(camera.targetYFt).toBe(13);
    expect(camera.targetZFt).toBe(17.5);
    expect(camera.distanceFt).toBeGreaterThan(40);
  });

  it("returns a usable fallback for an empty scene", () => {
    const camera = getDefaultOrbitCamera({
      items: [],
      vertices: [],
      extents: {
        minXFt: 0,
        minYFt: 0,
        minZFt: 0,
        maxXFt: 0,
        maxYFt: 0,
        maxZFt: 0
      },
      hasVisibleItems: false
    });

    expect(camera).toEqual({
      targetXFt: 0,
      targetYFt: 0,
      targetZFt: 0,
      distanceFt: 60,
      yawDeg: -35,
      pitchDeg: 35
    });
  });
});
