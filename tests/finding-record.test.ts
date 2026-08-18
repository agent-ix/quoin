/**
 * FR-034 — FindingRecord and the finding-shaped adapters (TC-165..TC-172).
 */

import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

import { describe, expect, it } from "vitest";

import { parseCargoAudit, parseSarif } from "../src/evidence/index.js";

const here = dirname(fileURLToPath(import.meta.url));
const realAudit = readFileSync(
  join(here, "fixtures", "evidence", "cargo-audit-real.json"),
  "utf8",
);

const SARIF_CLEAN = JSON.stringify({
  version: "2.1.0",
  runs: [
    {
      tool: {
        driver: { name: "semgrep", version: "1.2.3", rules: [{}, {}, {}] },
      },
      results: [],
    },
  ],
});

const SARIF_FINDING = JSON.stringify({
  version: "2.1.0",
  runs: [
    {
      tool: { driver: { name: "semgrep", version: "1.2.3", rules: [{}, {}] } },
      results: [
        {
          ruleId: "rules.no-eval",
          level: "error",
          message: { text: "eval is forbidden" },
          locations: [
            {
              physicalLocation: {
                artifactLocation: { uri: "src/a.ts" },
                region: { startLine: 42 },
              },
            },
          ],
        },
        { rule: { id: "rules.legacy-form" } },
        { level: "warning" },
      ],
    },
  ],
});

describe("a clean scan and an unrun scan must not look alike", () => {
  // Trace: FR-034-AC-1
  it("reads a SARIF run with no results as a scan that HAPPENED", () => {
    // The whole reason this record type exists. Zero findings on a present run
    // is evidence; the absence of a record is not.
    const result = parseSarif(SARIF_CLEAN);
    expect(result.findings).toEqual([]);
    expect(result.tool).toBe("semgrep 1.2.3");
    // …and the run says how much it evaluated, so "clean" can be told from
    // "clean because no rules were enabled".
    expect(result.rulesEvaluated).toBe(3);
  });

  // Trace: FR-034-AC-2
  it("rejects a SARIF log carrying no run at all", () => {
    // A file with `runs: []` proves nothing executed. Recording it would
    // manufacture the exact evidence this record type exists to distinguish.
    expect(() =>
      parseSarif(JSON.stringify({ version: "2.1.0", runs: [] })),
    ).toThrow(/no run proves no scan executed/);
  });
});

describe("the SARIF adapter", () => {
  // Trace: FR-034-AC-3
  it("reads rule id, level, message and location", () => {
    const { findings } = parseSarif(SARIF_FINDING);
    expect(findings[0]).toEqual({
      ruleId: "rules.no-eval",
      severity: "error",
      message: "eval is forbidden",
      path: "src/a.ts",
      line: 42,
    });
  });

  // Trace: FR-034-AC-4
  it("accepts the nested rule.id form and skips a result with no rule at all", () => {
    const { findings } = parseSarif(SARIF_FINDING);
    expect(findings).toHaveLength(2);
    expect(findings[1]).toEqual({ ruleId: "rules.legacy-form" });
  });

  // Trace: FR-034-AC-5
  it("rejects malformed input and a log with no runs array", () => {
    expect(() => parseSarif("{")).toThrow(/not JSON/);
    expect(() => parseSarif("{}")).toThrow(/no `runs` array/);
  });
});

describe("the cargo-audit adapter, against real tool output", () => {
  // Trace: FR-034-AC-6
  it("parses output captured from `cargo audit --json`, not a hand-written fixture", () => {
    // agent-ix/quoin#115 asked for the format decisions to be made by reading
    // real output. A fixture written to match the reader proves the reader
    // parses itself.
    const result = parseCargoAudit(realAudit);
    expect(result.tool).toBe("cargo-audit");
    // The advisory database it consulted — evidence the scan had rules.
    expect(result.rulesEvaluated).toBe(1217);
    // Zero vulnerabilities, one `unsound` warning: a scan that RAN and found
    // almost nothing.
    expect(result.findings).toHaveLength(1);
    expect(result.findings[0].ruleId).toMatch(/^RUSTSEC-/);
    expect(result.findings[0].severity).toBe("unsound");
    expect(result.findings[0].path).toMatch(/^anyhow@/);
  });

  // Trace: FR-034-AC-7
  it("keeps each warning kind as its own severity rather than flattening", () => {
    // `unsound`, `unmaintained` and `yanked` are distinctions cargo-audit drew.
    // Collapsing them to one word would discard information the tool produced,
    // and quoin normalizes no scanner's severities.
    const { findings } = parseCargoAudit(
      JSON.stringify({
        vulnerabilities: {
          found: true,
          list: [
            {
              advisory: { id: "RUSTSEC-1", title: "t" },
              package: { name: "p", version: "1" },
            },
          ],
        },
        warnings: {
          unmaintained: [
            { advisory: { id: "RUSTSEC-2" }, package: { name: "q" } },
          ],
          yanked: [{ advisory: { id: "RUSTSEC-3" }, package: { name: "r" } }],
        },
      }),
    );
    expect(findings.map((f) => f.severity)).toEqual([
      "vulnerability",
      "unmaintained",
      "yanked",
    ]);
    expect(findings[0].path).toBe("p@1");
    expect(findings[1].path).toBe("q");
  });

  // Trace: FR-034-AC-8
  it("rejects malformed input and output that is not cargo-audit's", () => {
    expect(() => parseCargoAudit("{")).toThrow(/not JSON/);
    expect(() => parseCargoAudit("{}")).toThrow(/no `vulnerabilities` object/);
    expect(() =>
      parseCargoAudit(JSON.stringify({ vulnerabilities: { found: false } })),
    ).not.toThrow();
    // An advisory with no id cannot be attributed and is skipped.
    const { findings } = parseCargoAudit(
      JSON.stringify({
        vulnerabilities: { found: true, list: [{ package: { name: "p" } }] },
      }),
    );
    expect(findings).toEqual([]);
  });
});
