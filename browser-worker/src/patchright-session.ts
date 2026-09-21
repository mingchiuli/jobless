import { randomUUID } from "node:crypto";

import type { BrowserContext, Page } from "patchright";

import type {
  BrowserPageEvent,
  BrowserSessionEnd,
  BrowserSessionHandle,
  EnsurePageResult,
  PlatformId,
} from "./engine.ts";
import type { PageSnapshot } from "./generated/protocol.ts";
import { PatchrightPage } from "./patchright-page.ts";
import { validatePlatformId, validateUrl } from "./validation.ts";

export class PatchrightSession implements BrowserSessionHandle {
  private readonly pages = new Map<PlatformId, PatchrightPage>();
  private readonly pending = new Map<PlatformId, Promise<EnsurePageResult>>();
  private readonly closedListeners = new Set<
    (reason: BrowserSessionEnd) => void
  >();
  private readonly pageListeners = new Set<
    (params: BrowserPageEvent) => void
  >();
  private initialPage: Page | null;
  private ended = false;
  private crashed = false;

  constructor(private readonly context: BrowserContext) {
    this.initialPage = context.pages()[0] ?? null;
    context.on("close", () => {
      this.finishSession(this.crashed ? "crashed" : "closed");
    });
  }

  async ensurePage(
    platformId: PlatformId,
    url: string,
  ): Promise<EnsurePageResult> {
    validatePlatformId(platformId);
    validateUrl(url);
    this.assertOpen();

    const existing = this.pages.get(platformId);
    if (existing && !existing.isClosed()) {
      return { page: existing, created: false };
    }

    const inFlight = this.pending.get(platformId);
    if (inFlight) {
      return inFlight;
    }

    const promise = this.createPage(platformId, url);
    this.pending.set(platformId, promise);
    try {
      return await promise;
    } finally {
      this.pending.delete(platformId);
    }
  }

  async navigatePage(
    platformId: PlatformId,
    url: string,
  ): Promise<PatchrightPage> {
    validateUrl(url);
    const page = this.requirePage(platformId);
    await page.navigate(url);
    return page;
  }

  async activatePage(platformId: PlatformId): Promise<boolean> {
    const page = this.pages.get(platformId);
    if (!page || page.isClosed()) {
      return false;
    }
    await page.activate();
    return true;
  }

  async closePage(platformId: PlatformId): Promise<boolean> {
    const page = this.pages.get(platformId);
    if (!page) {
      return false;
    }
    this.pages.delete(platformId);
    await page.close();
    return true;
  }

  listPages(): PageSnapshot[] {
    return [...this.pages.values()]
      .filter((page) => !page.isClosed())
      .map((page) => page.snapshot());
  }

  async close(): Promise<void> {
    if (this.ended) {
      return;
    }
    await this.context.close();
    this.finishSession("closed");
  }

  onClosed(listener: (reason: BrowserSessionEnd) => void): void {
    this.closedListeners.add(listener);
  }

  onPageEvent(listener: (params: BrowserPageEvent) => void): void {
    this.pageListeners.add(listener);
  }

  private async createPage(
    platformId: PlatformId,
    url: string,
  ): Promise<EnsurePageResult> {
    const nativePage =
      this.initialPage && !this.initialPage.isClosed()
        ? this.initialPage
        : await this.context.newPage();
    this.initialPage = null;

    const page = new PatchrightPage(randomUUID(), platformId, nativePage);
    this.pages.set(platformId, page);
    this.attachPageEvents(nativePage, page);
    this.emitPageEvent(page, "opened");

    if (url !== "about:blank") {
      await page.navigate(url);
    }
    return { page, created: true };
  }

  private attachPageEvents(nativePage: Page, page: PatchrightPage): void {
    nativePage.on("framenavigated", (frame) => {
      if (frame === nativePage.mainFrame()) {
        this.emitPageEvent(page, "navigated");
      }
    });
    nativePage.on("crash", () => {
      page.state = "crashed";
      this.finishPage(page, "crashed");
      void nativePage.close().catch(() => {});
    });
    nativePage.on("close", () => {
      if (page.state !== "crashed") {
        this.finishPage(page, "closed");
      }
    });
  }

  private finishPage(
    page: PatchrightPage,
    event: "closed" | "crashed",
  ): void {
    const current = this.pages.get(page.platformId);
    if (current?.pageId !== page.pageId) {
      return;
    }
    this.pages.delete(page.platformId);
    this.emitPageEvent(page, event);
  }

  private emitPageEvent(
    page: PatchrightPage,
    event: BrowserPageEvent["event"],
  ): void {
    const params: BrowserPageEvent = {
      pageId: page.pageId,
      platformId: page.platformId,
      url: page.url(),
      event,
    };
    for (const listener of this.pageListeners) {
      listener(params);
    }
  }

  private finishSession(reason: BrowserSessionEnd): void {
    if (this.ended) {
      return;
    }
    this.ended = true;
    this.pages.clear();
    this.pending.clear();
    for (const listener of this.closedListeners) {
      listener(reason);
    }
  }

  private requirePage(platformId: PlatformId): PatchrightPage {
    const page = this.pages.get(platformId);
    if (!page || page.isClosed()) {
      throw new Error(`browser page was not found for platform: ${platformId}`);
    }
    return page;
  }

  private assertOpen(): void {
    if (this.ended) {
      throw new Error("browser session is closed");
    }
  }
}
