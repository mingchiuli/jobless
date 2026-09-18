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

import {
  browserCloseRequest,
  browserEventNotification,
  browserOpenRequest,
  runtimeHealthRequest,
  runtimeReadyNotification,
  runtimeShutdownRequest,
} from "./rpc.ts";
import type { BrowserEventParams } from "./generated/protocol.ts";
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
  connection.onRequest(browserOpenRequest, (params, token) =>
    service.open(params, token),
  );
  connection.onRequest(browserCloseRequest, (params) => service.close(params));
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

export function sendBrowserEvent(
  connection: MessageConnection,
  event: BrowserEventParams,
): void {
  connection.sendNotification(browserEventNotification, event);
}
