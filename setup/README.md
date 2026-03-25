# Setup

- `check-web-3001.mjs`: smoke-checks `http://127.0.0.1:3001/editor` and fails fast if the local web server is down or unhealthy.
- `up-web-3001.mjs`: starts the local web server on port `3001`, waits for `/editor` to respond, and writes a pid/log file for later shutdown.
- `down-web-3001.mjs`: stops the background web server started by `up-web-3001.mjs`.
- `web-3001-runtime.mjs`: shared runtime helpers for health checks, port-owner lookup, and start/stop cleanup on port `3001`.
