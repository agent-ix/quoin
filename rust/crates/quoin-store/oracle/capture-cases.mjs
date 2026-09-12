// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//
// Capture the TypeScript oracle's answer for every adversarial case, once.
//
//   node --loader ts-node/esm oracle/capture-cases.mjs
//
// Writes `tests/fixtures/jcs-oracle.json`, which the Rust suite asserts
// against. The fixture is checked in so no TypeScript runs as a test oracle
// after cutover (EPIC #373 AC-5); this script exists to regenerate it when the
// case list grows, not to run in CI.
//
// `QUOIN_SRC_ROOT` overrides where the TypeScript is read from. It must be a
// checkout with `node_modules` installed, which a bare git worktree is not.

import { mkdirSync, writeFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";

import { cases } from "./cases.mjs";

const here = dirname(fileURLToPath(import.meta.url));
const repoRoot = resolve(process.env.QUOIN_SRC_ROOT ?? join(here, "..", "..", ".."));
const load = (relative) =>
  import(pathToFileURL(join(repoRoot, "src", relative)).href);

const { blake3Hex, canonicalizeJcs, canonicalBytes, digestValue, parseStrictJson } =
  await load("change-assurance/integrity.js");
const { canonicalJson } = await load("evidence/store.js");

// `blake3Hex` is the oracle's own single digest function. Hashing through it
// rather than through `@noble/hashes` directly keeps the TypeScript side of
// this comparison honest: it is the same call the store makes.
const encoder = new TextEncoder();
const hashText = (text) => blake3Hex(encoder.encode(text));

const captured = cases.map((entry) => {
  const record = {
    id: entry.id,
    expect: entry.expect,
    probes: entry.probes,
    input: entry.input,
  };
  // A measured, deliberate difference between the two implementations, declared
  // on the case so the Rust suite asserts it instead of failing on it.
  if (entry.divergence) record.divergence = entry.divergence;
  let value;
  try {
    value = parseStrictJson(entry.input);
  } catch (error) {
    record.accepted = false;
    record.error = error instanceof Error ? error.message : String(error);
    return record;
  }
  record.accepted = true;
  record.jcs = canonicalizeJcs(value);
  record.pretty = canonicalJson(value);
  record.digest = blake3Hex(canonicalBytes(value));
  record.jcs_digest = hashText(record.jcs);
  record.pretty_digest = hashText(record.pretty);
  if (value !== null && typeof value === "object" && !Array.isArray(value)) {
    record.record_digest = digestValue(value);
  }
  return record;
});

const out = join(here, "..", "tests", "fixtures", "jcs-oracle.json");
mkdirSync(dirname(out), { recursive: true });
writeFileSync(
  out,
  `${JSON.stringify({ source: "quoin TypeScript oracle: src/change-assurance/integrity.ts, src/evidence/store.ts", cases: captured }, null, 2)}\n`,
  "utf8",
);

const refused = captured.filter((entry) => !entry.accepted).length;
process.stderr.write(
  `captured ${captured.length} cases (${refused} refused by the oracle) -> ${out}\n`,
);
for (const entry of captured) {
  const actual = entry.accepted ? "accept" : "refuse";
  if (actual !== entry.expect) {
    process.stderr.write(
      `  AUTHORED INTENT DISAGREES: ${entry.id} authored ${entry.expect}, oracle ${actual}${entry.error ? ` (${entry.error})` : ""}\n`,
    );
  }
}
