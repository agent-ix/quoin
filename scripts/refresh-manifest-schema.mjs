#!/usr/bin/env node
/**
 * Refresh the vendored module-manifest schema from a filament-core-service
 * checkout (quoin FR-070; owner FR-035 there).
 *
 * Usage:
 *   node scripts/refresh-manifest-schema.mjs [--source <filament-core-service checkout>]
 *
 * The file is read from the git object named by
 * `SEMANTIC_CONTRACT.module_manifest_schema.source_revision` in
 * `rust/crates/quoin-semantic/src/contract.rs`, not from the working tree, so
 * the copied bytes are exactly reproducible. Bump the revision and the hash
 * there first; this script refuses to copy bytes whose hash does not match the
 * recorded one.
 *
 * The provenance moved from `src/semantic/contract.ts` at the quoin#452
 * cutover, which deleted that file. The vendored schema itself did not move:
 * it is DATA the npm package ships, and it stays in `src/semantic/schemas/`.
 */

import { execFileSync } from "node:child_process";
import { createHash } from "node:crypto";
import { readFileSync, writeFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const repo = resolve(here, "..");
const contractFile = join(
  repo,
  "rust",
  "crates",
  "quoin-semantic",
  "src",
  "contract.rs",
);
const target = join(
  repo,
  "src",
  "semantic",
  "schemas",
  "module-manifest.schema.json",
);

function arg(name, fallback) {
  const idx = process.argv.indexOf(name);
  return idx >= 0 ? process.argv[idx + 1] : fallback;
}

const source = resolve(
  arg("--source", join(repo, "..", "filament-core-service")),
);
const contract = readFileSync(contractFile, "utf8");
// `module_manifest_schema: VendoredSource { ... }` — the struct name sits
// between the field and its brace, and the fields are snake_case.
const block =
  /module_manifest_schema:\s*\w*\s*\{([\s\S]*?)\}/.exec(contract)?.[1] ?? "";
const revision = /source_revision:\s*"([0-9a-f]{40})"/.exec(block)?.[1];
const path = /source_path:\s*"([^"]+)"/.exec(block)?.[1];
const expected = /sha256:\s*"(sha256:[0-9a-f]{64})"/.exec(block)?.[1];
if (!revision || !path || !expected) {
  console.error(
    "could not read module_manifest_schema provenance from rust/crates/quoin-semantic/src/contract.rs",
  );
  process.exit(1);
}

const bytes = execFileSync("git", [
  "-C",
  source,
  "show",
  `${revision}:${path}`,
]);
const actual = `sha256:${createHash("sha256").update(bytes).digest("hex")}`;
if (actual !== expected) {
  console.error(
    `refusing to vendor: ${path}@${revision.slice(0, 12)} hashes to ${actual}, contract records ${expected}`,
  );
  process.exit(1);
}
writeFileSync(target, bytes);
console.log(`vendored ${path}@${revision.slice(0, 12)} → ${target}`);
