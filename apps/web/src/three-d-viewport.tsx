import { useEffect, useRef, useState, type PointerEvent, type WheelEvent as ReactWheelEvent } from "react";
import type { ProjectDoc } from "./project-doc";
import type { Selection } from "./ui-store";
import {
  buildSpaceScenePayload,
  getDefaultOrbitCamera,
  getOrbitCameraFrame,
  getOrbitCameraViewProjectionMatrix,
  type OrbitCamera
} from "./space-scene";

type ThreeDViewportProps = {
  project: ProjectDoc;
  activeLevelId: string;
  activeLevelName: string;
  selection: Selection;
  selectionLabel: string;
};

type RendererHandle = {
  free?: () => void;
  resize: (width: number, height: number) => void;
  set_scene: (scene: unknown) => void;
  set_camera: (camera: unknown) => void;
  render: () => void;
};

type RenderWasmModule = {
  default: (input?: unknown) => Promise<unknown>;
  create_renderer: (canvas: HTMLCanvasElement) => Promise<RendererHandle>;
};

type ViewportPhase = "loading" | "ready" | "empty" | "unsupported" | "error";
type DragMode = "orbit" | "pan";

let renderWasmModulePromise: Promise<RenderWasmModule> | null = null;

function getErrorMessage(error: unknown): string {
  if (error instanceof Error) {
    return error.message;
  }

  return typeof error === "string" ? error : "Renderer initialization failed.";
}

async function loadRenderWasmModule(): Promise<RenderWasmModule> {
  if (!renderWasmModulePromise) {
    renderWasmModulePromise = import("../../../crates/render-wasm/pkg/render_wasm.js").then(async (module) => {
      const renderWasm = module as unknown as RenderWasmModule;
      await renderWasm.default();
      return renderWasm;
    });
  }

  return renderWasmModulePromise;
}

function clamp(value: number, min: number, max: number): number {
  return Math.min(max, Math.max(min, value));
}

