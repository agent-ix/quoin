#!/usr/bin/env node
/**
 * Copy the vendored quire output schemas into `dist/` (FR-030).
 *
 * `src/quire/contract.ts` resolves them relative to its own module directory,
 * which is `src/quire/` when tests import the sources and `dist/` once the
 * bundler has flattened the chunks. The bundler moves code, not data, so the
 * JSON has to be placed beside the built output or every runtime read fails
 * with ENOENT — which is exactly how this was found.
 */

import { copyFileSync, mkdirSync, readdirSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const repo = dirname(dirname(fileURLToPath(import.meta.url)));
const from = join(repo, "src", "quire", "schemas");
const measurementFrom = join(repo, "src", "measurement", "schemas");
const to = join(repo, "dist", "schemas");

mkdirSync(to, { recursive: true });
for (const name of readdirSync(from)) {
  copyFileSync(join(from, name), join(to, name));
  console.log(`dist/schemas/${name}`);
}
for (const name of readdirSync(measurementFrom)) {
  copyFileSync(join(measurementFrom, name), join(to, name));
  console.log(`dist/schemas/${name}`);
}
