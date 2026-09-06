/** Quoin #331: native conformance trace metadata reaches the existing store. */
import { createHash } from "node:crypto";
import {
  existsSync,
  mkdirSync,
  mkdtempSync,
  readFileSync,
  rmSync,
  writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

import { loadConfig } from "@agent-ix/ix-cli-core";
import type { Config } from "@oclif/core";
import { afterEach, beforeAll, describe, expect, it, vi } from "vitest";

import EvidenceRecord from "../src/commands/evidence/record.js";
import {
  AdapterError,
  contractConformanceAdapter,
  readBindings,
  readRuns,
} from "../src/evidence/index.js";
import { parseCoverage, runQuire } from "../src/quire/index.js";

const repoRoot = join(dirname(fileURLToPath(import.meta.url)), "..");
const fixtureRoot = join(repoRoot, "tests/fixtures/evidence");
const real = readFileSync(
  join(fixtureRoot, "contract-conformance-traces-real.jsonl"),
  "utf8",
);
const row = JSON.parse(real) as Record<string, unknown>;
let config: Config;
const roots: string[] = [];

beforeAll(async () => {
  config = await loadConfig({ root: repoRoot });
});

afterEach(() => {
  vi.restoreAllMocks();
  for (const root of roots.splice(0))
    rmSync(root, { recursive: true, force: true });
});

function repository(): string {
  const root = mkdtempSync(join(tmpdir(), "quoin-conformance-traces-"));
  roots.push(root);
  mkdirSync(join(root, "spec", "functional"), { recursive: true });
  writeFileSync(
    join(root, "spec", "functional", "FR-001-contract.md"),
    "---\nid: FR-001\ntype: FR\ntitle: Contract fixture\n---\n\n" +
      "# FR-001: Contract fixture\n\n## Description\n\n" +
      "The system shall preserve the declared contract.\n\n" +
      "## Acceptance Criteria\n\n" +
      "| ID | Criteria | Verification |\n|---|---|---|\n" +
      "| FR-001-AC-1 | The namespace is checked. | Test (TC-015) |\n" +
      "| FR-001-AC-2 | The diagnostic is retained. | Test (TC-017) |\n" +
      "| FR-001-AC-3 | The result is attributable. | Test (TC-018) |\n",
  );
  return root;
}

async function record(
  root: string,
  input: string,
): Promise<{
  bound: string[];
  unmatched: string[];
}> {
  const path = join(root, "results.jsonl");
  writeFileSync(path, input);
  const output: string[] = [];
  vi.spyOn(console, "log").mockImplementation((value) =>
    output.push(String(value)),
  );
  await EvidenceRecord.run(
    [
      "--repo",
      root,
      "--suite",
      "SUITE-001",
      "--commit",
      "a".repeat(40),
      "--tool",
      "quire-contract-conformance git:9b9102c3806e9cda0ed70312f4f6c23a211f6fbf",
      "--adapter",
      "contract-conformance",
      "--results",
      path,
      "--timestamp",
      "2026-09-06T00:00:00Z",
      "--json",
    ],
    config,
  );
  return JSON.parse(output.join("\n")) as {
    bound: string[];
    unmatched: string[];
  };
}

describe("FR-069 conformance traces", () => {
  it("TC-1585 preserves real trace metadata and accepts legacy omission", () => {
    expect(row.trace_ids).toEqual(["TC-015", "TC-017", "TC-018"]);
    const entry = contractConformanceAdapter.parse(real).entries[0];
    expect(entry.traceIds).toEqual(row.trace_ids);
    expect(entry.outcome).toBe("pass");
    expect(entry.symbol).toBe(
      "contract-v0.1::package::package-invalid-namespace",
    );
    // Constructed variant proves this is preservation, not sorting/trimming.
    const ids = ["TC-018", " TC-015 ", "TC-017"];
    expect(
      contractConformanceAdapter.parse(
        JSON.stringify({ ...row, trace_ids: ids }),
      ).entries[0].traceIds,
    ).toEqual(ids);
    const legacy = readFileSync(
      join(fixtureRoot, "contract-conformance-real.jsonl"),
      "utf8",
    );
    for (const old of contractConformanceAdapter.parse(legacy).entries)
      expect(old).not.toHaveProperty("traceIds");
    expect(createHash("sha256").update(real).digest("hex")).toBe(
      "785018c631c8393c5d8f36712bf183431fab74a505c9b1d2c2059b5a249ef2d3",
    );
  });

  it("TC-1586 rejects malformed supplied trace metadata with line and field", () => {
    for (const trace_ids of [
      null,
      {},
      "TC-015",
      1,
      true,
      [],
      [null],
      [15],
      [true],
      [[]],
      [{}],
      [""],
      [" \t\n"],
      ["TC-015", "TC-015"],
      ["TC-015", ""],
    ]) {
      const input = `${real}${JSON.stringify({ ...row, trace_ids })}\n`;
      expect(() => contractConformanceAdapter.parse(input)).toThrow(
        AdapterError,
      );
      expect(() => contractConformanceAdapter.parse(input)).toThrow(
        /line 2.*trace_ids/,
      );
    }
  });

  it("TC-1587 records and binds the real producer ids through Quire targets", async () => {
    const root = repository();
    const coverage = parseCoverage(
      runQuire(["coverage", "--scope", root, "--json"]),
    );
    expect(coverage.ok).toBe(true);
    if (!coverage.ok) throw new Error(coverage.error.message);
    expect(coverage.value.obligations?.map((item) => item.target_ids)).toEqual([
      ["TC-015"],
      ["TC-017"],
      ["TC-018"],
    ]);
    const result = await record(root, real);
    expect(result.bound).toEqual(["FR-001-AC-1", "FR-001-AC-2", "FR-001-AC-3"]);
    expect(result.unmatched).toEqual([]);
    expect(readRuns(root, "SUITE-001")[0].entries[0].traceIds).toEqual(
      row.trace_ids,
    );
    expect(readBindings(root).bindings).toHaveLength(3);
    for (const binding of readBindings(root).bindings)
      expect(binding.symbols).toEqual([
        "contract-v0.1::package::package-invalid-namespace",
      ]);

    // Constructed adverse report: failure and unmatched ids remain visible.
    const failedRoot = repository();
    const failure = JSON.stringify({
      ...row,
      status: "mismatch",
      mismatch_kinds: ["diagnostics"],
      trace_ids: [...(row.trace_ids as string[]), "TC-999"],
    });
    const failed = await record(failedRoot, failure);
    expect(failed.bound).toEqual([]);
    expect(failed.unmatched).toEqual(["TC-999"]);
    expect(readBindings(failedRoot).bindings).toEqual([]);
    expect(readRuns(failedRoot, "SUITE-001")[0].entries[0]).toMatchObject({
      outcome: "fail",
      traceIds: ["TC-015", "TC-017", "TC-018", "TC-999"],
    });
  });

  it("TC-1588 records nothing when a later row has malformed trace metadata", async () => {
    const root = repository();
    await expect(
      record(root, `${real}${JSON.stringify({ ...row, trace_ids: [""] })}\n`),
    ).rejects.toThrow(/line 2.*trace_ids/);
    expect(existsSync(join(root, "spec", "evidence"))).toBe(false);
  });
});
