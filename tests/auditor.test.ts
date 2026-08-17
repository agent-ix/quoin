/**
 * FR-032 — the evidence auditor (TC-137..TC-144).
 *
 * A trace link is a string match that never expires. These tests are about the
 * three ways evidence rots invisibly, and about the auditor refusing to be
 * fooled by evidence that merely *exists*.
 */

import { describe, expect, it } from "vitest";

import {
  audit,
  delta,
  ratchet,
  type AuditInput,
} from "../src/auditor/index.js";
import type { MethodCatalog } from "../src/advisor/index.js";
import type { Binding, RunRecord } from "../src/evidence/index.js";
import type { Obligation } from "../src/quire/index.js";

const HASH_A = "a".repeat(64);
const HASH_B = "b".repeat(64);
const COMMIT = "abcdef0123456789";

function obligation(over: Partial<Obligation> = {}): Obligation {
  return {
    source: "acceptance-criterion",
    id: "FR-001-AC-1",
    document: "spec/functional/FR-001.md",
    statement: "The system rejects a malformed token.",
    statement_hash: HASH_A,
    ...over,
  };
}

function binding(over: Partial<Binding> = {}): Binding {
  return {
    obligation: "FR-001-AC-1",
    statementHashAtBinding: HASH_A,
    suite: "SUITE-001",
    commit: COMMIT,
    symbols: ["tests::tc001"],
    ...over,
  };
}

function run(over: Partial<RunRecord> = {}): RunRecord {
  return {
    schemaVersion: 1,
    suite: "SUITE-001",
    commit: COMMIT,
    tool: "cargo test",
    timestamp: "2026-08-17T00:00:00Z",
    entries: [{ symbol: "tests::tc001", outcome: "pass" }],
    ...over,
  };
}

function input(over: Partial<AuditInput> = {}): AuditInput {
  return {
    obligations: [obligation()],
    bindings: [binding()],
    runs: [run()],
    headCommit: COMMIT,
    ...over,
  };
}

describe("TC-137 healthy evidence produces no finding", () => {
  it("reports the obligation as healthy", () => {
    const report = audit(input());
    expect(report.findings).toEqual([]);
    expect(report.healthy).toEqual(["FR-001-AC-1"]);
  });
});

describe("TC-138 a suspect link is the highest-severity finding", () => {
  it("fires when the statement changed after binding, and says so", () => {
    const report = audit(
      input({ obligations: [obligation({ statement_hash: HASH_B })] }),
    );
    expect(report.findings).toHaveLength(1);
    const [finding] = report.findings;
    expect(finding.kind).toBe("suspect-link");
    // The requirement moved and the evidence did not follow, while the matrix
    // still reads as covered — the failure this whole program exists to close.
    expect(finding.severity).toBe("high");
    expect(finding.summary).toContain("reworded");
    expect(report.healthy).toEqual([]);
  });
});

describe("TC-139 stale evidence", () => {
  it("is high when the binding names a suite with no recorded run", () => {
    const report = audit(input({ runs: [] }));
    expect(report.findings[0].kind).toBe("stale-evidence");
    // The binding claims evidence that is not in the store — worse than an
    // old run, which at least happened.
    expect(report.findings[0].severity).toBe("high");
  });

  it("is medium when the run is merely behind HEAD", () => {
    const report = audit(input({ headCommit: "1111111111111111" }));
    const stale = report.findings.find((f) => f.kind === "stale-evidence");
    expect(stale?.severity).toBe("medium");
    expect(stale?.summary).toContain("not at HEAD");
  });
});

describe("TC-140 vacuous evidence", () => {
  it("fires when every bound symbol was skipped", () => {
    const report = audit(
      input({
        runs: [run({ entries: [{ symbol: "tests::tc001", outcome: "skip" }] })],
      }),
    );
    const [finding] = report.findings;
    expect(finding.kind).toBe("vacuous-evidence");
    expect(finding.severity).toBe("high");
    expect(finding.summary).toContain(
      "reads as covered and nothing was verified",
    );
  });

  it("fires when a bound symbol is absent from the run entirely", () => {
    // The strongest form: the binding names evidence the suite did not produce.
    const report = audit({
      ...input(),
      runs: [run({ entries: [] })],
    });
    expect(report.findings[0].kind).toBe("vacuous-evidence");
  });

  it("does not fire when at least one bound symbol actually passed", () => {
    const report = audit(
      input({
        bindings: [binding({ symbols: ["tests::tc001", "tests::tc002"] })],
        runs: [
          run({
            entries: [
              { symbol: "tests::tc001", outcome: "pass" },
              { symbol: "tests::tc002", outcome: "skip" },
            ],
          }),
        ],
      }),
    );
    expect(report.findings).toEqual([]);
  });
});

