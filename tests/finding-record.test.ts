/**
 * FR-034 — FindingRecord where it is consumed.
 *
 * The finding-shaped adapters themselves (SARIF, cargo-audit) and the
 * rules-evaluated distinction they turn on are `quoin-evidence`'s, asserted in
 * `rust/crates/quoin-evidence/src/adapters/` and by the golden corpus
 * (quoin#458). What stays here is what only this tree can see: the pure
 * auditor's reading of a scan, and the command that writes one.
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
import { auditInputs, gc, type FindingRecord } from "../src/core/evidence.js";

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
    // `rulesEvaluated: 0` means vacuous is decided by `quoin-evidence` and
    // reaches the auditor as `vacuousScanSuites` (quoin#458). That half of the
    // criterion is asserted against the real binary by
    // `tc_458_150_audit_inputs_names_the_scans_that_evaluated_no_rules`; this
    // half is what the auditor does with the answer. The scan still carries
    // the count so the two halves are stated over the same record.
    const report = audit({
      obligations: [obligation],
      bindings: [binding],
      runs: [],
      scans: [scan({ rulesEvaluated: 0 })],
      vacuousScanSuites: ["SUITE-SCAN"],
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

  // Trace: FR-034-AC-16
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

  // Trace: FR-034-AC-17
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

  // Trace: FR-034-AC-18
  it("enumerates a suite that recorded only scans", async () => {
    // `listRecordedSuites` read `runs/` alone, so a scan-only suite was
    // invisible to every caller that enumerates — the auditor included.
    const root = workspace();
    await recordScan(root);
    const scans = auditInputs(root).scans;
    expect(scans.map((s) => s.suite)).toContain("SUITE-SCAN");
    expect(scans.find((s) => s.suite === "SUITE-SCAN")?.tool).toBe(
      "semgrep 1.2.3",
    );
  });

  // Trace: FR-034-AC-19
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
    expect(
      auditInputs(root).scans.find((s) => s.suite === "SUITE-SCAN")?.commit,
    ).toBe("1".repeat(40));
  });
});
