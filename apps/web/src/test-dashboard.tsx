import { useEffect, useRef, useState, type PointerEvent as ReactPointerEvent, type RefObject } from "react";
import type { SampleCaseManifest } from "./test-cases";

type TestDashboardProps = {
  workspaceRef: RefObject<HTMLElement | null>;
  cases: SampleCaseManifest[];
  activeCaseId: string | null;
  onLoadCase: (sampleCase: SampleCaseManifest) => void;
  onClose: () => void;
};

type DashboardDragState = {
  pointerId: number;
  offsetX: number;
  offsetY: number;
};

type DashboardPosition = {
  x: number;
  y: number;
};

const INITIAL_DASHBOARD_POSITION: DashboardPosition = {
  x: 22,
  y: 220
};

function clamp(value: number, min: number, max: number): number {
  return Math.min(max, Math.max(min, value));
}

function clampDashboardPosition(
  position: DashboardPosition,
  panel: HTMLElement | null,
  workspace: HTMLElement | null
): DashboardPosition {
  if (!panel || !workspace) {
    return position;
  }

  const maxX = Math.max(0, workspace.clientWidth - panel.offsetWidth);
  const maxY = Math.max(0, workspace.clientHeight - panel.offsetHeight);

  return {
    x: clamp(position.x, 0, maxX),
    y: clamp(position.y, 0, maxY)
  };
}

function getActiveCaseNote(activeCase: SampleCaseManifest | null): string {
  if (!activeCase) {
    return "Choose one mixed case with levels, site polygon, setbacks, and spaces in one snapshot-compatible document.";
  }

  return `${activeCase.label}: ${activeCase.description}`;
}

export default function TestDashboard({
  workspaceRef,
  cases,
  activeCaseId,
  onLoadCase,
  onClose
}: TestDashboardProps) {
  const panelRef = useRef<HTMLDivElement | null>(null);
  const dragStateRef = useRef<DashboardDragState | null>(null);
  const [position, setPosition] = useState<DashboardPosition>(INITIAL_DASHBOARD_POSITION);
  const activeCase = activeCaseId
    ? cases.find((sampleCase) => sampleCase.id === activeCaseId) ?? null
    : null;

  useEffect(() => {
    setPosition((current) => clampDashboardPosition(current, panelRef.current, workspaceRef.current));
  }, [workspaceRef]);

  useEffect(() => {
    const handlePointerMove = (event: PointerEvent) => {
      const dragState = dragStateRef.current;
      const workspace = workspaceRef.current;
      const panel = panelRef.current;

      if (!dragState || !workspace || !panel) {
        return;
      }

      const workspaceRect = workspace.getBoundingClientRect();
      const nextPosition = clampDashboardPosition(
        {
          x: event.clientX - workspaceRect.left - dragState.offsetX,
          y: event.clientY - workspaceRect.top - dragState.offsetY
        },
        panel,
        workspace
      );

      setPosition(nextPosition);
    };

    const handlePointerUp = (event: PointerEvent) => {
      if (dragStateRef.current?.pointerId === event.pointerId) {
        dragStateRef.current = null;
      }
    };

    const handleWindowResize = () => {
      setPosition((current) => clampDashboardPosition(current, panelRef.current, workspaceRef.current));
    };

    window.addEventListener("pointermove", handlePointerMove);
    window.addEventListener("pointerup", handlePointerUp);
    window.addEventListener("resize", handleWindowResize);

    return () => {
      window.removeEventListener("pointermove", handlePointerMove);
      window.removeEventListener("pointerup", handlePointerUp);
      window.removeEventListener("resize", handleWindowResize);
    };
  }, [workspaceRef]);

  const handleHeaderPointerDown = (event: ReactPointerEvent<HTMLElement>) => {
    const panel = panelRef.current;
    const workspace = workspaceRef.current;

    if (!panel || !workspace) {
      return;
    }

    if (event.target instanceof HTMLElement && event.target.closest("button")) {
      return;
    }

    const panelRect = panel.getBoundingClientRect();
    dragStateRef.current = {
      pointerId: event.pointerId,
      offsetX: event.clientX - panelRect.left,
      offsetY: event.clientY - panelRect.top
    };
  };

  return (
    <section
      ref={panelRef}
      className="test-dashboard"
      role="dialog"
      aria-label="Test dashboard"
      style={{ left: position.x, top: position.y }}
    >
      <header className="test-dashboard-header" onPointerDown={handleHeaderPointerDown}>
        <div>
          <strong>Test Dashboard</strong>
          <span>Drag this window by the header. Mixed cases replace the local editor document.</span>
        </div>

        <button type="button" className="level-manager-button" onClick={onClose}>
          Close
        </button>
      </header>

      <section className="test-dashboard-section">
        <div className="test-dashboard-title-row">
          <h3>Mixed Cases</h3>
          <span>Levels + site polygon + setbacks + spaces</span>
        </div>

        <div className="test-dashboard-mixed-list">
          {cases.map((sampleCase) => (
            <button
              key={sampleCase.id}
              type="button"
              className={`test-dashboard-case-button ${activeCaseId === sampleCase.id ? "is-active" : ""}`}
              onClick={() => onLoadCase(sampleCase)}
            >
              <strong>{sampleCase.label}</strong>
              <span>{sampleCase.description}</span>
            </button>
          ))}
        </div>
      </section>

      <p className="test-dashboard-note">{getActiveCaseNote(activeCase)}</p>
    </section>
  );
}
