import assert from "node:assert/strict";
import { PassThrough } from "node:stream";
import test from "node:test";

import {
  createMessageConnection,
  type MessageConnection,
} from "vscode-jsonrpc";
import {
  StreamMessageReader,
  StreamMessageWriter,
} from "vscode-jsonrpc/node";

import {
  createWorkerConnection,
  stderrLogger,
} from "../connection.ts";
import type {
  BrowserEngine,
  BrowserLaunchOptions,
  BrowserPageHandle,
  BrowserSessionEnd,
  BrowserSessionHandle,
  EnsurePageResult,
  PlatformId,
} from "../engine.ts";
import type {
  BrowserInfo,
  PageEventParams,
  PageSnapshot,
  PageState,
  SessionEventParams,
} from "../generated/protocol.ts";
import {
  pageActivateRequest,
  pageCloseRequest,
  pageEnsureRequest,
  pageListRequest,
  pageNavigateRequest,
  runtimeHealthRequest,
  sessionCloseRequest,
  sessionStartRequest,
} from "../rpc.ts";
import { WorkerService } from "../worker-service.ts";

class FakePage implements BrowserPageHandle {
  state: PageState = "open";
  currentUrl: string;

  constructor(
    readonly pageId: string,
    readonly platformId: PlatformId,
    url: string,
  ) {
    this.currentUrl = url;
  }

  url(): string {
    return this.currentUrl;
  }

  isClosed(): boolean {
    return false;
  }

  async activate(): Promise<void> {}

  async navigate(url: string): Promise<string> {
    this.currentUrl = url;
    return url;
  }

  async close(): Promise<void> {}

  snapshot(): PageSnapshot {
    return {
      page_id: this.pageId,
      platform_id: this.platformId,
      url: this.currentUrl,
      state: this.state,
    };
  }
}

class FakeSession implements BrowserSessionHandle {
  readonly pages = new Map<PlatformId, FakePage>();
  private nextPage = 1;
  private readonly closedListeners = new Set<
    (reason: BrowserSessionEnd) => void
  >();
  private readonly pageListeners = new Set<
    (params: {
      pageId: string;
      platformId: PlatformId;
      url: string;
      event: PageEventParams["event"];
    }) => void
  >();

  async ensurePage(
    platformId: PlatformId,
    url: string,
  ): Promise<EnsurePageResult> {
    const existing = this.pages.get(platformId);
    if (existing) {
      return { page: existing, created: false };
    }
    const page = new FakePage(
      `page-${platformId}-${this.nextPage++}`,
      platformId,
      url,
    );
    this.pages.set(platformId, page);
    this.emitPage(page, "opened");
    return { page, created: true };
  }

  async navigatePage(
    platformId: PlatformId,
    url: string,
  ): Promise<FakePage> {
    const page = this.requirePage(platformId);
    await page.navigate(url);
    this.emitPage(page, "navigated");
    return page;
  }

  async activatePage(platformId: PlatformId): Promise<boolean> {
    return this.pages.has(platformId);
  }

  async closePage(platformId: PlatformId): Promise<boolean> {
    const page = this.pages.get(platformId);
    if (!page) {
      return false;
    }
    this.pages.delete(platformId);
    this.emitPage(page, "closed");
    return true;
  }

  listPages(): PageSnapshot[] {
    return [...this.pages.values()].map((page) => page.snapshot());
  }

  async close(): Promise<void> {
    this.pages.clear();
    for (const listener of this.closedListeners) {
      listener("closed");
    }
  }

  onClosed(listener: (reason: BrowserSessionEnd) => void): void {
    this.closedListeners.add(listener);
  }

  onPageEvent(
    listener: (params: {
      pageId: string;
      platformId: PlatformId;
      url: string;
      event: PageEventParams["event"];
    }) => void,
  ): void {
    this.pageListeners.add(listener);
  }

  crashPage(platformId: PlatformId): void {
    const page = this.pages.get(platformId);
    if (page) {
      page.state = "crashed";
      this.pages.delete(platformId);
      this.emitPage(page, "crashed");
    }
  }

  private requirePage(platformId: PlatformId): FakePage {
    const page = this.pages.get(platformId);
    if (!page) {
      throw new Error(`missing page: ${platformId}`);
    }
    return page;
  }

  private emitPage(
    page: FakePage,
    event: PageEventParams["event"],
  ): void {
    for (const listener of this.pageListeners) {
      listener({
        pageId: page.pageId,
        platformId: page.platformId,
        url: page.url(),
        event,
      });
    }
  }
}

class FakeEngine implements BrowserEngine {
  readonly name = "fake";
  readonly version = "1.0.0";
  readonly sessions: FakeSession[] = [];
  launches: BrowserLaunchOptions[] = [];

  async getBrowserInfo(): Promise<BrowserInfo> {
    return {
      available: true,
      executable_path: "/tmp/fake-browser",
      revision: "1",
      version: "1.0.0",
    };
  }

