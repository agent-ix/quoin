#!/usr/bin/env node
/**
 * Refresh the vendored quire output schemas from a quire-rs checkout.
 *
 * The schemas are published by quire-rs (FR-055) and consumed here (FR-029).
 * There is no dependency edge between a Rust crate and an npm package along
 * which a file could travel, so they are vendored — and a vendored file drifts
 * unless refreshing it is a deliberate, reviewable act with a recorded hash.
 *
 * Usage:
 *   node scripts/refresh-quire-schemas.mjs [--source <quire-rs checkout>]
 *
 * The source must contain `QUIRE_CONTRACT.sourceRevision`. Files are read from
 * that git object, not from the working tree, so the copied bytes are exactly
 * reproducible while tracking-branch work is in progress.
 */

import { execFileSync } from "node:child_process";
import { createHash } from "node:crypto";
import { readFileSync, writeFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const repo = resolve(here, "..");
const contractFile = join(repo, "src", "quire", "contract.ts");
const vendorDir = join(repo, "src", "quire", "schemas");

const SCHEMAS = [
  "assurance-v1.schema.json",
  "coverage-v1.schema.json",
  "properties-v1.schema.json",
];

function arg(name, fallback) {
  const idx = process.argv.indexOf(name);
  return idx >= 0 ? process.argv[idx + 1] : fallback;
}

const source = resolve(arg("--source", join(repo, "..", "quire-rs")));
const contract = readFileSync(contractFile, "utf8");
const expectedRevision = /sourceRevision:\s*"([0-9a-f]{40})"/.exec(
  contract,
)?.[1];
if (!expectedRevision) {
  console.error("could not read `sourceRevision` from src/quire/contract.ts");
  process.exit(1);
}

let resolvedRevision;
try {
  resolvedRevision = execFileSync(
    "git",
    ["-C", source, "rev-parse", `${expectedRevision}^{commit}`],
    {
      encoding: "utf8",
    },
  ).trim();
} catch (cause) {
  console.error(
    `${source} does not contain pinned quire-rs revision ${expectedRevision}. ` +
      `Fetch it or pass --source with a checkout that contains it. ` +
      `(${cause instanceof Error ? cause.message : String(cause)})`,
  );
  process.exit(1);
}

if (resolvedRevision !== expectedRevision) {
  console.error(
    `${source} resolved ${expectedRevision} to ${resolvedRevision}; refusing ` +
      `to record ambiguous provenance.`,
  );
  process.exit(1);
}

const hashes = {};
for (const name of SCHEMAS) {
  const to = join(vendorDir, name);
  const bytes = execFileSync("git", [
    "-C",
    source,
    "show",
    `${expectedRevision}:schemas/output/${name}`,
  ]);
  writeFileSync(to, bytes);
  hashes[name] = createHash("sha256").update(readFileSync(to)).digest("hex");
  console.log(`copied ${name}  ${hashes[name].slice(0, 12)}…`);
}

let updated = contract;
for (const [name, hash] of Object.entries(hashes)) {
  const pattern = new RegExp(`("${name}":\\s*\\n?\\s*")[0-9a-f]{64}(")`);
  updated = updated.replace(pattern, `$1${hash}$2`);
}
writeFileSync(contractFile, updated);
console.log(`updated ${contractFile} with the refreshed hashes`);
