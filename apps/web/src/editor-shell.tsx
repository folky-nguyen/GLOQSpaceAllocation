import { useEffect, useRef, useState, type KeyboardEvent, type PointerEvent as ReactPointerEvent } from "react";
import { logout, useAuth } from "./auth";
import {
  autoGenerateLevels,
  createLevel,
  createStarterProjectDoc,
  deleteLevel,
  deriveSitePlanFootprint,
  getLevelById,
  getLevelSpaces,
  getPolygonBoundsFt,
  getProjectSitePlan,
  getSitePlanEdges,
  getSpaceAreaSqFt,
  getSpaceBoundsFt,
  getSpaceLabelPointFt,
  getValidActiveLevelId,
  moveLevel,
  renameLevel,
  setSiteEdgeSetback,
  setDefaultStoryHeight,
  setLevelElevation,
  type AutoGenerateLevelsInput,
  type Level,
  type Point2Ft,
  type ProjectDoc,
  type SiteEdge,
  type Space
} from "./project-doc";
import { formatFeetAndInches, parseFeetAndInches } from "./units";
import {
  getSelectionSpaceIds,
  hasSelectionSpace,
  useUiStore,
  type SelectMode,
  type Selection,
  type ViewMode
} from "./ui-store";
import TestDashboard from "./test-dashboard";
import {
  MIXED_CASES,
  cloneSampleProjectDoc,
  type SampleCaseManifest
} from "./test-cases";
import type { ThreeDVisibilityMode } from "./space-scene";
import ThreeDViewport from "./three-d-viewport";
import UnitsInspector from "./units-inspector";

const selectModeItems: Array<{ value: SelectMode; label: string; hint: string }> = [
  {
    value: "pick-many",
    label: "Pick Many",
    hint: "Click spaces to add them. Click a selected space again to remove it."
  },
  {
    value: "sweep",
    label: "Sweep Select",
    hint: "Drag to replace. Hold Shift to add more. Hold Alt to remove from the current set."
  }
];

const ribbonGroups = [
  { title: "File", items: ["New", "Save"] },
  { title: "Edit", items: ["Undo", "Redo"] }
];

const planScalePx = 10;

type PlanBounds = {
  minX: number;
  minY: number;
  width: number;
  height: number;
};

type EditorState = {
  project: ProjectDoc;
  activeLevelId: string;
};

type SweepMode = "replace" | "add" | "remove";

type SweepSelectionDraft = {
  pointerId: number;
  startX: number;
  startY: number;
  currentX: number;
  currentY: number;
  mode: SweepMode;
};

type LevelManagerProps = {
  project: ProjectDoc;
  activeLevelId: string;
  onClose: () => void;
  onActivateLevel: (levelId: string) => void;
  onCreateLevel: () => void;
  onDeleteLevel: (levelId: string) => void;
  onRenameLevel: (levelId: string, name: string) => void;
  onMoveLevel: (levelId: string, direction: "up" | "down") => void;
  onSetLevelElevation: (levelId: string, elevationFt: number) => void;
  onSetDefaultStoryHeight: (heightFt: number) => void;
  onAutoGenerate: (input: AutoGenerateLevelsInput) => void;
};

function getViewSelection(view: ViewMode): Selection {
  return {
    kind: "view",
    id: view === "3d"
      ? "view-3d"
      : view === "site-plan"
        ? "view-site-plan"
        : "view-plan"
  };
}

function getSelectModeLabel(mode: SelectMode): string {
  return selectModeItems.find((item) => item.value === mode)?.label ?? "Pick Many";
}

function getSelectModeHint(mode: SelectMode): string {
  return selectModeItems.find((item) => item.value === mode)?.hint ?? selectModeItems[0].hint;
}

function getThreeDVisibilityModeLabel(mode: ThreeDVisibilityMode): string {
  return mode === "all-levels" ? "All Levels" : "Active Floor Only";
}

function getViewLabel(view: ViewMode, level: Level, sitePlanLevel: Level | null): string {
  if (view === "3d") {
    return "3D View";
  }

  if (view === "site-plan") {
    return sitePlanLevel ? `${sitePlanLevel.name} Site Plan` : "Site Plan";
  }

  return `${level.name} Floor Plan`;
}

function getSelectionLabel(
  selection: Selection,
  activeLevel: Level,
  sitePlanLevel: Level | null,
  selectedSiteEdge: SiteEdge | null,
  selectedLevel: Level | null,
  selectedSpaces: Space[],
  view: ViewMode
): string {
  if (!selection) {
    return "None";
  }

  if (selection.kind === "space" && selectedSpaces[0]) {
    return selectedSpaces[0].name;
  }

  if (selection.kind === "space-set" && selectedSpaces.length > 0) {
    return `${selectedSpaces.length} spaces`;
  }

  if (selection.kind === "level" && selectedLevel) {
    return selectedLevel.name;
  }

  if (selection.kind === "site-edge" && selectedSiteEdge) {
    return `Site Edge ${selectedSiteEdge.index + 1}`;
  }

  return getViewLabel(view, activeLevel, sitePlanLevel);
}

function getPlanBounds(polygons: Point2Ft[][]): PlanBounds {
  const validPolygons = polygons.filter((polygon) => polygon.length > 0);

  if (validPolygons.length === 0) {
    return { minX: 0, minY: 0, width: 0, height: 0 };
  }

  const bounds = validPolygons.reduce(
    (currentBounds, polygon) => {
      const polygonBounds = getPolygonBoundsFt(polygon);

      return {
        minX: Math.min(currentBounds.minX, polygonBounds.minXFt),
        minY: Math.min(currentBounds.minY, polygonBounds.minYFt),
        maxX: Math.max(currentBounds.maxX, polygonBounds.maxXFt),
        maxY: Math.max(currentBounds.maxY, polygonBounds.maxYFt)
      };
    },
    {
      minX: Number.POSITIVE_INFINITY,
      minY: Number.POSITIVE_INFINITY,
      maxX: Number.NEGATIVE_INFINITY,
      maxY: Number.NEGATIVE_INFINITY
    }
  );

  const originX = Math.min(bounds.minX, 0);
  const originY = Math.min(bounds.minY, 0);

  return {
    minX: originX,
    minY: originY,
    width: bounds.maxX - originX,
    height: bounds.maxY - originY
  };
}

function getPlanPoint(point: Point2Ft, bounds: PlanBounds): { x: number; y: number } {
  return {
    x: (point.xFt - bounds.minX) * planScalePx,
    y: (point.yFt - bounds.minY) * planScalePx
  };
}

