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
 * The source must be checked out at the tag `QUIRE_CONTRACT.sourceTag` names,
 * or this refuses: copying from an arbitrary working tree is how a contract
 * ends up pinned to something that was never released.
 */

import { execFileSync } from "node:child_process";
import { createHash } from "node:crypto";
import { copyFileSync, readFileSync, writeFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const repo = resolve(here, "..");
const contractFile = join(repo, "src", "quire", "contract.ts");
const vendorDir = join(repo, "src", "quire", "schemas");

const SCHEMAS = ["coverage-v1.schema.json", "properties-v1.schema.json"];

function arg(name, fallback) {
  const idx = process.argv.indexOf(name);
  return idx >= 0 ? process.argv[idx + 1] : fallback;
}

const source = resolve(arg("--source", join(repo, "..", "quire-rs")));
const contract = readFileSync(contractFile, "utf8");
const expectedTag = /sourceTag:\s*"([^"]+)"/.exec(contract)?.[1];
if (!expectedTag) {
  console.error("could not read `sourceTag` from src/quire/contract.ts");
  process.exit(1);
}

let describe;
try {
  describe = execFileSync(
    "git",
    ["-C", source, "describe", "--tags", "--exact-match"],
    {
      encoding: "utf8",
    },
  ).trim();
} catch {
  console.error(
    `${source} is not checked out at a tag. Check out ${expectedTag} (or pass ` +
      `--source) — copying from an arbitrary working tree pins the contract to ` +
      `something that was never released.`,
  );
  process.exit(1);
}

if (describe !== expectedTag) {
  console.error(
    `${source} is at ${describe} but contract.ts pins ${expectedTag}. Either ` +
      `check out ${expectedTag}, or update sourceTag first and re-run — the two ` +
      `must move together or the recorded provenance is a fiction.`,
  );
  process.exit(1);
}

const hashes = {};
for (const name of SCHEMAS) {
  const from = join(source, "schemas", "output", name);
  const to = join(vendorDir, name);
  copyFileSync(from, to);
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
