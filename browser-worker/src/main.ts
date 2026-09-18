import { resolve } from "node:path";

import { parseArgs } from "node:util";

import {
  createWorkerConnection,
  sendBrowserEvent,
  sendRuntimeReady,
  stderrLogger,
} from "./connection.ts";
import { PatchrightEngine } from "./patchright-engine.ts";
import { WorkerService } from "./worker-service.ts";

const { values } = parseArgs({
  args: process.argv.slice(2),
  options: {
    "data-dir": { type: "string" },
    engine: { type: "string", default: "patchright" },
    headless: { type: "string", default: "false" },
  },
  strict: true,
});

if (values.engine !== "patchright") {
  stderrLogger.error(`unsupported browser engine: ${values.engine}`);
  process.exit(2);
}

if (!values["data-dir"]) {
  stderrLogger.error("--data-dir is required");
  process.exit(2);
}

const dataDir = resolve(values["data-dir"]);
const engine = new PatchrightEngine();
let connection: ReturnType<typeof createWorkerConnection> | undefined;

const service = new WorkerService(engine, {
  profileRoot: resolve(dataDir, "browser-profiles"),
  headless: values.headless === "true",
  emitEvent(event) {
    if (connection) {
      sendBrowserEvent(connection, event);
    }
  },
});

connection = createWorkerConnection(process.stdin, process.stdout, service);
connection.onClose(() => {
  void service.dispose().finally(() => process.exit(0));
});

process.on("SIGINT", () => {
  void service.dispose().finally(() => process.exit(0));
});
process.on("SIGTERM", () => {
  void service.dispose().finally(() => process.exit(0));
});
process.on("uncaughtException", (error) => {
  stderrLogger.error(`uncaught exception: ${error.stack ?? error.message}`);
  process.exit(1);
});
process.on("unhandledRejection", (error) => {
  stderrLogger.error(`unhandled rejection: ${String(error)}`);
  process.exit(1);
});

connection.listen();
sendRuntimeReady(connection);
