# Jobless Engineering Architecture Rules

This file is normative for every human or automated agent working anywhere in
this repository. Read it before changing code, configuration, packaging, or
protocol definitions.

`AGENTS.md` is part of the architecture contract. If a change alters a layer,
crate boundary, process boundary, dependency direction, or public interface,
update this file in the same change.

## Instruction Precedence

1. Explicit user instructions.
2. This `AGENTS.md`.
3. Current crate manifests, source code, tests, and generated contracts.
4. Existing documentation and comments.

If these conflict, stop and report the conflict instead of silently choosing
one. Do not weaken a boundary described here to make an implementation easier.

The repository currently contains infrastructure only. Do not add
platform-specific job application behaviour unless the user explicitly asks
for it.

## Architecture at a Glance

The system has four Rust dependency layers, one out-of-process runtime, and one
orthogonal delivery area. Dependencies point downward. A lower layer must never
import or know about an upper layer.

```text
Layer 3  Presentation and feature layer
         crates/app (composition shell + GPUI views)
         future account/task/history/platform feature crates
                         │
                         ▼
Layer 2  Application services
         crates/application (runtime orchestration)
                         │
                         ▼
Layer 1  Rust infrastructure adapters
         crates/browser (worker process + RPC + browser-session API)
         crates/storage (SQLite connection and persistence boundary)
                         │
                         ▼
Layer 0  Foundation
         crates/config (configuration, paths, secrets contract)

Out-of-process runtime sidecar
         browser-worker (Bun + TypeScript)
         Patchright connector + bundled Bun/Chrome for Testing
         connected to crates/browser by Content-Length JSON-RPC

Orthogonal delivery and tooling
         scripts/, packaging/, .github/, runtime/, locales/, .bun-version
```

The Rust layers are compile-time dependencies. The transition between
`crates/browser` and `browser-worker` is a process boundary and must use the
versioned RPC contract, not shared memory, direct JavaScript execution, or
implicit filesystem coupling.

## Layer Definitions

### Layer 0: Foundation

`crates/config` is the only Layer 0 crate.

It owns:

- `AppConfig` and all typed configuration sections.
- Configuration precedence, validation, TOML loading, and atomic saving.
- Platform config/data/runtime path resolution through `AppPaths`.
- The `SecretStore` contract and platform keyring implementation.
- Secret-key validation and secret-value zeroization boundaries.

It must not know about:

- GPUI or any desktop view.
- SQLite schema or queries.
- Browser processes, Patchright, Playwright, or RPC methods.
- Accounts, tasks, history, or employer-platform behaviour.

No other Jobless crate may define a second configuration model or duplicate
platform path rules.

### Layer 1: Rust Infrastructure Adapters

`crates/storage` and `crates/browser` are Layer 1 crates.

`crates/storage` owns:

- SQLite connection lifecycle.
- WAL, foreign-key, and busy-timeout configuration.
- Schema ownership and, when business tables are introduced, migrations and
  repository implementations.

`crates/browser` owns:

- The Rust side of the RPC protocol.
- Content-Length framing and request correlation.
- Worker process startup, readiness, health, shutdown, and forced cleanup.
- Runtime path resolution for Bun, the worker entry point, and the bundled
  browser.
- The typed `BrowserManager` API exposed to upper layers.

Layer 1 must not know about:

- GPUI views, pages, widgets, or user interactions.
- Account/task workflow rules.
- Employer-specific selectors, URLs, or page state machines.

Layer 1 crates may depend on Layer 0. They must not depend on `crates/app` or
on each other.

### Layer 2: Application Services

`crates/application` owns use-case orchestration that is independent of GPUI.

It owns:

- Typed `RuntimeService` operations for the desktop shell.
- Composition of lower-level browser and persistence interfaces.
- State transitions that are not visual concerns.
- A stable seam for future feature crates and tests.

It must not know about GPUI pages, widgets, or concrete window layout.

### Layer 3: Presentation and Features

`crates/app` is the application shell and composition root.

It owns:

- Process bootstrap and shutdown.
- GPUI application/window initialization.
- Composition of configuration, storage, application services, and future
  feature crates.
- Global navigation and shell-level state.
- The hidden `--smoke-test` integration path.

Feature crates added later, such as `accounts`, `tasks`, `history`, or a
platform integration, own one coherent capability each. Following GPUI Kit's
architecture guidance, a feature crate contains its own model, commands,
services, views, dialogs, and workflow boundary. Do not scatter a feature
across global `models/`, `views/`, `commands/`, or `modals/` directories.

Layer 3 must not:

