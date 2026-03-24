import {
  createStarterProjectDoc,
  formatFeetAndInches,
  getLevelSpaces,
  getSpaceAreaSqFt,
  type Level,
  type Space
} from "./project-doc";
import { useUiStore, type Selection, type ToolMode, type ViewMode } from "./ui-store";

const project = createStarterProjectDoc();

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

function getViewSelection(view: ViewMode): Selection {
  return { kind: "view", id: view === "3d" ? "view-3d" : "view-plan" };
}

function getToolLabel(tool: ToolMode): string {
  return toolItems.find((item) => item.value === tool)?.label ?? "Select";
}

function getViewLabel(view: ViewMode, level: Level): string {
  return view === "3d" ? "3D View" : `${level.name} Floor Plan`;
}

function getSelectionLabel(selection: Selection, level: Level, space: Space | null, view: ViewMode): string {
  if (!selection) return "None";
  if (selection.kind === "space" && space) return space.name;
  if (selection.kind === "level") return level.name;
  return getViewLabel(view, level);
}

export default function App() {
  const activeView = useUiStore((state) => state.activeView);
  const activeTool = useUiStore((state) => state.activeTool);
  const selection = useUiStore((state) => state.selection);
  const setActiveTool = useUiStore((state) => state.setActiveTool);
  const setActiveView = useUiStore((state) => state.setActiveView);
  const setSelection = useUiStore((state) => state.setSelection);

  const defaultLevel = project.levels[0];
  const selectedSpace = selection?.kind === "space"
    ? project.spaces.find((space) => space.id === selection.id) ?? null
    : null;
  const selectedLevel = selection?.kind === "level"
    ? project.levels.find((level) => level.id === selection.id) ?? null
    : null;
  const activeLevel = selectedLevel
    ?? (selectedSpace ? project.levels.find((level) => level.id === selectedSpace.levelId) ?? defaultLevel : defaultLevel);
  const activeSpaces = getLevelSpaces(project, activeLevel.id);
  const grossArea = project.spaces.reduce((total, space) => total + getSpaceAreaSqFt(space), 0);
  const currentViewLabel = getViewLabel(activeView, activeLevel);
  const selectionLabel = getSelectionLabel(selection, activeLevel, selectedSpace, activeView);

  const sessionRows = [
    ["Tool", getToolLabel(activeTool)],
    ["View", currentViewLabel],
    ["Level", activeLevel.name],
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
    : selection?.kind === "level"
      ? [
          ["Type", "Level"],
          ["Name", activeLevel.name],
          ["Elevation", formatFeetAndInches(activeLevel.elevationFt)],
          ["Height", formatFeetAndInches(activeLevel.heightFt)],
          ["Spaces", String(activeSpaces.length)]
        ]
      : selection?.kind === "view"
        ? [
            ["Type", "View"],
            ["Name", currentViewLabel],
            ["Mode", activeView === "3d" ? "Perspective" : "Plan"],
            ["Selection", selectionLabel]
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
        </div>

        <div className="ribbon-summary">
          <span>{project.levels.length} level</span>
          <span>{project.spaces.length} spaces</span>
          <span>{grossArea.toFixed(0)} sf</span>
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
                  <p className="viewport-subtitle">Rust WebGPU renderer placeholder</p>
                </div>
                <dl className="viewport-stats">
                  <div>
                    <dt>Camera</dt>
                    <dd>Perspective</dd>
                  </div>
                  <div>
                    <dt>Level</dt>
                    <dd>{activeLevel.name}</dd>
                  </div>
                  <div>
                    <dt>Selection</dt>
                    <dd>{selectionLabel}</dd>
                  </div>
                </dl>
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
                    className={`browser-row ${selection?.kind === "level" && selection.id === level.id ? "is-active" : ""}`}
                    onClick={() => setSelection({ kind: "level", id: level.id })}
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
                {activeSpaces.map((space) => (
                  <button
                    key={space.id}
                    type="button"
                    className={`browser-row ${selection?.kind === "space" && selection.id === space.id ? "is-active" : ""}`}
                    onClick={() => setSelection({ kind: "space", id: space.id })}
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
        <span>Level: {activeLevel.name}</span>
        <span>View: {currentViewLabel}</span>
        <span>Tool: {getToolLabel(activeTool)}</span>
        <span>Selection: {selectionLabel}</span>
      </footer>
    </main>
  );
}
