/**
 * Quoin #331: native conformance trace metadata reaches the existing store.
 *
 * The adapter's own reading of `trace_ids` — preservation without sorting or
 * trimming, and the per-line refusals for every malformed spelling — is
 * `quoin-evidence`'s and is asserted by its golden corpus (quoin#458). What
 * stays here is the path only this tree has: the real producer ids through
 * `quire.coverage` targets, into bindings, in the store. Since quoin#502 that
 * derivation is asked of `quoin-core` rather than of a spawned `quire`.
 */
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
import { auditInputs, parseResults } from "../src/core/evidence.js";
import { coverage } from "../src/core/quire.js";

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
  producerRevision = "9b9102c3806e9cda0ed70312f4f6c23a211f6fbf",
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
      `quire-contract-conformance git:${producerRevision}`,
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
  it("records and binds the real producer ids through Quire targets", async () => {
    const root = repository();
    const derived = coverage(root);
    expect(derived.obligations?.map((item) => item.target_ids)).toEqual([
      ["TC-015"],
      ["TC-017"],
      ["TC-018"],
    ]);
    // The fixture is real captured producer output; the digest pins which
    // bytes these expectations were read from.
    expect(createHash("sha256").update(real).digest("hex")).toBe(
      "785018c631c8393c5d8f36712bf183431fab74a505c9b1d2c2059b5a249ef2d3",
    );
    const result = await record(root, real);
    expect(result.bound).toEqual(["FR-001-AC-1", "FR-001-AC-2", "FR-001-AC-3"]);
    expect(result.unmatched).toEqual([]);
    const inputs = auditInputs(root);
    expect(inputs.runs[0].entries[0].traceIds).toEqual(row.trace_ids);
    expect(inputs.bindings).toHaveLength(3);
    for (const binding of inputs.bindings)
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
    const failedInputs = auditInputs(failedRoot);
    expect(failedInputs.bindings).toEqual([]);
    expect(failedInputs.runs[0].entries[0]).toMatchObject({
      outcome: "fail",
      traceIds: ["TC-015", "TC-017", "TC-018", "TC-999"],
    });
  });

  it("records nothing when a later row has malformed trace metadata", async () => {
    const root = repository();
    await expect(
      record(root, `${real}${JSON.stringify({ ...row, trace_ids: [""] })}\n`),
    ).rejects.toThrow(/line 2.*trace_ids/);
    expect(existsSync(join(root, "spec", "evidence"))).toBe(false);
  });

  it("binds the corrected IR producer's exact criterion targets without sibling TC fanout", async () => {
    const root = repository();
    for (const name of [
      "FR-011-package-identity.md",
      "FR-018-conformance-corpus.md",
    ]) {
      writeFileSync(
        join(root, "spec", "functional", name),
        readFileSync(join(fixtureRoot, "contract-ir-criteria", name)),
      );
    }
    const sample = readFileSync(
      join(fixtureRoot, "contract-conformance-criteria-real.jsonl"),
      "utf8",
    );
    const targets = ["FR-011-AC-3", "FR-018-AC-1"];
    expect(createHash("sha256").update(sample).digest("hex")).toBe(
      "d5a962e80328c34897d839d2d553c9f7395144f94abd1e7f92717f0d3656ab1a",
    );
    const parsed = parseResults({
      text: sample,
      adapter: "contract-conformance",
    });
    expect(parsed.kind).toBe("run");
    if (parsed.kind !== "run") throw new Error("expected a run-shaped parse");
    expect(parsed.entries[0].traceIds).toEqual(targets);
    const result = await record(
      root,
      sample,
      "66a0399656624764f75873886d7de54a96afc7ea",
    );
    expect(result.bound).toEqual(targets);
    expect(result.unmatched).toEqual([]);
    const stored = auditInputs(root);
    expect(stored.bindings.map((binding) => binding.obligation)).toEqual(
      targets,
    );
    expect(stored.runs[0].entries[0].traceIds).toEqual(targets);
  });
});
