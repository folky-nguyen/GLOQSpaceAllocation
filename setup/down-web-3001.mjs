import {
  getListeningPid,
  getTrackedPid,
  stopWeb3001Processes,
  waitForPortRelease
} from "./web-3001-runtime.mjs";

const trackedPid = getTrackedPid();
const listeningPid = getListeningPid();

if (!trackedPid && !listeningPid) {
  console.log("No web server is listening on port 3001.");
  process.exit(0);
}

stopWeb3001Processes();

if (!(await waitForPortRelease())) {
  console.error("Port 3001 is still occupied after shutdown.");
  process.exit(1);
}

console.log("Web server on port 3001 has been stopped.");
