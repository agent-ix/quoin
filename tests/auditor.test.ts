/**
 * FR-032 — the evidence auditor (TC-137..TC-148).
 *
 * A trace link is a string match that never expires. These tests are about the
 * three ways evidence rots invisibly, and about the auditor refusing to be
 * fooled by evidence that merely *exists*.
 */

import { mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

import { describe, expect, it } from "vitest";

import {
  audit,
  delta,
  findingKey,
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
    mockInspectionSuites: ["SUITE-001"],
    ...over,
  };
}

describe("TC-137 healthy evidence produces no finding", () => {
  // TC-137
  it("reports the obligation as healthy", () => {
    const report = audit(input());
    expect(report.findings).toEqual([]);
    expect(report.healthy).toEqual(["FR-001-AC-1"]);
  });
});

describe("TC-138 a suspect link is the highest-severity finding", () => {
  // TC-138
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
  // TC-139
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
  // TC-140
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
  // TC-141
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

  // TC-142
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

  // TC-146
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
  // TC-143
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
  // TC-144
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

  // TC-145
  it("clears the multiplicity finding two independent suites satisfy", () => {
    const report = audit(
      input({
        obligations: [obligation({ criticality: "P0" })],
        bindings: twoSuites(),
        runs: twoRuns(),
        mockInspectionSuites: ["SUITE-001", "SUITE-002"],
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
  // TC-147
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

describe("TC-219 mutation score as the acceptance-criteria oracle", () => {
  // `RunEntry.score` is generic — "a mutation score, a coverage percentage, a
  // measured latency" — so what makes a score a MUTATION score is the entry's
  // own `metric`, declared by the adapter at the point of recording (#138).
  // No catalog and no tool allowlist are involved.
  const scored = (score: number, over: Partial<RunRecord> = {}) =>
    run({
      tool: "cargo-mutants",
      entries: [
        {
          symbol: "tests::tc001",
          outcome: "pass",
          score,
          metric: "mutation-score",
        },
      ],
      ...over,
    });

  // Trace: FR-039-AC-1
  // TC-219
  it("says nothing until a floor is declared", () => {
    // The CR-008 lesson, applied before it could bite: a built-in floor is a
    // rule nobody chose, firing on everything the moment a criticality column
    // appears. Unset means silent.
    const report = audit(
      input({
        obligations: [obligation({ criticality: "P0" })],
        runs: [scored(0.1)],
      }),
    );
    expect(
      report.findings.some((f) => f.kind.startsWith("insufficient-mutation")),
    ).toBe(false);
    expect(report.healthy).toEqual(["FR-001-AC-1"]);
  });

  // Trace: FR-039-AC-2
  it("flags a bound score below the declared floor", () => {
    const report = audit(
      input({
        obligations: [obligation({ criticality: "P0" })],
        runs: [scored(0.55)],
        mutationFloor: { P0: 0.8 },
      }),
    );
    const finding = report.findings.find(
      (f) => f.kind === "insufficient-mutation-score",
    );
    expect(finding?.severity).toBe("medium");
    expect(finding?.summary).toContain("0.55");
  });

  // Trace: FR-039-AC-3
  it("accepts a score at the floor", () => {
    // At, not above: a floor of 0.8 that rejects exactly 0.8 is a floor of
    // 0.8-plus-epsilon, and nobody can tell which from the flag.
    const report = audit(
      input({
        obligations: [obligation({ criticality: "P0" })],
        runs: [scored(0.8)],
        mutationFloor: { P0: 0.8 },
      }),
    );
    expect(report.healthy).toEqual(["FR-001-AC-1"]);
  });

  // Trace: FR-039-AC-4
  it("judges on the WORST bound symbol, not the mean", () => {
    // Averaging lets a well-tested symbol carry one whose mutants all survive,
    // which is precisely the case the threshold exists to find.
    const report = audit(
      input({
        obligations: [obligation({ criticality: "P0" })],
        runs: [
          run({
            tool: "cargo-mutants",
            entries: [
              {
                symbol: "tests::tc001",
                outcome: "pass",
                score: 1,
                metric: "mutation-score",
              },
              {
                symbol: "tests::tc002",
                outcome: "pass",
                score: 0.2,
                metric: "mutation-score",
              },
            ],
          }),
        ],
        mutationFloor: { P0: 0.8 },
      }),
    );
    expect(
      report.findings.find((f) => f.kind === "insufficient-mutation-score")
        ?.summary,
    ).toContain("0.2");
  });

  // Trace: FR-039-AC-5
  it("reports a demanded floor that nothing measured", () => {
    // Distinct from `undischarged`: this obligation is bound, its run passed,
    // and nothing says the tests detect anything. A threshold that cannot be
    // evaluated is not a threshold met.
    const report = audit(
      input({
        obligations: [obligation({ criticality: "P0" })],
        mutationFloor: { P0: 0.8 },
      }),
    );
    const finding = report.findings.find(
      (f) => f.kind === "unmeasured-mutation-score",
    );
    expect(finding?.summary).toContain("no run bound to it records one");
  });

  // Trace: FR-039-AC-6
  it("does not read a skipped symbol's absent score as zero", () => {
    // A skipped symbol carries no measurement. Treating it as 0 would fail the
    // obligation for a test nobody ran rather than one that failed to
    // discriminate — a different problem with a different remedy.
    const report = audit(
      input({
        obligations: [obligation({ criticality: "P0" })],
        runs: [
          run({
            tool: "cargo-mutants",
            entries: [
              {
                symbol: "tests::tc001",
                outcome: "pass",
                score: 0.9,
                metric: "mutation-score",
              },
              { symbol: "tests::tc002", outcome: "skip" },
            ],
          }),
        ],
        mutationFloor: { P0: 0.8 },
      }),
    );
    expect(report.healthy).toEqual(["FR-001-AC-1"]);
  });

  // Trace: FR-039-AC-10
  it("does not read a latency or an unlabelled score as a mutation score", () => {
    // `RunEntry.score` is deliberately generic. Reading every scored entry
    // compares a p95 latency in milliseconds against a floor of 0.8 and reports
    // the obligation as failing — which the first draft of this check did. The
    // entry's own `metric` is what says what the number measures (#138), and
    // an entry declaring another metric — or none — is not a mutation score.
    const report = audit(
      input({
        obligations: [obligation({ criticality: "P0" })],
        runs: [
          run({
            tool: "criterion",
            // The BOUND symbol, so the binding is satisfied and the only
            // open question is what its score means.
            entries: [
              {
                symbol: "tests::tc001",
                outcome: "pass",
                score: 4.2,
                metric: "latency-ms",
              },
            ],
          }),
        ],
        mutationFloor: { P0: 0.8 },
      }),
    );
    // Not `insufficient` — nothing labelled as a mutation score was measured.
    expect(
      report.findings.find((f) => f.kind.startsWith("insufficient-mutation")),
    ).toBeUndefined();
    expect(
      report.findings.find((f) => f.kind === "unmeasured-mutation-score"),
    ).toBeDefined();
  });

  // Trace: FR-039-AC-7
  it("applies only to the criticality the floor names", () => {
    const report = audit(
      input({
        obligations: [obligation({ criticality: "P2" })],
        runs: [scored(0.1)],
        mutationFloor: { P0: 0.8 },
      }),
    );
    expect(report.healthy).toEqual(["FR-001-AC-1"]);
  });

  // ── TC-269: the metric discriminator (#138) ──

  // Trace: FR-039-AC-11
  // TC-269
  it("judges a labelled score from a tool no catalog lists", () => {
    // The tool allowlist by another name: a consumer using a mutation tool the
    // catalog did not list got `unmeasured-mutation-score` while holding a
    // real score. The entry's `metric` is the discriminator now, and the tool
    // string contributes nothing.
    const report = audit(
      input({
        obligations: [obligation({ criticality: "P0" })],
        runs: [
          run({
            tool: "bespoke-mutator 1.0",
            entries: [
              {
                symbol: "tests::tc001",
                outcome: "fail",
                score: 0.4,
                metric: "mutation-score",
              },
            ],
          }),
        ],
        mutationFloor: { P0: 0.8 },
      }),
    );
    expect(
      report.findings.find((f) => f.kind === "insufficient-mutation-score")
        ?.summary,
    ).toContain("0.4");
  });

  // Trace: FR-039-AC-12
  // TC-269
  it("needs no catalog: the entry's declared metric is the whole answer", () => {
    // The old mechanism named `mutation-testing` — a method id, module data —
    // inside the engine, and went silent without a catalog. The discriminator
    // lives on the entry, so the catalog stays out of the engine entirely.
    const clean = audit(
      input({
        obligations: [obligation({ criticality: "P0" })],
        runs: [scored(0.95)],
        mutationFloor: { P0: 0.8 },
      }),
    );
    expect(clean.healthy).toEqual(["FR-001-AC-1"]);

    const failing = audit(
      input({
        obligations: [obligation({ criticality: "P0" })],
        runs: [scored(0.4)],
        mutationFloor: { P0: 0.8 },
      }),
    );
    expect(failing.findings.map((f) => f.kind)).toEqual([
      "insufficient-mutation-score",
    ]);
  });

  // TC-269
  it("an unlabelled score is not a mutation score — no fallback read of the tool name", () => {
    // Migration is migration, not dual-read: a pre-#138 record whose entries
    // carry no `metric` does not satisfy a mutation floor even when its tool
    // string says `cargo-mutants`. Re-record through the adapter, which now
    // labels every entry.
    const report = audit(
      input({
        obligations: [obligation({ criticality: "P0" })],
        runs: [
          run({
            tool: "cargo-mutants 25.0.0",
            entries: [{ symbol: "tests::tc001", outcome: "pass", score: 0.9 }],
          }),
        ],
        mutationFloor: { P0: 0.8 },
      }),
    );
    expect(report.findings.map((f) => f.kind)).toEqual([
      "unmeasured-mutation-score",
    ]);
  });
});

describe("TC-220 --mutation-floor is parsed, and a bad one is refused", () => {
  // Trace: FR-039-AC-8
  // TC-220
  it("reads <criticality>=<ratio> pairs", async () => {
    const { parseMutationFloor } =
      await import("../src/commands/evidence/audit.js");
    const fail = (m: string): never => {
      throw new Error(m);
    };
    expect(parseMutationFloor(["P0=0.8", "P1=0.6"], fail)).toEqual({
      P0: 0.8,
      P1: 0.6,
    });
    expect(parseMutationFloor(undefined, fail)).toBeUndefined();
    expect(parseMutationFloor([], fail)).toBeUndefined();
  });

  // Trace: FR-039-AC-9
  it("refuses a percentage, which would fail every obligation forever", async () => {
    // `--mutation-floor P0=80` is the natural thing to type and is a floor
    // nothing can reach. Silently accepting it reports every P0 as failing and
    // reads like a real finding.
    const { parseMutationFloor } =
      await import("../src/commands/evidence/audit.js");
    const fail = (m: string): never => {
      throw new Error(m);
    };
    expect(() => parseMutationFloor(["P0=80"], fail)).toThrow(
      /ratio in \[0, 1\]/,
    );
  });

  // Trace: FR-039-AC-9
  it("refuses a malformed entry rather than ignoring it", async () => {
    // Ignoring means the operator asked for a threshold and gets a clean report
    // saying nothing was below it. A floor that silently does not apply reads
    // as a passing gate.
    const { parseMutationFloor } =
      await import("../src/commands/evidence/audit.js");
    const fail = (m: string): never => {
      throw new Error(m);
    };
    expect(() => parseMutationFloor(["P0"], fail)).toThrow(/expects/);
    expect(() => parseMutationFloor(["P0=high"], fail)).toThrow(/expects/);
  });
});

describe("TC-148 the audit command reads the catalog from --module", () => {
  // TC-148
  it("loads only what the named module declares, not the installed roots", async () => {
    // The disagreement this closes: obligations derived from one catalog and
    // conformance checked against another, so a method the module declares
    // reads as `unknown-method`. FR-032-AC-12.
    const { loadMethodCatalog } = await import("../src/advisor/index.js");
    const root = mkdtempSync(join(tmpdir(), "quoin-tc148-"));
    writeFileSync(
      join(root, "manifest.yaml"),
      [
        "manifest_version: 1",
        "name: tc148-fixture",
        "version: 0.0.0",
        "verification_catalog:",
        "  tc148-only-method:",
        "    verification_class: Test",
        "    evidence_kind: Unit",
        "    summary: a method that exists in no installed module",
        "",
      ].join("\n"),
    );

    const scoped = loadMethodCatalog([root]);
    expect(scoped.methods.map((m) => m.id)).toContain("tc148-only-method");

    // And the default roots do not carry it, so the assertion above is a
    // measurement of the flag rather than of what happens to be installed.
    const installed = loadMethodCatalog();
    expect(installed.methods.map((m) => m.id)).not.toContain(
      "tc148-only-method",
    );

    rmSync(root, { recursive: true, force: true });
  });

  // TC-148
  it("passes the flag through from the command, so the two agree", () => {
    // The behavioural half above proves the loader honours a module root; this
    // proves the command hands it one. Without it a wiring regression would
    // pass every other test in this file.
    const source = readFileSync(
      join(
        dirname(fileURLToPath(import.meta.url)),
        "..",
        "src/commands/evidence/audit.ts",
      ),
      "utf8",
    );
    expect(source).toContain(
      "catalog: loadMethodCatalog(flags.module ? [flags.module] : undefined)",
    );
  });
});

describe("TC-264 unknown-method fires on unbound obligations (FR-032-AC-14)", () => {
  const catalog: MethodCatalog = {
    methods: [
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

  // TC-264
  it("an unbound obligation with an uncatalogued method yields BOTH undischarged and unknown-method", () => {
    // The battle-test shape (#165): a repository with no evidence store at all
    // — 1,107 findings, every one `undischarged`, zero `unknown-method`,
    // against 90+ Verification values in no catalog. The one check that pays
    // off on day one of adoption was gated behind having already adopted.
    const report = audit(
      input({
        obligations: [obligation({ method: "CI Measurement" })],
        bindings: [],
        runs: [],
        catalog,
      }),
    );
    expect(report.findings.map((f) => f.kind)).toEqual([
      "undischarged",
      "unknown-method",
    ]);
    expect(
      report.findings.find((f) => f.kind === "unknown-method")?.summary,
    ).toContain("CI Measurement");
    expect(report.healthy).toEqual([]);
  });

  // TC-264
  it("an unbound obligation with a catalogued method is only undischarged", () => {
    const report = audit(
      input({
        obligations: [obligation({ method: "unit-testing" })],
        bindings: [],
        runs: [],
        catalog,
      }),
    );
    expect(report.findings.map((f) => f.kind)).toEqual(["undischarged"]);
  });

  // TC-264
  it("with no catalog the question is not asked, bound or not", () => {
    const report = audit(
      input({
        obligations: [obligation({ method: "CI Measurement" })],
        bindings: [],
        runs: [],
      }),
    );
    expect(report.findings.map((f) => f.kind)).toEqual(["undischarged"]);
  });
});

describe("mocked confirmation (#204)", () => {
  const obligation = {
    source: "acceptance-criterion",
    id: "FR-017-AC-7",
    document: "spec/functional/FR-017.md",
    statement:
      "A destructive action shall require a trusted-UI confirmation before it proceeds.",
    statement_hash: "h1",
  };
  const binding = {
    obligation: "FR-017-AC-7",
    statementHashAtBinding: "h1",
    suite: "SUITE-001",
    commit: "abc",
    symbols: ["tests::confirms"],
  };
  const run = {
    schemaVersion: 1,
    suite: "SUITE-001",
    commit: "abc",
    tool: "cargo-test 1.0",
    entries: [{ symbol: "tests::confirms", outcome: "pass" as const }],
  };

  it("TC-936 an obligation discharged only by a mocked stand-in is reported", () => {
    // TC-936
    // The measured case: the trusted-UI confirmation had NO implementation,
    // and the test passed by injecting `Confirmation::allow()` — mocking
    // exactly the behaviour the criterion verifies.
    const report = audit({
      obligations: [obligation],
      bindings: [binding],
      runs: [run],
      injections: [
        {
          suite: "SUITE-001",
          symbol: "tests::confirms",
          injects: ["Confirmation::allow"],
        },
      ],
    });
    const found = report.findings.filter(
      (f) => f.kind === "mocked-confirmation",
    );
    expect(found).toHaveLength(1);
    expect(found[0].obligation).toBe("FR-017-AC-7");
    expect(found[0].severity).toBe("medium");
    expect(found[0].summary).toContain("Confirmation::allow");
    expect(found[0].summary).toContain("whether or not that behaviour exists");
  });

  it("TC-937 a mock unrelated to the statement's subject is not reported", () => {
    // TC-937
    // Tests legitimately mock clocks, filesystems and networks. What this is
    // looking for is the narrow case where the mock's NAME is the statement's
    // subject.
    const report = audit({
      obligations: [obligation],
      bindings: [binding],
      runs: [run],
      injections: [
        {
          suite: "SUITE-001",
          symbol: "tests::confirms",
          injects: ["FakeClock"],
        },
      ],
    });
    expect(
      report.findings.filter((f) => f.kind === "mocked-confirmation"),
    ).toHaveLength(0);
  });

  it("TC-938 one real suite alongside a mocked one is not reported", () => {
    // TC-938
    // Ordinary test design: a suite stands in a dependency while another
    // exercises the real path. Flagging it would fire across most of the
    // corpus for a reason unrelated to this defect.
    const second = { ...binding, suite: "SUITE-002" };
    const report = audit({
      obligations: [obligation],
      bindings: [binding, second],
      runs: [run, { ...run, suite: "SUITE-002" }],
      injections: [
        {
          suite: "SUITE-001",
          symbol: "tests::confirms",
          injects: ["Confirmation::allow"],
        },
      ],
    });
    expect(
      report.findings.filter((f) => f.kind === "mocked-confirmation"),
    ).toHaveLength(0);
  });

  it("TC-939 no injection data means silence, not a clean bill", () => {
    // TC-939
    // Absent means "nobody looked". Reporting healthy here would be the
    // silent-zero defect this whole programme is about.
    const report = audit({
      obligations: [obligation],
      bindings: [binding],
      runs: [run],
    });
    expect(
      report.findings.filter((f) => f.kind === "mocked-confirmation"),
    ).toHaveLength(0);
    expect(report.healthy).toEqual([]);
    expect(report.unevaluated).toEqual([
      expect.objectContaining({
        check: "mocked-confirmation",
        obligation: "FR-017-AC-7",
        suites: ["SUITE-001"],
      }),
    ]);
  });

  it("TC-940 the finding ratchets through the existing key form", () => {
    // TC-940
    // #204 asked to extend `evidence audit`, not to build a second system, so
    // the finding must be acceptable in a baseline like every other kind.
    const report = audit({
      obligations: [obligation],
      bindings: [binding],
      runs: [run],
      injections: [
        {
          suite: "SUITE-001",
          symbol: "tests::confirms",
          injects: ["Confirmation::allow"],
        },
      ],
    });
    const found = report.findings.find(
      (f) => f.kind === "mocked-confirmation",
    )!;
    expect(findingKey(found)).toBe("mocked-confirmation:FR-017-AC-7");
  });
});
