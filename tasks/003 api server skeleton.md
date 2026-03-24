# 003 API Server Skeleton

## Review bản 003 cũ

Bản trước có 4 vấn đề cần sửa:

1. Có chỗ đoán quá sớm.
   - `API_HOST`, `API_PORT`, `CORS_ALLOW_ORIGIN` được nêu ra nhưng chưa gắn chặt với cấu trúc repo hiện tại.
2. Danh sách file thay đổi chưa khớp convention workspace Rust của repo.
   - Nếu thêm `tower-http` theo pattern hiện có, cần sửa cả root `Cargo.toml`, không chỉ `apps/api/Cargo.toml`.
3. `health` response còn kéo theo `snapshot_strategy` từ prototype hiện tại.
   - Đây không nằm trong yêu cầu task.
4. JSON error type chưa có điểm chạm cụ thể trong HTTP surface.
   - Nếu không có fallback/error path rõ ràng thì rất dễ thêm type nhưng không dùng và không test được.

Plan dưới đây bỏ phần suy đoán, chỉ giữ những quyết định có thể suy ra trực tiếp từ repo hiện tại hoặc từ yêu cầu task.

## Sự thật hiện tại trong repo

### Workspace Rust hiện có

- Root workspace ở `Cargo.toml`
- Members hiện tại:
  - `apps/api`
  - `crates/render-wasm`
- Dependency pattern hiện tại là khai báo shared deps ở root rồi dùng `*.workspace = true` ở crate con

### API hiện có

Files hiện tại trong `apps/api`:

- `apps/api/Cargo.toml`
- `apps/api/src/main.rs`

Behavior hiện tại của `apps/api/src/main.rs`:

- bind cứng `127.0.0.1:4000`
- chỉ có route `GET /health`
- `AppState` đang là:

```rust
struct AppState {
    pool: Option<PgPool>,
}
```

- `DATABASE_URL` đang optional
- nếu connect DB fail thì chỉ log warning và server vẫn boot
- đã có 1 test route cho `/health`

### Frontend dev origins hiện có

Từ `apps/web/vite.config.ts` và `apps/web/package.json`:

- default web dev port: `5173`
- alternate scripted port: `3001`
- Vite server host đang là `0.0.0.0`

Như vậy, local browser origins hiện có cơ sở rõ ràng là:

- `http://localhost:5173`
- `http://127.0.0.1:5173`
- `http://localhost:3001`
- `http://127.0.0.1:3001`

### DB/snapshot direction đã có trong repo

Từ `supabase/migrations/20260324170000_init.sql`:

- persistence hiện định hướng theo `projects` + `project_versions`
- `project_versions.snapshot` là `jsonb`

Điều này xác nhận task API skeleton chỉ cần dựng thin server/pool wiring, chưa cần domain model Rust riêng.

## Mục tiêu đã khóa cho task này

Trong `apps/api`, tạo server skeleton với đúng các mục sau:

- config loading từ env
- tracing/logging
- CORS
- JSON error type
- SQLx Postgres pool wiring từ `DATABASE_URL`
- `GET /api/health`
- `GET /api/version`

Ngoài phạm vi:

- auth
- Supabase JWT verification
- persistence endpoints
- migrations mới
- repository/service abstractions
- domain schema Rust song song với editor document

## Quyết định triển khai, bám repo hiện tại

### 1. App state

Giữ state tối thiểu:

```rust
#[derive(Clone)]
struct AppState {
    pool: PgPool,
}
```

Lý do:

- task yêu cầu pool wiring thật
- repo hiện chưa có nhu cầu giữ thêm config hoặc service container trong state
- bỏ `Option<PgPool>` để không còn boot ở trạng thái degraded

### 2. Config từ env

Config sẽ được gom vào `config.rs` với một `AppConfig` nhỏ.

Các giá trị cần load:

- `DATABASE_URL` là bắt buộc
- bind host/port hiện đang hardcode trong `main.rs`, nên task này sẽ chuyển chúng thành config env

Biến env mới sẽ được introduce trực tiếp trong task:

