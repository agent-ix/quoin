#!/usr/bin/env node
/**
 * Copy runtime JSON Schema assets into `dist/` (FR-030, FR-063..FR-065).
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
const sources = [
  join(repo, "src", "quire", "schemas"),
  join(repo, "src", "change-assurance", "schemas"),
  join(repo, "src", "measurement", "schemas"),
];
const to = join(repo, "dist", "schemas");

mkdirSync(to, { recursive: true });
for (const from of sources) {
  for (const name of readdirSync(from)) {
    copyFileSync(join(from, name), join(to, name));
    console.log(`dist/schemas/${name}`);
  }
}
