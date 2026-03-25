# FB002 Vercel Preview Deployment NOT_FOUND

## Status

Implemented on `2026-03-25`.

Result:

- preview deployment no longer relies on implicit Vercel monorepo detection
- the repo now has an explicit Vercel build command and output directory
- SPA routes like `/`, `/login`, and `/editor` can be served through the frontend shell instead of failing at the platform edge

## Goal

Explain why the preview deployment returned Vercel `404: NOT_FOUND`, capture the direct root cause, and record the smallest stable fix so this class of deploy issue is faster to recognize next time.

This remained a deploy-fix task only. No editor redesign, no API changes, and no domain-model changes were introduced.

## Final Root Cause

The failure was a platform routing problem, not a React render crash:

- the deployed URL returned Vercel `404: NOT_FOUND`
- the web app uses `BrowserRouter` and client-side routes like `/login` and `/editor`
- the repo is a monorepo and the web build output lives under `apps/web/dist`
- the repo did not have an explicit `vercel.json` telling Vercel what to build, what directory to serve, or how to treat SPA routes

That means the immediate outage cause was:

- Vercel could not reliably resolve the correct deployment artifact and route behavior for this frontend from the repo layout alone

The routing gap was the real product issue:

- the project depended on implicit Vercel configuration in a monorepo
- the frontend used client-side routing but had no SPA rewrite fallback
- auth redirects and direct deep links expected `/editor` to load the SPA shell first
- Vercel resolves URLs before React runs, so missing platform config produced a 404 before the app could boot

## What Shipped

### 1. Explicit Vercel deploy config

`vercel.json`

- added `buildCommand: "corepack pnpm --filter web build"`
- added `outputDirectory: "apps/web/dist"`

Why:

- this removes ambiguity about which workspace Vercel should build
- this makes the served static output match the actual Vite production build location

### 2. SPA rewrite fallback for client routes

`vercel.json`

- added a rewrite from `/(.*)` to `/index.html`

Why:

- Vercel treats incoming URLs as platform-level paths first
- the web app needs `index.html` to load before React Router can resolve `/login`, `/editor`, or any future client-side path

## Prevention Flow

Use this order when a Vercel preview URL shows `404: NOT_FOUND`:

1. confirm whether the 404 is coming from Vercel itself or from the app
2. check the project root directory, build command, and output directory in Vercel
3. verify whether the app uses client-side routing and therefore needs an SPA rewrite
4. confirm that auth callback or deep-link paths are routable through `index.html`

Keep the diagnosis simple:

- platform 404 first
- build/output mapping second
- SPA rewrite third
- app code only after those are proven correct

## Verification Run

Commands run during implementation:

```bash
corepack pnpm install
corepack pnpm run verify:web
```

Observed results:

- the web package production build passed locally
- the emitted frontend output was present in `apps/web/dist`
- the repo now contains explicit Vercel routing and output config in `vercel.json`

## Done Criteria Check

1. the deploy note identifies why Vercel returned `NOT_FOUND`: done
2. the smallest repo-level Vercel fix is documented: done
3. the SPA routing requirement is documented for future deploys: done

## Kinh Nghiem

- Nếu preview URL hiện trang Vercel `404: NOT_FOUND`, hãy coi đó là lỗi cấu hình deploy hoặc routing ở platform trước, không phải lỗi React render.
- Trong monorepo, chỉ "build ra được ở đâu đó" là chưa đủ. Vercel phải biết chính xác workspace nào cần build và thư mục output nào cần serve.
- Khi app dùng `BrowserRouter`, server hoặc hosting platform phải trả các route app về `index.html`. Nếu thiếu fallback này, truy cập thẳng hoặc refresh tại `/editor` hay `/login` sẽ hỏng trước khi React kịp chạy.
- URL redirect của auth thực chất cũng là một bài test routing. Nếu Supabase trả browser về `/editor`, path đó phải hoạt động như một entry point ở mức platform, không chỉ là route điều hướng nội bộ sau khi app đã load.
- Đường chẩn đoán ngắn nhất cho nhóm lỗi này là: xác định 404 có phải do Vercel sinh ra không, rồi kiểm tra lần lượt root directory, build command, output directory, và SPA rewrites.
- Các default ngầm của hosting khá mong manh trong monorepo nhỏ. Chúng có thể chạy tạm cho đến khi cấu trúc thư mục hoặc project settings lệch đi. Cấu hình deploy explicit rẻ hơn nhiều so với việc triage lặp lại.
- Vite dev server ở local dễ che giấu nhóm lỗi này vì nó đã xử lý SPA fallback sẵn. Production hosting có ranh giới trách nhiệm khác và cần được cấu hình riêng.
