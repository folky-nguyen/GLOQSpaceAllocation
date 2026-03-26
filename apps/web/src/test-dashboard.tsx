import type { RefObject } from "react";
import { useDraggablePanel } from "./draggable-panel";
import type { SampleCaseManifest } from "./test-cases";

type TestDashboardProps = {
  workspaceRef: RefObject<HTMLElement | null>;
  cases: SampleCaseManifest[];
  activeCaseId: string | null;
  onLoadCase: (sampleCase: SampleCaseManifest) => void;
  onClose: () => void;
};

const INITIAL_DASHBOARD_POSITION = {
  x: 22,
  y: 220
};

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
  const { panelRef, handleHeaderPointerDown, panelStyle } = useDraggablePanel<HTMLDivElement>(
    workspaceRef,
    INITIAL_DASHBOARD_POSITION
  );
  const activeCase = activeCaseId
    ? cases.find((sampleCase) => sampleCase.id === activeCaseId) ?? null
    : null;

  return (
    <section
      ref={panelRef}
      className="test-dashboard"
      role="dialog"
      aria-label="Test dashboard"
      style={panelStyle}
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
