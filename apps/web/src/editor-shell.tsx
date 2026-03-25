import { useEffect, useState, type FocusEvent, type KeyboardEvent } from "react";
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
  {
    title: "File",
    items: ["New", "Save"]
  },
  {
    title: "Edit",
    items: ["Undo", "Redo"]
  }
];

const planScalePx = 10;
const planPaddingPx = 40;

type LevelManagerProps = {
  activeLevelId: string;
  onActivateLevel: (levelId: string) => void;
  onAutoGenerate: (input: AutoGenerateLevelsInput) => void;
  onClose: () => void;
  onCreateLevel: () => void;
  onDeleteLevel: (levelId: string) => void;
  onMoveLevel: (levelId: string, direction: "up" | "down") => void;
  onRenameLevel: (levelId: string, name: string) => void;
  onSetDefaultStoryHeight: (heightFt: number) => void;
  onSetLevelElevation: (levelId: string, elevationFt: number) => void;
  open: boolean;
  project: ProjectDoc;
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

function handleCommitKeyDown(event: KeyboardEvent<HTMLInputElement>) {
  if (event.key === "Enter") {
    event.currentTarget.blur();
  }
}

function getStoryBreakdown(project: ProjectDoc): { belowGrade: number; onGrade: number } {
  const belowGrade = project.levels.filter((level) => level.elevationFt < 0).length;
  return {
    belowGrade,
    onGrade: project.levels.length - belowGrade
  };
}

function LevelManager({
  activeLevelId,
  onActivateLevel,
  onAutoGenerate,
  onClose,
  onCreateLevel,
  onDeleteLevel,
  onMoveLevel,
  onRenameLevel,
  onSetDefaultStoryHeight,
  onSetLevelElevation,
  open,
  project
}: LevelManagerProps) {
  const [storiesBelowGrade, setStoriesBelowGrade] = useState("0");
  const [storiesOnGrade, setStoriesOnGrade] = useState(String(project.levels.length));
  const [storyHeightInput, setStoryHeightInput] = useState(formatFeetAndInches(project.defaultStoryHeightFt));
  const [defaultStoryHeightInput, setDefaultStoryHeightInput] = useState(formatFeetAndInches(project.defaultStoryHeightFt));
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (!open) {
      return;
    }

    const storyBreakdown = getStoryBreakdown(project);
    const formattedHeight = formatFeetAndInches(project.defaultStoryHeightFt);
    setStoriesBelowGrade(String(storyBreakdown.belowGrade));
    setStoriesOnGrade(String(storyBreakdown.onGrade));
    setStoryHeightInput(formattedHeight);
    setDefaultStoryHeightInput(formattedHeight);
  }, [open, project]);

  if (!open) {
    return null;
  }

  const resetElevationField = (event: FocusEvent<HTMLInputElement>, level: Level) => {
    event.currentTarget.value = formatFeetAndInches(level.elevationFt);
  };

  const handleElevationBlur = (event: FocusEvent<HTMLInputElement>, level: Level) => {
    const nextElevationFt = parseFeetAndInches(event.currentTarget.value);

    if (nextElevationFt === null) {
      setError("Level elevations must use ft-in format, for example 0', -10', or 12' 6\".");
      resetElevationField(event, level);
      return;
    }

    setError(null);
    onSetLevelElevation(level.id, nextElevationFt);
    event.currentTarget.value = formatFeetAndInches(nextElevationFt);
  };

  const commitDefaultStoryHeight = () => {
    const nextHeightFt = parseFeetAndInches(defaultStoryHeightInput);

    if (nextHeightFt === null || nextHeightFt <= 0) {
      setError("Default story height must use ft-in format and be greater than zero.");
      setDefaultStoryHeightInput(formatFeetAndInches(project.defaultStoryHeightFt));
      return;
    }

    setError(null);
    onSetDefaultStoryHeight(nextHeightFt);
    setDefaultStoryHeightInput(formatFeetAndInches(nextHeightFt));
    setStoryHeightInput(formatFeetAndInches(nextHeightFt));
  };

  const handleAutoGenerate = () => {
    const nextStoriesBelowGrade = Number.parseInt(storiesBelowGrade, 10);
    const nextStoriesOnGrade = Number.parseInt(storiesOnGrade, 10);
    const nextStoryHeightFt = parseFeetAndInches(storyHeightInput);

    if (
      Number.isNaN(nextStoriesBelowGrade)
      || Number.isNaN(nextStoriesOnGrade)
      || nextStoriesBelowGrade < 0
      || nextStoriesOnGrade < 0
    ) {
      setError("Story counts must be whole numbers greater than or equal to zero.");
      return;
    }

    if (nextStoriesBelowGrade + nextStoriesOnGrade < 1) {
      setError("Generate at least one story.");
      return;
    }

    if (nextStoryHeightFt === null || nextStoryHeightFt <= 0) {
      setError("Story height must use ft-in format and be greater than zero.");
      return;
    }

    setError(null);
    onAutoGenerate({
      storiesBelowGrade: nextStoriesBelowGrade,
      storiesOnGrade: nextStoriesOnGrade,
      storyHeightFt: nextStoryHeightFt
    });
    setDefaultStoryHeightInput(formatFeetAndInches(nextStoryHeightFt));
    setStoryHeightInput(formatFeetAndInches(nextStoryHeightFt));
  };

  return (
    <section className="level-manager" role="dialog" aria-labelledby="level-manager-title">
      <div className="level-manager-header">
        <div>
          <strong id="level-manager-title">Level Manager</strong>
          <span>All level math stays in internal feet. UI stays feet-inch.</span>
        </div>
        <div className="level-manager-header-actions">
          <button type="button" className="units-inspector-close" onClick={onCreateLevel}>
            Add level
          </button>
          <button type="button" className="units-inspector-close" onClick={onClose}>
            Close
          </button>
        </div>
      </div>

      <section className="level-manager-section">
        <div className="units-inspector-title-row">
          <h3>Auto-generate</h3>
          <span>Rebuild the stack from story counts.</span>
        </div>

        <div className="level-manager-grid">
          <label className="units-inspector-field">
            <span>Stories below grade</span>
            <input
              inputMode="numeric"
              min={0}
              step={1}
              type="number"
              value={storiesBelowGrade}
              onChange={(event) => setStoriesBelowGrade(event.target.value)}
            />
          </label>

          <label className="units-inspector-field">
            <span>Stories on grade</span>
            <input
              inputMode="numeric"
              min={0}
              step={1}
              type="number"
              value={storiesOnGrade}
              onChange={(event) => setStoriesOnGrade(event.target.value)}
            />
          </label>

          <label className="units-inspector-field level-manager-grid-span">
            <span>Story height</span>
            <input
              type="text"
              value={storyHeightInput}
              onChange={(event) => setStoryHeightInput(event.target.value)}
              onKeyDown={handleCommitKeyDown}
            />
          </label>
        </div>

        <div className="level-manager-actions">
          <button type="button" className="ribbon-button" onClick={handleAutoGenerate}>
            Generate levels
          </button>
          <span>Keeps spaces only on levels that survive by name.</span>
        </div>
      </section>

      <section className="level-manager-section">
        <div className="units-inspector-title-row">
          <h3>Defaults</h3>
          <span>Used for new levels and generation.</span>
        </div>

        <label className="units-inspector-field">
          <span>Default story height</span>
          <input
            type="text"
            value={defaultStoryHeightInput}
            onBlur={commitDefaultStoryHeight}
            onChange={(event) => setDefaultStoryHeightInput(event.target.value)}
            onKeyDown={handleCommitKeyDown}
          />
        </label>
      </section>

      <section className="level-manager-section">
        <div className="units-inspector-title-row">
          <h3>Levels</h3>
          <span>{project.levels.length} total</span>
        </div>

        <div className="level-manager-list">
          {project.levels.map((level, index) => {
            const levelSpaceCount = getLevelSpaces(project, level.id).length;
            const isActive = level.id === activeLevelId;

            return (
              <article key={level.id} className={`level-row ${isActive ? "is-active" : ""}`}>
                <div className="level-row-main">
                  <button
                    type="button"
                    className={`level-row-activate ${isActive ? "is-active" : ""}`}
                    aria-pressed={isActive}
                    onClick={() => onActivateLevel(level.id)}
                  >
                    {isActive ? "Active" : "Set active"}
                  </button>

                  <label className="units-inspector-field">
                    <span>Name</span>
                    <input
                      type="text"
                      value={level.name}
                      onChange={(event) => onRenameLevel(level.id, event.target.value)}
                    />
                  </label>

                  <label className="units-inspector-field">
                    <span>Elevation</span>
                    <input
                      key={`${level.id}-${level.elevationFt}`}
                      type="text"
                      defaultValue={formatFeetAndInches(level.elevationFt)}
                      onBlur={(event) => handleElevationBlur(event, level)}
                      onKeyDown={handleCommitKeyDown}
                    />
                  </label>

                  <div className="level-row-meta">
                    <span>Height {formatFeetAndInches(level.heightFt)}</span>
                    <span>{levelSpaceCount} spaces</span>
                  </div>
                </div>

                <div className="level-row-actions">
                  <button
                    type="button"
                    className="ribbon-button"
                    disabled={index === 0}
                    onClick={() => onMoveLevel(level.id, "up")}
                  >
                    Up
                  </button>
                  <button
                    type="button"
                    className="ribbon-button"
                    disabled={index === project.levels.length - 1}
                    onClick={() => onMoveLevel(level.id, "down")}
                  >
                    Down
                  </button>
                  <button
                    type="button"
                    className="ribbon-button"
                    disabled={project.levels.length === 1}
                    onClick={() => onDeleteLevel(level.id)}
                  >
                    Delete
                  </button>
                </div>
              </article>
            );
          })}
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
  const [editorState, setEditorState] = useState(() => {
    const project = createStarterProjectDoc();
    return {
      project,
      activeLevelId: project.levels[0].id
    };
  });
  const [logoutPending, setLogoutPending] = useState(false);
  const [logoutError, setLogoutError] = useState<string | null>(null);
  const [showUnitsInspector, setShowUnitsInspector] = useState(false);
  const [showLevelManager, setShowLevelManager] = useState(false);

  const project = editorState.project;
  const activeLevelId = getValidActiveLevelId(project, editorState.activeLevelId);

  useEffect(() => {
    if (activeLevelId !== editorState.activeLevelId) {
      setEditorState((current) => ({
        ...current,
        activeLevelId
      }));
    }
  }, [activeLevelId, editorState.activeLevelId]);

  useEffect(() => {
    if (!selection) {
      return;
    }

    if (selection.kind === "level" && !getLevelById(project, selection.id)) {
      setSelection({ kind: "level", id: activeLevelId });
      return;
    }

    if (selection.kind === "space") {
      const selectedSpace = project.spaces.find((space) => space.id === selection.id) ?? null;

      if (!selectedSpace || selectedSpace.levelId !== activeLevelId) {
        setSelection(getViewSelection(activeView));
      }
    }
  }, [activeLevelId, activeView, project, selection, setSelection]);

  const activeLevel = getLevelById(project, activeLevelId) ?? project.levels[0];
  const selectedSpace = selection?.kind === "space"
    ? project.spaces.find((space) => space.id === selection.id) ?? null
    : null;
  const selectedLevel = selection?.kind === "level"
    ? getLevelById(project, selection.id)
    : null;
  const selectedSpaceLevel = selectedSpace ? getLevelById(project, selectedSpace.levelId) : null;
  const activeSpaces = getLevelSpaces(project, activeLevel.id);
  const grossArea = project.spaces.reduce((total, space) => total + getSpaceAreaSqFt(space), 0);
  const currentViewLabel = getViewLabel(activeView, activeLevel);
  const selectionLabel = getSelectionLabel(selection, activeLevel, selectedLevel, selectedSpace, activeView);
  const userEmail = auth.user?.email ?? "Signed in";

  const sessionRows = [
    ["Tool", getToolLabel(activeTool)],
    ["View", currentViewLabel],
    ["Active level", activeLevel.name],
    ["Units", "Imperial ft-in"]
  ];

  const selectionRows = selection?.kind === "space" && selectedSpace
    ? [
        ["Type", "Space"],
        ["Name", selectedSpace.name],
        ["Level", selectedSpaceLevel?.name ?? activeLevel.name],
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
            ["Visible level", activeLevel.name]
          ]
        : [["Selection", "No selection"]];

  const planBounds = activeSpaces.reduce(
    (bounds, space) => ({
      maxX: Math.max(bounds.maxX, space.xFt + space.widthFt),
      maxY: Math.max(bounds.maxY, space.yFt + space.depthFt)
    }),
    { maxX: 0, maxY: 0 }
  );

  const planWidth = Math.max(640, planBounds.maxX * planScalePx + planPaddingPx * 2);
  const planHeight = Math.max(420, planBounds.maxY * planScalePx + planPaddingPx * 2);

  const viewItems: Array<{ id: "view-3d" | "view-plan"; label: string; view: ViewMode }> = [
    { id: "view-3d", label: "3D View", view: "3d" },
    { id: "view-plan", label: `${activeLevel.name} Floor Plan`, view: "plan" }
  ];

  const showView = (view: ViewMode) => {
    setActiveView(view);
    setSelection(getViewSelection(view));
  };

  const setNextProject = (nextProject: ProjectDoc, nextActiveLevelId: string) => {
    setEditorState({
      project: nextProject,
      activeLevelId: nextActiveLevelId
    });
  };

  const activateLevel = (levelId: string) => {
    setEditorState((current) => ({
      ...current,
      activeLevelId: getValidActiveLevelId(current.project, levelId)
    }));
    setSelection({ kind: "level", id: levelId });
  };

  const handleCreateLevel = () => {
    const result = createLevel(project, activeLevelId);
    setNextProject(result.doc, result.activeLevelId);
    setSelection({ kind: "level", id: result.activeLevelId });
  };

  const handleDeleteLevel = (levelId: string) => {
    const result = deleteLevel(project, levelId, activeLevelId);
    setNextProject(result.doc, result.activeLevelId);
    setSelection({ kind: "level", id: result.activeLevelId });
  };

  const handleAutoGenerate = (input: AutoGenerateLevelsInput) => {
    const result = autoGenerateLevels(project, input);
    setNextProject(result.doc, result.activeLevelId);
    setSelection({ kind: "level", id: result.activeLevelId });
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
            <span>Active {activeLevel.name}</span>
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
          <section className="tool-strip" aria-label="Editor tools">
            <div className="panel-title-row">
              <h2>Tools</h2>
            </div>
            <div className="tool-strip-buttons">
              {toolItems.map((tool) => (
                <button
                  key={tool.value}
                  type="button"
                  className={`tool-button ${activeTool === tool.value ? "is-active" : ""}`}
                  aria-pressed={activeTool === tool.value}
                  onClick={() => setActiveTool(tool.value)}
                >
                  <strong>{tool.label}</strong>
                  <span>{tool.hint}</span>
                </button>
              ))}
            </div>
          </section>

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
          </section>
        </aside>

        <section className="workspace-shell">
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
              <div className="viewport viewport-3d">
                <div className="viewport-badge">wgpu + wasm-bindgen</div>
                <div className="viewport-copy">
                  <p className="viewport-title">3D View</p>
                  <p className="viewport-subtitle">Visible content is filtered by the active level.</p>
                </div>
                <dl className="viewport-stats">
                  <div>
                    <dt>Camera</dt>
                    <dd>Perspective</dd>
                  </div>
                  <div>
                    <dt>Visible level</dt>
                    <dd>{activeLevel.name}</dd>
                  </div>
                  <div>
                    <dt>Elevation</dt>
                    <dd>{formatFeetAndInches(activeLevel.elevationFt)}</dd>
                  </div>
                  <div>
                    <dt>Visible spaces</dt>
                    <dd>{activeSpaces.length}</dd>
                  </div>
                </dl>

                <div className="viewport-level-stack" aria-label="Visible level in 3D">
                  <div className="viewport-level-card">
                    <strong>{activeLevel.name}</strong>
                    <span>{formatFeetAndInches(activeLevel.elevationFt)} datum</span>
                    <small>{formatFeetAndInches(activeLevel.heightFt)} story height</small>
                  </div>

                  <div className="viewport-space-list">
                    {activeSpaces.length === 0 ? (
                      <p className="viewport-space-empty">No spaces on the active level.</p>
                    ) : (
                      activeSpaces.map((space) => (
                        <div key={space.id} className="viewport-space-card">
                          <strong>{space.name}</strong>
                          <span>{getSpaceAreaSqFt(space)} sf</span>
                        </div>
                      ))
                    )}
                  </div>
                </div>
              </div>
            ) : (
              <div className="viewport viewport-plan">
                <div className="plan-header">
                  <div>
                    <p className="viewport-title">Floor Plan</p>
                    <p className="viewport-subtitle">{activeLevel.name}</p>
                  </div>
                  <div className="plan-meta">
                    <span>{activeSpaces.length} spaces</span>
                    <span>{grossArea.toFixed(0)} sf total</span>
                    <span>{formatFeetAndInches(activeLevel.elevationFt)} elev.</span>
                  </div>
                </div>

                <div className="plan-canvas-wrap">
                  <div className="plan-canvas" style={{ width: planWidth, height: planHeight }}>
                    {activeSpaces.map((space) => (
                      <button
                        key={space.id}
                        type="button"
                        className={`plan-space ${selection?.kind === "space" && selection.id === space.id ? "is-active" : ""}`}
                        style={{
                          left: planPaddingPx + space.xFt * planScalePx,
                          top: planPaddingPx + space.yFt * planScalePx,
                          width: space.widthFt * planScalePx,
                          height: space.depthFt * planScalePx
                        }}
                        onClick={() => {
                          setActiveView("plan");
                          setSelection({ kind: "space", id: space.id });
                        }}
                      >
                        <strong>{space.name}</strong>
                        <span>{getSpaceAreaSqFt(space)} sf</span>
                        <small>
                          {formatFeetAndInches(space.widthFt)} x {formatFeetAndInches(space.depthFt)}
                        </small>
                      </button>
                    ))}
                  </div>
                </div>
              </div>
            )}
          </section>

          <LevelManager
            open={showLevelManager}
            project={project}
            activeLevelId={activeLevelId}
            onClose={() => setShowLevelManager(false)}
            onCreateLevel={handleCreateLevel}
            onDeleteLevel={handleDeleteLevel}
            onRenameLevel={(levelId, name) => {
              setEditorState((current) => ({
                ...current,
                project: renameLevel(current.project, levelId, name)
              }));
            }}
            onMoveLevel={(levelId, direction) => {
              setEditorState((current) => ({
                ...current,
                project: moveLevel(current.project, levelId, direction)
              }));
            }}
            onSetLevelElevation={(levelId, elevationFt) => {
              setEditorState((current) => ({
                ...current,
                project: setLevelElevation(current.project, levelId, elevationFt)
              }));
            }}
            onSetDefaultStoryHeight={(heightFt) => {
              setEditorState((current) => ({
                ...current,
                project: setDefaultStoryHeight(current.project, heightFt)
              }));
            }}
            onActivateLevel={activateLevel}
            onAutoGenerate={handleAutoGenerate}
          />

          <UnitsInspector open={showUnitsInspector} onClose={() => setShowUnitsInspector(false)} />
        </section>

        <aside className="sidebar sidebar-right">
          <section className="project-browser">
            <div className="panel-title-row">
              <h2>Project Browser</h2>
              <span>{activeLevel.name}</span>
            </div>

            <section className="browser-group">
              <h3>Views</h3>
              <div className="browser-list">
                {viewItems.map((viewItem) => (
                  <button
                    key={viewItem.id}
                    type="button"
                    className={`browser-row ${selection?.kind === "view" && selection.id === viewItem.id ? "is-active" : ""}`}
                    onClick={() => showView(viewItem.view)}
                  >
                    <span className="browser-row-kind">View</span>
                    <span className="browser-row-title">
                      <strong>{viewItem.label}</strong>
                      <small>{viewItem.view === "3d" ? "Filtered by active level" : "Editing view"}</small>
                    </span>
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
                    className={`browser-row ${activeLevelId === level.id ? "is-active" : ""}`}
                    onClick={() => activateLevel(level.id)}
                  >
                    <span className="browser-row-kind">Level</span>
                    <span className="browser-row-title">
                      <strong>{level.name}</strong>
                      <small>{formatFeetAndInches(level.elevationFt)} {activeLevelId === level.id ? "active" : ""}</small>
                    </span>
                  </button>
                ))}
              </div>
            </section>

            <section className="browser-group">
              <h3>Spaces</h3>
              <div className="browser-list">
                {activeSpaces.map((space) => (
                  <button
                    key={space.id}
                    type="button"
                    className={`browser-row ${selection?.kind === "space" && selection.id === space.id ? "is-active" : ""}`}
                    onClick={() => {
                      setActiveView("plan");
                      setSelection({ kind: "space", id: space.id });
                    }}
                  >
                    <span className="browser-row-kind">Space</span>
                    <span className="browser-row-title">
                      <strong>{space.name}</strong>
                      <small>{getSpaceAreaSqFt(space)} sf</small>
                    </span>
                  </button>
                ))}
              </div>
            </section>
          </section>
        </aside>
      </div>

      <footer className="status-bar">
        <span>Units: Imperial ft-in</span>
        <span>Active Level: {activeLevel.name}</span>
        <span>View: {currentViewLabel}</span>
        <span>Tool: {getToolLabel(activeTool)}</span>
        <span>Selection: {selectionLabel}</span>
      </footer>
    </main>
  );
}
