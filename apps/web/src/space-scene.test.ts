import { describe, expect, it } from "vitest";
import type { Selection } from "./ui-store";
import {
  buildSpaceScenePayload,
  getDefaultOrbitCamera,
  getVisibleSpacePrisms,
  pickVisibleSpaceAtCanvasPoint,
  type SpaceScenePayload,
  type ThreeDVisibilityMode
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

function buildScene(
  selection: Selection = null,
  visibilityMode: ThreeDVisibilityMode = "active-floor-only"
): SpaceScenePayload {
  return buildSpaceScenePayload(createSceneDoc(), {
    activeLevelId: "level-2",
    selection,
    visibilityMode
  });
}

describe("buildSpaceScenePayload", () => {
  it("renders only the active level and precomputes mesh and edge vertices", () => {
    const scene = buildScene();

    expect(scene.items.map((item) => item.id)).toEqual(["space-l2-a"]);
    expect(scene.vertices).toHaveLength(36);
    expect(scene.edgeVertices).toHaveLength(24);
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
    const scene = buildScene({ kind: "element", element: { kind: "space", id: "space-l2-a" } });
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

  it("shows spaces across all levels when the visibility mode is all-levels", () => {
    const scene = buildScene(null, "all-levels");

    expect(scene.items.map((item) => item.id)).toEqual(["space-l1-a", "space-l2-a", "space-b1-a"]);
    expect(scene.items.map((item) => item.emphasis)).toEqual(["normal", "active-level", "normal"]);
    expect(scene.extents).toEqual({
      minXFt: -6,
      minYFt: -4,
      minZFt: -9,
      maxXFt: 40,
      maxYFt: 22,
      maxZFt: 23
    });
  });

  it("keeps the selected space strongest even when all levels are visible", () => {
    const scene = buildScene({ kind: "element", element: { kind: "space", id: "space-b1-a" } }, "all-levels");

    expect(scene.items.find((item) => item.id === "space-b1-a")?.emphasis).toBe("selected");
    expect(scene.items.find((item) => item.id === "space-l2-a")?.emphasis).toBe("active-level");
  });

  it("keeps multiple selected spaces emphasized through the shared element refs", () => {
    const scene = buildScene(
      {
        kind: "element-set",
        elements: [
          { kind: "space", id: "space-l1-a" },
          { kind: "space", id: "space-b1-a" }
        ]
      },
      "all-levels"
    );

    expect(scene.items.find((item) => item.id === "space-l1-a")?.emphasis).toBe("selected");
    expect(scene.items.find((item) => item.id === "space-b1-a")?.emphasis).toBe("selected");
    expect(scene.items.find((item) => item.id === "space-l2-a")?.emphasis).toBe("active-level");
  });

  it("returns a safe empty scene when no spaces are present", () => {
    const scene = buildSpaceScenePayload(
      {
        ...createSceneDoc(),
        spaces: []
      },
      {
        activeLevelId: "level-1",
        selection: null,
        visibilityMode: "active-floor-only"
      }
    );

    expect(scene.items).toEqual([]);
    expect(scene.vertices).toEqual([]);
    expect(scene.edgeVertices).toEqual([]);
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

describe("getVisibleSpacePrisms", () => {
  it("keeps the visible-space descriptors aligned with the current 3D scope", () => {
    const prisms = getVisibleSpacePrisms(createSceneDoc(), {
      activeLevelId: "level-2",
      selection: null,
      visibilityMode: "active-floor-only"
    });

    expect(prisms).toHaveLength(1);
    expect(prisms[0]).toMatchObject({
      id: "space-l2-a",
      levelId: "level-2",
      minZFt: 12,
      maxZFt: 23
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
      edgeVertices: [],
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

describe("pickVisibleSpaceAtCanvasPoint", () => {
  it("picks the centered visible prism when clicking through the middle of the viewport", () => {
    const scene = buildScene();
    const picked = pickVisibleSpaceAtCanvasPoint({
      prisms: getVisibleSpacePrisms(createSceneDoc(), {
        activeLevelId: "level-2",
        selection: null,
        visibilityMode: "active-floor-only"
      }),
      camera: getDefaultOrbitCamera(scene),
      canvasX: 400,
      canvasY: 300,
      viewportWidth: 800,
      viewportHeight: 600
    });

    expect(picked?.id).toBe("space-l2-a");
  });

  it("returns null when the click misses every visible prism", () => {
    const scene = buildScene();
    const picked = pickVisibleSpaceAtCanvasPoint({
      prisms: getVisibleSpacePrisms(createSceneDoc(), {
        activeLevelId: "level-2",
        selection: null,
        visibilityMode: "active-floor-only"
      }),
      camera: getDefaultOrbitCamera(scene),
      canvasX: 10,
      canvasY: 10,
      viewportWidth: 800,
      viewportHeight: 600
    });

    expect(picked).toBeNull();
  });
});
