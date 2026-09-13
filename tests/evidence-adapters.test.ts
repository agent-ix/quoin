/**
 * FR-033 — the format adapters as `quoin evidence record` reaches them.
 *
 * Every criterion here is stated over the command, not over a parse function.
 * The P1 review found three of four P0 gaps were a Test Matrix reading ✅ over
 * a capability nothing could reach, because the criteria were written at the
 * function boundary — and that is exactly the reach these four assert, now
 * that the adapters themselves are `quoin-evidence`'s (quoin#458).
 */

import {
  mkdirSync,
  mkdtempSync,
  readFileSync,
  readdirSync,
  writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

import type { Config } from "@oclif/core";
import { loadConfig } from "@agent-ix/ix-cli-core";
import { beforeAll, describe, expect, it } from "vitest";

import EvidenceRecord from "../src/commands/evidence/record";

const repoRoot = join(dirname(fileURLToPath(import.meta.url)), "..");
let config: Config;

beforeAll(async () => {
  config = await loadConfig({ root: repoRoot });
});

/**
 * A repository the command will accept: `quoin evidence record` runs
 * `quire coverage` over `--repo`, which needs a real `spec/` tree. One FR is
 * enough — these criteria are about the adapter seam, not about coverage.
 */
function workspace(): string {
  const root = mkdtempSync(join(tmpdir(), "quoin-adapters-"));
  mkdirSync(join(root, "spec", "functional"), { recursive: true });
  writeFileSync(
    join(root, "spec", "functional", "FR-001-a-requirement.md"),
    "---\nid: FR-001\ntype: FR\ntitle: A requirement\n---\n\n" +
      "## Description\n\nThe system shall do it.\n",
  );
  return root;
}

const JUNIT = `<?xml version="1.0"?>
<testsuites>
  <testsuite name="corpus">
    <testcase classname="tests.corpus" name="tc001_resolves">
      <properties><property name="trace" value="FR-001-AC-1, FR-001-AC-2"/></properties>
    </testcase>
    <testcase classname="tests.corpus" name="tc002_dangles"><failure message="boom"/></testcase>
    <testcase classname="tests.corpus" name="tc003_errors"><error message="panic"/></testcase>
    <testcase classname="tests.corpus" name="tc004_skipped"><skipped/></testcase>
  </testsuite>
</testsuites>`;

const MUTANTS = JSON.stringify({
  outcomes: [
    { scenario: "Baseline", summary: "Success" },
    {
      scenario: {
        Mutant: { file: "src/a.rs", function: { function_name: "resolve" } },
      },
      summary: "CaughtMutant",
    },
    {
      scenario: {
        Mutant: { file: "src/a.rs", function: { function_name: "resolve" } },
      },
      summary: "MissedMutant",
    },
    {
      scenario: {
        Mutant: { file: "src/a.rs", function: { function_name: "resolve" } },
      },
      summary: "Unviable",
    },
    {
      scenario: {
        Mutant: { file: "src/b.rs", function: { function_name: "harvest" } },
      },
      summary: "CaughtMutant",
    },
  ],
});

/** Record a run and return the entries the store persisted. */
async function record(
  root: string,
  args: string[],
  results: string,
  body: string,
): Promise<Array<Record<string, unknown>>> {
  writeFileSync(join(root, results), body);
  await EvidenceRecord.run(
    [
      "--repo",
      root,
      "--suite",
      "SUITE-001",
      "--commit",
      "0".repeat(40),
      "--results",
      join(root, results),
      ...args,
    ],
    config,
  );
  const runs = join(root, "spec", "evidence", "runs", "SUITE-001");
  const file = readdirSync(runs).find((f) => f.endsWith(".json"));
  const parsed = JSON.parse(
    readFileSync(join(runs, file as string), "utf8"),
  ) as {
    entries: Array<Record<string, unknown>>;
  };
  return parsed.entries;
}
describe("quoin evidence record --adapter", () => {
  // Trace: FR-033-AC-11
  it("records a JUnit file end to end, through the command", async () => {
    const root = workspace();
    const entries = await record(
      root,
      ["--tool", "pytest 8.0", "--adapter", "junit"],
      "results.xml",
      JUNIT,
    );
    expect(entries).toHaveLength(4);
    expect(entries[0].symbol).toBe("tests::corpus::tc001_resolves");
    expect(entries[0].traceIds).toEqual(["FR-001-AC-1", "FR-001-AC-2"]);
  });

  // Trace: FR-033-AC-12
  it("records a cargo-mutants report end to end, preserving score", async () => {
    const root = workspace();
    const entries = await record(
      root,
      ["--tool", "cargo-mutants 27.1.0", "--adapter", "cargo-mutants"],
      "outcomes.json",
      MUTANTS,
    );
    expect(entries.map((e) => e.score)).toEqual([0.5, 1]);
    // The discriminator survives the whole record path (#138): what lands in
    // the store is what the auditor's mutation floor will filter on.
    expect(entries.map((e) => e.metric)).toEqual([
      "mutation-score",
      "mutation-score",
    ]);
  });

  // Trace: FR-033-AC-13
  it("selects the adapter from --tool when none is named", async () => {
    const root = workspace();
    const entries = await record(
      root,
      ["--tool", "pytest 8.0"],
      "results.xml",
      JUNIT,
    );
    expect(entries).toHaveLength(4);
  });

  // Trace: FR-033-AC-14
  it("still accepts the normalized shape with no adapter at all", async () => {
    // The escape hatch: a consumer whose tool no adapter reads writes entries
    // by hand, so the registry is never a gate on recording evidence.
    const root = workspace();
    const entries = await record(
      root,
      ["--tool", "bespoke 1.0"],
      "run.json",
      JSON.stringify({
        entries: [{ symbol: "tests::tc001", outcome: "pass" }],
      }),
    );
    expect(entries).toEqual([{ symbol: "tests::tc001", outcome: "pass" }]);
  });
});
