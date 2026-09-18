# Jobless

Jobless is a cross-platform desktop automation client built with GPUI Kit,
Rust, Bun and Patchright. The repository currently contains the runtime
architecture only; platform-specific job application workflows are added in
later phases.

## Architecture

```text
GPUI Kit desktop app (Rust)
        │
        ▼
Application services (RuntimeService)
        │
        ▼
Browser adapter + SQLite storage + configuration
        │ Content-Length JSON-RPC over stdin/stdout
        ▼
browser-worker (Bun + TypeScript)
        │ Playwright-compatible API
        ▼
Patchright + bundled Chrome for Testing
```

Important properties:

- Rust owns configuration, SQLite, application state and worker lifecycle.
- Application services own use-case orchestration and keep GPUI views thin.
- `browser-worker` is the only component that knows about Patchright.
- Bun and the Patchright browser are bundled into `runtime/`.
- Browser profiles are isolated per account/profile and remain outside SQLite.
- Sensitive values use the operating system credential store; secrets are
  never written to `config.toml`.
- CAPTCHA handling is human-in-the-loop through the visible browser window;
  this scaffold does not include a CAPTCHA solver.

## Prerequisites

- Rust 1.98
- Bun 1.4.2
- macOS 15+, Windows 10+, or Ubuntu 24.04+

## Development

Prepare the local browser runtime once:

```sh
bun run scripts/prepare-runtime.ts
```

If the Playwright CDN is slow, use the npmmirror browser mirror:

```sh
JOBLESS_BROWSER_DOWNLOAD_HOST=https://cdn.npmmirror.com/binaries/playwright \
JOBLESS_BUN_DOWNLOAD_BASE=https://cdn.npmmirror.com/binaries/bun/bun-v1.4.2 \
  bun run scripts/prepare-runtime.ts
```

Run the desktop application:

```sh
cargo run -p jobless-app
```

The generated configuration file is stored in the platform configuration
directory under `com.jobless.desktop/config.toml`. Database files and Chromium
profiles are stored in the platform data directory.

## Checks

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --locked
bun run scripts/check-mod-layout.ts

bun install --cwd browser-worker --frozen-lockfile
bun run --cwd browser-worker typecheck
bun run --cwd browser-worker typecheck:scripts
bun test browser-worker/src/test
```

## Packaging

```sh
bun run scripts/prepare-runtime.ts
cargo build --release --locked
bun run scripts/package.ts
```

GitHub Actions builds portable archives for Linux, macOS and Windows. Linux
release verification runs the real application and headed Chromium under Xvfb
with Mesa lavapipe. It does not require a physical display.