- Import Patchright or Playwright.
- Execute raw SQL outside `jobless-storage`.
- Reach into another feature crate's private modules.
- Depend on the worker's filesystem layout except through `BrowserManager`.

`crates/app` may compose feature crates. Feature crates must not depend on
`crates/app` or on sibling feature internals.

### Out-of-Process Automation Runtime

`browser-worker` is an out-of-process TypeScript runtime.

It owns:

- JSON-RPC request handlers and notifications.
- Generic browser sessions and profile-directory validation.
- Browser health information.
- The `BrowserEngine` abstraction.
- The Patchright adapter, including Playwright-compatible launch options and
  Chrome for Testing/Chromium version discovery.

It must not own:

- Application configuration loading.
- SQLite access.
- Account selection or job-application workflow rules.
- Employer-platform selectors or page flows.
- CAPTCHA solving or detection bypass beyond the selected browser engine's
  standard launch behaviour.

Only `browser-worker/src/patchright-engine.ts` may import `patchright`.
`BrowserEngine` exists so the engine can be replaced without changing Rust.

### Orthogonal Delivery Area

The following paths are build, staging, release, and presentation assets:

- `runtime/` contains generated runtime binaries and must not contain
  handwritten application logic.
- `scripts/` prepares and packages runtimes.
- `packaging/` contains platform-specific operator instructions.
- `.github/` contains CI and release automation.
- `locales/` contains user-facing UI strings only.

Delivery code may orchestrate builds across layers. Runtime code must not
import delivery code.

## Dependency Matrix

| From | May depend on | Must not depend on |
| --- | --- | --- |
| `jobless-config` | Third-party foundation libraries | `storage`, `browser`, `app`, GPUI, worker internals |
| `jobless-storage` | `jobless-config`, SQLite libraries | `browser`, `app`, GPUI, worker internals |
| `jobless-browser` | `jobless-config`, RPC/serialization libraries | `storage`, `app`, GPUI, TypeScript modules |
| `jobless-application` | `jobless-config`, `jobless-browser`, `jobless-storage` when needed | GPUI, `jobless-app`, worker internals |
| `jobless-app` | `jobless-config`, `jobless-storage`, `jobless-application`, future feature crates, GPUI Kit | Patchright directly, raw SQL, sibling feature internals |
| Future feature crate | Stable Layer 0/1/2 APIs, GPUI Kit | `jobless-app`, another feature's private modules |
| `browser-worker` | Generated protocol, `vscode-jsonrpc`, Bun/Node built-ins, `BrowserEngine` | Rust crates, application business rules, SQLite |
| Delivery scripts | Build outputs and public commands | Runtime imports from application modules |

Dependency direction must remain acyclic. Run `cargo metadata` or inspect the
relevant `Cargo.toml` files before adding a dependency.

## Required Module Boundaries

### Public APIs

- Every crate must expose a narrow public surface from `lib.rs` or its package
  entry point.
- Other crates must import public APIs, not private modules.
- Do not use `pub(crate)` items as cross-crate contracts.
- Prefer typed request/response structs and enums over `serde_json::Value`.
- Public seam types should keep fields private when future fields are likely;
  use builders or constructors and readers.
- Prefer explicit error enums at infrastructure boundaries. Do not cross
  threads, processes, or crates with panics as error handling.

### File Size and Cohesion

- A handwritten Rust or TypeScript source file must stay at or below 500
  lines. The limit applies to tests, scripts, and application code.
- When a file exceeds 500 lines, split it by cohesive responsibility, not by
  line count. Do not create artificial `part1.rs`, `part2.rs`, or equivalent
  files.
- Prefer extracting a real module with a narrow interface: types/state,
  commands, rendering, persistence, transport, or one feature capability.
- Move a large inline test module to a sibling test module or integration
  test file when it materially reduces the production file.
- Generated files, lock files, locale data, and vendored assets are excluded
  from the line limit.
- Any exception must have a concrete cohesion or generated-code justification
  in review. "It is easier not to split" is not a valid exception.

### UI Decomposition

- Graphical code is organized by page first and reusable component second.
- `app.rs` owns bootstrap, root composition, shared application state, and
  command handlers. It must not accumulate page-specific rendering.
- One user-visible page maps to one file under `app/pages/<page>.rs`, such as
  `browser.rs`, `settings.rs`, `accounts.rs`, or `tasks.rs`.
- The parent `app/pages.rs` contains module declarations and shared page
  metadata only. It must not become a second dumping ground for page views.
- Reusable visual elements live under `app/components/<component>.rs` and are
  presentational. Components may accept state and callbacks but must not load
  data, execute SQL, or control the worker directly.
