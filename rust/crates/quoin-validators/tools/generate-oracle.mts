// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//
// The oracle capture for quoin#377.
//
// This runs the RETAINED TypeScript — `src/validators/index.ts` and the shipped
// `quoin validate` command — over `tests/golden/cases.json` and reproduces
// `tests/golden/expected.json`. The Rust suite then asserts against that file
// and never shells out to Node again, which is the whole point: a Rust test
// that asks TypeScript whether it passed is not a port.
//
// It is a CAPTURE SCRIPT, not a test, and its name says so. An earlier revision
// of this file was called `generate-oracle.test.ts` and rewrote `expected.json`
// unconditionally on every run — a golden that regenerates itself under the
// test runner is a test that cannot fail. It also claimed in this header that
// `vite.config.ts` did not collect `rust/`, which was false: the exclude list
// held no `rust/**` entry, vitest collected this file, and the unresolvable
// relative import broke the whole TypeScript lane at collection time. Both
// halves are fixed — the exclude entry now exists AND the file is off the
// `*.test.ts` suffix, so neither one alone is load-bearing.
//
// Verify that the committed golden still reproduces (writes nothing):
//
//   pnpm vitest run \
//     --include rust/crates/quoin-validators/tools/generate-oracle.mts \
//     --exclude 'node_modules/**'
//
// Regenerate it, deliberately:
//
//   QUOIN_ORACLE_WRITE=1 pnpm vitest run \
//     --include rust/crates/quoin-validators/tools/generate-oracle.mts \
//     --exclude 'node_modules/**'
//
// Regenerating is a decision, not a convenience: a golden that is cheap to
// refresh stops being a gate the first time the implementation changes. Say in
// the commit message what behaviour changed, and refresh the hashes in
// `tests/golden/PROVENANCE.md`.

import { mkdirSync, mkdtempSync, readFileSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

import type { Config } from "@oclif/core";
import { loadConfig } from "@agent-ix/ix-cli-core";
import { beforeAll, describe, expect, it, vi } from "vitest";

import Validate from "../../../../src/commands/validate.js";
import { inspectEmptyGates } from "../../../../src/validators/index.js";

const here = dirname(fileURLToPath(import.meta.url));
// tools -> quoin-validators -> crates -> rust -> repository root.
const repoRoot = join(here, "..", "..", "..", "..");
const goldenDir = join(here, "..", "tests", "golden");
const expectedPath = join(goldenDir, "expected.json");

// Writing is opt-in. Without it this script is a reproduction check: it proves
// the committed bytes are still what the retained TypeScript produces.
const write = process.env.QUOIN_ORACLE_WRITE === "1";

interface Case {
  name: string;
  covers: string;
  files: Record<string, string[]>;
}

let config: Config;

beforeAll(async () => {
  config = await loadConfig({ root: repoRoot });
});

function materialise(spec: Case): string {
  const root = mkdtempSync(join(tmpdir(), "quoin-golden-"));
  for (const [relative, lines] of Object.entries(spec.files)) {
    const target = join(root, relative);
    mkdirSync(dirname(target), { recursive: true });
    writeFileSync(target, lines.join("\n"));
  }
  return root;
}

async function capture(argv: string[]): Promise<string[]> {
  const lines: string[] = [];
  const spy = vi.spyOn(console, "log").mockImplementation((line) => {
    lines.push(String(line));
  });
  try {
    await Validate.run(argv, config);
  } finally {
    spy.mockRestore();
  }
  return lines;
}

async function strictExit(root: string): Promise<number> {
  const spy = vi.spyOn(console, "log").mockImplementation(() => {});
  try {
    await Validate.run(["--repo", root, "--strict"], config);
    return 0;
  } catch (error) {
    const oclif = (error as { oclif?: { exit?: number } }).oclif;
    return oclif?.exit ?? -1;
  } finally {
    spy.mockRestore();
  }
}

describe("quoin#377 golden oracle capture", () => {
  it("reproduces the verdicts recorded in expected.json", async () => {
    const corpus = JSON.parse(
      readFileSync(join(goldenDir, "cases.json"), "utf8"),
    ) as { cases: Case[] };

    const expected: unknown[] = [];
    for (const spec of corpus.cases) {
      const root = materialise(spec);
      const findings = inspectEmptyGates(root);
      const json = (await capture(["--repo", root, "--json"])).join("\n");
      const human = await capture(["--repo", root]);
      const strict = await strictExit(root);

      // The library result and the command payload must agree, or the golden
      // records two different oracles under one name.
      expect(JSON.parse(json)).toEqual({ findings });

      expected.push({
        name: spec.name,
        findings,
        json,
        human,
        strictExit: strict,
      });
    }

    expect(expected.length).toBe(corpus.cases.length);
    const serialised = `${JSON.stringify({ cases: expected }, null, 2)}\n`;

    if (write) {
      writeFileSync(expectedPath, serialised);
      return;
    }

    // Byte comparison, not a structural one: the Rust suite asserts against
    // these exact bytes, so "equivalent JSON" is not the property under test.
    expect(serialised).toBe(readFileSync(expectedPath, "utf8"));
  });
});