- `API_HOST`
- `API_PORT`
- `DATABASE_URL`

Giá trị default chỉ áp dụng cho bind address để giữ backward compatibility với behavior hiện tại:

- `API_HOST` default `127.0.0.1`
- `API_PORT` default `4000`

`DATABASE_URL` không có default.

Lý do chọn đúng 3 biến này:

- chúng map 1:1 với hardcoded values đang tồn tại
- không đụng naming của web app/Vite
- không kéo thêm env/config surface không được yêu cầu

Không thêm:

- `.env` loader
- crate config
- `CORS_ALLOW_ORIGIN`

Vì repo hiện chưa có pattern env nào khác để bám theo, và task không cần nhiều hơn mức này.

### 3. Startup flow

Startup flow cần đổi từ “best effort” sang “fail-fast”:

1. load `AppConfig`
2. init tracing subscriber
3. create `PgPool` từ `DATABASE_URL`
4. build router
5. bind listener
6. serve

Nếu thiếu `DATABASE_URL` hoặc connect Postgres fail:

- process exit với error
- không serve HTTP

Đây là thay đổi có chủ đích so với `apps/api/src/main.rs` hiện tại.

### 4. Tracing/logging

Giữ tracing theo dependency đang có:

- `tracing`
- `tracing-subscriber`

Thay đổi cụ thể:

- tách init tracing thành helper nhỏ trong `main.rs`
- giữ `RUST_LOG` là nguồn override
- giữ default filter string, nhưng bổ sung log HTTP thông qua `tower-http` `TraceLayer`

Mức thay đổi đủ cho skeleton:

- log startup
- log request/response HTTP

Không thêm observability stack khác.

### 5. CORS

CORS sẽ được triển khai bằng `tower-http`, vì repo chưa có middleware crate nào khác và đây là lớp tối thiểu phù hợp với axum.

Policy cho task này sẽ không đoán origin mới.
Chỉ allow các origins đã xác thực được từ repo:

- `http://localhost:5173`
- `http://127.0.0.1:5173`
- `http://localhost:3001`
- `http://127.0.0.1:3001`

Methods cho skeleton:

- `GET`
- `OPTIONS`

Lý do:

- hai endpoints của task đều là read-only
- chưa có write endpoints trong scope
- tránh mở rộng policy trước khi có route thật cần dùng

### 6. JSON error type

JSON error type sẽ được đưa vào `error.rs` và dùng ngay cho fallback path để bảo đảm nó là behavior thật, không phải dead code.

Shape cố định:

```json
{
  "error": {
    "code": "not_found",
    "message": "Route not found."
  }
}
```

Tối thiểu cần hỗ trợ:

- `not_found`
- `internal_error`

Điểm áp dụng trong task này:

- fallback cho `/api/*` không match route

Ghi chú quan trọng:

- startup config errors và DB connect errors không phải HTTP responses
- vì vậy chúng không cần dùng `ApiError`
- chúng nên trả error ở process boundary, không bọc thành JSON

### 7. Health endpoint

`GET /api/health`

Response sẽ được thu gọn còn:

```json
{
  "status": "ok"
}
```

Không giữ `snapshotStrategy` từ prototype cũ vì:

- không có trong yêu cầu task
- không phản ánh health của server skeleton

`health` cũng không cần query DB mỗi request.

Lý do:

- startup đã fail-fast trên DB connect
- tránh làm route test phụ thuộc database thật

### 8. Version endpoint

`GET /api/version`

Response:

```json
{
  "name": "gloq-api",
  "version": "0.1.0"
}
```

Nguồn dữ liệu:

- `env!("CARGO_PKG_NAME")`
- `env!("CARGO_PKG_VERSION")`

Điều này bám đúng package metadata hiện có trong `apps/api/Cargo.toml` và root `Cargo.toml`.

## File-by-file plan

### 1. Root Cargo workspace

Sửa `Cargo.toml`:

- thêm `tower-http` vào `[workspace.dependencies]`
- chỉ bật features cần dùng:
  - `cors`
  - `trace`

