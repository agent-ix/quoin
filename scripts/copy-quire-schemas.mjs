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

import { copyFileSync, cpSync, mkdirSync, readdirSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const repo = dirname(dirname(fileURLToPath(import.meta.url)));
const sources = [
  join(repo, "src", "quire", "schemas"),
  join(repo, "src", "store", "schemas"),
];
const to = join(repo, "dist", "schemas");

mkdirSync(to, { recursive: true });
for (const from of sources) {
  for (const name of readdirSync(from)) {
    copyFileSync(join(from, name), join(to, name));
    console.log(`dist/schemas/${name}`);
  }
}

// The vendored semantic contract, which `src/semantic/contract.ts` resolves the
// same way off `SEMANTIC_ROOT` — its own module directory, so `src/semantic/` in
// a source run and `dist/` once the chunks are flattened. Copied recursively
// because it has subdirectories (`semantic-core/`, `filament-core-data/`) rather
// than a flat file list.
//
// Two paths and not one: the schema tree lands under `dist/schemas/`, and
// `sweep-report.schema.json` sits at the root of the contract beside it. Since
// quoin#446 the semantic gate runs inside `quoin-core`, which is handed
// `SEMANTIC_ROOT` as `QUOIN_SEMANTIC_ROOT` and reads both paths from there, so a
// missing copy is no longer a latent gap — it refuses every install.
cpSync(join(repo, "src", "semantic", "schemas"), to, { recursive: true });
console.log("dist/schemas/ (vendored semantic contract)");
copyFileSync(
  join(repo, "src", "semantic", "sweep-report.schema.json"),
  join(repo, "dist", "sweep-report.schema.json"),
);
console.log("dist/sweep-report.schema.json");
