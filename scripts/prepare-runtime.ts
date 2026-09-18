#!/usr/bin/env bun

import { spawnSync } from "node:child_process";
import { createHash } from "node:crypto";
import {
  chmodSync,
  copyFileSync,
  cpSync,
  existsSync,
  mkdirSync,
  readFileSync,
  readdirSync,
  rmSync,
  writeFileSync,
} from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const scriptDir = dirname(fileURLToPath(import.meta.url));
const root = resolve(scriptDir, "..");
const bunVersion = readFileSync(join(root, ".bun-version"), "utf8").trim();
const runtimeDir = join(root, "runtime");
const bunDir = join(runtimeDir, "bun");
const chromiumDir = join(runtimeDir, "chromium");
const stagedWorkerDir = join(runtimeDir, "browser-worker");
const workerDir = join(root, "browser-worker");
const args = new Set(process.argv.slice(2));
const force = args.has("--force");
const checkOnly = args.has("--check");
const browserDownloadHost = process.env.JOBLESS_BROWSER_DOWNLOAD_HOST?.replace(
  /\/$/,
  "",
);
const bunDownloadBase = (
  process.env.JOBLESS_BUN_DOWNLOAD_BASE ??
  `https://github.com/oven-sh/bun/releases/download/bun-v${bunVersion}`
).replace(/\/$/, "");
const bunBinary = process.platform === "win32"
  ? join(bunDir, "bun.exe")
  : join(bunDir, "bun");

if (checkOnly) {
  assertWorkerVersions();
  checkRuntime();
  process.exit(0);
}

assertWorkerVersions();
await downloadBun();
stageWorker();
installChromium();
checkRuntime();

function assertWorkerVersions() {
  const packageJson = JSON.parse(
    readFileSync(join(workerDir, "package.json"), "utf8"),
  );
  if (packageJson.packageManager !== `bun@${bunVersion}`) {
    throw new Error(
      `browser-worker packageManager must be bun@${bunVersion}, found ${packageJson.packageManager}`,
    );
  }
  if (typeof packageJson.dependencies?.patchright !== "string") {
    throw new Error("browser-worker must declare a pinned patchright dependency");
  }
  if (!existsSync(join(workerDir, "bun.lock"))) {
    throw new Error("browser-worker/bun.lock is missing; run `bun install`");
  }
}

async function downloadBun() {
  const archiveName = bunArchiveName();
  const archivePath = join(runtimeDir, ".downloads", archiveName);
  const url = `${bunDownloadBase}/${archiveName}`;

  if (force || !existsSync(archivePath)) {
    mkdirSync(dirname(archivePath), { recursive: true });
    log(`downloading Bun ${bunVersion} from ${url}`);
    const response = await fetch(url);
    if (!response.ok) {
      throw new Error(`Bun download failed: ${response.status}`);
    }
    writeFileSync(archivePath, Buffer.from(await response.arrayBuffer()));
  }
  await verifyBunArchive(archivePath, archiveName);

  cleanGeneratedDirectory(bunDir);
  const extractDir = join(runtimeDir, ".downloads", "bun-extracted");
  rmSync(extractDir, { recursive: true, force: true });
  mkdirSync(extractDir, { recursive: true });
  run("tar", ["-xf", archivePath, "-C", extractDir]);

  const extractedRoot = join(
    extractDir,
    archiveName.replace(/\.zip$/, ""),
  );
  const executableName = process.platform === "win32" ? "bun.exe" : "bun";
  const source = join(extractedRoot, executableName);
  copyFileSync(source, bunBinary);
  if (process.platform !== "win32") {
    chmodSync(bunBinary, 0o755);
  }
  const license = join(extractedRoot, "LICENSE.md");
  if (existsSync(license)) {
    copyFileSync(license, join(bunDir, "LICENSE.md"));
  }
  rmSync(join(runtimeDir, ".downloads"), { recursive: true, force: true });
  log(`staged ${bunBinary}`);
}

async function verifyBunArchive(archivePath: string, archiveName: string) {
  const checksumsUrl = `${bunDownloadBase}/SHASUMS256.txt`;
  const response = await fetch(checksumsUrl);
  if (!response.ok) {
    throw new Error(`Bun checksum download failed: ${response.status}`);
  }

  const checksums = await response.text();
  const expected = checksums
    .split("\n")
    .map((line) => line.trim().split(/\s+/))
    .find(([, name]) => name === archiveName)?.[0];
  if (!expected) {
    throw new Error(`missing checksum for ${archiveName}`);
  }

  const actual = createHash("sha256")
    .update(readFileSync(archivePath))
    .digest("hex");
  if (actual !== expected) {
    throw new Error(`Bun archive checksum mismatch for ${archiveName}`);
  }
  log(`verified Bun archive checksum for ${archiveName}`);
}

