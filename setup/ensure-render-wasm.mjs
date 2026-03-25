import { spawnSync } from "node:child_process";
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const setupDir = path.dirname(fileURLToPath(import.meta.url));
const repoRoot = path.resolve(setupDir, "..");
const pkgDir = path.join(repoRoot, "crates", "render-wasm", "pkg");
const pkgGitignorePath = path.join(pkgDir, ".gitignore");
const requiredPkgFiles = [
  "package.json",
  "render_wasm.js",
  "render_wasm.d.ts",
  "render_wasm_bg.wasm",
  "render_wasm_bg.wasm.d.ts"
];
const pkgGitignoreContents = `# Keep the generated wasm package checked into the repo so web builds can
# consume it in environments where wasm-pack is unavailable.
`;

function hasPrebuiltPkg() {
  return requiredPkgFiles.every((fileName) => fs.existsSync(path.join(pkgDir, fileName)));
}

function hasWasmPack() {
  const result = spawnSync("wasm-pack", ["--version"], {
    cwd: repoRoot,
    stdio: "ignore",
    shell: process.platform === "win32"
  });

  return result.status === 0;
}

function writePkgGitignore() {
  fs.mkdirSync(pkgDir, { recursive: true });
  fs.writeFileSync(pkgGitignorePath, pkgGitignoreContents);
}

function runWasmPackBuild() {
  const result = spawnSync(
    "wasm-pack",
    ["build", "crates/render-wasm", "--target", "web", "--out-dir", "pkg"],
    {
      cwd: repoRoot,
      stdio: "inherit",
      shell: process.platform === "win32"
    }
  );

  if (result.status === 0) {
    writePkgGitignore();
  }

  process.exit(result.status ?? 1);
}

if (hasWasmPack()) {
  console.log("wasm-pack detected; rebuilding crates/render-wasm/pkg.");
  runWasmPackBuild();
}

if (hasPrebuiltPkg()) {
  writePkgGitignore();
  console.log("wasm-pack not found; reusing checked-in crates/render-wasm/pkg artifacts.");
  process.exit(0);
}

console.error("wasm-pack is not available and crates/render-wasm/pkg is missing required artifacts.");
console.error("Run `pnpm build:wasm` on a machine with wasm-pack, then commit crates/render-wasm/pkg.");
process.exit(1);
