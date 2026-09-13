// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//
// The adapter-oracle capture for quoin#456.
//
// This runs the RETAINED TypeScript — `src/evidence/adapters/` — over
// `tests/golden/cases.json` and reproduces `tests/golden/expected.json`. The
// Rust suite then asserts against that file and never shells out to Node
// again, which is the whole point: a Rust test that asks TypeScript whether it
// passed is not a port (FR-101-AC-5).
//
// It is a CAPTURE SCRIPT, not a test, and its name says so — it does not end
// in `*.test.ts`, so `pnpm test` cannot collect it even if the `rust/**`
// exclude in `vite.config.ts` were dropped. Both halves are load-bearing on
// their own.
//
// Verify that the committed golden still reproduces (writes nothing), from the
// repository root:
//
//   pnpm vitest run \
//     --config rust/crates/quoin-evidence/tools/vitest.oracle.config.mts
//
// Regenerate it, deliberately:
//
//   QUOIN_ORACLE_WRITE=1 pnpm vitest run \
//     --config rust/crates/quoin-evidence/tools/vitest.oracle.config.mts
//
// Regenerating is a decision, not a convenience: a golden that is cheap to
// refresh stops being a gate the first time the implementation changes. Say in
// the commit message what behaviour changed, and refresh the hashes in
// `tests/golden/PROVENANCE.md`.
//
// The oracle is `src/evidence/adapters/` as of the revision named in
// PROVENANCE.md. quoin#458 retires it; after that lands, restore it from that
// revision before capturing, exactly as quoin-validators' capture does.

import { readFileSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

import { describe, expect, it } from "vitest";

import {
  ADAPTERS,
  FINDING_ADAPTERS,
} from "../../../../src/evidence/adapters/registry.js";

const here = dirname(fileURLToPath(import.meta.url));
// tools -> quoin-evidence -> crates -> rust -> repository root.
const repoRoot = join(here, "..", "..", "..", "..");
const goldenDir = join(here, "..", "tests", "golden");
const expectedPath = join(goldenDir, "expected.json");

// Writing is opt-in. Without it this script is a reproduction check: it proves
// the committed bytes are still what the retained TypeScript produces.
const write = process.env.QUOIN_ORACLE_WRITE === "1";

interface Case {
  name: string;
  adapter: string;
  shape: "run" | "finding";
  inline?: string;
  file?: string;
}

function inputOf(spec: Case): string {
  if (spec.inline !== undefined) return spec.inline;
  if (spec.file === undefined) {
    throw new Error(`case ${spec.name} names neither inline nor file`);
  }
  return readFileSync(join(repoRoot, spec.file), "utf8");
}

function parserFor(spec: Case): (raw: string) => unknown {
  const pool = spec.shape === "run" ? ADAPTERS : FINDING_ADAPTERS;
  const adapter = pool.find((a) => a.name === spec.adapter);
  if (adapter === undefined) {
    throw new Error(`case ${spec.name} names unknown adapter ${spec.adapter}`);
  }
  return (raw) => adapter.parse(raw);
}

describe("quoin#456 adapter golden oracle capture", () => {
  it("reproduces the parses recorded in expected.json", () => {
    const corpus = JSON.parse(
      readFileSync(join(goldenDir, "cases.json"), "utf8"),
    ) as { cases: Case[] };

    const expected: unknown[] = [];
    let accepted = 0;
    let refused = 0;
    for (const spec of corpus.cases) {
      const parse = parserFor(spec);
      try {
        const result = parse(inputOf(spec));
        accepted += 1;
        expected.push({
          name: spec.name,
          adapter: spec.adapter,
          shape: spec.shape,
          ok: result,
        });
      } catch (error) {
        refused += 1;
        expected.push({
          name: spec.name,
          adapter: spec.adapter,
          shape: spec.shape,
          err: (error as Error).message,
        });
      }
    }

    // Anti-vacuity: a corpus that only accepts, or only refuses, carries no
    // refusal criterion at all. Both populations must be non-empty.
    expect(accepted).toBeGreaterThan(0);
    expect(refused).toBeGreaterThan(0);
    expect(expected.length).toBe(corpus.cases.length);

    const serialised = `${JSON.stringify({ cases: expected }, null, 2)}\n`;

    if (write) {
      writeFileSync(expectedPath, serialised);
      return;
    }

    // Byte comparison, not a structural one: the Rust suite asserts against
    // these exact bytes, so "equivalent JSON" is not the property under test.
    expect(readFileSync(expectedPath, "utf8")).toBe(serialised);
  });
});
