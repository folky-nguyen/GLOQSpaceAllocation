import { spawn, spawnSync } from "node:child_process";
import { openSync, writeFileSync } from "node:fs";
import { setTimeout as delay } from "node:timers/promises";
import {
  fetchHealth,
  getListeningPid,
  logPath,
  pidPath,
  removePidFile,
  stopWeb3001Processes,
  targetUrl,
  waitForPortRelease
} from "./web-3001-runtime.mjs";

const startupTimeoutMs = 20000;
const pollIntervalMs = 500;

async function waitForHealthyServer() {
  const deadline = Date.now() + startupTimeoutMs;

  while (Date.now() < deadline) {
    const health = await fetchHealth();

    if (health.ok) {
      return true;
    }

    await delay(pollIntervalMs);
  }

  return false;
}

const initialHealth = await fetchHealth();

if (initialHealth.ok) {
  console.log(`Web already available at ${targetUrl}.`);
  process.exit(0);
}

const listeningPid = getListeningPid();

if (listeningPid) {
  stopWeb3001Processes();

  if (!(await waitForPortRelease())) {
    console.error("Port 3001 is still occupied after cleanup.");
    console.error(`Inspect the existing listener before retrying ${targetUrl}.`);
    process.exit(1);
  }
}

removePidFile();

const stdoutFd = openSync(logPath, "w");
const stderrFd = openSync(logPath, "a");
const shellCommand = "corepack pnpm run dev:3001";
const child = process.platform === "win32"
  ? spawn("cmd.exe", ["/c", shellCommand], {
      cwd: process.cwd(),
      detached: true,
      stdio: ["ignore", stdoutFd, stderrFd]
    })
  : spawn("sh", ["-lc", shellCommand], {
      cwd: process.cwd(),
      detached: true,
      stdio: ["ignore", stdoutFd, stderrFd]
    });

child.unref();

writeFileSync(
  pidPath,
  `${JSON.stringify({ pid: child.pid, targetUrl, logPath, startedAt: new Date().toISOString() })}\n`
);

if (await waitForHealthyServer()) {
  console.log(`Web is ready at ${targetUrl}.`);
  console.log(`Log file: ${logPath}`);
  process.exit(0);
}

stopWeb3001Processes();
await waitForPortRelease();

console.error(`Web failed to start on ${targetUrl} within ${startupTimeoutMs}ms.`);
console.error(`Inspect the startup log at ${logPath}.`);
process.exit(1);