function stageWorker() {
  run(bunBinary, ["install", "--frozen-lockfile"], { cwd: workerDir });
  run(bunBinary, ["run", "typecheck"], { cwd: workerDir });

  cleanGeneratedDirectory(stagedWorkerDir);
  cpSync(join(workerDir, "src"), join(stagedWorkerDir, "src"), {
    recursive: true,
    filter(source) {
      return (
        !source.split(/[\\/]/).includes("test") &&
        !source.endsWith("healthcheck.ts")
      );
    },
  });
  copyFileSync(
    join(workerDir, "package.json"),
    join(stagedWorkerDir, "package.json"),
  );
  copyFileSync(
    join(workerDir, "bun.lock"),
    join(stagedWorkerDir, "bun.lock"),
  );
  run(
    bunBinary,
    ["install", "--frozen-lockfile", "--production"],
    { cwd: stagedWorkerDir },
  );
  log(`staged browser worker at ${stagedWorkerDir}`);
}

function installChromium() {
  const cli = join(workerDir, "node_modules", "patchright", "cli.js");
  if (!existsSync(cli)) {
    throw new Error(`Patchright CLI not found at ${cli}`);
  }

  log(`installing Patchright Chromium into ${chromiumDir}`);
  run(bunBinary, [cli, "install", "chromium"], {
    cwd: workerDir,
    env: {
      ...process.env,
      PLAYWRIGHT_BROWSERS_PATH: chromiumDir,
      ...(browserDownloadHost
        ? { PLAYWRIGHT_DOWNLOAD_HOST: browserDownloadHost }
        : {}),
    },
  });
}

function checkRuntime() {
  const descriptor = chromiumDescriptor();
  const worker = join(stagedWorkerDir, "src", "main.ts");
  const chromiumInstalled =
    existsSync(chromiumDir) &&
    existsSync(join(chromiumDir, `chromium-${descriptor.revision}`));

  const missing = [];
  if (!existsSync(bunBinary)) missing.push(bunBinary);
  if (!existsSync(worker)) missing.push(worker);
  if (!chromiumInstalled) {
    missing.push(`${chromiumDir}/chromium-${descriptor.revision}`);
  }

  if (missing.length > 0) {
    throw new Error(`runtime is incomplete:\n${missing.join("\n")}`);
  }

  const installedBun = spawnSync(bunBinary, ["--version"], {
    encoding: "utf8",
  });
  if (installedBun.status !== 0 || installedBun.stdout.trim() !== bunVersion) {
    throw new Error(
      `runtime Bun version mismatch: expected ${bunVersion}, found ${installedBun.stdout.trim()}`,
    );
  }

  log("runtime check passed");
}

function chromiumDescriptor(): BrowserDescriptor {
  const packagePath = join(
    stagedWorkerDir,
    "node_modules",
    "patchright-core",
    "package.json",
  );
  const registryPath = join(dirname(packagePath), "browsers.json");
  if (!existsSync(registryPath)) {
    throw new Error(
      "runtime browser registry is missing; run `bun run scripts/prepare-runtime.ts`",
    );
  }
  const registry = JSON.parse(readFileSync(registryPath, "utf8")) as {
    browsers: BrowserDescriptor[];
  };
  const descriptor = registry.browsers.find((browser) => browser.name === "chromium");
  if (!descriptor) {
    throw new Error("Patchright browser registry has no chromium descriptor");
  }
  return descriptor;
}

function bunArchiveName() {
  if (process.platform === "darwin") {
    if (process.arch === "arm64") return "bun-darwin-aarch64.zip";
    if (process.arch === "x64") return "bun-darwin-x64-baseline.zip";
  }
  if (process.platform === "linux") {
    if (process.arch === "arm64") return "bun-linux-aarch64.zip";
    if (process.arch === "x64") return "bun-linux-x64-baseline.zip";
  }
  if (process.platform === "win32" && process.arch === "x64") {
    return "bun-windows-x64-baseline.zip";
  }
  throw new Error(`unsupported Bun runtime target: ${process.platform}-${process.arch}`);
}

function cleanGeneratedDirectory(directory: string) {
  mkdirSync(directory, { recursive: true });
  for (const entry of readdirSync(directory)) {
    if (entry !== ".gitkeep") {
      rmSync(join(directory, entry), { recursive: true, force: true });
    }
  }
  const keep = join(directory, ".gitkeep");
  if (!existsSync(keep)) {
    writeFileSync(keep, "");
  }
}

function run(
  command: string,
  commandArgs: string[],
  options: {
    cwd?: string;
    env?: Record<string, string | undefined>;
  } = {},
) {
  const result = spawnSync(command, commandArgs, {
    cwd: options.cwd ?? root,
    env: options.env ?? process.env,
    stdio: "inherit",
  });
  if (result.status !== 0) {
    throw new Error(`${command} ${commandArgs.join(" ")} failed`);
  }
}

function log(message: string) {
  process.stderr.write(`[prepare-runtime] ${message}\n`);
}

interface BrowserDescriptor {
  name: string;
  revision: string;
  browserVersion: string;
}
