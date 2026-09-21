import { randomUUID } from "node:crypto";

import type { CancellationToken } from "vscode-jsonrpc";

import type {
  BrowserEngine,
  BrowserSessionHandle,
} from "./engine.ts";
import type {
  PageActivateParams,
  PageActivateResult,
  PageCloseParams,
  PageCloseResult,
  PageEnsureParams,
  PageEnsureResult,
  PageEventParams,
  PageListParams,
  PageListResult,
  PageNavigateParams,
  PageNavigateResult,
  RuntimeHealthResult,
  RuntimeShutdownResult,
  SessionCloseParams,
  SessionCloseResult,
  SessionEventParams,
  SessionId,
  SessionStartParams,
  SessionStartResult,
} from "./generated/protocol.ts";
import { profileDirectory, validateUrl } from "./validation.ts";

export interface WorkerServiceOptions {
  profileRoot: string;
  headless: boolean;
  emitSessionEvent: (event: SessionEventParams) => void;
  emitPageEvent: (event: PageEventParams) => void;
}

export class WorkerService {
  private readonly sessions = new Map<SessionId, BrowserSessionHandle>();

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

  async startSession(
    params: SessionStartParams,
    token: CancellationToken,
  ): Promise<SessionStartResult> {
    const sessionId = randomUUID();
    const session = await this.engine.launchPersistent(
      profileDirectory(this.options.profileRoot, params.profile_id),
      { headless: this.options.headless },
    );
    this.sessions.set(sessionId, session);

    session.onClosed((reason) => {
      this.sessions.delete(sessionId);
      this.options.emitSessionEvent({ session_id: sessionId, event: reason });
    });
    session.onPageEvent((event) => {
      this.options.emitPageEvent({
        session_id: sessionId,
        page_id: event.pageId,
        platform_id: event.platformId,
        url: event.url,
        event: event.event,
      });
    });
    this.options.emitSessionEvent({ session_id: sessionId, event: "opened" });

    token.onCancellationRequested(() => {
      void session.close();
    });

    return { session_id: sessionId };
  }

  async closeSession(params: SessionCloseParams): Promise<SessionCloseResult> {
    const targets = params.session_id
      ? [params.session_id]
      : [...this.sessions.keys()];

    let closed = 0;
    for (const sessionId of targets) {
      const session = this.sessions.get(sessionId);
      if (!session) {
        continue;
      }
      this.sessions.delete(sessionId);
      await session.close();
      closed += 1;
    }
    return { closed };
  }

  async ensurePage(params: PageEnsureParams): Promise<PageEnsureResult> {
    validateUrl(params.url);
    const result = await this.requireSession(params.session_id).ensurePage(
      params.platform_id,
      params.url,
    );
    return {
      page_id: result.page.pageId,
      platform_id: result.page.platformId,
      url: result.page.url(),
      created: result.created,
    };
  }

  async navigatePage(
    params: PageNavigateParams,
  ): Promise<PageNavigateResult> {
    validateUrl(params.url);
    const page = await this.requireSession(params.session_id).navigatePage(
      params.platform_id,
      params.url,
    );
    return { page_id: page.pageId, url: page.url() };
  }

  async activatePage(
    params: PageActivateParams,
  ): Promise<PageActivateResult> {
    const activated = await this.requireSession(
      params.session_id,
    ).activatePage(params.platform_id);
    return { activated };
  }

  async closePage(params: PageCloseParams): Promise<PageCloseResult> {
    const closed = await this.requireSession(params.session_id).closePage(
      params.platform_id,
    );
    return { closed };
  }

  async listPages(params: PageListParams): Promise<PageListResult> {
    return { pages: this.requireSession(params.session_id).listPages() };
  }

  async shutdown(): Promise<RuntimeShutdownResult> {
    await this.closeSession({ session_id: null });
    await this.engine.dispose();
    return { ok: true };
  }

  async dispose(): Promise<void> {
    await this.closeSession({ session_id: null });
    await this.engine.dispose();
  }

  private requireSession(sessionId: SessionId): BrowserSessionHandle {
    const session = this.sessions.get(sessionId);
    if (!session) {
      throw new Error(`browser session was not found: ${sessionId}`);
    }
    return session;
  }
}
