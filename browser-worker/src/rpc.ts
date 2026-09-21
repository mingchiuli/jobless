import {
  NotificationType,
  RequestType,
} from "vscode-jsonrpc";

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
  RpcErrorPayload,
  RuntimeHealthResult,
  RuntimeShutdownResult,
  SessionCloseParams,
  SessionCloseResult,
  SessionEventParams,
  SessionStartParams,
  SessionStartResult,
} from "./generated/protocol.ts";

export type EmptyParams = Record<string, never>;

export const runtimeReadyNotification = new NotificationType<EmptyParams>(
  "runtime.ready",
);
export const sessionEventNotification = new NotificationType<SessionEventParams>(
  "browser.session.event",
);
export const pageEventNotification = new NotificationType<PageEventParams>(
  "browser.page.event",
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

export const sessionStartRequest = new RequestType<
  SessionStartParams,
  SessionStartResult,
  RpcErrorPayload
>("browser.session.start");

export const sessionCloseRequest = new RequestType<
  SessionCloseParams,
  SessionCloseResult,
  RpcErrorPayload
>("browser.session.close");

export const pageEnsureRequest = new RequestType<
  PageEnsureParams,
  PageEnsureResult,
  RpcErrorPayload
>("browser.page.ensure");

export const pageNavigateRequest = new RequestType<
  PageNavigateParams,
  PageNavigateResult,
  RpcErrorPayload
>("browser.page.navigate");

export const pageActivateRequest = new RequestType<
  PageActivateParams,
  PageActivateResult,
  RpcErrorPayload
>("browser.page.activate");

export const pageCloseRequest = new RequestType<
  PageCloseParams,
  PageCloseResult,
  RpcErrorPayload
>("browser.page.close");

export const pageListRequest = new RequestType<
  PageListParams,
  PageListResult,
  RpcErrorPayload
>("browser.page.list");
