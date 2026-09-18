import type { BrowserInfo } from "./generated/protocol.ts";

export interface BrowserLaunchOptions {
  headless: boolean;
  url: string;
}

export interface BrowserSessionHandle {
  readonly url: string;
  close(): Promise<void>;
  onClosed(listener: (reason: BrowserSessionEnd) => void): void;
}

export type BrowserSessionEnd = "closed" | "crashed";

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