Lý do:

- đây là pattern dependency đang dùng trong repo
- tránh hardcode version riêng ở crate con

### 2. API crate manifest

Sửa `apps/api/Cargo.toml`:

- thêm `tower-http.workspace = true`

Không thêm crate khác trong task này.

### 3. Config module

Tạo `apps/api/src/config.rs`:

- `AppConfig`
- `AppConfig::from_env() -> Result<AppConfig, ConfigError>`
- `ConfigError` nhỏ cho:
  - missing `DATABASE_URL`
  - invalid `API_PORT`
  - invalid bind address nếu có

Output của module này nên là:

- `database_url: String`
- `bind_address: SocketAddr`

Không nhét CORS policy vào config module vì origin list cho task này đã suy ra trực tiếp từ repo và chưa cần thành runtime config.

### 4. Error module

Tạo `apps/api/src/error.rs`:

- struct response body cho error
- `ApiError`
- `impl IntoResponse for ApiError`
- constructor/helper nhỏ cho:
  - `ApiError::not_found(message)`
  - `ApiError::internal(message)`

Giữ module này nhỏ, không dùng `thiserror`.

### 5. Main module

Refactor `apps/api/src/main.rs`:

- thêm:
  - `mod config;`
  - `mod error;`
- đổi `AppState` từ `Option<PgPool>` sang `PgPool`
- tách helpers nhỏ:
  - `init_tracing()`
  - `connect_pool(database_url: &str) -> Result<PgPool, sqlx::Error>`
  - `app(state: AppState) -> Router`
  - `api_router() -> Router<AppState>` nếu cần để giữ `/api` tách biệt
- router shape:

```text
/
└── /api
    ├── /health
    ├── /version
    └── fallback -> JSON 404
```

- attach layers:
  - `CorsLayer`
  - `TraceLayer::new_for_http()`

Không tạo thêm:

- `lib.rs`
- `routes/`
- `handlers/`
- `services/`
- `repositories/`

## Test plan

### Test giữ lại và đổi

Test hiện có trong `apps/api/src/main.rs` sẽ cần đổi theo routes mới.

### Test mới cần có

1. `GET /api/health` trả `200` và body `{"status":"ok"}`
2. `GET /api/version` trả `200` và body chứa:
   - `name = "gloq-api"`
   - `version = env!("CARGO_PKG_VERSION")`
3. `GET /api/does-not-exist` trả `404` với JSON error shape ổn định
4. request có `Origin: http://localhost:5173` nhận được CORS header tương ứng

### Cách test mà không cần Postgres thật

Vì `AppState` sẽ giữ `PgPool` thật thay vì `Option<PgPool>`, test route không nên phụ thuộc DB chạy thật.

Approach trong test:

- dựng pool bằng lazy connection cho một Postgres URL hợp lệ về mặt cú pháp
- không gọi query trong `health`

Như vậy:

- route tests vẫn thuần HTTP
- startup path với connect thật vẫn được kiểm soát ở runtime code

### Lệnh verify sau khi code

Chạy tối thiểu:

```bash
cargo test -p gloq-api
```

Nếu có format drift:

```bash
cargo fmt --all
```

Manual smoke:

```bash
$env:DATABASE_URL="postgres://postgres:postgres@127.0.0.1:54322/postgres"
cargo run -p gloq-api
```

Rồi kiểm tra:

- `GET http://127.0.0.1:4000/api/health`
- `GET http://127.0.0.1:4000/api/version`

## Done criteria

Task được xem là hoàn tất khi:

1. API bind address không còn hardcode trực tiếp trong `main.rs`
2. `DATABASE_URL` là bắt buộc và startup fail-fast nếu thiếu/sai
3. request logging hoạt động qua `TraceLayer`
4. CORS hoạt động cho các local origins đang có trong repo
5. `GET /api/health` và `GET /api/version` chạy dưới `/api`
6. `/api/*` route không tồn tại trả JSON error ổn định
7. `cargo test -p gloq-api` pass