function getPlanPolygonPoints(footprint: Point2Ft[], bounds: PlanBounds): string {
  return footprint
    .map((point) => {
      const planPoint = getPlanPoint(point, bounds);
      return `${planPoint.x},${planPoint.y}`;
    })
    .join(" ");
}

function getPlanLabelPosition(space: Space, bounds: PlanBounds): { x: number; y: number } {
  return getPlanPoint(getSpaceLabelPointFt(space), bounds);
}

function getPlanEdgeLine(edge: SiteEdge, bounds: PlanBounds) {
  const start = getPlanPoint(edge.start, bounds);
  const end = getPlanPoint(edge.end, bounds);

  return {
    x1: start.x,
    y1: start.y,
    x2: end.x,
    y2: end.y
  };
}

function normalizeSpaceSelection(ids: string[], fallback: Selection): Selection {
  const uniqueIds = [...new Set(ids)];

  if (uniqueIds.length === 0) {
    return fallback;
  }

  if (uniqueIds.length === 1) {
    return { kind: "space", id: uniqueIds[0] };
  }

  return { kind: "space-set", ids: uniqueIds };
}

function toggleSpaceSelection(selection: Selection, spaceId: string, fallback: Selection): Selection {
  const currentIds = getSelectionSpaceIds(selection);
  const nextIds = currentIds.includes(spaceId)
    ? currentIds.filter((id) => id !== spaceId)
    : [...currentIds, spaceId];

  return normalizeSpaceSelection(nextIds, fallback);
}

function getSweepMode(event: ReactPointerEvent<HTMLElement>): SweepMode {
  if (event.altKey) {
    return "remove";
  }

  if (event.shiftKey) {
    return "add";
  }

  return "replace";
}

function getSweepSelectionBounds(draft: SweepSelectionDraft) {
  const left = Math.min(draft.startX, draft.currentX);
  const right = Math.max(draft.startX, draft.currentX);
  const top = Math.min(draft.startY, draft.currentY);
  const bottom = Math.max(draft.startY, draft.currentY);

  return {
    left,
    right,
    top,
    bottom,
    width: right - left,
    height: bottom - top
  };
}

function getSweptSpaceIds(spaces: Space[], bounds: PlanBounds, selectionBounds: ReturnType<typeof getSweepSelectionBounds>): string[] {
  return spaces
    .filter((space) => {
      const spaceBounds = getSpaceBoundsFt(space);
      const left = (spaceBounds.minXFt - bounds.minX) * planScalePx;
      const top = (spaceBounds.minYFt - bounds.minY) * planScalePx;
      const right = left + spaceBounds.widthFt * planScalePx;
      const bottom = top + spaceBounds.depthFt * planScalePx;

      return !(
        right < selectionBounds.left
        || left > selectionBounds.right
        || bottom < selectionBounds.top
        || top > selectionBounds.bottom
      );
    })
    .map((space) => space.id);
}

function mergeSpaceSelection(
  selection: Selection,
  nextSpaceIds: string[],
  mode: SweepMode,
  fallback: Selection
): Selection {
  const currentIds = getSelectionSpaceIds(selection);

  if (mode === "replace") {
    return normalizeSpaceSelection(nextSpaceIds, fallback);
  }

  if (mode === "add") {
    return normalizeSpaceSelection([...currentIds, ...nextSpaceIds], fallback);
  }

  return normalizeSpaceSelection(currentIds.filter((id) => !nextSpaceIds.includes(id)), fallback);
}

function getInitialStoryCounts(project: ProjectDoc): { belowGrade: number; onGrade: number } {
  return project.levels.reduce(
    (counts, level) => ({
      belowGrade: counts.belowGrade + (level.elevationFt < 0 ? 1 : 0),
      onGrade: counts.onGrade + (level.elevationFt >= 0 ? 1 : 0)
    }),
    { belowGrade: 0, onGrade: 0 }
  );
}

function blurOnEnter(event: KeyboardEvent<HTMLInputElement>): void {
  if (event.key === "Enter") {
    event.preventDefault();
    event.currentTarget.blur();
  }
}

