/**
 * FR-035 — t-way coverage over a declared configuration space (TC-180..TC-186).
 */

import { describe, expect, it } from "vitest";

import { audit } from "../src/auditor/index.js";
import { characteristicsOf } from "../src/advisor/index.js";
import {
  demandedTuples,
  parseSpace,
  twayCoverage,
} from "../src/auditor/combinatorial.js";

const STATEMENT =
  "2-way over features(default|python|wasm) target(linux|wasm32) policy(tolerant|strict)";

describe("parsing a configuration space back out of the statement", () => {
  // Trace: FR-035-AC-1
  it("reads strength, dimensions and values", () => {
    const space = parseSpace(STATEMENT);
    expect(space?.strength).toBe(2);
    expect(space?.dimensions.map((d) => d.name)).toEqual([
      "features",
      "target",
      "policy",
    ]);
    expect(space?.dimensions[0].values).toEqual(["default", "python", "wasm"]);
  });

  // Trace: FR-035-AC-2
  it("returns null for any statement that is not a configuration space", () => {
    // This IS how a caller tells a combinatorial obligation from every other
    // kind. A second flag would be a second thing to keep in agreement.
    expect(parseSpace("The system shall respond within 200ms.")).toBeNull();
    expect(parseSpace("2-way over onlyone(a|b)")).toBeNull();
    expect(parseSpace("0-way over a(1|2) b(3|4)")).toBeNull();
  });

  // Trace: FR-035-AC-3
  it("reads exclusions without mistaking them for dimensions", () => {
    const space = parseSpace(
      `${STATEMENT} excluding[features=python,target=wasm32]`,
    );
    expect(space?.dimensions).toHaveLength(3);
    expect(space?.exclusions).toEqual([
      {
        assignments: [
          ["features", "python"],
          ["target", "wasm32"],
        ],
      },
    ]);
  });
});

describe("what the space demands", () => {
  // Trace: FR-035-AC-4
  it("counts every t-way tuple, and agrees with the engine's number", () => {
    // 3·2 + 3·2 + 2·2 = 16. quire-rs computes the same figure from the same
    // space (its TC-925); if these two ever disagree, an obligation and its
    // audit are describing different spaces.
    expect(demandedTuples(parseSpace(STATEMENT)!).size).toBe(16);
  });

  // Trace: FR-035-AC-5
  it("does not demand a forbidden combination", () => {
    const space = parseSpace(
      `${STATEMENT} excluding[features=python,target=wasm32]`,
    )!;
    expect(demandedTuples(space).size).toBe(15);
  });
});

describe("what a run reached", () => {
  // Trace: FR-035-AC-6
  it("names the combinations that never ran, not just how many", () => {
    // The gap list is the point. A percentage says how much is missing; the
    // list says which combinations to run.
    const space = parseSpace("2-way over a(1|2) b(x|y)")!;
    const result = twayCoverage(space, [
      { a: "1", b: "x" },
      { a: "2", b: "y" },
    ]);
    expect(result.demanded).toBe(4);
    expect(result.covered).toBe(2);
    expect(result.gaps).toEqual(["a=1 & b=y", "a=2 & b=x"]);
  });

  // Trace: FR-035-AC-7
  it("ignores a configuration exercising values the spec never declared", () => {
    // Counting it would let a coverage number rise by testing something else.
    const space = parseSpace("2-way over a(1|2) b(x|y)")!;
    const result = twayCoverage(space, [
      { a: "1", b: "x" },
      { a: "99", b: "zzz" },
      { a: "1", b: "x", c: "undeclared" },
    ]);
    expect(result.covered).toBe(1);
    expect(result.gaps).toHaveLength(3);
  });

  // Trace: FR-035-AC-8
  it("reports full coverage as no gap at all", () => {
    const space = parseSpace("2-way over a(1|2) b(x|y)")!;
    const result = twayCoverage(space, [
      { a: "1", b: "x" },
      { a: "1", b: "y" },
      { a: "2", b: "x" },
      { a: "2", b: "y" },
    ]);
    expect(result.covered).toBe(result.demanded);
    expect(result.gaps).toEqual([]);
  });
});

describe("the auditor over a combinatorial obligation", () => {
  const obligation = {
    id: "FR-001-COMB",
    statement: "2-way over a(1|2) b(x|y)",
    statement_hash: "h",
    parameters: { strength: "2", dimensions: "2", tuples: "4" },
  };
  const binding = {
    obligation: "FR-001-COMB",
    suite: "SUITE-CFG",
    symbols: ["tests::matrix"],
    statementHashAtBinding: "h",
    commit: "a".repeat(40),
  };
  const run = (configs: Array<Record<string, string>>) => ({
    schemaVersion: 1,
    suite: "SUITE-CFG",
    commit: "a".repeat(40),
    tool: "cargo test",
    timestamp: "2026-08-18T00:00:00Z",
    entries: configs.map((config) => ({
      symbol: "tests::matrix",
      outcome: "pass" as const,
      config,
    })),
  });

  // Trace: FR-035-AC-9
  it("reports the missing combinations by name", () => {
    const report = audit({
      obligations: [obligation],
      bindings: [binding],
      runs: [run([{ a: "1", b: "x" }])],
    });
    const gap = report.findings.find((f) => f.kind === "combinatorial-gap");
    expect(gap?.severity).toBe("medium");
    expect(gap?.summary).toMatch(
      /demands 4 2-way combinations and its runs reached 1/,
    );
    expect(gap?.summary).toMatch(/a=1 & b=y/);
  });

  // Trace: FR-035-AC-10
  it("reports no gap when every demanded combination ran", () => {
    const report = audit({
      obligations: [obligation],
      bindings: [binding],
      runs: [
        run([
          { a: "1", b: "x" },
          { a: "1", b: "y" },
          { a: "2", b: "x" },
          { a: "2", b: "y" },
        ]),
      ],
    });
    expect(report.findings.map((f) => f.kind)).not.toContain(
      "combinatorial-gap",
    );
    expect(report.healthy).toContain("FR-001-COMB");
  });

  // Trace: FR-035-AC-11
  it("says nothing about an obligation that declares no configuration space", () => {
    const report = audit({
      obligations: [
        {
          id: "FR-002-AC-1",
          statement: "The system shall respond.",
          statement_hash: "h",
        },
      ],
      bindings: [{ ...binding, obligation: "FR-002-AC-1" }],
      runs: [
        {
          ...run([]),
          entries: [{ symbol: "tests::matrix", outcome: "pass" as const }],
        },
      ],
    });
    expect(report.findings.map((f) => f.kind)).not.toContain(
      "combinatorial-gap",
    );
  });
});

describe("the advisor over a declared configuration space", () => {
  // Trace: FR-035-AC-12
  it("advises the combinatorial method structurally, not from prose", () => {
    // The prose regex would miss it entirely: a minted space reads
    // "2-way over features(default|python|wasm) target(linux|wasm32)" — no
    // "configuration", no "feature flag", no "combination of". The very
    // obligations that most need this method would be the ones never advised
    // for it.
    expect(characteristicsOf(STATEMENT)).toContain("configuration-matrix");
    // …and the structural test cannot false-positive on prose, which a widened
    // regex would.
    expect(characteristicsOf("The parser shall decode input.")).not.toContain(
      "configuration-matrix",
    );
    // The existing prose signal still works.
    expect(
      characteristicsOf("Every feature flag combination shall be exercised."),
    ).toContain("configuration-matrix");
  });
});
