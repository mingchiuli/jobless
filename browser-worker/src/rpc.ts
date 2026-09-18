import {
  NotificationType,
  RequestType,
} from "vscode-jsonrpc";

import type {
  BrowserCloseParams,
  BrowserCloseResult,
  BrowserEventParams,
  BrowserOpenParams,
  BrowserOpenResult,
  RpcErrorPayload,
  RuntimeHealthResult,
  RuntimeShutdownResult,
} from "./generated/protocol.ts";

export type EmptyParams = Record<string, never>;

export const runtimeReadyNotification = new NotificationType<EmptyParams>(
  "runtime.ready",
);
export const browserEventNotification = new NotificationType<BrowserEventParams>(
  "browser.event",
);

export const runtimeHealthRequest = new RequestType<
  EmptyParams,
  RuntimeHealthResult,
  RpcErrorPayload
>("runtime.health");

export const runtimeShutdownRequest = new RequestType<
  EmptyParams,
  RuntimeShutdownResult,
  RpcErrorPayload
>("runtime.shutdown");

export const browserOpenRequest = new RequestType<
  BrowserOpenParams,
  BrowserOpenResult,
  RpcErrorPayload
>("browser.open");

export const browserCloseRequest = new RequestType<
  BrowserCloseParams,
  BrowserCloseResult,
  RpcErrorPayload
>("browser.close");