  async launchPersistent(
    _profileDir: string,
    options: BrowserLaunchOptions,
  ): Promise<BrowserSessionHandle> {
    this.launches.push(options);
    const session = new FakeSession();
    this.sessions.push(session);
    return session;
  }

  async dispose(): Promise<void> {}
}

interface Harness {
  client: MessageConnection;
  worker: MessageConnection;
  engine: FakeEngine;
  sessionEvents: SessionEventParams[];
  pageEvents: PageEventParams[];
  dispose(): void;
}

function harness(): Harness {
  const clientToWorker = new PassThrough();
  const workerToClient = new PassThrough();
  const engine = new FakeEngine();
  const sessionEvents: SessionEventParams[] = [];
  const pageEvents: PageEventParams[] = [];
  const service = new WorkerService(engine, {
    profileRoot: "/tmp/jobless-profiles",
    headless: false,
    emitSessionEvent: (event) => sessionEvents.push(event),
    emitPageEvent: (event) => pageEvents.push(event),
  });
  const worker = createWorkerConnection(
    clientToWorker,
    workerToClient,
    service,
    stderrLogger,
  );
  const client = createMessageConnection(
    new StreamMessageReader(workerToClient),
    new StreamMessageWriter(clientToWorker),
    stderrLogger,
  );
  worker.listen();
  client.listen();

  return {
    client,
    worker,
    engine,
    sessionEvents,
    pageEvents,
    dispose() {
      client.dispose();
      worker.dispose();
    },
  };
}

async function startSession(context: Harness): Promise<string> {
  const result = await context.client.sendRequest(sessionStartRequest, {
    profile_id: "default",
  });
  return result.session_id;
}

test("health returns engine and browser details", async () => {
  const context = harness();
  try {
    const result = await context.client.sendRequest(runtimeHealthRequest, {});
    assert.equal(result.engine.name, "fake");
    assert.equal(result.browser.available, true);
    assert.equal(result.runtime.name, "bun");
  } finally {
    context.dispose();
  }
});

test("ensure is idempotent per platform", async () => {
  const context = harness();
  try {
    const sessionId = await startSession(context);
    const first = await context.client.sendRequest(pageEnsureRequest, {
      session_id: sessionId,
      platform_id: "boss",
      url: "https://example.com/boss",
    });
    const second = await context.client.sendRequest(pageEnsureRequest, {
      session_id: sessionId,
      platform_id: "boss",
      url: "https://ignored.example.com",
    });

    assert.equal(first.page_id, second.page_id);
    assert.equal(first.created, true);
    assert.equal(second.created, false);
    assert.equal(second.url, "https://example.com/boss");
  } finally {
    context.dispose();
  }
});

test("manages independent platform pages", async () => {
  const context = harness();
  try {
    const sessionId = await startSession(context);
    await context.client.sendRequest(pageEnsureRequest, {
      session_id: sessionId,
      platform_id: "boss",
      url: "https://example.com/boss",
    });
    await context.client.sendRequest(pageEnsureRequest, {
      session_id: sessionId,
      platform_id: "liepin",
      url: "https://example.com/liepin",
    });

    const listed = await context.client.sendRequest(pageListRequest, {
      session_id: sessionId,
    });
    assert.deepEqual(
      listed.pages.map((page) => page.platform_id).sort(),
      ["boss", "liepin"],
    );

    await context.client.sendRequest(pageNavigateRequest, {
      session_id: sessionId,
      platform_id: "boss",
      url: "https://example.com/jobs",
    });
    await context.client.sendRequest(pageActivateRequest, {
      session_id: sessionId,
      platform_id: "boss",
    });
    const closed = await context.client.sendRequest(pageCloseRequest, {
      session_id: sessionId,
      platform_id: "boss",
    });
    assert.equal(closed.closed, true);
  } finally {
    context.dispose();
  }
});

test("reports page crashes and recreates on next ensure", async () => {
  const context = harness();
  try {
    const sessionId = await startSession(context);
    const first = await context.client.sendRequest(pageEnsureRequest, {
      session_id: sessionId,
      platform_id: "boss",
      url: "about:blank",
    });
    context.engine.sessions[0]?.crashPage("boss");

    assert.equal(context.pageEvents.at(-1)?.event, "crashed");
    assert.equal(context.pageEvents.at(-1)?.page_id, first.page_id);

    const recreated = await context.client.sendRequest(pageEnsureRequest, {
      session_id: sessionId,
      platform_id: "boss",
      url: "about:blank",
    });
    assert.equal(recreated.created, true);
    assert.notEqual(recreated.page_id, first.page_id);
  } finally {
    context.dispose();
  }
});

test("closes a session", async () => {
  const context = harness();
  try {
    const sessionId = await startSession(context);
    const result = await context.client.sendRequest(sessionCloseRequest, {
      session_id: sessionId,
    });
    assert.equal(result.closed, 1);
    assert.equal(context.sessionEvents.at(-1)?.event, "closed");
  } finally {
    context.dispose();
  }
});
