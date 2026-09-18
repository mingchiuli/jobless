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
  BrowserSessionHandle,
  BrowserSessionEnd,
} from "../engine.ts";
import type {
  BrowserEventParams,
  BrowserInfo,
} from "../generated/protocol.ts";
import {
  browserCloseRequest,
  browserOpenRequest,
  runtimeHealthRequest,
} from "../rpc.ts";
import { WorkerService } from "../worker-service.ts";

class FakeEngine implements BrowserEngine {
  readonly name = "fake";
  readonly version = "1.0.0";
  launches: BrowserLaunchOptions[] = [];
  closed = 0;
  lastClosed?: (reason: BrowserSessionEnd) => void;

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
    const listeners = new Set<(reason: BrowserSessionEnd) => void>();
    return {
      url: options.url,
      async close() {
        for (const listener of listeners) {
          listener("closed");
        }
      },
      onClosed: (listener) => {
        this.lastClosed = listener;
        listeners.add(listener);
      },
    };
  }

  async dispose(): Promise<void> {}
}

interface Harness {
  client: MessageConnection;
  worker: MessageConnection;
  engine: FakeEngine;
  events: BrowserEventParams[];
  dispose(): void;
}

function harness(): Harness {
  const clientToWorker = new PassThrough();
  const workerToClient = new PassThrough();
  const engine = new FakeEngine();
  const events: BrowserEventParams[] = [];
  const service = new WorkerService(engine, {
    profileRoot: "/tmp/jobless-profiles",
    headless: false,
    emitEvent: (event) => events.push(event),
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
    events,
    dispose() {
      client.dispose();
      worker.dispose();
    },
  };
}

test("health returns engine and browser details", async () => {
  const context = harness();
  try {
    const result = await context.client.sendRequest(runtimeHealthRequest, {});
    assert.equal(result.engine.name, "fake");
    assert.equal(result.browser.available, true);
    assert.equal(result.runtime.name, "bun");
    assert.equal(result.runtime.version, Bun.version);
    assert.equal(result.runtime.node_compat_version, process.version);
  } finally {
    context.dispose();
  }
});

test("open and close manage a persistent session", async () => {
  const context = harness();
  try {
    const opened = await context.client.sendRequest(browserOpenRequest, {
      url: "about:blank",
      profile_id: "default",
    });
    assert.equal(opened.url, "about:blank");
    assert.equal(context.engine.launches.length, 1);

    const closed = await context.client.sendRequest(browserCloseRequest, {
      session_id: opened.session_id,
    });
    assert.equal(closed.closed, 1);
  } finally {
    context.dispose();
  }
});

test("rejects invalid profile ids", async () => {
  const context = harness();
  try {
    await assert.rejects(
      context.client.sendRequest(browserOpenRequest, {
        url: "about:blank",
        profile_id: "../escape",
      }),
    );
  } finally {
    context.dispose();
  }
});

test("reports browser crashes distinctly", async () => {
  const context = harness();
  try {
    await context.client.sendRequest(browserOpenRequest, {
      url: "about:blank",
      profile_id: "default",
    });
    context.engine.lastClosed?.("crashed");
    assert.deepEqual(context.events.at(-1), {
      session_id: context.events[0]?.session_id,
      event: "crashed",
    });
  } finally {
    context.dispose();
  }
});
