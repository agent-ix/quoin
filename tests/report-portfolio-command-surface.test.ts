/**
 * The `quoin report` portfolio command surface (FR-045-AC-1).
 *
 * Split out of `tests/portfolio-report.test.ts` (quoin#479). That file is
 * deleted with `src/measurement/`, but two of its assertions are about
 * `src/commands/report.ts` — which SURVIVES, because the command shell was
 * rewired through `quoin-core measurement.build_portfolio` at quoin#478 and is
 * not in the FR-101 delete set. A test file that also covers a surviving
 * module is split, not deleted.
 *
 * Both halves were checked for a second home before this file was written and
 * neither had one: `tests/cli-usage.test.ts` and `tests/command-entries.test.ts`
 * never mention `--portfolio`, and `tests/graph-portfolio-command.test.ts`
 * passes it exactly once per invocation, so it does not restate the repetition
 * property. Deleting the file wholesale would have dropped both silently.
 *
 * Nothing here imports `src/measurement/`: the repository fixtures are written
 * with `node:fs` and the report is produced by the Rust boundary, which is what
 * the command now calls.
 */

import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

import { loadConfig } from "@agent-ix/ix-cli-core";
import type { Config } from "@oclif/core";
import { afterEach, beforeAll, expect, test, vi } from "vitest";

import ReportCommand from "../src/commands/report.js";

const projectRoot = join(dirname(fileURLToPath(import.meta.url)), "..");
const roots: string[] = [];
let config: Config;

const PROFILE = `---
id: AP-001
title: Fixture assurance
type: AssuranceProfile
status: active
---

# Fixture assurance
`;

const PLAN = `---
id: MP-001
title: Fixture quality
type: MeasurementPlan
status: active
stage: branch-comparison
metric: quality.fixture
definition_version: quality.fixture-v1
---

# Fixture quality
`;

beforeAll(async () => {
  config = await loadConfig({ root: projectRoot });
});

afterEach(() => {
  vi.restoreAllMocks();
  for (const root of roots.splice(0))
    rmSync(root, { recursive: true, force: true });
});

/** One governed repository named `name`, with an authored profile and plan. */
function repo(name: string): string {
  const parent = mkdtempSync(join(tmpdir(), "quoin-report-portfolio-"));
  roots.push(parent);
  const root = join(parent, name);
  const assurance = join(root, "spec", "assurance");
  mkdirSync(assurance, { recursive: true });
  writeFileSync(join(assurance, "AP-001.md"), PROFILE);
  writeFileSync(join(assurance, "MP-001.md"), PLAN);
  return root;
}

/** Run `quoin report` and return what it printed. */
async function run(argv: string[]): Promise<string> {
  const output: string[] = [];
  vi.spyOn(console, "log").mockImplementation((value) =>
    output.push(String(value)),
  );
  await ReportCommand.run(argv, config);
  return output.join("\n");
}

// Trace: FR-045-AC-1
// Provenance: quoin#479
test("one report invocation accepts repeated repository locations", async () => {
  const first = repo("first");
  const second = repo("second");

  const value = JSON.parse(
    await run([
      "--portfolio",
      first,
      "--portfolio",
      second,
      "--format",
      "json",
    ]),
  );

  // One invocation, one report, both locations in it — the property the
  // repeated flag exists for. A single `--portfolio` cannot show this.
  expect(
    value.repositories
      .map((repository: { name: string }) => repository.name)
      .sort(),
  ).toEqual(["first", "second"]);
});

// Trace: FR-045-AC-1
// Provenance: quoin#479
test("the report command exposes no typed observation-value flag", () => {
  // FR-045-AC-1 says the command "exposes no typed observation-value flag": a
  // portfolio reports what producers recorded, and a `--value` would let the
  // caller type a number into evidence at the command line. The absence is the
  // criterion, so it is asserted of the flag set rather than of a run.
  const flags = Object.keys(ReportCommand.flags);
  expect(flags).not.toContain("value");
  // Anti-vacuity: an empty or renamed flag set would satisfy the line above
  // while asserting nothing at all.
  expect(flags).toContain("portfolio");
  expect(flags.length).toBeGreaterThan(1);
});
