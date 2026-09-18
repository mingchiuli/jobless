import { existsSync, readFileSync } from "node:fs";
import { createRequire } from "node:module";
import { dirname, join } from "node:path";

import { chromium } from "patchright";

import type {
  BrowserEngine,
  BrowserLaunchOptions,
  BrowserSessionHandle,
  BrowserSessionEnd,
} from "./engine.ts";
import type { BrowserInfo } from "./generated/protocol.ts";

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

    const page = context.pages()[0] ?? (await context.newPage());
    if (options.url !== "about:blank") {
      await page.goto(options.url, {
        waitUntil: "domcontentloaded",
        timeout: 30_000,
      });
    }

    const listeners = new Set<(reason: BrowserSessionEnd) => void>();
    let ended = false;
    let crashed = false;
    const finish = (reason: BrowserSessionEnd) => {
      if (ended) {
        return;
      }
      ended = true;
      for (const listener of listeners) {
        listener(reason);
      }
    };

    page.on("crash", () => {
      crashed = true;
      finish("crashed");
      void context.close().catch(() => {});
    });
    context.on("close", () => {
      finish(crashed ? "crashed" : "closed");
    });

    return {
      url: page.url() || options.url,
      async close() {
        if (!ended) {
          await context.close();
        }
      },
      onClosed(listener: (reason: BrowserSessionEnd) => void) {
        listeners.add(listener);
      },
    };
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
