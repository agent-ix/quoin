/**
 * FR-034 — FindingRecord and the finding-shaped adapters (TC-165..TC-172).
 */

import {
  existsSync,
  mkdirSync,
  mkdtempSync,
  readFileSync,
  writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

import type { Config } from "@oclif/core";
import { loadConfig } from "@agent-ix/ix-cli-core";
import { beforeAll, describe, expect, it } from "vitest";

import EvidenceRecord from "../src/commands/evidence/record";

import { audit } from "../src/auditor/index.js";
import {
  gc,
  latestScan,
  listRecordedSuites,
  parseCargoAudit,
  parseSarif,
  type FindingRecord,
} from "../src/evidence/index.js";

const here = dirname(fileURLToPath(import.meta.url));
const repoRoot = join(here, "..");
let config: Config;

beforeAll(async () => {
  config = await loadConfig({ root: repoRoot });
});

/** A repository `quoin evidence record` will accept: it needs a real spec/. */
function workspace(): string {
  const root = mkdtempSync(join(tmpdir(), "quoin-scan-"));
  mkdirSync(join(root, "spec", "functional"), { recursive: true });
  writeFileSync(
    join(root, "spec", "functional", "FR-001-a.md"),
    "---\nid: FR-001\ntype: FR\ntitle: A requirement\n---\n\n" +
      "## Description\n\nThe system shall do it.\n\n" +
      "## Acceptance Criteria\n\n" +
      "| ID | Criteria | Verification |\n" +
      "|----|----------|--------------|\n" +
      "| FR-001-AC-1 | It does the thing. | Test (TC-001) |\n",
  );
  return root;
}
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

describe("the auditor over finding-shaped scans", () => {
  const obligation = { id: "FR-001-AC-1", statement: "s", statement_hash: "h" };

  function scan(over: Partial<FindingRecord> = {}): FindingRecord {
    return {
      schemaVersion: 1,
      suite: "SUITE-SCAN",
      commit: "a".repeat(40),
      tool: "semgrep 1.2.3",
      timestamp: "2026-08-18T00:00:00Z",
      findings: [],
      ...over,
    };
  }

  const binding = {
    obligation: "FR-001-AC-1",
    suite: "SUITE-SCAN",
    symbols: [],
    statementHashAtBinding: "h",
    commit: "a".repeat(40),
  };

  // Trace: FR-034-AC-9
  it("treats a clean scan as evidence, not as an undischarged obligation", () => {
    // Zero findings with rules evaluated is a RESULT. Reporting it as
    // undischarged would be the defect this record type exists to prevent,
    // wearing the opposite hat.
    const report = audit({
      obligations: [obligation],
      bindings: [binding],
      runs: [],
      scans: [scan({ rulesEvaluated: 400 })],
    });
    expect(report.findings.map((f) => f.kind)).not.toContain("undischarged");
    expect(report.findings.map((f) => f.kind)).not.toContain(
      "vacuous-evidence",
    );
  });

  // Trace: FR-034-AC-10
  it("reports a scan that evaluated no rules as vacuous", () => {
    const report = audit({
      obligations: [obligation],
      bindings: [binding],
      runs: [],
      scans: [scan({ rulesEvaluated: 0 })],
    });
    const vacuous = report.findings.find((f) => f.kind === "vacuous-evidence");
    expect(vacuous?.severity).toBe("high");
    expect(vacuous?.summary).toMatch(/looked for nothing/);
  });

  // Trace: FR-034-AC-11
  it("stays silent when the tool does not say how many rules it evaluated", () => {
    // The question cannot be asked, so the check says nothing rather than
    // something wrong — the posture method conformance takes for an absent
    // evidence kind (agent-ix/quoin#105).
    const report = audit({
      obligations: [obligation],
      bindings: [binding],
      runs: [],
      scans: [scan()],
    });
    expect(report.findings.map((f) => f.kind)).not.toContain(
      "vacuous-evidence",
    );
  });

  // Trace: FR-034-AC-12
  it("pairs each run-shaped binding with its OWN run when a scan is also bound", () => {
    // The regression this exists for: `runs` is built from the run-backed
    // bindings, so any check indexing the full binding list would pair a
    // binding with another suite's run as soon as one binding is scan-backed.
    // Silent until scans existed, wrong from the moment they did.
    const report = audit({
      obligations: [obligation],
      bindings: [
        { ...binding, suite: "SUITE-SCAN" },
        { ...binding, suite: "SUITE-RUN", symbols: ["tests::tc001"] },
      ],
      runs: [
        {
          schemaVersion: 1,
          suite: "SUITE-RUN",
          commit: "a".repeat(40),
          tool: "cargo test",
          timestamp: "2026-08-18T00:00:00Z",
          entries: [{ symbol: "tests::tc001", outcome: "pass" }],
        },
      ],
      scans: [scan({ rulesEvaluated: 400 })],
      mockInspectionSuites: ["SUITE-RUN"],
    });
    // The run's symbol passed, so nothing is vacuous. Misaligned indexing
    // would look for `tests::tc001` in the scan-backed slot and report it
    // absent.
    expect(report.findings.map((f) => f.kind)).not.toContain(
      "vacuous-evidence",
    );
    expect(report.unevaluated).toEqual([]);
    expect(report.healthy).toContain("FR-001-AC-1");
  });
});

describe("quoin evidence record --adapter sarif", () => {
  // Trace: FR-034-AC-13
  it("writes a FindingRecord, not a run, end to end through the command", async () => {
    // The gap this test exists for: FindingRecord, the SARIF adapter and
    // writeScan all shipped without a single command that could reach them —
    // a capability nothing could use, which is exactly the P1 defect the
    // ticket's acceptance shape was written to prevent.
    const root = workspace();
    const results = join(root, "scan.sarif");
    writeFileSync(results, SARIF_FINDING);
    await EvidenceRecord.run(
      [
        "--repo",
        root,
        "--suite",
        "SUITE-SCAN",
        "--commit",
        "b".repeat(40),
        "--tool",
        "semgrep 1.2.3",
        "--adapter",
        "sarif",
        "--results",
        results,
      ],
      config,
    );
    const record = JSON.parse(
      readFileSync(
        join(
          root,
          "spec",
          "evidence",
          "scans",
          "SUITE-SCAN",
          "bbbbbbbbbbbb.json",
        ),
        "utf8",
      ),
    ) as FindingRecord;
    expect(record.findings).toHaveLength(2);
    expect(record.rulesEvaluated).toBe(2);
    expect(record.tool).toBe("semgrep 1.2.3");
    // And nothing was written to runs/ — a scan in runs/ would lose the
    // distinction at the point of intake.
    expect(
      existsSync(join(root, "spec", "evidence", "runs", "SUITE-SCAN")),
    ).toBe(false);
  });

  // Trace: FR-034-AC-14
  it("records a clean scan as a scan, so zero findings is still evidence", async () => {
    const root = workspace();
    const results = join(root, "clean.sarif");
    writeFileSync(results, SARIF_CLEAN);
    await EvidenceRecord.run(
      [
        "--repo",
        root,
        "--suite",
        "SUITE-SCAN",
        "--commit",
        "c".repeat(40),
        "--tool",
        "semgrep 1.2.3",
        "--adapter",
        "sarif",
        "--results",
        results,
      ],
      config,
    );
    const record = JSON.parse(
      readFileSync(
        join(
          root,
          "spec",
          "evidence",
          "scans",
          "SUITE-SCAN",
          "cccccccccccc.json",
        ),
        "utf8",
      ),
    ) as FindingRecord;
    expect(record.findings).toEqual([]);
    expect(record.rulesEvaluated).toBe(3);
  });

  // Trace: FR-034-AC-15
  it("selects the finding adapter from --tool when none is named", async () => {
    const root = workspace();
    const results = join(root, "audit.json");
    writeFileSync(results, realAudit);
    await EvidenceRecord.run(
      [
        "--repo",
        root,
        "--suite",
        "SUITE-AUDIT",
        "--commit",
        "d".repeat(40),
        "--tool",
        "cargo-audit 0.21",
        "--results",
        results,
      ],
      config,
    );
    const record = JSON.parse(
      readFileSync(
        join(
          root,
          "spec",
          "evidence",
          "scans",
          "SUITE-AUDIT",
          "dddddddddddd.json",
        ),
        "utf8",
      ),
    ) as FindingRecord;
    expect(record.rulesEvaluated).toBe(1217);
    expect(record.findings[0].ruleId).toMatch(/^RUSTSEC-/);
  });
});

describe("a scan is reachable from every side of the store", () => {
  // These exist because SR-005 found FR-034 inert end to end: the record was
  // written and nothing else in the system could see it. Each criterion here
  // is a path that had no test at all.

  async function recordScan(root: string, args: string[] = []): Promise<void> {
    const results = join(root, "scan.sarif");
    writeFileSync(results, SARIF_CLEAN);
    await EvidenceRecord.run(
      [
        "--repo",
        root,
        "--suite",
        "SUITE-SCAN",
        "--commit",
        "e".repeat(40),
        "--tool",
        "semgrep 1.2.3",
        "--adapter",
        "sarif",
        "--results",
        results,
        ...args,
      ],
      config,
    );
  }

  // Trace: FR-034-AC-16, TC-192
  it("binds the obligations it was run to check", async () => {
    // A CLEAN scan is the strongest evidence a scanner produces and carries no
    // finding to bind from, so the obligations are stated rather than inferred.
    const root = workspace();
    await recordScan(root, ["--discharges", "FR-001-AC-1"]);
    const bindings = JSON.parse(
      readFileSync(join(root, "spec", "evidence", "bindings.json"), "utf8"),
    ) as { bindings: Array<{ obligation: string; suite: string }> };
    expect(bindings.bindings).toContainEqual(
      expect.objectContaining({
        obligation: "FR-001-AC-1",
        suite: "SUITE-SCAN",
      }),
    );
  });

  // Trace: FR-034-AC-17, TC-193
  it("binds nothing when the scan evaluated no rules", async () => {
    // Binding on a rule-less scan would put the store's strongest claim behind
    // its weakest evidence.
    const root = workspace();
    const results = join(root, "empty.sarif");
    writeFileSync(
      results,
      JSON.stringify({
        version: "2.1.0",
        runs: [
          { tool: { driver: { name: "semgrep", rules: [] } }, results: [] },
        ],
      }),
    );
    await EvidenceRecord.run(
      [
        "--repo",
        root,
        "--suite",
        "SUITE-SCAN",
        "--commit",
        "f".repeat(40),
        "--tool",
        "semgrep 1.2.3",
        "--adapter",
        "sarif",
        "--results",
        results,
        "--discharges",
        "FR-001-AC-1",
      ],
      config,
    );
    // Assert the claim, not the file: what must not happen is a BINDING.
    const path = join(root, "spec", "evidence", "bindings.json");
    const bindings = existsSync(path)
      ? (JSON.parse(readFileSync(path, "utf8")) as { bindings: unknown[] })
          .bindings
      : [];
    expect(bindings).toEqual([]);
  });

  // Trace: FR-034-AC-18, TC-194
  it("enumerates a suite that recorded only scans", async () => {
    // `listRecordedSuites` read `runs/` alone, so a scan-only suite was
    // invisible to every caller that enumerates — the auditor included.
    const root = workspace();
    await recordScan(root);
    expect(listRecordedSuites(root)).toContain("SUITE-SCAN");
    expect(latestScan(root, "SUITE-SCAN")?.tool).toBe("semgrep 1.2.3");
  });

  // Trace: FR-034-AC-19, TC-195
  it("collects superseded scans, so the store does not grow without bound", async () => {
    const root = workspace();
    await recordScan(root);
    const results = join(root, "scan.sarif");
    await EvidenceRecord.run(
      [
        "--repo",
        root,
        "--suite",
        "SUITE-SCAN",
        "--commit",
        "1".repeat(40),
        "--tool",
        "semgrep 1.2.3",
        "--adapter",
        "sarif",
        "--results",
        results,
        "--timestamp",
        "2027-01-01T00:00:00Z",
      ],
      config,
    );
    const deleted = gc(root);
    // The newest scan is kept; the superseded one is collected.
    expect(deleted.some((p) => p.includes("eeeeeeeeeeee.json"))).toBe(true);
    expect(latestScan(root, "SUITE-SCAN")?.commit).toBe("1".repeat(40));
  });
});

// Trace: FR-034-AC-20, TC-196
it("tells a tool reporting ZERO rules from a tool reporting no count", () => {
  // The distinction FR-034 turns on. The adapter defaulted the counter to 0 and
  // omitted the field when it was 0, which erased exactly this: a scan
  // declaring `rules: []` read as a tool that had said nothing, so the vacuity
  // check stayed silent on the one input it exists to catch.
  const declaredZero = parseSarif(
    JSON.stringify({
      version: "2.1.0",
      runs: [{ tool: { driver: { name: "semgrep", rules: [] } }, results: [] }],
    }),
  );
  expect(declaredZero.rulesEvaluated).toBe(0);

  const saidNothing = parseSarif(
    JSON.stringify({
      version: "2.1.0",
      runs: [{ tool: { driver: { name: "semgrep" } }, results: [] }],
    }),
  );
  expect(saidNothing.rulesEvaluated).toBeUndefined();
});
