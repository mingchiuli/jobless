import type { Page } from "patchright";

import type { BrowserPageHandle, PlatformId } from "./engine.ts";
import type {
  PageSnapshot,
  PageState,
} from "./generated/protocol.ts";

export class PatchrightPage implements BrowserPageHandle {
  state: PageState = "open";

  constructor(
    readonly pageId: string,
    readonly platformId: PlatformId,
    private readonly page: Page,
  ) {}

  url(): string {
    return this.page.url();
  }

  isClosed(): boolean {
    return this.page.isClosed();
  }

  async activate(): Promise<void> {
    await this.page.bringToFront();
  }

  async navigate(url: string): Promise<string> {
    this.state = "navigating";
    await this.page.goto(url, {
      waitUntil: "domcontentloaded",
      timeout: 30_000,
    });
    this.state = "open";
    return this.page.url() || url;
  }

  async close(): Promise<void> {
    if (!this.page.isClosed()) {
      await this.page.close();
    }
  }

  snapshot(): PageSnapshot {
    return {
      page_id: this.pageId,
      platform_id: this.platformId,
      url: this.url(),
      state: this.state,
    };
  }
}
