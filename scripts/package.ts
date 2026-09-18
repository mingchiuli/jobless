#!/usr/bin/env bun

import { spawnSync } from "node:child_process";
import {
  chmodSync,
  copyFileSync,
  cpSync,
  existsSync,
  mkdirSync,
  rmSync,
  writeFileSync,
} from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const scriptDir = dirname(fileURLToPath(import.meta.url));
const root = resolve(scriptDir, "..");
const version = (process.env.JOBLESS_VERSION ?? "0.1.0").replace(/^v/, "");
const target =
  process.env.JOBLESS_TARGET ??
  `${platformName()}-${process.arch === "arm64" ? "aarch64" : "x86_64"}`;
const binaryName = process.platform === "win32" ? "jobless.exe" : "jobless";
const binary = process.env.JOBLESS_BIN ?? join(root, "target", "release", binaryName);
const dist = join(root, "dist");
const stage = join(dist, `jobless-${version}-${target}`);
const artifact = process.platform === "linux"
  ? join(dist, `jobless-${version}-${target}.tar.gz`)
  : join(dist, `jobless-${version}-${target}.zip`);

if (!existsSync(binary)) {
  throw new Error(`release binary not found: ${binary}`);
}
assertRuntime();

rmSync(stage, { recursive: true, force: true });
mkdirSync(stage, { recursive: true });

if (process.platform === "darwin") {
  packageMacOs();
} else {
  packageArchiveContents(stage);
  if (process.platform === "win32") {
    run("tar", ["-a", "-c", "-f", artifact, "-C", dist, `jobless-${version}-${target}`]);
  } else {
    run("tar", ["-czf", artifact, "-C", dist, `jobless-${version}-${target}`]);
  }
}

process.stdout.write(`${artifact}\n`);

function packageArchiveContents(destination: string) {
  copyFileSync(binary, join(destination, binaryName));
  if (process.platform !== "win32") {
    chmodSync(join(destination, binaryName), 0o755);
  }
  cpSync(join(root, "runtime"), join(destination, "runtime"), {
    recursive: true,
    filter(source) {
      return !source.split(/[\\/]/).includes(".downloads");
    },
  });
  cpSync(join(root, "locales"), join(destination, "locales"), {
    recursive: true,
  });
  copyFileSync(join(root, "LICENSE"), join(destination, "LICENSE"));
  copyFileSync(join(root, "README.md"), join(destination, "README.md"));
  copyFileSync(
    join(root, "packaging", platformName(), "README.md"),
    join(destination, "RUNNING.md"),
  );
}

function packageMacOs() {
  const app = join(stage, "Jobless.app");
  const contents = join(app, "Contents");
  const macos = join(contents, "MacOS");
  const resources = join(contents, "Resources");
  mkdirSync(macos, { recursive: true });
  mkdirSync(resources, { recursive: true });

  copyFileSync(binary, join(macos, "jobless"));
  chmodSync(join(macos, "jobless"), 0o755);
  cpSync(join(root, "runtime"), join(resources, "runtime"), {
    recursive: true,
    filter(source) {
      return !source.split(/[\\/]/).includes(".downloads");
    },
  });
  cpSync(join(root, "locales"), join(resources, "locales"), {
    recursive: true,
  });
  copyFileSync(join(root, "LICENSE"), join(resources, "LICENSE"));
  copyFileSync(join(root, "README.md"), join(resources, "README.md"));
  writeFileSync(
    join(contents, "Info.plist"),
    `<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleIdentifier</key><string>com.jobless.desktop</string>
  <key>CFBundleName</key><string>Jobless</string>
  <key>CFBundleDisplayName</key><string>Jobless</string>
  <key>CFBundleExecutable</key><string>jobless</string>
  <key>CFBundlePackageType</key><string>APPL</string>
  <key>CFBundleShortVersionString</key><string>${version}</string>
  <key>CFBundleVersion</key><string>${version}</string>
  <key>NSHighResolutionCapable</key><true/>
</dict>
</plist>
`,
  );

  run("ditto", [
    "-c",
    "-k",
    "--sequesterRsrc",
    "--keepParent",
    app,
    artifact,
  ]);
}

function assertRuntime() {
  const runtimeBinary = process.platform === "win32"
    ? join(root, "runtime", "bun", "bun.exe")
    : join(root, "runtime", "bun", "bun");
  const required = [
    runtimeBinary,
    join(root, "runtime", "browser-worker", "src", "main.ts"),
    join(root, "runtime", "chromium"),
  ];
  const missing = required.filter((path) => !existsSync(path));
  if (missing.length > 0) {
    throw new Error(`runtime is incomplete; missing:\n${missing.join("\n")}`);
  }
}

function platformName(): "macos" | "windows" | "linux" {
  if (process.platform === "darwin") return "macos";
  if (process.platform === "win32") return "windows";
  if (process.platform === "linux") return "linux";
  throw new Error(`unsupported packaging platform: ${process.platform}`);
}

function run(command: string, commandArgs: string[]) {
  const result = spawnSync(command, commandArgs, { stdio: "inherit" });
  if (result.status !== 0) {
    throw new Error(`${command} ${commandArgs.join(" ")} failed`);
  }
}
