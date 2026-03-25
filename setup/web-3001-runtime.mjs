import { spawnSync } from "node:child_process";
import { existsSync, readFileSync, rmSync } from "node:fs";
import { join } from "node:path";
import { setTimeout as delay } from "node:timers/promises";

export const port = 3001;
export const targetUrl = `http://127.0.0.1:${port}/editor`;
export const expectedHtmlMarker = "<title>GLOQ Space Allocation</title>";
export const setupDir = join(process.cwd(), "setup");
export const pidPath = join(setupDir, "web-3001.pid");
export const logPath = join(setupDir, "web-3001.log");

export async function fetchHealth() {
  try {
    const response = await fetch(targetUrl, { redirect: "manual" });
    const body = await response.text();
    return {
      ok: response.ok && body.includes(expectedHtmlMarker),
      status: response.status,
      body
    };
  } catch (error) {
    return {
      ok: false,
      status: null,
      body: "",
      error: error instanceof Error ? error.message : String(error)
    };
  }
}

export function removePidFile() {
  if (existsSync(pidPath)) {
    rmSync(pidPath, { force: true });
  }
}

export function getTrackedPid() {
  if (!existsSync(pidPath)) {
    return null;
  }

  try {
    const tracked = JSON.parse(readFileSync(pidPath, "utf8"));
    return typeof tracked.pid === "number" ? tracked.pid : null;
  } catch {
    removePidFile();
    return null;
  }
}

function runCommand(command, args) {
  return spawnSync(command, args, {
    encoding: "utf8",
    stdio: ["ignore", "pipe", "ignore"]
  });
}

export function getListeningPid() {
  if (process.platform === "win32") {
    const script = `$ErrorActionPreference='SilentlyContinue'; (Get-NetTCPConnection -LocalPort ${port} -State Listen | Select-Object -First 1 -ExpandProperty OwningProcess)`;
    const result = runCommand("powershell.exe", ["-NoProfile", "-Command", script]);
    const trimmed = result.stdout.trim();
    return trimmed ? Number(trimmed) : null;
  }

  const result = runCommand("lsof", ["-ti", `tcp:${port}`, "-sTCP:LISTEN"]);
  const trimmed = result.stdout.trim();
  return trimmed ? Number(trimmed.split(/\s+/)[0]) : null;
}

export function killProcessTree(pid) {
  if (!pid) {
    return;
  }

  if (process.platform === "win32") {
    spawnSync("taskkill", ["/PID", String(pid), "/T", "/F"], { stdio: "ignore" });
    return;
  }

  try {
    process.kill(-pid, "SIGTERM");
  } catch {
    try {
      process.kill(pid, "SIGTERM");
    } catch {
      // Ignore stale pid failures.
    }
  }
}

export function stopWeb3001Processes() {
  const trackedPid = getTrackedPid();
  const listeningPid = getListeningPid();

  if (trackedPid) {
    killProcessTree(trackedPid);
  }

  if (listeningPid && listeningPid !== trackedPid) {
    killProcessTree(listeningPid);
  }

  removePidFile();
}

export async function waitForPortRelease(timeoutMs = 5000, pollIntervalMs = 500) {
  const deadline = Date.now() + timeoutMs;
  let emptyPollCount = 0;

  while (Date.now() < deadline) {
    if (!getListeningPid()) {
      emptyPollCount += 1;

      if (emptyPollCount >= 3) {
        return true;
      }
    } else {
      emptyPollCount = 0;
    }

    await delay(pollIntervalMs);
  }

  return !getListeningPid();
}
