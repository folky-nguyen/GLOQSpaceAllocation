import { useEffect, useState, type KeyboardEvent } from "react";
import { logout, useAuth } from "./auth";
import {
  autoGenerateLevels,
  createLevel,
  createStarterProjectDoc,
  deleteLevel,
  getLevelById,
  getLevelSpaces,
  getSpaceAreaSqFt,
  getValidActiveLevelId,
  moveLevel,
  renameLevel,
  setDefaultStoryHeight,
  setLevelElevation,
  type AutoGenerateLevelsInput,
  type Level,
  type ProjectDoc,
  type Space
} from "./project-doc";
import { formatFeetAndInches, parseFeetAndInches } from "./units";
import { useUiStore, type Selection, type ToolMode, type ViewMode } from "./ui-store";
import UnitsInspector from "./units-inspector";

const toolItems: Array<{ value: ToolMode; label: string; hint: string }> = [
  { value: "select", label: "Select", hint: "Inspect model items" },
  { value: "space", label: "Space", hint: "Author room-like areas" },
  { value: "level", label: "Level", hint: "Manage vertical datums" }
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
  return { kind: "view", id: view === "3d" ? "view-3d" : "view-plan" };
}

function getToolLabel(tool: ToolMode): string {
  return toolItems.find((item) => item.value === tool)?.label ?? "Select";
}

function getViewLabel(view: ViewMode, level: Level): string {
  return view === "3d" ? "3D View" : `${level.name} Floor Plan`;
}

function getSelectionLabel(
  selection: Selection,
  activeLevel: Level,
  selectedLevel: Level | null,
  selectedSpace: Space | null,
  view: ViewMode
): string {
  if (!selection) {
    return "None";
  }

  if (selection.kind === "space" && selectedSpace) {
    return selectedSpace.name;
  }

  if (selection.kind === "level" && selectedLevel) {
    return selectedLevel.name;
  }

  return getViewLabel(view, activeLevel);
}

