#!/usr/bin/env node
/**
 * Copy runtime JSON schemas into `dist/` (FR-029, FR-043).
 *
 * The quire contract and measurement validator resolve them relative to their
 * own module directory, which is below `src/` when tests import sources and
 * `dist/` once the bundler has flattened chunks. The bundler moves code, not
 * data, so JSON has to be placed beside built output or runtime reads fail.
 */

import { copyFileSync, mkdirSync, readdirSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const repo = dirname(dirname(fileURLToPath(import.meta.url)));
const from = join(repo, "src", "quire", "schemas");
const to = join(repo, "dist", "schemas");

mkdirSync(to, { recursive: true });
for (const name of readdirSync(from)) {
  copyFileSync(join(from, name), join(to, name));
  console.log(`dist/schemas/${name}`);
}

const evidenceSchemas = join(repo, "src", "evidence", "schemas");
for (const name of readdirSync(evidenceSchemas)) {
  copyFileSync(join(evidenceSchemas, name), join(to, name));
  console.log(`dist/schemas/${name}`);
}
