/**
 * FR-032 — the evidence auditor (TC-137..TC-148).
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
        evidenceKind: "Static",
        applicability: {},
        tooling: [],
        moduleName: "m",
      },
      {
        id: "unit-testing",
        name: "Unit",
        class: "Test",
        definition: "d",
        evidenceKind: "Unit",
        applicability: {},
        tooling: [],
        moduleName: "m",
      },
    ],
    duplicates: [],
  };

  it("compares kind to kind, not entry count", () => {
    // The old test was `run.entries.length > 0`, read as "this was a test run"
    // — true of a transcribed inspection too, so every Analysis obligation
    // recorded through `quoin evidence record` was flagged (#105).
    const report = audit(
      input({
        obligations: [obligation({ method: "sast" })],
        runs: [run({ evidenceKind: "Unit" })],
        catalog,
      }),
    );
    const finding = report.findings.find(
      (f) => f.kind === "method-conformance",
    );
    expect(finding?.summary).toContain("evidence kind is Static");
    expect(finding?.summary).toContain("a Unit run in SUITE-001");
  });

  it("accepts a run whose kind IS the method's kind", () => {
    const report = audit(
      input({
        obligations: [obligation({ method: "sast" })],
        runs: [run({ evidenceKind: "Static" })],
        catalog,
      }),
    );
    expect(report.findings).toEqual([]);
  });

  it("says nothing when the run declares no kind", () => {
    // An undeclared kind means the question cannot be asked, which is a
    // different answer from "it conformed". Guessing here is what produced the
    // false positive on every recorded inspection.
    const report = audit(
      input({ obligations: [obligation({ method: "sast" })], catalog }),
    );
    expect(report.findings).toEqual([]);
  });

  it("reports a method no catalog carries rather than skipping it", () => {
    // `declaredClasses.size === 0` used to return null — a silent skip — so the
    // requirements whose verification is LEAST well defined were exactly the
    // ones nothing questioned. Measured: 55 of 577 across the ecosystem (#105).
    const report = audit(
      input({ obligations: [obligation({ method: "CI Gate" })], catalog }),
    );
    expect(report.findings.map((f) => f.kind)).toEqual(["unknown-method"]);
    expect(report.findings[0].summary).toContain("CI Gate");
  });

  it("accepts a Test obligation discharged by a Unit run", () => {
    const report = audit(
      input({
        obligations: [obligation({ method: "Test" })],
        runs: [run({ evidenceKind: "Unit" })],
        catalog,
      }),
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
      accepted: ["suspect-link:FR-001-AC-1"],
    });
    expect(remaining).toHaveLength(1);
    expect(remaining[0].obligation).toBe("FR-001-AC-2");
  });

  it("can baseline EVERY finding kind, not two named buckets", () => {
    // `stale-evidence`, `vacuous-evidence`, `method-conformance`,
    // `unknown-method` and `insufficient-multiplicity` could never appear in a
    // baseline, so `--ratchet` reported the whole existing backlog for all five
    // — the outcome ratchet mode exists to prevent (#105).
    const report = audit(
      input({
        obligations: [obligation()],
        bindings: [binding({ suite: "SUITE-404" })],
      }),
    );
    expect(report.findings.map((f) => f.kind)).toEqual(["stale-evidence"]);
    expect(
      ratchet(report, { accepted: ["stale-evidence:FR-001-AC-1"] }),
    ).toEqual([]);
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

describe("TC-145 one obligation, two suites (FR-032-AC-8)", () => {
  // While `bind()` keyed on the obligation alone, the second suite's binding
  // OVERWROTE the first — so this set could never hold two suites, and
  // `insufficient-multiplicity` fired on every demanding obligation with no
  // way to clear it (agent-ix/quoin#102). The auditor now folds over the group.
  const twoSuites = (over: Partial<Binding> = {}) => [
    binding(),
    binding({ suite: "SUITE-002", symbols: ["bench::tc900"], ...over }),
  ];
  const twoRuns = () => [
    run(),
    run({
      suite: "SUITE-002",
      entries: [{ symbol: "bench::tc900", outcome: "pass" }],
    }),
  ];

  it("clears the multiplicity finding two independent suites satisfy", () => {
    const report = audit(
      input({
        obligations: [obligation({ criticality: "P0" })],
        bindings: twoSuites(),
        runs: twoRuns(),
        multiplicityRequires: ["P0"],
      }),
    );
    expect(report.findings).toEqual([]);
    expect(report.healthy).toEqual(["FR-001-AC-1"]);
  });

  it("still reports it when both bindings name the same suite", () => {
    const report = audit(
      input({
        obligations: [obligation({ criticality: "P0" })],
        multiplicityRequires: ["P0"],
      }),
    );
    expect(report.findings.map((f) => f.kind)).toEqual([
      "insufficient-multiplicity",
    ]);
  });

  it("reports a suspect link when one suite bound before the reword", () => {
    // A sibling that re-bound after the reword does not absolve the one that
    // did not: that binding still claims to discharge a statement it never saw.
    const report = audit(
      input({
        bindings: twoSuites({ statementHashAtBinding: HASH_B }),
        runs: twoRuns(),
      }),
    );
    expect(report.findings.map((f) => f.kind)).toEqual(["suspect-link"]);
    expect(report.findings[0].summary).toContain("SUITE-002");
  });

  it("is not vacuous when one suite skipped and the other ran", () => {
    const report = audit(
      input({
        bindings: twoSuites(),
        runs: [
          run({ entries: [{ symbol: "tests::tc001", outcome: "skip" }] }),
          run({
            suite: "SUITE-002",
            entries: [{ symbol: "bench::tc900", outcome: "pass" }],
          }),
        ],
      }),
    );
    // One suite that genuinely ran is evidence. Calling the obligation vacuous
    // because a second suite skipped is the false alarm that gets a check
    // switched off.
    expect(report.findings).toEqual([]);
  });

  it("is vacuous only when every symbol in every suite was skipped", () => {
    const report = audit(
      input({
        bindings: twoSuites(),
        runs: [
          run({ entries: [{ symbol: "tests::tc001", outcome: "skip" }] }),
          run({
            suite: "SUITE-002",
            entries: [{ symbol: "bench::tc900", outcome: "skip" }],
          }),
        ],
      }),
    );
    expect(report.findings.map((f) => f.kind)).toEqual(["vacuous-evidence"]);
    expect(report.findings[0].summary).toContain("SUITE-001:tests::tc001");
    expect(report.findings[0].summary).toContain("SUITE-002:bench::tc900");
  });
});

describe("TC-147 every finding kind can be baselined (FR-032-AC-11)", () => {
  // The baseline used to be two named buckets. `stale-evidence`,
  // `vacuous-evidence`, `method-conformance`, `unknown-method` and
  // `insufficient-multiplicity` could never appear in one, so `--ratchet`
  // reported the whole existing backlog for five of the six kinds — the
  // outcome ratchet mode exists to prevent (agent-ix/quoin#105).
  it("accepts each kind by its own key", () => {
    const kinds: Array<[string, AuditInput]> = [
      ["undischarged", input({ bindings: [] })],
      [
        "suspect-link",
        input({ obligations: [obligation({ statement_hash: HASH_B })] }),
      ],
      [
        "stale-evidence",
        input({ bindings: [binding({ suite: "SUITE-404" })] }),
      ],
      [
        "vacuous-evidence",
        input({
          runs: [
            run({ entries: [{ symbol: "tests::tc001", outcome: "skip" }] }),
          ],
        }),
      ],
    ];
    for (const [kind, given] of kinds) {
      const report = audit(given);
      expect(report.findings.map((f) => f.kind)).toContain(kind);
      expect(
        ratchet(report, { accepted: [`${kind}:FR-001-AC-1`] }).map(
          (f) => f.kind,
        ),
      ).not.toContain(kind);
    }
  });
});
