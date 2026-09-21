import type { Readable, Writable } from "node:stream";

import {
  createMessageConnection,
  type Logger,
  type MessageConnection,
} from "vscode-jsonrpc";
import {
  StreamMessageReader,
  StreamMessageWriter,
} from "vscode-jsonrpc/node";

import type {
  PageEventParams,
  SessionEventParams,
} from "./generated/protocol.ts";
import {
  pageActivateRequest,
  pageCloseRequest,
  pageEnsureRequest,
  pageEventNotification,
  pageListRequest,
  pageNavigateRequest,
  runtimeHealthRequest,
  runtimeReadyNotification,
  runtimeShutdownRequest,
  sessionCloseRequest,
  sessionEventNotification,
  sessionStartRequest,
} from "./rpc.ts";
import type { WorkerService } from "./worker-service.ts";

export const stderrLogger: Logger = {
  error(message: string) {
    process.stderr.write(`${message}\n`);
  },
  warn(message: string) {
    process.stderr.write(`${message}\n`);
  },
  info(message: string) {
    process.stderr.write(`${message}\n`);
  },
  log(message: string) {
    process.stderr.write(`${message}\n`);
  },
};

export function createWorkerConnection(
  input: Readable,
  output: Writable,
  service: WorkerService,
  logger: Logger = stderrLogger,
): MessageConnection {
  const connection = createMessageConnection(
    new StreamMessageReader(input),
    new StreamMessageWriter(output),
    logger,
  );

  connection.onRequest(runtimeHealthRequest, () => service.health());
  connection.onRequest(sessionStartRequest, (params, token) =>
    service.startSession(params, token),
  );
  connection.onRequest(sessionCloseRequest, (params) =>
    service.closeSession(params),
  );
  connection.onRequest(pageEnsureRequest, (params) => service.ensurePage(params));
  connection.onRequest(pageNavigateRequest, (params) =>
    service.navigatePage(params),
  );
  connection.onRequest(pageActivateRequest, (params) =>
    service.activatePage(params),
  );
  connection.onRequest(pageCloseRequest, (params) => service.closePage(params));
  connection.onRequest(pageListRequest, (params) => service.listPages(params));
  connection.onRequest(runtimeShutdownRequest, async () => {
    const result = await service.shutdown();
    setImmediate(() => {
      connection.dispose();
      process.exit(0);
    });
    return result;
  });

  return connection;
}

export function sendRuntimeReady(connection: MessageConnection): void {
  connection.sendNotification(runtimeReadyNotification, {});
}

export function sendSessionEvent(
  connection: MessageConnection,
  event: SessionEventParams,
): void {
  connection.sendNotification(sessionEventNotification, event);
}

export function sendPageEvent(
  connection: MessageConnection,
  event: PageEventParams,
): void {
  connection.sendNotification(pageEventNotification, event);
}