export default function ThreeDViewport({
  project,
  activeLevelId,
  activeLevelName,
  selection,
  selectionLabel
}: ThreeDViewportProps) {
  const scene = buildSpaceScenePayload(project, { activeLevelId, selection });
  const [phase, setPhase] = useState<ViewportPhase>(scene.hasVisibleItems ? "loading" : "empty");
  const [errorMessage, setErrorMessage] = useState<string | null>(null);
  const [camera, setCamera] = useState<OrbitCamera>(() => getDefaultOrbitCamera(scene));
  const [viewportSize, setViewportSize] = useState({ width: 1, height: 1 });
  const surfaceRef = useRef<HTMLDivElement | null>(null);
  const canvasRef = useRef<HTMLCanvasElement | null>(null);
  const rendererRef = useRef<RendererHandle | null>(null);
  const dragRef = useRef<{ pointerId: number; mode: DragMode; lastX: number; lastY: number } | null>(null);
  const hasFitVisibleSceneRef = useRef(false);

  useEffect(() => {
    if (!scene.hasVisibleItems) {
      hasFitVisibleSceneRef.current = false;
      setCamera(getDefaultOrbitCamera(scene));
      setPhase("empty");
      setErrorMessage(null);
      return;
    }

    setPhase((current) => (current === "ready" ? current : "loading"));

    if (!hasFitVisibleSceneRef.current) {
      hasFitVisibleSceneRef.current = true;
      setCamera(getDefaultOrbitCamera(scene));
    }
  }, [
    scene.hasVisibleItems,
    scene.extents.minXFt,
    scene.extents.minYFt,
    scene.extents.minZFt,
    scene.extents.maxXFt,
    scene.extents.maxYFt,
    scene.extents.maxZFt
  ]);

  useEffect(() => {
    const surface = surfaceRef.current;
    const canvas = canvasRef.current;

    if (!surface || !canvas) {
      return;
    }

    const syncCanvasSize = () => {
      const nextWidth = Math.max(1, Math.round(surface.clientWidth * window.devicePixelRatio));
      const nextHeight = Math.max(1, Math.round(surface.clientHeight * window.devicePixelRatio));

      if (canvas.width !== nextWidth) {
        canvas.width = nextWidth;
      }

      if (canvas.height !== nextHeight) {
        canvas.height = nextHeight;
      }

      setViewportSize((current) => (
        current.width === nextWidth && current.height === nextHeight
          ? current
          : { width: nextWidth, height: nextHeight }
      ));

      rendererRef.current?.resize(nextWidth, nextHeight);
    };

    syncCanvasSize();

    const observer = new ResizeObserver(syncCanvasSize);
    observer.observe(surface);

    return () => observer.disconnect();
  }, []);

  useEffect(() => {
    let isCancelled = false;

    if (!scene.hasVisibleItems || rendererRef.current || !canvasRef.current) {
      return;
    }

    if (!("gpu" in navigator)) {
      setPhase("unsupported");
      setErrorMessage("This browser does not expose navigator.gpu.");
      return;
    }

    setPhase("loading");
    setErrorMessage(null);

    void (async () => {
      try {
        const canvas = canvasRef.current;

        if (!canvas) {
          return;
        }

        const renderWasm = await loadRenderWasmModule();

        if (isCancelled) {
          return;
        }

        const renderer = await renderWasm.create_renderer(canvas);

        if (isCancelled) {
          renderer.free?.();
          return;
        }

        rendererRef.current = renderer;
        renderer.resize(viewportSize.width, viewportSize.height);
        setPhase("ready");
      } catch (error) {
        if (isCancelled) {
          return;
        }

        setPhase("error");
        setErrorMessage(getErrorMessage(error));
      }
    })();

    return () => {
      isCancelled = true;
    };
  }, [scene.hasVisibleItems, viewportSize.height, viewportSize.width]);

  useEffect(() => {
    const renderer = rendererRef.current;

    if (!renderer || phase !== "ready" || !scene.hasVisibleItems) {
      return;
    }

    try {
      renderer.set_scene(scene);
      renderer.set_camera({
        viewProjection: getOrbitCameraViewProjectionMatrix(camera, viewportSize.width / viewportSize.height)
      });
      renderer.render();
    } catch (error) {
      setPhase("error");
      setErrorMessage(getErrorMessage(error));
    }
  }, [activeLevelId, camera, phase, project, selection, viewportSize.height, viewportSize.width]);

  useEffect(() => (
    () => {
      const renderer = rendererRef.current;

      if (renderer) {
        renderer.free?.();
        rendererRef.current = null;
      }
    }
  ), []);

  const handlePointerDown = (event: PointerEvent<HTMLCanvasElement>) => {
    if (event.button !== 0) {
      return;
    }

    dragRef.current = {
      pointerId: event.pointerId,
      mode: event.shiftKey ? "pan" : "orbit",
      lastX: event.clientX,
      lastY: event.clientY
    };

    event.currentTarget.setPointerCapture(event.pointerId);
  };

  const handlePointerMove = (event: PointerEvent<HTMLCanvasElement>) => {
    const drag = dragRef.current;

    if (!drag || drag.pointerId !== event.pointerId || phase !== "ready") {
      return;
    }

    const deltaX = event.clientX - drag.lastX;
    const deltaY = event.clientY - drag.lastY;

    dragRef.current = {
      ...drag,
      lastX: event.clientX,
      lastY: event.clientY
    };

    setCamera((current) => {
      if (drag.mode === "orbit") {
        return {
          ...current,
          yawDeg: current.yawDeg - deltaX * 0.35,
          pitchDeg: clamp(current.pitchDeg + deltaY * 0.25, 10, 80)
        };
      }

      const frame = getOrbitCameraFrame(current);
      const panScaleFt = Math.max(current.distanceFt * 0.0025, 0.02);

      return {
        ...current,
        targetXFt: current.targetXFt - frame.right[0] * deltaX * panScaleFt + frame.up[0] * deltaY * panScaleFt,
        targetYFt: current.targetYFt - frame.right[1] * deltaX * panScaleFt + frame.up[1] * deltaY * panScaleFt,
        targetZFt: current.targetZFt - frame.right[2] * deltaX * panScaleFt + frame.up[2] * deltaY * panScaleFt
      };
    });
  };

  const handlePointerUp = (event: PointerEvent<HTMLCanvasElement>) => {
    if (dragRef.current?.pointerId === event.pointerId) {
      dragRef.current = null;
    }

    if (event.currentTarget.hasPointerCapture(event.pointerId)) {
      event.currentTarget.releasePointerCapture(event.pointerId);
    }
  };

  const handleWheel = (event: ReactWheelEvent<HTMLCanvasElement>) => {
    if (phase !== "ready") {
      return;
    }

    event.preventDefault();

    setCamera((current) => ({
      ...current,
      distanceFt: clamp(current.distanceFt * Math.exp(event.deltaY * 0.0015), 6, 5000)
    }));
  };

  const stateTitle = phase === "loading"
    ? "Starting 3D renderer"
    : phase === "unsupported"
      ? "WebGPU unavailable"
      : phase === "error"
        ? "3D renderer error"
        : phase === "empty"
          ? "No spaces to render"
          : null;
  const stateBody = phase === "loading"
    ? "Initializing the wasm renderer and preparing the current project scene."
    : phase === "unsupported"
      ? errorMessage
      : phase === "error"
        ? errorMessage
        : phase === "empty"
          ? "Add spaces to the project to generate 3D massing boxes."
          : null;

  return (
    <div className="viewport viewport-3d">
      <div ref={surfaceRef} className="three-d-surface">
        <canvas
          ref={canvasRef}
          className="three-d-canvas"
          onPointerDown={handlePointerDown}
          onPointerMove={handlePointerMove}
          onPointerUp={handlePointerUp}
          onPointerCancel={handlePointerUp}
          onWheel={handleWheel}
        />

        <div className="viewport-overlay viewport-overlay-top">
          <div className="viewport-badge">wgpu + wasm-bindgen</div>
          <div className="viewport-copy">
            <p className="viewport-title">3D View</p>
            <p className="viewport-subtitle">Orbit drag, Shift+drag pan, wheel zoom.</p>
          </div>
          <div className="three-d-controls">
            <button
              type="button"
              className="level-manager-button"
              onClick={() => setCamera(getDefaultOrbitCamera(scene))}
              disabled={!scene.hasVisibleItems}
            >
              Fit
            </button>
          </div>
        </div>

        <div className="viewport-overlay viewport-overlay-left">
          <dl className="viewport-stats">
            <div>
              <dt>Camera</dt>
              <dd>Perspective</dd>
            </div>
            <div>
              <dt>Active level</dt>
              <dd>{activeLevelName}</dd>
            </div>
            <div>
              <dt>Selection</dt>
              <dd>{selectionLabel}</dd>
            </div>
            <div>
              <dt>Spaces</dt>
              <dd>{scene.items.length}</dd>
            </div>
            <div>
              <dt>Viewport</dt>
              <dd>{Math.round(viewportSize.width)} x {Math.round(viewportSize.height)}</dd>
            </div>
          </dl>
        </div>

        {stateTitle ? (
          <div className="three-d-state">
            <strong>{stateTitle}</strong>
            <span>{stateBody}</span>
          </div>
        ) : null}
      </div>
    </div>
  );
}
