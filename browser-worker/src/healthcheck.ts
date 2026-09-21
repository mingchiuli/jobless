import { randomUUID } from "node:crypto";
import { rmSync } from "node:fs";
import { join } from "node:path";
import { tmpdir } from "node:os";

import { PatchrightEngine } from "./patchright-engine.ts";

const profileDir = join(tmpdir(), `jobless-healthcheck-${randomUUID()}`);
const engine = new PatchrightEngine();

try {
  const browser = await engine.getBrowserInfo();
  if (!browser.available) {
    throw new Error("bundled Patchright browser is unavailable");
  }

  const session = await engine.launchPersistent(profileDir, {
    headless: true,
  });
  await session.ensurePage("default", "about:blank");
  await session.ensurePage("healthcheck", "about:blank");
  const pageCount = session.listPages().length;
  if (pageCount < 2) {
    throw new Error(`expected multiple browser pages, found ${pageCount}`);
  }
  await session.activatePage("healthcheck");
  await session.closePage("healthcheck");
  await session.close();
  process.stdout.write(
    `${JSON.stringify({
      runtime: `bun@${Bun.version}`,
      engine: `${engine.name}@${engine.version}`,
      browser: browser.version,
      revision: browser.revision,
      page_count: pageCount,
    })}\n`,
  );
} finally {
  await engine.dispose();
  rmSync(profileDir, { recursive: true, force: true });
}
