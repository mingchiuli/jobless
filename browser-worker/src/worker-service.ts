import { randomUUID } from "node:crypto";
import { isAbsolute, relative, resolve, sep } from "node:path";

import type { CancellationToken } from "vscode-jsonrpc";

import type {
  BrowserCloseParams,
  BrowserCloseResult,
  BrowserEventParams,
  BrowserOpenParams,
  BrowserOpenResult,
  RuntimeHealthResult,
  RuntimeShutdownResult,
} from "./generated/protocol.ts";
import type {
  BrowserEngine,
  BrowserSessionHandle,
} from "./engine.ts";

export interface WorkerServiceOptions {
  profileRoot: string;
  headless: boolean;
  emitEvent: (event: BrowserEventParams) => void;
}

export class WorkerService {
  private readonly sessions = new Map<string, BrowserSessionHandle>();

  constructor(
    private readonly engine: BrowserEngine,
    private readonly options: WorkerServiceOptions,
  ) {}

  async health(): Promise<RuntimeHealthResult> {
    return {
      runtime: {
        name: "bun",
        version: Bun.version,
        node_compat_version: process.version,
      },
      engine: {
        name: this.engine.name,
        version: this.engine.version,
      },
      browser: await this.engine.getBrowserInfo(),
    };
  }

  async open(
    params: BrowserOpenParams,
    token: CancellationToken,
  ): Promise<BrowserOpenResult> {
    validateUrl(params.url);
    const profileDir = this.profileDirectory(params.profile_id);
    const handle = await this.engine.launchPersistent(profileDir, {
      headless: this.options.headless,
      url: params.url,
    });
    const sessionId = randomUUID();
    this.sessions.set(sessionId, handle);

    handle.onClosed((reason) => {
      this.sessions.delete(sessionId);
      this.options.emitEvent({ session_id: sessionId, event: reason });
    });
    this.options.emitEvent({ session_id: sessionId, event: "opened" });

    token.onCancellationRequested(() => {
      void handle.close();
    });

    return {
      session_id: sessionId,
      url: handle.url,
    };
  }

  async close(params: BrowserCloseParams): Promise<BrowserCloseResult> {
    const targets = params.session_id
      ? [params.session_id]
      : [...this.sessions.keys()];

    let closed = 0;
    for (const sessionId of targets) {
      const handle = this.sessions.get(sessionId);
      if (!handle) {
        continue;
      }
      this.sessions.delete(sessionId);
      await handle.close();
      closed += 1;
    }

    return { closed };
  }

  async shutdown(): Promise<RuntimeShutdownResult> {
    await this.close({ session_id: null });
    await this.engine.dispose();
    return { ok: true };
  }

  async dispose(): Promise<void> {
    await this.close({ session_id: null });
    await this.engine.dispose();
  }

  private profileDirectory(profileId: string): string {
    if (!/^[A-Za-z0-9._-]+$/.test(profileId) || profileId === "." || profileId === "..") {
      throw new Error(`invalid browser profile id: ${profileId}`);
    }

    const root = resolve(this.options.profileRoot);
    const profile = resolve(root, profileId);
    const child = relative(root, profile);
    if (child === "" || child.startsWith(`..${sep}`) || child === ".." || isAbsolute(child)) {
      throw new Error(`browser profile escapes profile root: ${profileId}`);
    }
    return profile;
  }
}

function validateUrl(url: string): void {
  if (url === "about:blank") {
    return;
  }

  let parsed: URL;
  try {
    parsed = new URL(url);
  } catch {
    throw new Error(`invalid browser URL: ${url}`);
  }

  if (parsed.protocol !== "http:" && parsed.protocol !== "https:") {
    throw new Error(`unsupported browser URL scheme: ${parsed.protocol}`);
  }
}
