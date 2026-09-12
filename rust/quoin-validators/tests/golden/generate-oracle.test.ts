// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//
// The one-time oracle capture for quoin#377.
//
// This runs the RETAINED TypeScript — `src/validators/index.ts` and the shipped
// `quoin validate` command — over `cases.json` and writes `expected.json`. The
// Rust suite then asserts against that file and never shells out to Node again,
// which is the whole point: a Rust test that asks TypeScript whether it passed
// is not a port.
//
// It is committed so the capture is reproducible and auditable, and it is NOT
// wired into any lane: it lives under `rust/`, which `vite.config.ts` does not
// collect, and running it requires deliberately copying it into `tests/`.
//
//   cp rust/quoin-validators/tests/golden/generate-oracle.test.ts tests/zz-oracle.test.ts
//   npx vitest run tests/zz-oracle.test.ts
//   rm tests/zz-oracle.test.ts
//
// Regenerating it is a decision, not a convenience: a golden that is cheap to
// refresh stops being a gate the first time the implementation changes.

import { mkdirSync, mkdtempSync, readFileSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

import type { Config } from "@oclif/core";
import { loadConfig } from "@agent-ix/ix-cli-core";
import { beforeAll, describe, expect, it, vi } from "vitest";

import Validate from "../src/commands/validate.js";
import { inspectEmptyGates } from "../src/validators/index.js";

const repoRoot = join(dirname(fileURLToPath(import.meta.url)), "..");
// Set QUOIN_GOLDEN_DIR when the capture is run from a checkout whose `rust/`
// tree is elsewhere (a worktree holding the port branch, for instance).
const goldenDir =
  process.env.QUOIN_GOLDEN_DIR ??
  join(repoRoot, "rust", "quoin-validators", "tests", "golden");

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
  it("captures verdicts from the retained TypeScript", async () => {
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

    writeFileSync(
      join(goldenDir, "expected.json"),
      `${JSON.stringify({ cases: expected }, null, 2)}\n`,
    );
    expect(expected.length).toBe(corpus.cases.length);
  });
});
