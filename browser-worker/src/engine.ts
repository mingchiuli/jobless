import type {
  BrowserInfo,
  PageEvent,
  PageSnapshot,
  PageState,
} from "./generated/protocol.ts";

export type PlatformId = string;

export interface BrowserLaunchOptions {
  headless: boolean;
}

export interface BrowserPageHandle {
  readonly pageId: string;
  readonly platformId: PlatformId;
  readonly state: PageState;
  url(): string;
  isClosed(): boolean;
  activate(): Promise<void>;
  navigate(url: string): Promise<string>;
  close(): Promise<void>;
  snapshot(): PageSnapshot;
}

export interface EnsurePageResult {
  page: BrowserPageHandle;
  created: boolean;
}

export type BrowserSessionEnd = "closed" | "crashed";

export interface BrowserPageEvent {
  pageId: string;
  platformId: PlatformId;
  url: string;
  event: PageEvent;
}

export interface BrowserSessionHandle {
  ensurePage(platformId: PlatformId, url: string): Promise<EnsurePageResult>;
  navigatePage(platformId: PlatformId, url: string): Promise<BrowserPageHandle>;
  activatePage(platformId: PlatformId): Promise<boolean>;
  closePage(platformId: PlatformId): Promise<boolean>;
  listPages(): PageSnapshot[];
  close(): Promise<void>;
  onClosed(listener: (reason: BrowserSessionEnd) => void): void;
  onPageEvent(listener: (params: BrowserPageEvent) => void): void;
}

export interface BrowserEngine {
  readonly name: string;
  readonly version: string;
  getBrowserInfo(): Promise<BrowserInfo>;
  launchPersistent(
    profileDir: string,
    options: BrowserLaunchOptions,
  ): Promise<BrowserSessionHandle>;
  dispose(): Promise<void>;
}