function LevelManager({
  project,
  activeLevelId,
  onClose,
  onActivateLevel,
  onCreateLevel,
  onDeleteLevel,
  onRenameLevel,
  onMoveLevel,
  onSetLevelElevation,
  onSetDefaultStoryHeight,
  onAutoGenerate
}: LevelManagerProps) {
  const initialStories = getInitialStoryCounts(project);
  const initialStoryHeight = formatFeetAndInches(project.defaultStoryHeightFt);
  const [storiesBelowGrade, setStoriesBelowGrade] = useState(String(initialStories.belowGrade));
  const [storiesOnGrade, setStoriesOnGrade] = useState(String(initialStories.onGrade));
  const [storyHeightInput, setStoryHeightInput] = useState(initialStoryHeight);
  const [defaultStoryHeightInput, setDefaultStoryHeightInput] = useState(initialStoryHeight);
  const [error, setError] = useState<string | null>(null);

  const handleDefaultStoryHeightCommit = (input: string) => {
    const parsedHeight = parseFeetAndInches(input);

    if (parsedHeight === null || parsedHeight <= 0) {
      setDefaultStoryHeightInput(formatFeetAndInches(project.defaultStoryHeightFt));
      return;
    }

    const formattedHeight = formatFeetAndInches(parsedHeight);
    onSetDefaultStoryHeight(parsedHeight);
    setDefaultStoryHeightInput(formattedHeight);
    setStoryHeightInput(formattedHeight);
    setError(null);
  };

  const handleAutoGenerate = () => {
    const belowGrade = Number(storiesBelowGrade);
    const onGrade = Number(storiesOnGrade);
    const parsedHeight = parseFeetAndInches(storyHeightInput);

    if (!Number.isInteger(belowGrade) || belowGrade < 0) {
      setError("Stories below grade must be a whole number 0 or greater.");
      return;
    }

    if (!Number.isInteger(onGrade) || onGrade < 0) {
      setError("Stories on grade must be a whole number 0 or greater.");
      return;
    }

    if (belowGrade + onGrade < 1) {
      setError("Auto-generate needs at least one story.");
      return;
    }

    if (parsedHeight === null || parsedHeight <= 0) {
      setError("Story height must be a positive ft-in value.");
      return;
    }

    const formattedHeight = formatFeetAndInches(parsedHeight);
    onAutoGenerate({ storiesBelowGrade: belowGrade, storiesOnGrade: onGrade, storyHeightFt: parsedHeight });
    setDefaultStoryHeightInput(formattedHeight);
    setStoryHeightInput(formattedHeight);
    setError(null);
  };

  return (
    <section className="level-manager" role="dialog" aria-label="Level manager">
      <header className="level-manager-header">
        <div>
          <strong>Level Manager</strong>
          <span>All level math stays in internal feet.</span>
        </div>

        <div className="level-manager-header-actions">
          <button type="button" className="level-manager-button" onClick={onCreateLevel}>
            Create Level
          </button>
          <button type="button" className="level-manager-button" onClick={onClose}>
            Close
          </button>
        </div>
      </header>

      <section className="level-manager-section">
        <div className="level-manager-title-row">
          <h3>Auto-generate</h3>
          <button type="button" className="level-manager-button" onClick={handleAutoGenerate}>
            Generate
          </button>
        </div>

        <div className="level-manager-grid">
          <label className="level-manager-field">
            <span>Stories below grade</span>
            <input
              value={storiesBelowGrade}
              inputMode="numeric"
              onChange={(event) => setStoriesBelowGrade(event.currentTarget.value)}
            />
          </label>

          <label className="level-manager-field">
            <span>Stories on grade</span>
            <input
              value={storiesOnGrade}
              inputMode="numeric"
              onChange={(event) => setStoriesOnGrade(event.currentTarget.value)}
            />
          </label>

          <label className="level-manager-field">
            <span>Story height</span>
            <input value={storyHeightInput} onChange={(event) => setStoryHeightInput(event.currentTarget.value)} />
          </label>
        </div>

        <p className="units-inspector-note">
          Reuses matching level ids by name and keeps spaces only on levels that survive generation.
        </p>
      </section>

      <section className="level-manager-section">
        <div className="level-manager-title-row">
          <h3>Defaults</h3>
        </div>

        <div className="level-manager-grid">
          <label className="level-manager-field level-manager-grid-span">
            <span>Default story height</span>
            <input
              value={defaultStoryHeightInput}
              onChange={(event) => setDefaultStoryHeightInput(event.currentTarget.value)}
              onBlur={(event) => handleDefaultStoryHeightCommit(event.currentTarget.value)}
              onKeyDown={(event) => {
                if (event.key === "Enter") {
                  event.preventDefault();
                  handleDefaultStoryHeightCommit(event.currentTarget.value);
                  event.currentTarget.blur();
                }
              }}
            />
          </label>
        </div>
      </section>

      <section className="level-manager-section">
        <div className="level-manager-title-row">
          <h3>Levels</h3>
          <span>{project.levels.length} total</span>
        </div>

        <div className="level-manager-list">
          {project.levels.map((level, index) => (
            <article
              key={`${level.id}:${level.name}:${level.elevationFt}:${level.heightFt}`}
              className={`level-row ${level.id === activeLevelId ? "is-active" : ""}`}
            >
              <button
                type="button"
                className={`level-row-activate ${level.id === activeLevelId ? "is-active" : ""}`}
                onClick={() => {
                  onActivateLevel(level.id);
                  setError(null);
                }}
              >
                {level.id === activeLevelId ? "Active" : "Make Active"}
              </button>

              <div className="level-row-main">
                <label className="level-manager-field">
                  <span>Name</span>
                  <input
                    defaultValue={level.name}
                    aria-label={`${level.name} name`}
                    onBlur={(event) => {
                      const trimmedName = event.currentTarget.value.trim();

                      if (!trimmedName) {
                        event.currentTarget.value = level.name;
                        return;
                      }

                      event.currentTarget.value = trimmedName;
                      onRenameLevel(level.id, trimmedName);
                    }}
                    onKeyDown={blurOnEnter}
                  />
                </label>

                <label className="level-manager-field">
                  <span>Elevation</span>
                  <input
                    defaultValue={formatFeetAndInches(level.elevationFt)}
                    aria-label={`${level.name} elevation`}
                    onBlur={(event) => {
                      const parsedElevation = parseFeetAndInches(event.currentTarget.value);

                      if (parsedElevation === null) {
                        event.currentTarget.value = formatFeetAndInches(level.elevationFt);
                        return;
                      }

                      event.currentTarget.value = formatFeetAndInches(parsedElevation);
                      onSetLevelElevation(level.id, parsedElevation);
                    }}
                    onKeyDown={blurOnEnter}
                  />
                </label>

                <div className="level-row-meta">
                  <span>Height</span>
                  <strong>{formatFeetAndInches(level.heightFt)}</strong>
                </div>
              </div>

              <div className="level-row-actions">
                <button
                  type="button"
                  className="level-manager-button"
                  disabled={index === 0}
                  onClick={() => onMoveLevel(level.id, "up")}
                >
                  Up
                </button>
                <button
                  type="button"
                  className="level-manager-button"
                  disabled={index === project.levels.length - 1}
                  onClick={() => onMoveLevel(level.id, "down")}
                >
                  Down
                </button>
                <button
                  type="button"
                  className="level-manager-button level-manager-button-danger"
                  disabled={project.levels.length <= 1}
                  onClick={() => onDeleteLevel(level.id)}
                >
                  Delete
                </button>
              </div>
            </article>
          ))}
        </div>
      </section>

      {error ? <p className="level-manager-error">{error}</p> : null}
    </section>
  );
}

