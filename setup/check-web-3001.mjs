import { fetchHealth, targetUrl as defaultTargetUrl } from "./web-3001-runtime.mjs";

const targetUrl = process.argv[2] ?? defaultTargetUrl;
const timeoutMs = 5000;

try {
  const health = await Promise.race([
    fetchHealth(),
    new Promise((_, reject) => {
      setTimeout(() => reject(new Error("request timed out")), timeoutMs);
    })
  ]);

  if (!health.ok) {
    const suffix = health.status === null ? "no healthy response" : `HTTP ${health.status}`;
    console.error(`Smoke check failed: ${targetUrl} returned ${suffix}.`);
    console.error("The endpoint must serve the GLOQ web app, not just any process on port 3001.");
    process.exit(1);
  }

  console.log(`Smoke check passed: ${targetUrl} returned HTTP ${health.status}.`);
} catch (error) {
  const detail = error instanceof Error ? error.message : String(error);
  console.error(`Smoke check failed: could not reach ${targetUrl}.`);
  console.error("Start or restore the local preview with `pnpm up:web:3001`, then rerun `pnpm smoke:web:3001`.");
  console.error(detail);
  process.exit(1);
}