- Extract a component only when it has a clear visual responsibility or more
  than one real consumer. Do not create generic `common` or `widgets`
  directories for one-off markup.
- A page module may call application/feature services. It must not import
  Patchright, raw SQL, or another page's private implementation.
- If a page exceeds 500 lines, split its own subviews, state, and actions into
  child modules before extracting unrelated shared components.

### Logic Decomposition

- Non-UI logic is organized by functional module and capability, not by global
  technical folders such as `models/`, `services/`, `handlers/`, or `utils/`.
- One capability owns one module or crate boundary. Its model, commands,
  services, ports, persistence adapters, and tests remain together behind that
  boundary.
- Features must communicate through explicit public APIs, commands, events, or
  shared ports. They must not reach into sibling internals.
- Infrastructure is split by adapter responsibility: configuration, storage,
  browser RPC, transport, or process lifecycle. One adapter file must not mix
  unrelated concerns.
- TypeScript worker logic is split by runtime responsibility: entry point,
  connection/RPC, worker service, engine contract, and concrete browser-engine
  adapter.
- Extract shared logic only after at least two real consumers exist. Avoid
  speculative generic modules.

### Rust Module Layout

- `mod.rs` is forbidden everywhere in the Rust workspace, including current
  crates and all future feature or infrastructure crates.
- A leaf module must use `foo.rs`.
- A module with children must use a sibling `foo.rs` plus a `foo/` directory.
  Declare the child modules from `foo.rs`; do not create `foo/mod.rs`.
- Crate entry points remain `lib.rs` or `main.rs`. A directory must never be
  used as a substitute for an explicit parent module file.
- Do not mix `foo.rs` with `foo/mod.rs`, even temporarily.
- Code review and automated checks must reject newly introduced `mod.rs`
  files.

### Bun and Node Built-ins

- Internal TypeScript imports must use explicit `.ts` extensions. Do not refer
  to a sibling source module through a `.js` specifier.
- External package imports remain package specifiers; do not rewrite `.js`
  paths inside third-party packages.
- Bun does not provide a `bun:fs` replacement module.
- Bun officially implements Node built-ins such as `node:fs`, `node:path`,
  `node:child_process`, and `node:crypto`; these imports are correct in Bun
  TypeScript and must remain type-checkable through `@types/bun`.
- Prefer `node:*` for portable filesystem, path, process, and crypto APIs.
  Use Bun-native APIs such as `Bun.file`, `Bun.write`, `Bun.spawn`,
  `Bun.CryptoHasher`, or `Bun.Glob` only when they make a specific operation
  clearer or faster.
- `scripts/tsconfig.json` owns type checking for root scripts. Keep its
  `typeRoots` pointed at the installed Bun types and run
  `bun run --cwd browser-worker typecheck:scripts` after script changes.

### RPC Boundary

- The Rust DTOs in `crates/browser/src/protocol.rs` are the single source of
  truth for the wire contract.
- `browser-worker/src/generated/protocol.ts` is generated by `ts-rs`.
  Never edit it manually.
- The transport is JSON-RPC 2.0 framed with `Content-Length`. stdout is
  protocol-only; all logs go to stderr.
- Add methods in this order: Rust DTO, typed Rust method, regenerated
  TypeScript, typed worker handler, tests, generated-file drift check.
- Worker requests must be generic browser operations. Employer-specific page
  behaviour belongs in an upper-layer platform feature.
- Breaking an existing request or response shape requires an explicit
  compatibility decision and migration plan.

### Configuration and Secrets

- `config.toml` contains non-secret settings only.
- Passwords, tokens, API keys, and other secrets must use `SecretStore`.
- Never log secret values or include them in error messages.
- Configuration precedence is:
  compiled defaults < built-in defaults < user TOML < `JOBLESS__*`
  environment variables.
- Do not add another config file, duplicate environment prefix, or custom
  path resolver.
- Preserve atomic-save behavior and schema validation when extending config.

### Browser Profiles and Account State

- One account maps to one Chromium profile directory.
- Cookies, localStorage, IndexedDB, and site sessions remain in the Chromium
  profile. Do not copy them into SQLite, TOML, or logs.
- SQLite stores account metadata, not browser authentication payloads.
- Every profile ID must be validated and prevented from escaping the profile
  root.
- CAPTCHA handling is human-in-the-loop through the visible browser window.
  Do not add a CAPTCHA solver without an explicit product and security
  decision.

### Generated and Binary Files

