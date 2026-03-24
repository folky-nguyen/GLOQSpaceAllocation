Act as a principal engineer shipping a lean Revit-like MVP from an empty repo.

Product scope:
- floor plan editor
- 3D view
- levels
- spaces
- US feet-inch UI
- WebGPU-based rendering

Mandatory stack:
- frontend shell: React + Vite + TypeScript
- 3D renderer: Rust compiled to WebAssembly with wasm-bindgen + wgpu
- backend API: Rust + axum + sqlx
- auth/db/realtime/storage: Supabase

Architecture rules:
- TypeScript owns the canonical ProjectDoc and all editing/geometry logic for the MVP.
- Rust wasm owns GPU rendering only.
- Rust API owns auth-aware persistence, versioning, and thin server endpoints.
- Do not create a second full BIM/domain schema in Rust; persist the editor document as versioned JSONB snapshots plus small relational metadata.
- One shared domain model must drive both 2D and 3D.
- Prefer plain objects + pure functions over class hierarchies.
- One function = one responsibility.
- Keep file count and abstraction count low.
- Do not add three.js, react-three-fiber, Babylon, Tailwind, MUI, Prisma, tRPC, Redux Toolkit, or a custom ECS/scene graph unless absolutely required.
- Do not ask follow-up questions; make the smallest sensible decision and document it.

Execution rules:
- Change only files needed for the current task.
- Keep code production-grade but minimal.
- Add tests only for pure logic or critical API behavior.
- After each task, run the smallest relevant build/test, fix obvious breakages, then print:
  1) changed files
  2) commands run
  3) remaining TODOs