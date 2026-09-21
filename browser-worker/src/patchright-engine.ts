import { existsSync, readFileSync } from "node:fs";
import { createRequire } from "node:module";
import { dirname, join } from "node:path";

import { chromium } from "patchright";

import type {
  BrowserEngine,
  BrowserLaunchOptions,
  BrowserSessionHandle,
} from "./engine.ts";
import type { BrowserInfo } from "./generated/protocol.ts";
import { PatchrightSession } from "./patchright-session.ts";

const require = createRequire(import.meta.url);

interface PatchrightPackage {
  version: string;
}

interface BrowserDescriptor {
  name: string;
  revision: string;
  browserVersion: string;
}

interface BrowserRegistry {
  browsers: BrowserDescriptor[];
}

export class PatchrightEngine implements BrowserEngine {
  readonly name = "patchright";
  readonly version: string;

  constructor() {
    const packageJson = require("patchright/package.json") as PatchrightPackage;
    this.version = packageJson.version;
  }

  async getBrowserInfo(): Promise<BrowserInfo> {
    let executablePath: string | null = null;
    try {
      executablePath = chromium.executablePath();
    } catch {
      executablePath = null;
    }

    const descriptor = this.browserDescriptor();
    return {
      available: executablePath !== null && existsSync(executablePath),
      executable_path: executablePath,
      revision: descriptor?.revision ?? null,
      version: descriptor?.browserVersion ?? null,
    };
  }

  async launchPersistent(
    profileDir: string,
    options: BrowserLaunchOptions,
  ): Promise<BrowserSessionHandle> {
    const context = await chromium.launchPersistentContext(profileDir, {
      headless: options.headless,
      viewport: null,
    });
    return new PatchrightSession(context);
  }

  async dispose(): Promise<void> {
    // Every session owns its own persistent context and closes it directly.
  }

  private browserDescriptor(): BrowserDescriptor | undefined {
    try {
      const packagePath = require.resolve("patchright-core/package.json");
      const registryPath = join(dirname(packagePath), "browsers.json");
      const registry = JSON.parse(
        readFileSync(registryPath, "utf8"),
      ) as BrowserRegistry;
      return registry.browsers.find((browser) => browser.name === "chromium");
    } catch {
      return undefined;
    }
  }
}