describe("TC-141 an obligation with no binding is undischarged", () => {
  it("reports it at medium, not high", () => {
    const report = audit(input({ bindings: [] }));
    const [finding] = report.findings;
    expect(finding.kind).toBe("undischarged");
    // An unwritten test is ordinary work in progress. What is high is evidence
    // that CLAIMS to exist and does not hold.
    expect(finding.severity).toBe("medium");
  });
});

describe("TC-142 method conformance", () => {
  const catalog: MethodCatalog = {
    methods: [
      {
        id: "sast",
        name: "SAST",
        class: "Analysis",
        definition: "d",
        applicability: {},
        tooling: [],
        moduleName: "m",
      },
      {
        id: "unit-testing",
        name: "Unit",
        class: "Test",
        definition: "d",
        applicability: {},
        tooling: [],
        moduleName: "m",
      },
    ],
    duplicates: [],
  };

  it("flags an Analysis obligation discharged by a test run", () => {
    const report = audit(
      input({ obligations: [obligation({ method: "sast" })], catalog }),
    );
    const finding = report.findings.find(
      (f) => f.kind === "method-conformance",
    );
    expect(finding?.summary).toContain("not a test is not discharged by one");
  });

  it("accepts a Test obligation discharged by a test run", () => {
    const report = audit(
      input({ obligations: [obligation({ method: "Test" })], catalog }),
    );
    expect(report.findings).toEqual([]);
  });

  it("asks nothing when no catalog is available", () => {
    // An absent catalog means the question cannot be asked, which is different
    // from the answer being yes.
    const report = audit(
      input({ obligations: [obligation({ method: "sast" })] }),
    );
    expect(report.findings.some((f) => f.kind === "method-conformance")).toBe(
      false,
    );
  });
});

describe("TC-143 multiplicity", () => {
  it("flags a critical obligation whose evidence is all one suite", () => {
    const report = audit(
      input({
        obligations: [obligation({ criticality: "P0" })],
        multiplicityRequires: ["P0"],
      }),
    );
    const finding = report.findings.find(
      (f) => f.kind === "insufficient-multiplicity",
    );
    expect(finding?.summary).toContain("two independent methods");
  });

  it("accepts two bindings from two different suites", () => {
    // Independent means two SUITES: two symbols in one suite share a harness,
    // fixtures and a failure mode, so counting them as two would let one broken
    // assumption look like corroboration.
    const report = audit(
      input({
        obligations: [obligation({ criticality: "P0" })],
        bindings: [binding(), binding({ suite: "SUITE-002" })],
        runs: [run(), run({ suite: "SUITE-002" })],
        multiplicityRequires: ["P0"],
      }),
    );
    expect(
      report.findings.some((f) => f.kind === "insufficient-multiplicity"),
    ).toBe(false);
  });

  it("does not apply when criticality is below the threshold", () => {
    const report = audit(
      input({
        obligations: [obligation({ criticality: "P2" })],
        multiplicityRequires: ["P0"],
      }),
    );
    expect(report.findings).toEqual([]);
  });
});

describe("TC-144 ratchet and per-PR delta", () => {
  it("reports only violations absent from the baseline", () => {
    const report = audit(
      input({
        obligations: [
          obligation({ statement_hash: HASH_B }),
          obligation({ id: "FR-001-AC-2" }),
        ],
        bindings: [binding()],
      }),
    );
    expect(report.findings).toHaveLength(2);

    // A gate that fails on the whole existing backlog gets disabled within a
    // week, which is why the baseline exists.
    const remaining = ratchet(report, {
      suspect: ["FR-001-AC-1"],
      undischarged: [],
    });
    expect(remaining).toHaveLength(1);
    expect(remaining[0].obligation).toBe("FR-001-AC-2");
  });

  it("computes what a PR added and resolved", () => {
    const before = audit(input({ bindings: [] }));
    const after = audit(input());
    const changed = delta(before, after);
    expect(changed.added).toEqual([]);
    expect(changed.resolved.map((f) => f.kind)).toEqual(["undischarged"]);
  });
});

describe("the report is deterministic", () => {
  it("orders findings by obligation then kind", () => {
    const report = audit(
      input({
        obligations: [
          obligation({ id: "FR-001-AC-3", statement_hash: HASH_B }),
          obligation({ id: "FR-001-AC-2" }),
          obligation({ id: "FR-001-AC-1" }),
        ],
        bindings: [binding({ obligation: "FR-001-AC-3" })],
      }),
    );
    const ids = report.findings.map((f) => f.obligation);
    expect(ids).toEqual([...ids].sort());
    expect(audit(input())).toEqual(audit(input()));
  });
});
