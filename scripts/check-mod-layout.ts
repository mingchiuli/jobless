#!/usr/bin/env bun

import { readdirSync } from "node:fs";
import { join, relative, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const scriptDir = fileURLToPath(new URL(".", import.meta.url));
const root = resolve(scriptDir, "..");
const cratesDir = join(root, "crates");
const forbidden: string[] = [];

walk(cratesDir);

if (forbidden.length > 0) {
  process.stderr.write(
    [
      "Rust modules must not use mod.rs.",
      "Use a sibling module.rs plus module/ directory instead.",
      "",
      ...forbidden.map((path) => `- ${relative(root, path)}`),
      "",
    ].join("\n"),
  );
  process.exit(1);
}

process.stdout.write("Rust module layout check passed.\n");

function walk(directory: string) {
  for (const entry of readdirSync(directory, { withFileTypes: true })) {
    const path = join(directory, entry.name);
    if (entry.isDirectory()) {
      walk(path);
    } else if (entry.isFile() && entry.name === "mod.rs") {
      forbidden.push(path);
    }
  }
}
