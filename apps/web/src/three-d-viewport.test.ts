import { describe, expect, it, vi } from "vitest";
import {
  getBrowserGpuRequestAdapterAttempts,
  isWindowsUserAgent,
  openChromeSystemSettingsTab,
  requestBrowserGpuAdapterWithFallback,
  sanitizeBrowserGpuRequestAdapterOptions
} from "./three-d-viewport";

describe("three-d viewport WebGPU adapter fallback", () => {
  const linuxUserAgent = "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 Chrome/136.0.0.0 Safari/537.36";
  const windowsUserAgent = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 Chrome/136.0.0.0 Safari/537.36";

  it("builds a fallback chain for strict high-performance startup", () => {
    expect(getBrowserGpuRequestAdapterAttempts({
      forceFallbackAdapter: false,
      powerPreference: "high-performance"
    })).toEqual([
      {
        label: "requested adapter settings",
        options: {
          forceFallbackAdapter: false,
          powerPreference: "high-performance"
        }
      },
      {
        label: "browser-default adapter request",
        options: {
          forceFallbackAdapter: false
        }
      },
      {
        label: "low-power adapter request",
        options: {
          forceFallbackAdapter: false,
          powerPreference: "low-power"
        }
      },
      {
        label: "fallback adapter request",
        options: {
          forceFallbackAdapter: true
        }
      }
    ]);
  });

  it("recovers when a softer adapter request succeeds", async () => {
    const defaultAdapter = {};
    const requestAdapter = vi
      .fn<(
        options?: {
          forceFallbackAdapter?: boolean;
          powerPreference?: "high-performance" | "low-power";
        }
      ) => Promise<object | null>>()
      .mockResolvedValueOnce(null)
      .mockResolvedValueOnce(defaultAdapter);

    await expect(requestBrowserGpuAdapterWithFallback(requestAdapter, {
      forceFallbackAdapter: false,
      powerPreference: "high-performance"
    }, linuxUserAgent)).resolves.toEqual({
      adapter: defaultAdapter,
      diagnostic: "Browser WebGPU adapter recovered with browser-default adapter request.",
      thrownError: null
    });
    expect(requestAdapter).toHaveBeenNthCalledWith(1, {
      forceFallbackAdapter: false,
      powerPreference: "high-performance"
    });
    expect(requestAdapter).toHaveBeenNthCalledWith(2, {
      forceFallbackAdapter: false
    });
  });

  it("keeps a diagnostic trail when every adapter attempt fails", async () => {
    const requestAdapter = vi.fn().mockResolvedValue(null);

    await expect(requestBrowserGpuAdapterWithFallback(requestAdapter, {
      forceFallbackAdapter: false,
      powerPreference: "high-performance"
    }, linuxUserAgent)).resolves.toEqual({
      adapter: null,
      diagnostic: "Browser WebGPU adapter attempts failed: requested adapter settings: no adapter; browser-default adapter request: no adapter; low-power adapter request: no adapter; fallback adapter request: no adapter",
      thrownError: null
    });
  });

  it("detects Windows user agents and strips powerPreference there", () => {
    expect(isWindowsUserAgent(windowsUserAgent)).toBe(true);
    expect(isWindowsUserAgent(linuxUserAgent)).toBe(false);
    expect(sanitizeBrowserGpuRequestAdapterOptions({
      forceFallbackAdapter: false,
      powerPreference: "high-performance"
    }, windowsUserAgent)).toEqual({
      forceFallbackAdapter: false
    });
  });

  it("dedupes runtime adapter attempts after Windows option sanitization", async () => {
    const hardwareAdapter = {};
    const requestAdapter = vi.fn().mockResolvedValueOnce(hardwareAdapter);

    await expect(requestBrowserGpuAdapterWithFallback(requestAdapter, {
      forceFallbackAdapter: false,
      powerPreference: "high-performance"
    }, windowsUserAgent)).resolves.toEqual({
      adapter: hardwareAdapter,
      diagnostic: null,
      thrownError: null
    });
    expect(requestAdapter).toHaveBeenCalledTimes(1);
    expect(requestAdapter).toHaveBeenCalledWith({
      forceFallbackAdapter: false
    });
  });

  it("opens Chrome system settings in a new tab as a best-effort action", () => {
    const openWindow = vi.fn().mockReturnValue({} as WindowProxy);

    expect(openChromeSystemSettingsTab(openWindow)).toBe(true);
    expect(openWindow).toHaveBeenCalledWith(
      "chrome://settings/system",
      "_blank",
      "noopener,noreferrer"
    );
  });

  it("returns false when the browser blocks opening Chrome system settings", () => {
    const openWindow = vi.fn().mockReturnValue(null);

    expect(openChromeSystemSettingsTab(openWindow)).toBe(false);
    expect(openWindow).toHaveBeenCalledWith(
      "chrome://settings/system",
      "_blank",
      "noopener,noreferrer"
    );
  });
});