- `browser-worker/src/generated/protocol.ts` is generated and committed.
- `Cargo.lock` and `browser-worker/bun.lock` are committed lock files.
- `runtime/bun`, `runtime/chromium`, staged worker dependencies, `target/`, and
  `dist/` are generated and ignored.
- `.bun-version` is the canonical Bun version. Patchright is pinned in
  `browser-worker/package.json`, and the browser revision comes from
  `patchright-core/browsers.json`; do not duplicate those versions in scripts.
- Do not hand-edit generated bindings or commit runtime binaries.
- Regenerate bindings through Rust tests rather than editing TypeScript.

## Where New Work Belongs

Use this routing table before creating files:

| Change | Owner |
| --- | --- |
| Config field, precedence, path, or secret contract | `crates/config` |
| SQLite schema, migration, query, or repository | `crates/storage` |
| Worker lifecycle, RPC framing, protocol DTO, browser manager API | `crates/browser` |
| Non-visual use-case orchestration | `crates/application` |
| Window, navigation, global app state, composition | `crates/app` |
| Accounts, tasks, history, or another cohesive capability | New feature crate |
| Generic browser/session mechanics | `browser-worker` |
| Boss-specific selectors or application flow | Future platform feature crate |
| Runtime download/staging | `scripts/prepare-runtime.ts` |
| Portable archive layout | `scripts/package.ts` and `packaging/` |
| CI or release orchestration | `.github/workflows/` |
| User-visible strings | `locales/` |

Do not create a new crate for one screen, helper, or type. Split a crate only
when a capability has its own state/lifecycle, a stable public seam, and
meaningful independent tests.

## Change Checklist

Before editing:

- Identify the owning layer and crate.
- Inspect the relevant `Cargo.toml`, `package.json`, public API, and tests.
- Confirm that the dependency direction remains downward.
- Check whether the change affects the RPC or generated TypeScript contract.
- Check whether the change introduces secrets, account state, or profile data.

After editing:

- Keep the diff limited to the requested scope.
- Add or update tests at the boundary being changed.
- Check that touched handwritten source files remain at or below 500 lines.
- Regenerate protocol types when Rust DTOs change.
- Update `AGENTS.md` when a layer, boundary, or dependency rule changes.
- Do not leave duplicate configuration models, path resolvers, browser
  launchers, or parallel implementations of the same capability.

## Required Verification

Run the smallest relevant set, but never skip a category affected by the
change.

Rust changes:

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --locked
```

Worker or protocol changes:

```sh
cargo test -p jobless-browser --lib
git diff --exit-code -- browser-worker/src/generated
bun install --cwd browser-worker --frozen-lockfile
bun run --cwd browser-worker typecheck
bun run --cwd browser-worker typecheck:scripts
bun test browser-worker/src/test
```

Runtime changes:

```sh
bun run scripts/prepare-runtime.ts --check
JOBLESS_RUNTIME_DIR="$PWD/runtime" cargo run -p jobless-app -- --smoke-test
```

Module-layout changes:

```sh
bun run scripts/check-mod-layout.ts
```

Packaging changes:

```sh
bun run scripts/prepare-runtime.ts
cargo build --release --locked
bun run scripts/package.ts
```

Linux release verification uses Xvfb, Mesa lavapipe, and
`dbus-run-session`. It must run the real GPUI window and headed Chromium smoke
test; setting `ZED_HEADLESS` is not a substitute. Follow
`.github/workflows/release.yml` for the authoritative command.

## Prohibited Shortcuts

- Do not put business rules in `jobless-config`, `jobless-storage`,
  `jobless-browser`, `browser-worker`, or delivery scripts.
- Do not import Patchright outside its worker adapter.
- Do not access SQLite outside `jobless-storage`.
- Do not import GPUI outside the application/feature layer.
- Do not let an upper-layer type leak into a lower crate.
- Do not create bidirectional dependencies between feature crates.
- Do not store secrets or browser cookies in plaintext configuration.
- Do not design packaged releases to require system Bun or Chromium. The
  bundled runtime is canonical; any PATH fallback is development-only and
  must produce an explicit diagnostic when runtime files are missing.
- Do not upgrade Patchright and its browser independently.
- Do not hand-edit generated protocol bindings.
- Do not create `mod.rs`; use sibling `module.rs` plus `module/` layout.
- Do not use a global `ui/` directory as a dumping ground for unrelated
  feature views.
- Do not split large files into `part1`, `part2`, or line-numbered fragments.

Simplicity is not permission to collapse boundaries. If a boundary is
inconvenient, document the concrete need and propose an explicit architectural
change rather than bypassing it.
