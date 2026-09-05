import { describe, expect, it } from "vitest";

import {
  RateError,
  assertNotSummed,
  divergences,
  populationId,
  rate,
  verifyFigures,
  type Report,
} from "../src/measurement/rates.js";

const POP = populationId({
  corpusId: "ix-workspace-2026-09-04",
  repositoryCommits: ["aaa", "bbb"],
  moduleCommits: ["ccc"],
  engineRevision: "ddd",
});

describe("TC-1560..1567 a rate carries what it counted", () => {
  // TC-1560
  it("binds a population identifier to the exact revisions measured", () => {
    const same = populationId({
      corpusId: "ix-workspace-2026-09-04",
      repositoryCommits: ["bbb", "aaa"],
      moduleCommits: ["ccc"],
      engineRevision: "ddd",
    });
    // Order must not change identity; a different commit must.
    expect(same).toBe(POP);
    expect(
      populationId({
        corpusId: "ix-workspace-2026-09-04",
        repositoryCommits: ["aaa", "zzz"],
        moduleCommits: ["ccc"],
        engineRevision: "ddd",
      }),
    ).not.toBe(POP);
  });

  // TC-1561
  it("publishes a zero-denominator partition with no value rather than omitting it", () => {
    const empty = rate({
      id: "by-module:spec-objects-safety",
      numerator: 0,
      denominator: 0,
      unit: "documents",
      populationId: POP,
      methodId: "structural-v1",
    });
    expect(empty.value).toBeNull();
    // Not zero: "no documents" and "no document passed" are different facts.
    expect(empty.value).not.toBe(0);
  });

  // TC-1562
  it("refuses a numerator above its denominator", () => {
    expect(() =>
      rate({
        id: "bad",
        numerator: 5,
        denominator: 3,
        unit: "documents",
        populationId: POP,
        methodId: "structural-v1",
      }),
    ).toThrow(RateError);
  });

  // TC-1563 — the margin is read on every call, so a test can move it.
  it("names partitions below the aggregate by more than the declared margin", () => {
    const aggregate = rate({
      id: "aggregate",
      numerator: 90,
      denominator: 100,
      unit: "documents",
      populationId: POP,
      methodId: "structural-v1",
    });
    const low = rate({
      id: "by-repository:laggard",
      numerator: 5,
      denominator: 10,
      unit: "documents",
      populationId: POP,
      methodId: "structural-v1",
    });
    const near = rate({
      id: "by-repository:close",
      numerator: 88,
      denominator: 100,
      unit: "documents",
      populationId: POP,
      methodId: "structural-v1",
    });

    expect(divergences(aggregate, [low, near], 0.1).map((d) => d.partition)).toEqual(
      ["by-repository:laggard"],
    );
    // Moving the margin moves the list, which is how we know it is read.
    expect(divergences(aggregate, [low, near], 0.01).map((d) => d.partition)).toEqual(
      ["by-repository:laggard", "by-repository:close"],
    );
    expect(divergences(aggregate, [low, near], 0.9)).toEqual([]);
  });

  // TC-1564 — the anti-fabrication check.
  it("recomputes every printed figure from the artifact it names", () => {
    const figures = [
      { printed: "7501", artifact: "states.json", field: "measured" },
      { printed: "154", artifact: "census.json", field: "freeColumnTable" },
    ];
    const actual = new Map([
      ["states.json:measured", "7501"],
      ["census.json:freeColumnTable", "154"],
    ]);
    const ok = verifyFigures(figures, (a, f) => actual.get(`${a}:${f}`));
    expect(ok.verified).toBe(2);
    expect(ok.mismatched).toEqual([]);

    // Prose that drifted from its artifact is reported, never silently fixed.
    const drifted = verifyFigures(
      [{ printed: "7500", artifact: "states.json", field: "measured" }],
      (a, f) => actual.get(`${a}:${f}`),
    );
    expect(drifted.mismatched).toHaveLength(1);
    expect(drifted.verified).toBe(0);
  });

  // TC-1565
  it("reports a figure whose artifact or field does not resolve", () => {
    const result = verifyFigures(
      [{ printed: "12", artifact: "gone.json", field: "nope" }],
      () => undefined,
    );
    expect(result.unresolved).toHaveLength(1);
    expect(result.verified).toBe(0);
  });

  // TC-1566
  it("refuses to report the structural rate and the form census as one method", () => {
    const shared = rate({
      id: "structural",
      numerator: 1,
      denominator: 2,
      unit: "documents",
      populationId: POP,
      methodId: "same",
    });
    const report = {
      populationId: POP,
      structural: shared,
      formCensus: { ...shared, id: "census" },
      byModule: [],
      byType: [],
      byRepository: [],
      divergences: [],
      unstableRepositories: 0,
      dirtyRepositories: 3,
      figures: [],
    } as Report;
    expect(() => assertNotSummed(report)).toThrow(/must not be reported as one figure/);
  });
});
