import { useEffect, useRef, useState, type PointerEvent, type WheelEvent as ReactWheelEvent } from "react";
import type { ProjectDoc } from "./project-doc";
import { getSelectionElementKey, type Selection } from "./ui-store";
import {
  buildSpaceScenePayload,
  getDefaultOrbitCamera,
  getOrbitCameraFrame,
  getOrbitCameraViewProjectionMatrix,
  getVisibleSpacePrisms,
  pickVisibleSpaceAtCanvasPoint,
  type OrbitCamera,
  type ThreeDVisibilityMode
} from "./space-scene";
import { getTrappedErrorCode } from "./error-codes";

type ThreeDViewportProps = {
  project: ProjectDoc;
  activeLevelId: string;
  activeLevelName: string;
  selection: Selection;
  selectionLabel: string;
  visibilityMode: ThreeDVisibilityMode;
  onChangeVisibilityMode: (mode: ThreeDVisibilityMode) => void;
  onOpenFreshWindow: () => void;
  onPickSpace: (spaceId: string) => void;
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
type ViewportIssue = {
  detail: string | null;
  summary: string;
  typeLabel: string;
};

type StartupFailure = {
  issue: ViewportIssue;
  phase: Extract<ViewportPhase, "unsupported" | "error">;
};

type BrowserWebGpuProbeResult = {
  blockingFailure: StartupFailure | null;
  startupDiagnostic: string | null;
};

const visibilityModeOptions: Array<{ value: ThreeDVisibilityMode; label: string }> = [
  { value: "active-floor-only", label: "Active Floor Only" },
  { value: "all-levels", label: "All Levels" }
];

let renderWasmModulePromise: Promise<RenderWasmModule> | null = null;
const browserNoAdapterShortDetail = "No available adapter.";
const browserNoAdapterRecoveryHint = "Go to chrome://settings/system, turn on Use graphics acceleration when available, then relaunch Chrome.";
const chromeSystemSettingsUrl = "chrome://settings/system";

function getRawErrorMessage(error: unknown): string {
  return error instanceof Error
    ? error.message
    : typeof error === "string"
      ? error
      : "3D renderer initialization failed.";
}

function isPayloadMismatchMessage(message: string): boolean {
  return message.includes("Failed to parse JSON payload");
}

function getPayloadMismatchIssue(): ViewportIssue {
  return {
    detail: "Run `pnpm build:wasm` and restart the web app so the JS payload and wasm package match again.",
    summary: "The web app and the checked-in wasm renderer package are out of sync.",
    typeLabel: "Renderer package mismatch"
  };
}

function getMissingWebGpuIssue(): ViewportIssue {
  return {
    detail: "Use a WebGPU-enabled browser build on a machine with supported graphics drivers.",
    summary: "This browser does not expose `navigator.gpu`, so the wasm renderer cannot start.",
    typeLabel: "Browser missing WebGPU API"
  };
}

function mergeIssueDetail(...parts: Array<string | null>): string | null {
  const merged = Array.from(new Set(
    parts
      .map((part) => part?.trim())
      .filter((part): part is string => Boolean(part))
  ));

  return merged.length > 0 ? merged.join(" ") : null;
}

function getUnavailableAdapterIssue(): ViewportIssue {
  return {
    detail: mergeIssueDetail(browserNoAdapterShortDetail, browserNoAdapterRecoveryHint),
    summary: "The browser exposed WebGPU, but this device did not return a usable graphics adapter.",
    typeLabel: "WebGPU adapter unavailable"
  };
}

type BrowserGpuAdapter = object;
type BrowserGpuRequestAdapterOptions = {
  forceFallbackAdapter?: boolean;
  powerPreference?: "high-performance" | "low-power";
};
type BrowserGpu = {
  requestAdapter: (options?: BrowserGpuRequestAdapterOptions) => Promise<BrowserGpuAdapter | null>;
};

type BrowserGpuRequestAdapterAttempt = {
  label: string;
  options?: BrowserGpuRequestAdapterOptions;
};

type BrowserGpuRequestAdapterResult = {
  adapter: BrowserGpuAdapter | null;
  diagnostic: string | null;
  thrownError: unknown | null;
};

type BrowserWindowOpen = (
  url?: string | URL,
  target?: string,
  features?: string
) => WindowProxy | null;

function getBrowserGpu(): BrowserGpu | null {
  return (navigator as Navigator & { gpu?: BrowserGpu }).gpu ?? null;
}

function getBrowserUserAgent(): string {
  return typeof navigator === "undefined" ? "" : navigator.userAgent;
}

export function isWindowsUserAgent(userAgent: string): boolean {
  return /\bWindows\b/i.test(userAgent);
}

function cloneBrowserGpuRequestAdapterOptions(
  options?: BrowserGpuRequestAdapterOptions
): BrowserGpuRequestAdapterOptions | undefined {
  return options ? { ...options } : undefined;
}

function normalizeBrowserGpuRequestAdapterOptions(
  options?: BrowserGpuRequestAdapterOptions
): BrowserGpuRequestAdapterOptions | undefined {
  if (!options) {
    return undefined;
  }

  const normalized = { ...options };

  if (normalized.powerPreference === undefined) {
    delete normalized.powerPreference;
  }

  if (normalized.forceFallbackAdapter === undefined) {
    delete normalized.forceFallbackAdapter;
  }

  return Object.keys(normalized).length > 0 ? normalized : undefined;
}

export function sanitizeBrowserGpuRequestAdapterOptions(
  options?: BrowserGpuRequestAdapterOptions,
  userAgent = getBrowserUserAgent()
): BrowserGpuRequestAdapterOptions | undefined {
  const normalized = normalizeBrowserGpuRequestAdapterOptions(options);

  if (!normalized) {
    return undefined;
  }

  if (!isWindowsUserAgent(userAgent)) {
    return normalized;
  }

  const sanitized = { ...normalized };
  delete sanitized.powerPreference;
  return normalizeBrowserGpuRequestAdapterOptions(sanitized);
}

export function openChromeSystemSettingsTab(
  openWindow: BrowserWindowOpen = (url, target, features) => window.open(url, target, features)
): boolean {
  try {
    return openWindow(chromeSystemSettingsUrl, "_blank", "noopener,noreferrer") !== null;
  } catch {
    return false;
  }
}

function getBrowserGpuRequestAdapterAttemptKey(options?: BrowserGpuRequestAdapterOptions): string {
  const normalized = normalizeBrowserGpuRequestAdapterOptions(options);

  if (!normalized) {
    return "default";
  }

  return JSON.stringify(
    Object.entries(normalized).sort(([leftKey], [rightKey]) => leftKey.localeCompare(rightKey))
  );
}

export function getBrowserGpuRequestAdapterAttempts(
  options?: BrowserGpuRequestAdapterOptions
): BrowserGpuRequestAdapterAttempt[] {
  const requestedOptions = cloneBrowserGpuRequestAdapterOptions(options);
  const attempts: BrowserGpuRequestAdapterAttempt[] = [
    {
      label: "requested adapter settings",
      options: requestedOptions
    }
  ];

  if (requestedOptions?.powerPreference === "high-performance") {
    const browserDefaultOptions = normalizeBrowserGpuRequestAdapterOptions({
      ...requestedOptions,
      powerPreference: undefined
    });

    if (browserDefaultOptions || requestedOptions) {
      attempts.push({
        label: "browser-default adapter request",
        options: browserDefaultOptions
      });
    }

    attempts.push({
      label: "low-power adapter request",
      options: {
        ...requestedOptions,
        powerPreference: "low-power"
      }
    });
  }

  if (requestedOptions && requestedOptions.powerPreference !== "high-performance") {
    const browserDefaultOptions = normalizeBrowserGpuRequestAdapterOptions({
      ...requestedOptions,
      powerPreference: undefined
    });

    attempts.push({
      label: "browser-default adapter request",
      options: browserDefaultOptions
    });
  }

  if (requestedOptions?.forceFallbackAdapter !== true) {
    attempts.push({
      label: "fallback adapter request",
      options: normalizeBrowserGpuRequestAdapterOptions({
        ...requestedOptions,
        forceFallbackAdapter: true,
        powerPreference: undefined
      })
    });
  }

  const seenKeys = new Set<string>();

  return attempts.filter((attempt) => {
    const key = getBrowserGpuRequestAdapterAttemptKey(attempt.options);

    if (seenKeys.has(key)) {
      return false;
    }

    seenKeys.add(key);
    return true;
  });
}

export async function requestBrowserGpuAdapterWithFallback(
  requestAdapter: BrowserGpu["requestAdapter"],
  options?: BrowserGpuRequestAdapterOptions,
  userAgent = getBrowserUserAgent()
): Promise<BrowserGpuRequestAdapterResult> {
  const attempts = getBrowserGpuRequestAdapterAttempts(options);
  const failures: string[] = [];
  const runtimeAttemptKeys = new Set<string>();
  let firstThrownError: unknown | null = null;
  let sawNullAdapter = false;

  for (const attempt of attempts) {
    const requestOptions = sanitizeBrowserGpuRequestAdapterOptions(attempt.options, userAgent);
    const runtimeAttemptKey = getBrowserGpuRequestAdapterAttemptKey(requestOptions);

    if (runtimeAttemptKeys.has(runtimeAttemptKey)) {
      continue;
    }

    runtimeAttemptKeys.add(runtimeAttemptKey);

    try {
      const adapter = await requestAdapter(requestOptions);

      if (adapter) {
        return {
          adapter,
          diagnostic: attempt === attempts[0]
            ? null
            : `Browser WebGPU adapter recovered with ${attempt.label}.`,
          thrownError: null
        };
      }

      sawNullAdapter = true;
      failures.push(`${attempt.label}: no adapter`);
    } catch (error) {
      if (firstThrownError === null) {
        firstThrownError = error;
      }

      failures.push(`${attempt.label}: ${getRawErrorMessage(error)}`);
    }
  }

  return {
    adapter: null,
    diagnostic: failures.length > 0
      ? `Browser WebGPU adapter attempts failed: ${failures.join("; ")}`
      : null,
    thrownError: sawNullAdapter ? null : firstThrownError
  };
}

function installBrowserGpuRequestAdapterFallback(
  onDiagnostic: (detail: string) => void
): (() => void) | null {
  const gpu = getBrowserGpu();

  if (!gpu) {
    return null;
  }

  const originalRequestAdapter = gpu.requestAdapter.bind(gpu);
  const previousDescriptor = Object.getOwnPropertyDescriptor(gpu, "requestAdapter");

  const wrappedRequestAdapter: BrowserGpu["requestAdapter"] = async (options) => {
    const result = await requestBrowserGpuAdapterWithFallback(originalRequestAdapter, options);

    if (result.diagnostic) {
      onDiagnostic(result.diagnostic);
    }

    if (result.thrownError) {
      throw result.thrownError;
    }

    return result.adapter;
  };

  try {
    Object.defineProperty(gpu, "requestAdapter", {
      configurable: true,
      writable: true,
      value: wrappedRequestAdapter
    });
  } catch {
    return null;
  }

  return () => {
    if (previousDescriptor) {
      Object.defineProperty(gpu, "requestAdapter", previousDescriptor);
      return;
    }

    delete (gpu as { requestAdapter?: BrowserGpu["requestAdapter"] }).requestAdapter;
  };
}

async function probeBrowserWebGpu(): Promise<BrowserWebGpuProbeResult> {
  const gpu = getBrowserGpu();

  if (!gpu) {
    return {
      blockingFailure: {
        issue: getMissingWebGpuIssue(),
        phase: "unsupported"
      },
      startupDiagnostic: null
    };
  }

  return {
    blockingFailure: null,
    startupDiagnostic: null
  };
}

function getStartupFailureState(error: unknown, startupDiagnostic?: string | null): StartupFailure {
  const message = getRawErrorMessage(error);
  const startupDetail = startupDiagnostic ?? null;

  if (isPayloadMismatchMessage(message)) {
    return {
      issue: getPayloadMismatchIssue(),
      phase: "error"
    };
  }

  const normalizedMessage = message.toLowerCase();
  const isNoAdapterFailure = normalizedMessage.includes("no suitable graphics adapter")
    || normalizedMessage.includes("webgpu found no adapters")
    || normalizedMessage.includes("no adapters");

  if (isNoAdapterFailure) {
    return {
      issue: getUnavailableAdapterIssue(),
      phase: "unsupported"
    };
  }

  return {
    issue: {
      detail: mergeIssueDetail(message, startupDetail),
      summary: "The wasm renderer threw before the first frame could be drawn.",
      typeLabel: "3D renderer startup failed"
    },
    phase: "error"
  };
}

function getRenderFailureIssue(error: unknown): ViewportIssue {
  const message = getRawErrorMessage(error);

  if (isPayloadMismatchMessage(message)) {
    return getPayloadMismatchIssue();
  }

  return {
    detail: message,
    summary: "The renderer started, but failed while sending the current scene to WebGPU.",
    typeLabel: "3D renderer draw failed"
  };
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

function getSelectionRecoveryKey(selection: Selection): string {
  if (!selection) {
    return "none";
  }

  if (selection.kind === "element") {
    return `element:${getSelectionElementKey(selection.element)}`;
  }

  if (selection.kind === "element-set") {
    return `element-set:${selection.elements.map(getSelectionElementKey).join(",")}`;
  }

  if (selection.kind === "site-edge") {
    return `site-edge:${selection.edgeIndex}`;
  }

  return `${selection.kind}:${selection.id}`;
}

export default function ThreeDViewport({
  project,
  activeLevelId,
  activeLevelName,
  selection,
  selectionLabel,
  visibilityMode,
  onChangeVisibilityMode,
  onOpenFreshWindow,
  onPickSpace
}: ThreeDViewportProps) {
  const scene = buildSpaceScenePayload(project, { activeLevelId, selection, visibilityMode });
  const visiblePrisms = getVisibleSpacePrisms(project, { activeLevelId, selection, visibilityMode });
  const [phase, setPhase] = useState<ViewportPhase>(scene.hasVisibleItems ? "loading" : "empty");
  const [issue, setIssue] = useState<ViewportIssue | null>(null);
  const [camera, setCamera] = useState<OrbitCamera>(() => getDefaultOrbitCamera(scene));
  const [viewportSize, setViewportSize] = useState({ width: 1, height: 1 });
  const surfaceRef = useRef<HTMLDivElement | null>(null);
  const canvasRef = useRef<HTMLCanvasElement | null>(null);
  const rendererRef = useRef<RendererHandle | null>(null);
  const dragRef = useRef<{
    pointerId: number;
    mode: DragMode;
    startX: number;
    startY: number;
    lastX: number;
    lastY: number;
    moved: boolean;
  } | null>(null);
  const hasFitVisibleSceneRef = useRef(false);
  const previousVisibilityModeRef = useRef<ThreeDVisibilityMode>(visibilityMode);
  const errorRecoveryKeyRef = useRef<string | null>(null);
  const hasSpacesOutsideActiveLevel = visibilityMode === "active-floor-only"
    && project.spaces.some((space) => space.levelId !== activeLevelId);
  const recoveryKey = [
    project.id,
    activeLevelId,
    visibilityMode,
    getSelectionRecoveryKey(selection),
    scene.hasVisibleItems ? "visible" : "empty",
    scene.items.map((item) => `${item.id}:${item.emphasis}`).join(";"),
    scene.extents.minXFt,
    scene.extents.minYFt,
    scene.extents.minZFt,
    scene.extents.maxXFt,
    scene.extents.maxYFt,
    scene.extents.maxZFt
  ].join("|");

  useEffect(() => {
    if (!scene.hasVisibleItems) {
      hasFitVisibleSceneRef.current = false;
      setCamera(getDefaultOrbitCamera(scene));
      if (phase !== "error" && phase !== "unsupported") {
        setIssue(null);
      }
      setPhase((current) => (current === "error" || current === "unsupported" ? current : "empty"));
      return;
    }

    setPhase((current) => {
      if (current === "error" || current === "unsupported") {
        return current;
      }

      return rendererRef.current ? "ready" : "loading";
    });

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
    if (previousVisibilityModeRef.current === visibilityMode) {
      return;
    }

    previousVisibilityModeRef.current = visibilityMode;

    if (!scene.hasVisibleItems) {
      return;
    }

    hasFitVisibleSceneRef.current = true;
    setCamera(getDefaultOrbitCamera(scene));
  }, [
    visibilityMode,
    scene.hasVisibleItems,
    scene.extents.minXFt,
    scene.extents.minYFt,
    scene.extents.minZFt,
    scene.extents.maxXFt,
    scene.extents.maxYFt,
    scene.extents.maxZFt
  ]);

  useEffect(() => {
    if (phase !== "error") {
      errorRecoveryKeyRef.current = null;
      return;
    }

    if (errorRecoveryKeyRef.current === null) {
      errorRecoveryKeyRef.current = recoveryKey;
      return;
    }

    if (errorRecoveryKeyRef.current === recoveryKey) {
      return;
    }

    errorRecoveryKeyRef.current = recoveryKey;
    rendererRef.current?.free?.();
    rendererRef.current = null;
    dragRef.current = null;
    setIssue(null);
    setPhase(scene.hasVisibleItems ? "loading" : "empty");
  }, [phase, recoveryKey, scene.hasVisibleItems]);

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
    let startupDiagnostic: string | null = null;
    let startupTimerId: number | null = null;

    if (!scene.hasVisibleItems || rendererRef.current || !canvasRef.current || phase === "unsupported") {
      return;
    }

    setPhase("loading");
    setIssue(null);

    startupTimerId = window.setTimeout(() => {
      void (async () => {
        let restoreBrowserGpuRequestAdapter: (() => void) | null = null;

        try {
          const canvas = canvasRef.current;

          if (!canvas) {
            return;
          }

          const browserProbe = await probeBrowserWebGpu();

          if (isCancelled) {
            return;
          }

          startupDiagnostic = browserProbe.startupDiagnostic;

          if (browserProbe.blockingFailure) {
            setPhase(browserProbe.blockingFailure.phase);
            setIssue(browserProbe.blockingFailure.issue);
            return;
          }

          const renderWasm = await loadRenderWasmModule();

          if (isCancelled) {
            return;
          }

          restoreBrowserGpuRequestAdapter = installBrowserGpuRequestAdapterFallback((detail) => {
            startupDiagnostic = mergeIssueDetail(startupDiagnostic, detail);
          });

          if (!restoreBrowserGpuRequestAdapter) {
            startupDiagnostic = mergeIssueDetail(
              startupDiagnostic,
              "Could not install the browser WebGPU adapter fallback wrapper before renderer startup."
            );
          }

          const renderer = await renderWasm.create_renderer(canvas);

          restoreBrowserGpuRequestAdapter?.();
          restoreBrowserGpuRequestAdapter = null;

          if (isCancelled) {
            renderer.free?.();
            return;
          }

          rendererRef.current = renderer;
          renderer.resize(viewportSize.width, viewportSize.height);
          setIssue(null);
          setPhase("ready");
        } catch (error) {
          if (isCancelled) {
            return;
          }

          const failureState = getStartupFailureState(error, startupDiagnostic);
          setPhase(failureState.phase);
          setIssue(failureState.issue);
        } finally {
          restoreBrowserGpuRequestAdapter?.();
        }
      })();
    }, 0);

    return () => {
      isCancelled = true;

      if (startupTimerId !== null) {
        window.clearTimeout(startupTimerId);
      }
    };
  }, [phase, scene.hasVisibleItems, viewportSize.height, viewportSize.width]);

  useEffect(() => {
    const renderer = rendererRef.current;

    if (!renderer || phase === "loading" || phase === "unsupported" || phase === "error") {
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
      setIssue(getRenderFailureIssue(error));
    }
  }, [activeLevelId, camera, phase, project, selection, visibilityMode, viewportSize.height, viewportSize.width]);

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
      startX: event.clientX,
      startY: event.clientY,
      lastX: event.clientX,
      lastY: event.clientY,
      moved: false
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
    const moved = drag.moved || Math.hypot(event.clientX - drag.startX, event.clientY - drag.startY) > 4;

    dragRef.current = {
      ...drag,
      lastX: event.clientX,
      lastY: event.clientY,
      moved
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
    const drag = dragRef.current;

    if (drag?.pointerId === event.pointerId) {
      dragRef.current = null;
    }

    if (event.currentTarget.hasPointerCapture(event.pointerId)) {
      event.currentTarget.releasePointerCapture(event.pointerId);
    }

    if (!drag || drag.pointerId !== event.pointerId || drag.mode !== "orbit" || drag.moved || phase !== "ready") {
      return;
    }

    const rect = event.currentTarget.getBoundingClientRect();

    if (rect.width <= 0 || rect.height <= 0) {
      return;
    }

    const canvasX = (event.clientX - rect.left) * (event.currentTarget.width / rect.width);
    const canvasY = (event.clientY - rect.top) * (event.currentTarget.height / rect.height);

    if (canvasX < 0 || canvasX > event.currentTarget.width || canvasY < 0 || canvasY > event.currentTarget.height) {
      return;
    }

    const pickedPrism = pickVisibleSpaceAtCanvasPoint({
      prisms: visiblePrisms,
      camera,
      canvasX,
      canvasY,
      viewportWidth: viewportSize.width,
      viewportHeight: viewportSize.height
    });

    if (pickedPrism) {
      onPickSpace(pickedPrism.id);
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

  const handleRetryStartup = () => {
    rendererRef.current?.free?.();
    rendererRef.current = null;
    dragRef.current = null;
    setIssue(null);
    setPhase(scene.hasVisibleItems ? "loading" : "empty");
  };

  const handleOpenChromeSystemSettings = () => {
    openChromeSystemSettingsTab();
  };

  const stateTitle = phase === "loading"
    ? "Starting 3D renderer"
    : phase === "unsupported"
      ? "WebGPU unavailable"
      : phase === "error"
        ? "3D renderer error"
        : phase === "empty"
          ? (hasSpacesOutsideActiveLevel ? "No spaces on the active floor" : "No spaces to render")
          : null;
  const stateBody = phase === "loading"
    ? "Initializing the wasm renderer and preparing the current project scene."
    : phase === "unsupported"
      ? issue?.summary ?? null
      : phase === "error"
        ? issue?.summary ?? null
        : phase === "empty"
          ? (
            hasSpacesOutsideActiveLevel
              ? "The active floor has no spaces in the current 3D scope. Switch to All Levels to inspect the rest of the model."
              : "Add spaces to the project to generate 3D polygon extrusions."
          )
          : null;
  const stateIssueType = phase === "unsupported" || phase === "error" ? issue?.typeLabel ?? null : null;
  const stateIssueCode = phase === "unsupported" || phase === "error"
    ? getTrappedErrorCode(issue?.summary)
    : null;
  const stateIssueDetail = phase === "unsupported" || phase === "error" ? issue?.detail ?? null : null;
  const showRetryStartupAction = phase === "unsupported" && stateIssueCode === "WEB-3D-002";
  const showOpenFreshWindowAction = phase === "unsupported" && stateIssueCode === "WEB-3D-002";

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
            <div className="three-d-scope" role="group" aria-label="3D visibility scope">
              <span className="three-d-scope-label">Scope</span>
              <div className="three-d-scope-buttons">
                {visibilityModeOptions.map((option) => (
                  <button
                    key={option.value}
                    type="button"
                    className={`three-d-scope-button ${visibilityMode === option.value ? "is-active" : ""}`}
                    aria-pressed={visibilityMode === option.value}
                    onClick={() => onChangeVisibilityMode(option.value)}
                  >
                    {option.label}
                  </button>
                ))}
              </div>
            </div>
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
              <dt>Scope</dt>
              <dd>{visibilityMode === "all-levels" ? "All Levels" : "Active Floor Only"}</dd>
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
            {stateBody ? <span>{stateBody}</span> : null}
            {stateIssueCode ? <span className="three-d-state-type">Code: {stateIssueCode}</span> : null}
            {stateIssueType ? <span className="three-d-state-type">Type: {stateIssueType}</span> : null}
            {stateIssueDetail ? <span className="three-d-state-detail">{stateIssueDetail}</span> : null}
            {showRetryStartupAction || showOpenFreshWindowAction ? (
              <div className="three-d-state-actions">
                {showRetryStartupAction ? (
                  <button
                    type="button"
                    className="level-manager-button three-d-state-action"
                    onClick={handleRetryStartup}
                  >
                    Retry 3D Startup
                  </button>
                ) : null}
                {showRetryStartupAction ? (
                  <button
                    type="button"
                    className="level-manager-button three-d-state-action"
                    onClick={handleOpenChromeSystemSettings}
                  >
                    Open Chrome System Settings
                  </button>
                ) : null}
                {showOpenFreshWindowAction ? (
                  <button
                    type="button"
                    className="level-manager-button three-d-state-action"
                    onClick={onOpenFreshWindow}
                  >
                    Open 3D In New Window
                  </button>
                ) : null}
              </div>
            ) : null}
            {phase === "empty" && hasSpacesOutsideActiveLevel ? (
              <button
                type="button"
                className="level-manager-button three-d-state-action"
                onClick={() => onChangeVisibilityMode("all-levels")}
              >
                Show All Levels
              </button>
            ) : null}
          </div>
        ) : null}
      </div>
    </div>
  );
}
