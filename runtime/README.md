# Runtime staging

`scripts/prepare-runtime.ts` populates this directory for the current target:

```text
runtime/
├── bun/
│   └── bun[.exe]
├── browser-worker/
│   ├── src/
│   └── node_modules/
└── chromium/
    └── chromium-<revision>/
```

The downloaded Bun runtime and Patchright browsers are build artifacts and are not
committed. Run:

```sh
bun run scripts/prepare-runtime.ts
bun run scripts/prepare-runtime.ts --check
```

Set `JOBLESS_BROWSER_DOWNLOAD_HOST=https://cdn.npmmirror.com/binaries/playwright`
when the official Playwright CDN is slow, for example on networks in China.
Set `JOBLESS_BUN_DOWNLOAD_BASE=https://cdn.npmmirror.com/binaries/bun/bun-v1.4.2`
if the official Bun release server is slow.

The Bun version is read from `.bun-version`. Patchright and Chromium versions
come from `browser-worker/package.json` and `patchright-core/browsers.json`.
