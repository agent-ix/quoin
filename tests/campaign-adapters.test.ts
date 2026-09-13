/**
 * Quoin #323 — campaign-native result adapters (FR-069), where the command
 * reaches them.
 *
 * The adapters themselves are `quoin-evidence`'s, asserted by its golden
 * corpus and by `tests/adapter_purity.rs` (quoin#458). What is stated here is
 * what only this tree can see: that an unrepresented result is reported to the
 * reader in both output modes and stored as no fifth outcome, and that the
 * inventory dispositions every format #323 named.
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

import { loadConfig } from "@agent-ix/ix-cli-core";
import type { Config } from "@oclif/core";
import { beforeAll, describe, expect, it, vi } from "vitest";

import EvidenceRecord from "../src/commands/evidence/record.js";

const repoRoot = join(dirname(fileURLToPath(import.meta.url)), "..");
const fixtures = join(repoRoot, "tests/fixtures/evidence");
const inventory = join(repoRoot, "docs/campaign-native-result-inventory.md");

const differential = readFileSync(
  join(fixtures, "differential-report-real.json"),
  "utf8",
);

let config: Config;

beforeAll(async () => {
  config = await loadConfig({ root: repoRoot });
});

/** A minimal repository the record command can resolve obligations against. */
function repository(): string {
  const root = mkdtempSync(join(tmpdir(), "quoin-campaign-adapters-"));
  mkdirSync(join(root, "spec", "functional"), { recursive: true });
  writeFileSync(
    join(root, "spec", "functional", "FR-001-a-requirement.md"),
    "---\nid: FR-001\ntype: FR\ntitle: A requirement\n---\n\n" +
      "## Description\n\nThe system shall do it.\n",
  );
  return root;
}
describe("FR-069 recording", () => {
  it("prints every unrepresented result in human and JSON output", async () => {
    const root = repository();
    const results = join(root, "differential.json");
    writeFileSync(results, differential);

    const args = [
      "--repo",
      root,
      "--suite",
      "SUITE-001",
      "--commit",
      "0".repeat(40),
      "--tool",
      "tl-mltl 0.1.0",
      "--adapter",
      "differential-report",
      "--results",
      results,
    ];

    const human: string[] = [];
    vi.spyOn(console, "log").mockImplementation((line) =>
      human.push(String(line)),
    );
    await EvidenceRecord.run(args, config);
    // Named, not counted, and never dropped: the reader is told which result
    // the record is short by and why.
    expect(human.join("\n")).toContain("not transcribed");
    expect(human.join("\n")).toContain("closed-profile-not-mapped-v1");
    expect(human.join("\n")).toContain("unsupported");

    const machine: string[] = [];
    vi.spyOn(console, "log").mockImplementation((line) =>
      machine.push(String(line)),
    );
    await EvidenceRecord.run([...args, "--json"], config);
    const payload = JSON.parse(machine.join("\n")) as {
      unrepresented?: { symbol: string; state: string }[];
    };
    expect(payload.unrepresented).toEqual([
      {
        symbol: "closed-profile-not-mapped-v1",
        state: "unsupported",
        reason: expect.stringContaining("neither a skip nor an error"),
      },
    ]);
    vi.restoreAllMocks();

    // No new record family: the persisted run is the existing shape, and the
    // unrepresented results are reported rather than stored as a fifth state.
    const runs = join(root, "spec", "evidence", "runs", "SUITE-001");
    const file = readdirSync(runs).find((name) => name.endsWith(".json"));
    const stored = JSON.parse(
      readFileSync(join(runs, file as string), "utf8"),
    ) as { entries: { symbol: string; outcome: string }[] };
    expect(stored.entries.map((entry) => entry.outcome)).toEqual(
      Array(8).fill("pass"),
    );
    expect(stored.entries.map((entry) => entry.symbol)).not.toContain(
      "closed-profile-not-mapped-v1",
    );
  });
});

describe("FR-069 inventory and boundaries", () => {
  it("records a producer and a verdict for every scope item", () => {
    const text = readFileSync(inventory, "utf8");

    // Every format #323 names is dispositioned, by name.
    for (const item of [
      "JUnit",
      "SARIF",
      "cargo-mutants",
      "Audit-script",
      "Kani",
      "Solver analysis",
      "Contract conformance JSONL",
      "Corpus reports",
      "Measurements",
      "Counterexamples",
      "differential report",
    ]) {
      expect(text, `${item} is not dispositioned`).toMatch(
        new RegExp(item.replace(/[.*+?^${}()|[\]\\]/g, "\\$&"), "i"),
      );
    }

    // Every added adapter names a real producer and a pinned sample.
    for (const sample of [
      "contract-conformance-real.jsonl",
      "differential-report-real.json",
    ]) {
      expect(text).toContain(sample);
      expect(readdirSync(fixtures)).toContain(sample);
    }
    expect(text).toContain("agent-ix/quire-contract-ir");
    expect(text).toContain("agent-ix/tl-mltl");

    // The out-of-scope rule is stated, not implied.
    expect(text).toMatch(/stdout or stderr scraping is out of\nscope/);
  });
});