export default function EditorShell() {
  const auth = useAuth();
  const activeView = useUiStore((state) => state.activeView);
  const selectMode = useUiStore((state) => state.selectMode);
  const selection = useUiStore((state) => state.selection);
  const setSelectMode = useUiStore((state) => state.setSelectMode);
  const setActiveView = useUiStore((state) => state.setActiveView);
  const setSelection = useUiStore((state) => state.setSelection);
  const resetSessionUi = useUiStore((state) => state.resetSessionUi);
  const [editorState, setEditorState] = useState<EditorState>(() => {
    const project = createStarterProjectDoc();

    return { project, activeLevelId: project.levels[0]?.id ?? "" };
  });
  const [logoutPending, setLogoutPending] = useState(false);
  const [logoutError, setLogoutError] = useState<string | null>(null);
  const [showUnitsInspector, setShowUnitsInspector] = useState(false);
  const [showLevelManager, setShowLevelManager] = useState(false);
  const [showTestDashboard, setShowTestDashboard] = useState(false);
  const [activeSampleCaseId, setActiveSampleCaseId] = useState<string | null>(null);
  const [showSelectMenu, setShowSelectMenu] = useState(false);
  const [sweepDraft, setSweepDraft] = useState<SweepSelectionDraft | null>(null);
  const [threeDVisibilityMode, setThreeDVisibilityMode] = useState<ThreeDVisibilityMode>("active-floor-only");
  const [siteSetbackInput, setSiteSetbackInput] = useState("");
  const [siteSetbackError, setSiteSetbackError] = useState<string | null>(null);
  const selectMenuRef = useRef<HTMLDivElement | null>(null);
  const workspaceRef = useRef<HTMLElement | null>(null);
  const project = editorState.project;
  const activeLevelId = getValidActiveLevelId(project, editorState.activeLevelId);
  const activeLevel = getLevelById(project, activeLevelId) ?? project.levels[0];
  const sitePlan = getProjectSitePlan(project);
  const sitePlanLevel = sitePlan ? getLevelById(project, sitePlan.levelId) : null;
  const sitePlanEdges = getSitePlanEdges(sitePlan);
  const siteFootprintResult = deriveSitePlanFootprint(sitePlan);
  const siteFootprint = siteFootprintResult.footprint;
  const sitePlanSpaces = sitePlanLevel ? getLevelSpaces(project, sitePlanLevel.id) : [];
  const selectedSpaceIds = getSelectionSpaceIds(selection);
  const selectedSpaces = selectedSpaceIds.flatMap((spaceId) => {
    const space = project.spaces.find((candidate) => candidate.id === spaceId);
    return space ? [space] : [];
  });
  const selectedSpace = selection?.kind === "space" ? selectedSpaces[0] ?? null : null;
  const selectedLevel = selection?.kind === "level" ? getLevelById(project, selection.id) : null;
  const selectedSiteEdge = selection?.kind === "site-edge"
    ? sitePlanEdges[selection.edgeIndex] ?? null
    : null;
  const activeSpaces = activeLevel ? getLevelSpaces(project, activeLevel.id) : [];
  const browserSpaces = activeView === "site-plan" ? sitePlanSpaces : activeSpaces;
  const grossArea = project.spaces.reduce((total, space) => total + getSpaceAreaSqFt(space), 0);
  const currentViewLabel = activeLevel ? getViewLabel(activeView, activeLevel, sitePlanLevel) : "3D View";
  const selectionLabel = activeLevel
    ? getSelectionLabel(selection, activeLevel, sitePlanLevel, selectedSiteEdge, selectedLevel, selectedSpaces, activeView)
    : "None";
  const userEmail = auth.user?.email ?? "Signed in";
  const floorPlanBounds = getPlanBounds(activeSpaces.map((space) => space.footprint));
  const floorPlanWidth = floorPlanBounds.width * planScalePx;
  const floorPlanHeight = floorPlanBounds.height * planScalePx;
  const sitePlanBounds = getPlanBounds([
    ...(sitePlan ? [sitePlan.boundary] : []),
    ...(siteFootprint ? [siteFootprint] : []),
    ...sitePlanSpaces.map((space) => space.footprint)
  ]);
  const sitePlanWidth = sitePlanBounds.width * planScalePx;
  const sitePlanHeight = sitePlanBounds.height * planScalePx;
  const sweepSelectionBounds = sweepDraft ? getSweepSelectionBounds(sweepDraft) : null;
  const selectionAreaSqFt = selectedSpaces.reduce((total, space) => total + getSpaceAreaSqFt(space), 0);
  const selectedSpaceBounds = selectedSpace ? getSpaceBoundsFt(selectedSpace) : null;

  useEffect(() => {
    if (activeLevelId !== editorState.activeLevelId) {
      setEditorState((current) => (
        current.activeLevelId === activeLevelId
          ? current
          : { ...current, activeLevelId }
      ));
    }
  }, [activeLevelId, editorState.activeLevelId]);

  useEffect(() => {
    const handlePointerDown = (event: PointerEvent) => {
      const container = selectMenuRef.current;

      if (!container || container.contains(event.target as Node)) {
        return;
      }

      setShowSelectMenu(false);
    };

    window.addEventListener("pointerdown", handlePointerDown);
    return () => window.removeEventListener("pointerdown", handlePointerDown);
  }, []);

  useEffect(() => {
    if (!selectedSiteEdge) {
      setSiteSetbackInput("");
      setSiteSetbackError(null);
      return;
    }

    setSiteSetbackInput(formatFeetAndInches(selectedSiteEdge.setbackFt));
    setSiteSetbackError(null);
  }, [selectedSiteEdge?.index, selectedSiteEdge?.setbackFt]);

  useEffect(() => {
    if (!selection || !activeLevel) {
      return;
    }

    if (selection.kind === "site-edge") {
      if (activeView !== "site-plan" || !selectedSiteEdge) {
        setSelection(getViewSelection(activeView));
      }

      return;
    }

    if (selection.kind === "level" && !getLevelById(project, selection.id)) {
      setSelection({ kind: "level", id: activeLevel.id });
      return;
    }

    if (selection.kind === "space") {
      const space = project.spaces.find((item) => item.id === selection.id);

      if (!space || space.levelId !== activeLevel.id) {
        setSelection(getViewSelection(activeView));
      }

      return;
    }

    if (selection.kind === "space-set") {
      const visibleSpaceIds = selection.ids.filter((spaceId) => {
        const space = project.spaces.find((item) => item.id === spaceId);
        return Boolean(space && space.levelId === activeLevel.id);
      });

      if (visibleSpaceIds.length !== selection.ids.length) {
        setSelection(normalizeSpaceSelection(visibleSpaceIds, getViewSelection(activeView)));
      }
    }
  }, [activeLevel, activeView, project, selectedSiteEdge, selection, setSelection]);

  if (!activeLevel) {
    return null;
  }

  const sessionRows = [
    ["Select mode", getSelectModeLabel(selectMode)],
    ["View", currentViewLabel],
    ["Active level", activeLevel.name],
    ["Site host", sitePlanLevel?.name ?? "None"],
    ["Default height", formatFeetAndInches(project.defaultStoryHeightFt)],
    ["Units", "Imperial ft-in"]
  ];

  const selectionRows = selection?.kind === "space" && selectedSpace
    ? [
        ["Type", "Space"],
        ["Name", selectedSpace.name],
        ["Area", `${getSpaceAreaSqFt(selectedSpace)} sf`],
        ["Points", String(selectedSpace.footprint.length)],
        ["Bounds width", selectedSpaceBounds ? formatFeetAndInches(selectedSpaceBounds.widthFt) : "0\""],
        ["Bounds depth", selectedSpaceBounds ? formatFeetAndInches(selectedSpaceBounds.depthFt) : "0\""]
      ]
    : selection?.kind === "space-set" && selectedSpaces.length > 0
      ? [
          ["Type", "Multi-space"],
          ["Count", String(selectedSpaces.length)],
          ["Area", `${selectionAreaSqFt} sf`],
          ["Select mode", getSelectModeLabel(selectMode)],
          ["Clear", "Select > Clear Selection"]
        ]
    : selection?.kind === "level" && selectedLevel
      ? [
          ["Type", "Level"],
          ["Name", selectedLevel.name],
          ["Elevation", formatFeetAndInches(selectedLevel.elevationFt)],
          ["Height", formatFeetAndInches(selectedLevel.heightFt)],
          ["Spaces", String(getLevelSpaces(project, selectedLevel.id).length)]
        ]
      : selection?.kind === "site-edge" && selectedSiteEdge
        ? [
            ["Type", "Site Edge"],
            ["Host level", sitePlanLevel?.name ?? "None"],
            ["Edge", `${selectedSiteEdge.index + 1} of ${sitePlanEdges.length}`],
            ["Length", formatFeetAndInches(selectedSiteEdge.lengthFt)],
            ["Setback", formatFeetAndInches(selectedSiteEdge.setbackFt)]
          ]
      : selection?.kind === "view"
        ? [
            ["Type", "View"],
            ["Name", currentViewLabel],
            ["Mode", activeView === "3d" ? "Perspective" : activeView === "site-plan" ? "Site Plan" : "Plan"],
            ["Select mode", getSelectModeLabel(selectMode)]
          ]
        : [["Selection", "No selection"]];
  const viewItems: Array<{ id: "view-site-plan" | "view-3d" | "view-plan"; label: string; view: ViewMode }> = [
    { id: "view-site-plan", label: sitePlanLevel ? `${sitePlanLevel.name} Site Plan` : "Site Plan", view: "site-plan" },
    { id: "view-3d", label: "3D View", view: "3d" },
    { id: "view-plan", label: `${activeLevel.name} Floor Plan`, view: "plan" }
  ];

  const showView = (view: ViewMode) => {
    setActiveView(view);
    setSelection(getViewSelection(view));
  };

  const handleSelectModeChange = (mode: SelectMode) => {
    setSelectMode(mode);
    setShowSelectMenu(false);
  };

  const handleClearSelection = () => {
    setSelection(getViewSelection(activeView));
    setShowSelectMenu(false);
  };

  const handleSelectAllVisible = () => {
    setActiveView("plan");
    setSelection(normalizeSpaceSelection(activeSpaces.map((space) => space.id), getViewSelection("plan")));
    setShowSelectMenu(false);
  };

  const handlePlanSpaceSelection = (spaceId: string) => {
    setActiveView("plan");
    setSelection(toggleSpaceSelection(selection, spaceId, getViewSelection("plan")));
  };

  const handleBrowserSpaceSelection = (spaceId: string) => {
    const fallbackSelection = getViewSelection(activeView === "3d" ? "3d" : "plan");

    if (activeView !== "3d") {
      setActiveView("plan");
    }

    if (selectMode === "pick-many") {
      setSelection(toggleSpaceSelection(selection, spaceId, fallbackSelection));
      return;
    }

    setSelection({ kind: "space", id: spaceId });
  };

  const handleSiteEdgeSelection = (edgeIndex: number) => {
    setActiveView("site-plan");
    setSelection({ kind: "site-edge", edgeIndex });
    setSiteSetbackError(null);
  };

  const handleSiteSetbackCommit = (input: string) => {
    if (!selectedSiteEdge) {
      return;
    }

    const parsedSetback = parseFeetAndInches(input);

    if (parsedSetback === null || parsedSetback < 0) {
      setSiteSetbackInput(formatFeetAndInches(selectedSiteEdge.setbackFt));
      setSiteSetbackError("Setback must be 0 or greater.");
      return;
    }

    const formattedSetback = formatFeetAndInches(parsedSetback);

    setEditorState((current) => ({
      ...current,
      project: setSiteEdgeSetback(current.project, selectedSiteEdge.index, parsedSetback)
    }));
    setSiteSetbackInput(formattedSetback);
    setSiteSetbackError(null);
    setActiveSampleCaseId(null);
  };

  const handlePlanCanvasPointerDown = (event: ReactPointerEvent<HTMLDivElement>) => {
    if (selectMode !== "sweep" || event.button !== 0) {
      return;
    }

    const bounds = event.currentTarget.getBoundingClientRect();
    const x = event.clientX - bounds.left;
    const y = event.clientY - bounds.top;

    setSweepDraft({
      pointerId: event.pointerId,
      startX: x,
      startY: y,
      currentX: x,
      currentY: y,
      mode: getSweepMode(event)
    });

    event.currentTarget.setPointerCapture(event.pointerId);
    event.preventDefault();
  };

  const handlePlanCanvasPointerMove = (event: ReactPointerEvent<HTMLDivElement>) => {
    setSweepDraft((current) => {
      if (!current || current.pointerId !== event.pointerId) {
        return current;
      }

      const bounds = event.currentTarget.getBoundingClientRect();

      return {
        ...current,
        currentX: event.clientX - bounds.left,
        currentY: event.clientY - bounds.top
      };
    });
  };

  const handlePlanCanvasPointerEnd = (event: ReactPointerEvent<HTMLDivElement>) => {
    const currentDraft = sweepDraft;

    if (!currentDraft || currentDraft.pointerId !== event.pointerId) {
      return;
    }

    const bounds = event.currentTarget.getBoundingClientRect();
    const completedDraft: SweepSelectionDraft = {
      ...currentDraft,
      currentX: event.clientX - bounds.left,
      currentY: event.clientY - bounds.top
    };
    const sweptSpaceIds = getSweptSpaceIds(activeSpaces, floorPlanBounds, getSweepSelectionBounds(completedDraft));

    setActiveView("plan");
    setSelection(mergeSpaceSelection(selection, sweptSpaceIds, completedDraft.mode, getViewSelection("plan")));
    setSweepDraft(null);

    if (event.currentTarget.hasPointerCapture(event.pointerId)) {
      event.currentTarget.releasePointerCapture(event.pointerId);
    }

    event.preventDefault();
  };

  const setActiveLevel = (levelId: string) => {
    const nextLevelId = getValidActiveLevelId(project, levelId);
    setEditorState((current) => ({ ...current, activeLevelId: nextLevelId }));
    setSelection({ kind: "level", id: nextLevelId });
  };

  const handleCreateLevel = () => {
    let createdLevelId = "";

    setEditorState((current) => {
      const result = createLevel(current.project, current.activeLevelId);
      createdLevelId = result.activeLevelId;
      return { project: result.doc, activeLevelId: result.activeLevelId };
    });

    if (createdLevelId) {
      setSelection({ kind: "level", id: createdLevelId });
    }

    setActiveSampleCaseId(null);
  };

  const handleDeleteLevel = (levelId: string) => {
    let nextActiveLevelId = "";

    setEditorState((current) => {
      const result = deleteLevel(current.project, levelId, current.activeLevelId);
      nextActiveLevelId = result.activeLevelId;
      return { project: result.doc, activeLevelId: result.activeLevelId };
    });

    if (nextActiveLevelId) {
      setSelection({ kind: "level", id: nextActiveLevelId });
    }

    setActiveSampleCaseId(null);
  };

  const handleRenameLevel = (levelId: string, name: string) => {
    setEditorState((current) => {
      const nextProject = renameLevel(current.project, levelId, name);
      return { project: nextProject, activeLevelId: getValidActiveLevelId(nextProject, current.activeLevelId) };
    });
    setActiveSampleCaseId(null);
  };

  const handleMoveLevel = (levelId: string, direction: "up" | "down") => {
    setEditorState((current) => {
      const nextProject = moveLevel(current.project, levelId, direction);
      return { project: nextProject, activeLevelId: getValidActiveLevelId(nextProject, current.activeLevelId) };
    });
    setActiveSampleCaseId(null);
  };

  const handleSetLevelElevation = (levelId: string, elevationFt: number) => {
    setEditorState((current) => {
      const nextProject = setLevelElevation(current.project, levelId, elevationFt);
      return { project: nextProject, activeLevelId: getValidActiveLevelId(nextProject, current.activeLevelId) };
    });
    setActiveSampleCaseId(null);
  };

  const handleSetDefaultStoryHeight = (heightFt: number) => {
    setEditorState((current) => {
      const nextProject = setDefaultStoryHeight(current.project, heightFt);
      return { project: nextProject, activeLevelId: getValidActiveLevelId(nextProject, current.activeLevelId) };
    });
    setActiveSampleCaseId(null);
  };

  const handleAutoGenerateLevels = (input: AutoGenerateLevelsInput) => {
    let nextActiveLevelId = "";

    setEditorState((current) => {
      const result = autoGenerateLevels(current.project, input);
      nextActiveLevelId = result.activeLevelId;
      return { project: result.doc, activeLevelId: result.activeLevelId };
    });

    if (nextActiveLevelId) {
      setSelection({ kind: "level", id: nextActiveLevelId });
    }

    setActiveSampleCaseId(null);
  };

  const handleLoadSampleCase = (sampleCase: SampleCaseManifest) => {
    const projectDoc = cloneSampleProjectDoc(sampleCase.doc);
    const nextActiveLevelId = getValidActiveLevelId(projectDoc, sampleCase.preferredActiveLevelId);

    setEditorState({
      project: projectDoc,
      activeLevelId: nextActiveLevelId
    });
    setThreeDVisibilityMode("active-floor-only");
    setActiveView(sampleCase.preferredView);
    setSelection(sampleCase.preferredView === "plan"
      ? { kind: "level", id: nextActiveLevelId }
      : getViewSelection(sampleCase.preferredView));
    setActiveSampleCaseId(sampleCase.id);
    setShowSelectMenu(false);
    setSiteSetbackError(null);
  };

  const handleLogout = async () => {
    setLogoutPending(true);
    setLogoutError(null);

    const result = await logout();

    if (result.error) {
      setLogoutError("Sign-out failed.");
    } else {
      resetSessionUi();
    }

    setLogoutPending(false);
  };

  return (
    <main className="app-shell">
      <header className="ribbon">
        <div className="ribbon-brand">
          <strong>{project.name}</strong>
          <span>Lean editor shell</span>
        </div>

        <div className="ribbon-groups" aria-label="Ribbon commands">
          {ribbonGroups.map((group) => (
            <section key={group.title} className="ribbon-group">
              <div className="ribbon-buttons">
                {group.items.map((item) => (
                  <button key={item} type="button" className="ribbon-button">
                    {item}
                  </button>
                ))}
              </div>
              <span className="ribbon-group-label">{group.title}</span>
            </section>
          ))}

          <section className="ribbon-group ribbon-group-view">
            <div className="ribbon-buttons">
              <button
                type="button"
                className={`ribbon-button ${activeView === "site-plan" ? "is-active" : ""}`}
                aria-pressed={activeView === "site-plan"}
                onClick={() => showView("site-plan")}
              >
                Site
              </button>
              <button
                type="button"
                className={`ribbon-button ${activeView === "3d" ? "is-active" : ""}`}
                aria-pressed={activeView === "3d"}
                onClick={() => showView("3d")}
              >
                3D
              </button>
              <button
                type="button"
                className={`ribbon-button ${activeView === "plan" ? "is-active" : ""}`}
                aria-pressed={activeView === "plan"}
                onClick={() => showView("plan")}
              >
                Plan
              </button>
            </div>
            <span className="ribbon-group-label">View</span>
          </section>

          <section className="ribbon-group ribbon-group-select">
            <div ref={selectMenuRef} className="select-menu">
              <button
                type="button"
                className={`ribbon-button ${showSelectMenu ? "is-active" : ""}`}
                aria-expanded={showSelectMenu}
                onClick={() => setShowSelectMenu((current) => !current)}
              >
                Select
              </button>
              <span className="select-menu-summary">{getSelectModeLabel(selectMode)}</span>

              {showSelectMenu ? (
                <div className="select-menu-panel" role="menu" aria-label="Select tools">
                  <div className="select-menu-section">
                    {selectModeItems.map((item) => (
                      <button
                        key={item.value}
                        type="button"
                        className={`select-menu-item ${selectMode === item.value ? "is-active" : ""}`}
                        onClick={() => handleSelectModeChange(item.value)}
                      >
                        <strong>{item.label}</strong>
                        <span>{item.hint}</span>
                      </button>
                    ))}
                  </div>

                  <div className="select-menu-divider" />

                  <div className="select-menu-section">
                    <button type="button" className="select-menu-item" onClick={handleSelectAllVisible}>
                      <strong>Select All Visible</strong>
                      <span>Select every visible space on the active plan. Clear Selection removes them all.</span>
                    </button>

                    <button type="button" className="select-menu-item" onClick={handleClearSelection}>
                      <strong>Clear Selection</strong>
                      <span>Drop the current selection set and return focus to the current view.</span>
                    </button>
                  </div>
                </div>
              ) : null}
            </div>
            <span className="ribbon-group-label">Select</span>
          </section>

          <section className="ribbon-group ribbon-group-utility">
            <div className="ribbon-buttons">
              <button
                type="button"
                className={`ribbon-button ${showTestDashboard ? "is-active" : ""}`}
                aria-pressed={showTestDashboard}
                onClick={() => setShowTestDashboard((current) => !current)}
              >
                Test
              </button>
              <button
                type="button"
                className={`ribbon-button ${showLevelManager ? "is-active" : ""}`}
                aria-pressed={showLevelManager}
                onClick={() => setShowLevelManager((current) => !current)}
              >
                Levels
              </button>
              <button
                type="button"
                className={`ribbon-button ${showUnitsInspector ? "is-active" : ""}`}
                aria-pressed={showUnitsInspector}
                onClick={() => setShowUnitsInspector((current) => !current)}
              >
                Unit
              </button>
            </div>
            <span className="ribbon-group-label">Inspect</span>
          </section>
        </div>

        <div className="ribbon-side">
          <div className="ribbon-summary">
            <span>{project.levels.length} level</span>
            <span>{project.spaces.length} spaces</span>
            <span>{grossArea.toFixed(0)} sf</span>
          </div>

          <div className="ribbon-auth">
            <span className="ribbon-user" title={userEmail}>{userEmail}</span>
            <button
              type="button"
              className="ribbon-button"
              disabled={logoutPending}
              onClick={() => void handleLogout()}
            >
              {logoutPending ? "Logging out..." : "Log out"}
            </button>
            {logoutError ? <span className="ribbon-error">{logoutError}</span> : null}
          </div>
        </div>
      </header>

      <div className="main-shell">
        <aside className="sidebar sidebar-left">
          <section className="properties-panel">
            <div className="panel-title-row">
              <h2>Properties</h2>
              <span>{selectionLabel}</span>
            </div>

            <section className="property-group">
              <h3>Session</h3>
              <dl className="property-list">
                {sessionRows.map(([label, value]) => (
                  <div key={label}>
                    <dt>{label}</dt>
                    <dd>{value}</dd>
                  </div>
                ))}
              </dl>
            </section>

            <section className="property-group">
              <h3>Selection</h3>
              <dl className="property-list">
                {selectionRows.map(([label, value]) => (
                  <div key={label}>
                    <dt>{label}</dt>
                    <dd>{value}</dd>
                  </div>
                ))}
              </dl>
            </section>

            {selectedSiteEdge ? (
              <section className="property-group">
                <h3>Setback</h3>

                <label className="site-setback-field">
                  <span>Selected edge setback</span>
                  <input
                    value={siteSetbackInput}
                    onChange={(event) => setSiteSetbackInput(event.currentTarget.value)}
                    onBlur={(event) => handleSiteSetbackCommit(event.currentTarget.value)}
                    onKeyDown={(event) => {
                      if (event.key === "Enter") {
                        event.preventDefault();
                        handleSiteSetbackCommit(event.currentTarget.value);
                        event.currentTarget.blur();
                      }
                    }}
                  />
                </label>

                <p className="site-setback-note">
                  Edge {selectedSiteEdge.index + 1} on {sitePlanLevel?.name ?? "the site"} updates the derived building footprint.
                </p>

                {siteSetbackError ? <p className="level-manager-error">{siteSetbackError}</p> : null}
                {siteFootprintResult.error ? <p className="level-manager-error">{siteFootprintResult.error}</p> : null}
              </section>
            ) : null}
          </section>
        </aside>

        <section ref={workspaceRef} className="workspace-shell">
          <header className="view-tabs" aria-label="Workspace views">
            {viewItems.map((viewItem) => (
              <button
                key={viewItem.id}
                type="button"
                className={`view-tab ${activeView === viewItem.view ? "is-active" : ""}`}
                aria-pressed={activeView === viewItem.view}
                onClick={() => showView(viewItem.view)}
              >
                {viewItem.label}
              </button>
            ))}
          </header>

          <section className="viewport-shell">
            {activeView === "3d" ? (
              <ThreeDViewport
                project={project}
                activeLevelId={activeLevel.id}
                activeLevelName={activeLevel.name}
                selection={selection}
                selectionLabel={selectionLabel}
                visibilityMode={threeDVisibilityMode}
                onChangeVisibilityMode={setThreeDVisibilityMode}
              />
            ) : activeView === "site-plan" ? (
              <div className="viewport viewport-plan">
                {sitePlan ? (
                  <div className="plan-canvas-wrap">
                    <div
                      className="plan-canvas"
                      style={{ width: sitePlanWidth, height: sitePlanHeight }}
                    >
                      <svg
                        className="plan-svg"
                        width={Math.max(sitePlanWidth, 1)}
                        height={Math.max(sitePlanHeight, 1)}
                        viewBox={`0 0 ${Math.max(sitePlanWidth, 1)} ${Math.max(sitePlanHeight, 1)}`}
                      >
                        <polygon
                          className="site-plan-boundary"
                          points={getPlanPolygonPoints(sitePlan.boundary, sitePlanBounds)}
                        />

                        {siteFootprint ? (
                          <polygon
                            className="site-plan-footprint"
                            points={getPlanPolygonPoints(siteFootprint, sitePlanBounds)}
                          />
                        ) : null}

                        {sitePlanSpaces.map((space) => {
                          const labelPoint = getPlanLabelPosition(space, sitePlanBounds);

                          return (
                            <g key={space.id} className="plan-space site-plan-space">
                              <polygon
                                className="plan-space-shape"
                                points={getPlanPolygonPoints(space.footprint, sitePlanBounds)}
                              />
                              <text className="plan-space-label" x={labelPoint.x} y={labelPoint.y - 4} textAnchor="middle">
                                {space.name}
                              </text>
                              <text className="plan-space-metrics" x={labelPoint.x} y={labelPoint.y + 10} textAnchor="middle">
                                {getSpaceAreaSqFt(space)} sf
                              </text>
                            </g>
                          );
                        })}

                        {sitePlanEdges.map((edge) => {
                          const line = getPlanEdgeLine(edge, sitePlanBounds);
                          const isActive = selectedSiteEdge?.index === edge.index;

                          return (
                            <g key={`site-edge-${edge.index}`} className={`site-plan-edge ${isActive ? "is-active" : ""}`}>
                              <line
                                className="site-plan-edge-hit"
                                {...line}
                                onClick={() => handleSiteEdgeSelection(edge.index)}
                              />
                              <line
                                className="site-plan-edge-stroke"
                                {...line}
                              />
                            </g>
                          );
                        })}
                      </svg>

                      {siteFootprintResult.error ? (
                        <div className="site-plan-banner">
                          {siteFootprintResult.error}
                        </div>
                      ) : null}
                    </div>
                  </div>
                ) : (
                  <div className="plan-empty-state">
                    Load one of the mixed cases to open a site polygon with editable setbacks.
                  </div>
                )}
              </div>
            ) : (
              <div className="viewport viewport-plan">
                <div className="plan-canvas-wrap">
                  <div
                    className={`plan-canvas ${selectMode === "sweep" ? "is-sweep-mode" : ""}`}
                    style={{ width: floorPlanWidth, height: floorPlanHeight }}
                    onPointerDown={handlePlanCanvasPointerDown}
                    onPointerMove={handlePlanCanvasPointerMove}
                    onPointerUp={handlePlanCanvasPointerEnd}
                    onPointerCancel={handlePlanCanvasPointerEnd}
                  >
                    <svg
                      className="plan-svg"
                      width={Math.max(floorPlanWidth, 1)}
                      height={Math.max(floorPlanHeight, 1)}
                      viewBox={`0 0 ${Math.max(floorPlanWidth, 1)} ${Math.max(floorPlanHeight, 1)}`}
                    >
                      {activeSpaces.map((space) => {
                        const labelPoint = getPlanLabelPosition(space, floorPlanBounds);

                        return (
                          <g
                            key={space.id}
                            className={`plan-space ${hasSelectionSpace(selection, space.id) ? "is-active" : ""}`}
                            role="button"
                            tabIndex={0}
                            onClick={(event) => {
                              if (selectMode === "sweep") {
                                event.preventDefault();
                                return;
                              }

                              handlePlanSpaceSelection(space.id);
                            }}
                            onKeyDown={(event) => {
                              if (event.key === "Enter" || event.key === " ") {
                                event.preventDefault();
                                handlePlanSpaceSelection(space.id);
                              }
                            }}
                          >
                            <polygon
                              className="plan-space-shape"
                              points={getPlanPolygonPoints(space.footprint, floorPlanBounds)}
                            />
                            <text className="plan-space-label" x={labelPoint.x} y={labelPoint.y - 4} textAnchor="middle">
                              {space.name}
                            </text>
                            <text className="plan-space-metrics" x={labelPoint.x} y={labelPoint.y + 10} textAnchor="middle">
                              {getSpaceAreaSqFt(space)} sf
                            </text>
                          </g>
                        );
                      })}
                    </svg>

                    {sweepSelectionBounds ? (
                      <div
                        className={`plan-sweep-box is-${sweepDraft?.mode ?? "replace"}`}
                        style={{
                          left: sweepSelectionBounds.left,
                          top: sweepSelectionBounds.top,
                          width: sweepSelectionBounds.width,
                          height: sweepSelectionBounds.height
                        }}
                      />
                    ) : null}
                  </div>
                </div>
              </div>
            )}
            {showLevelManager ? (
              <LevelManager
                project={project}
                activeLevelId={activeLevel.id}
                onClose={() => setShowLevelManager(false)}
                onActivateLevel={setActiveLevel}
                onCreateLevel={handleCreateLevel}
                onDeleteLevel={handleDeleteLevel}
                onRenameLevel={handleRenameLevel}
                onMoveLevel={handleMoveLevel}
                onSetLevelElevation={handleSetLevelElevation}
                onSetDefaultStoryHeight={handleSetDefaultStoryHeight}
                onAutoGenerate={handleAutoGenerateLevels}
              />
            ) : null}

            {showTestDashboard ? (
              <TestDashboard
                workspaceRef={workspaceRef}
                cases={MIXED_CASES}
                activeCaseId={activeSampleCaseId}
                onLoadCase={handleLoadSampleCase}
                onClose={() => setShowTestDashboard(false)}
              />
            ) : null}

            <UnitsInspector open={showUnitsInspector} onClose={() => setShowUnitsInspector(false)} />
          </section>
        </section>

        <aside className="sidebar sidebar-right">
          <section className="project-browser">
            <div className="panel-title-row">
              <h2>Project Browser</h2>
              <span>{activeView === "site-plan" ? (sitePlanLevel?.name ?? "No Site Plan") : activeLevel.name}</span>
            </div>

            <section className="browser-group">
              <h3>Views</h3>
              <div className="browser-list">
                {viewItems.map((viewItem) => (
                  <button
                    key={viewItem.id}
                    type="button"
                    className={`browser-row ${activeView === viewItem.view ? "is-active" : ""}`}
                    onClick={() => showView(viewItem.view)}
                  >
                    <span className="browser-row-kind">View</span>
                    <span>{viewItem.label}</span>
                  </button>
                ))}
              </div>
            </section>

            <section className="browser-group">
              <h3>Levels</h3>
              <div className="browser-list">
                {project.levels.map((level) => (
                  <button
                    key={level.id}
                    type="button"
                    className={`browser-row ${activeLevel.id === level.id ? "is-active" : ""}`}
                    onClick={() => setActiveLevel(level.id)}
                  >
                    <span className="browser-row-kind">Level</span>
                    <span>{level.name}</span>
                  </button>
                ))}
              </div>
            </section>

            <section className="browser-group">
              <h3>Spaces</h3>
              <div className="browser-list">
                {browserSpaces.map((space) => (
                  <button
                    key={space.id}
                    type="button"
                    className={`browser-row ${hasSelectionSpace(selection, space.id) ? "is-active" : ""}`}
                    onClick={() => handleBrowserSpaceSelection(space.id)}
                  >
                    <span className="browser-row-kind">Space</span>
                    <span>{space.name}</span>
                  </button>
                ))}
              </div>
            </section>
          </section>
        </aside>
      </div>

      <footer className="status-bar">
        <span>Units: Imperial ft-in</span>
        <span>Active level: {activeLevel.name}</span>
        <span>Plan spaces: {browserSpaces.length}</span>
        <span>View: {currentViewLabel}</span>
        {activeView === "3d" ? <span>3D scope: {getThreeDVisibilityModeLabel(threeDVisibilityMode)}</span> : null}
        <span>Select: {getSelectModeLabel(selectMode)}</span>
        <span>Hint: {activeView === "site-plan" ? "Click a site edge to edit setback." : getSelectModeHint(selectMode)}</span>
        <span>Case: {activeSampleCaseId ?? "Local"}</span>
        <span>Selection: {selectionLabel}</span>
      </footer>
    </main>
  );
}