function getPlanBounds(spaces: Space[]): PlanBounds {
  if (spaces.length === 0) {
    return { minX: 0, minY: 0, width: 0, height: 0 };
  }

  const bounds = spaces.reduce(
    (currentBounds, space) => ({
      minX: Math.min(currentBounds.minX, space.xFt),
      minY: Math.min(currentBounds.minY, space.yFt),
      maxX: Math.max(currentBounds.maxX, space.xFt + space.widthFt),
      maxY: Math.max(currentBounds.maxY, space.yFt + space.depthFt)
    }),
    {
      minX: Number.POSITIVE_INFINITY,
      minY: Number.POSITIVE_INFINITY,
      maxX: Number.NEGATIVE_INFINITY,
      maxY: Number.NEGATIVE_INFINITY
    }
  );

  return {
    minX: bounds.minX,
    minY: bounds.minY,
    width: bounds.maxX - bounds.minX,
    height: bounds.maxY - bounds.minY
  };
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
  const activeTool = useUiStore((state) => state.activeTool);
  const selection = useUiStore((state) => state.selection);
  const setActiveTool = useUiStore((state) => state.setActiveTool);
  const setActiveView = useUiStore((state) => state.setActiveView);
  const setSelection = useUiStore((state) => state.setSelection);
  const [editorState, setEditorState] = useState<EditorState>(() => {
    const project = createStarterProjectDoc();

    return { project, activeLevelId: project.levels[0]?.id ?? "" };
  });
  const [logoutPending, setLogoutPending] = useState(false);
  const [logoutError, setLogoutError] = useState<string | null>(null);
  const [showUnitsInspector, setShowUnitsInspector] = useState(false);
  const [showLevelManager, setShowLevelManager] = useState(false);
  const project = editorState.project;
  const activeLevelId = getValidActiveLevelId(project, editorState.activeLevelId);
  const activeLevel = getLevelById(project, activeLevelId) ?? project.levels[0];
  const selectedSpace = selection?.kind === "space"
    ? project.spaces.find((space) => space.id === selection.id) ?? null
    : null;
  const selectedLevel = selection?.kind === "level" ? getLevelById(project, selection.id) : null;
  const activeSpaces = activeLevel ? getLevelSpaces(project, activeLevel.id) : [];
  const grossArea = project.spaces.reduce((total, space) => total + getSpaceAreaSqFt(space), 0);
  const currentViewLabel = activeLevel ? getViewLabel(activeView, activeLevel) : "3D View";
  const selectionLabel = activeLevel
    ? getSelectionLabel(selection, activeLevel, selectedLevel, selectedSpace, activeView)
    : "None";
  const userEmail = auth.user?.email ?? "Signed in";

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
    if (!selection || !activeLevel) {
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
    }
  }, [activeLevel, activeView, project, selection, setSelection]);

  if (!activeLevel) {
    return null;
  }

  const sessionRows = [
    ["Tool", getToolLabel(activeTool)],
    ["View", currentViewLabel],
    ["Active level", activeLevel.name],
    ["Default height", formatFeetAndInches(project.defaultStoryHeightFt)],
    ["Units", "Imperial ft-in"]
  ];

  const selectionRows = selection?.kind === "space" && selectedSpace
    ? [
        ["Type", "Space"],
        ["Name", selectedSpace.name],
        ["Area", `${getSpaceAreaSqFt(selectedSpace)} sf`],
        ["Width", formatFeetAndInches(selectedSpace.widthFt)],
        ["Depth", formatFeetAndInches(selectedSpace.depthFt)]
      ]
    : selection?.kind === "level" && selectedLevel
      ? [
          ["Type", "Level"],
          ["Name", selectedLevel.name],
          ["Elevation", formatFeetAndInches(selectedLevel.elevationFt)],
          ["Height", formatFeetAndInches(selectedLevel.heightFt)],
          ["Spaces", String(getLevelSpaces(project, selectedLevel.id).length)]
        ]
      : selection?.kind === "view"
        ? [
            ["Type", "View"],
            ["Name", currentViewLabel],
            ["Mode", activeView === "3d" ? "Perspective" : "Plan"],
            ["Selection", selectionLabel]
          ]
        : [["Selection", "No selection"]];

  const planBounds = getPlanBounds(activeSpaces);
  const planWidth = planBounds.width * planScalePx;
  const planHeight = planBounds.height * planScalePx;
  const viewItems: Array<{ id: "view-3d" | "view-plan"; label: string; view: ViewMode }> = [
    { id: "view-3d", label: "3D View", view: "3d" },
    { id: "view-plan", label: `${activeLevel.name} Floor Plan`, view: "plan" }
  ];
  const visibleSpaceNames = activeSpaces.map((space) => space.name).join(", ");

  const showView = (view: ViewMode) => {
    setActiveView(view);
    setSelection(getViewSelection(view));
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
  };

  const handleRenameLevel = (levelId: string, name: string) => {
    setEditorState((current) => {
      const nextProject = renameLevel(current.project, levelId, name);
      return { project: nextProject, activeLevelId: getValidActiveLevelId(nextProject, current.activeLevelId) };
    });
  };

  const handleMoveLevel = (levelId: string, direction: "up" | "down") => {
    setEditorState((current) => {
      const nextProject = moveLevel(current.project, levelId, direction);
      return { project: nextProject, activeLevelId: getValidActiveLevelId(nextProject, current.activeLevelId) };
    });
  };

  const handleSetLevelElevation = (levelId: string, elevationFt: number) => {
    setEditorState((current) => {
      const nextProject = setLevelElevation(current.project, levelId, elevationFt);
      return { project: nextProject, activeLevelId: getValidActiveLevelId(nextProject, current.activeLevelId) };
    });
  };

  const handleSetDefaultStoryHeight = (heightFt: number) => {
    setEditorState((current) => {
      const nextProject = setDefaultStoryHeight(current.project, heightFt);
      return { project: nextProject, activeLevelId: getValidActiveLevelId(nextProject, current.activeLevelId) };
    });
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
  };

  const handleLogout = async () => {
    setLogoutPending(true);
    setLogoutError(null);

    const result = await logout();

    if (result.error) {
      setLogoutError("Sign-out failed.");
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

          <section className="ribbon-group ribbon-group-utility">
            <div className="ribbon-buttons">
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
