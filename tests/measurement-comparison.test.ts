/** FR-044 — generic measurement query and comparison (TC-283..TC-289). */

import { mkdtempSync, readFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

import { beforeEach, describe, expect, it } from "vitest";

import {
  compareMeasurementSets,
  listMeasurementCollectionPaths,
  measurementTrend,
  queryMeasurements,
  renderMeasurementComparisonJson,
  renderMeasurementComparisonMarkdown,
  writeMeasurementCollection,
  type MeasurementCollection,
  type MeasurementPolicy,
  type MeasurementRecord,
} from "../src/evidence/index.js";

const here = dirname(fileURLToPath(import.meta.url));
const fixtures = join(here, "fixtures", "evidence");
const latency = fixture("measurement-cli-latency.json");
const complexity = fixture("measurement-function-complexity.json");
let repo: string;

beforeEach(() => {
  repo = mkdtempSync(join(tmpdir(), "quoin-comparison-"));
});

function fixture(name: string): MeasurementRecord {
  return JSON.parse(readFileSync(join(fixtures, name), "utf8"));
}

function observed(
  record: MeasurementRecord,
  sourceRevision: string,
  value: number,
  collectedAt = "2026-08-21T19:00:00Z",
): MeasurementRecord {
  return { ...record, sourceRevision, value, collectedAt };
}

function store(records: MeasurementRecord[]): void {
  const [first] = records;
  const collection: MeasurementCollection = {
    schemaVersion: 1,
    plan: first.plan,
    repository: first.scope.repository,
    sourceRevision: first.sourceRevision,
    tool: first.tool,
    environment: first.environment,
    ...(first.sampling === undefined ? {} : { sampling: first.sampling }),
    collectedAt: first.collectedAt,
    rawEvidence: first.rawEvidence,
    observations: records.map((record) => ({
      subject: record.subject,
      ...(record.scope.path === undefined ? {} : { path: record.scope.path }),
      value: record.value,
      unit: record.unit,
      ...(record.distribution === undefined
        ? {}
        : { distribution: record.distribution }),
    })),
  };
  writeMeasurementCollection(repo, collection);
}

const lowerIsBetter: MeasurementPolicy = {
  direction: "increase-is-regression",
  tolerance: { kind: "absolute", value: 0 },
};

describe("TC-283 stored measurements have an exact deterministic query surface", () => {
  // Trace: FR-044-AC-1
  // Trace: FR-044-CON-4
  it("queries every authored identity and orders by time/revision/key", () => {
    const later = observed(latency, "b".repeat(40), 45);
    const earlier = observed(
      latency,
      "a".repeat(40),
      40,
      "2026-08-21T18:00:00Z",
    );
    store([later]);
    store([complexity]);
    store([earlier]);

    expect(listMeasurementCollectionPaths(repo, latency.plan.id)).toHaveLength(
      2,
    );
    expect(queryMeasurements(repo, { planId: latency.plan.id })).toEqual([
      earlier,
      later,
    ]);
    expect(
      queryMeasurements(repo, {
        planId: latency.plan.id,
        definitionVersion: latency.plan.definitionVersion,
        subject: latency.subject,
        scope: latency.scope,
        sourceRevision: later.sourceRevision,
      }),
    ).toEqual([later]);
    expect(queryMeasurements(repo, { planId: "MP-ABSENT" })).toEqual([]);
    expect(queryMeasurements(repo)).toHaveLength(3);
    for (const query of [
      { definitionVersion: "absent" },
      { subject: { ...latency.subject, id: "absent" } },
      { scope: { ...latency.scope, path: "absent" } },
      { sourceRevision: "absent" },
    ]) {
      expect(queryMeasurements(repo, query)).toEqual([]);
    }
  });

  it("uses revision and population key as deterministic timestamp ties", () => {
    const time = "2026-08-21T20:00:00Z";
    const sameTimeA = observed(latency, "a".repeat(40), 40, time);
    const sameTimeB = observed(latency, "b".repeat(40), 41, time);
    const sameRevisionOtherSubject = {
      ...sameTimeB,
      subject: { ...sameTimeB.subject, id: "another subject" },
    };
    const sameIdentityOtherDefinition = {
      ...sameTimeB,
      plan: { ...sameTimeB.plan, definitionVersion: "other" },
    };
    store([sameTimeB, sameRevisionOtherSubject]);
    store([sameIdentityOtherDefinition]);
    store([sameTimeA]);
    expect(
      queryMeasurements(repo).map((record) => record.sourceRevision),
    ).toEqual(["a".repeat(40), "b".repeat(40), "b".repeat(40), "b".repeat(40)]);
  });
});

describe("TC-284 one comparison path handles unlike measures", () => {
  // Trace: FR-044-AC-2
  it("compares latency and complexity without a measure-specific branch", () => {
    for (const [record, candidateValue] of [
      [latency, latency.value + 2],
      [complexity, complexity.value - 2],
    ] as const) {
      const result = compareMeasurementSets(
        [record],
        [observed(record, "e".repeat(40), candidateValue)],
      );
      expect(result.state).toBe("comparable");
      if (result.state !== "comparable") continue;
      expect(result.deltas[0].delta).toBe(candidateValue - record.value);
      expect(result.deltas[0].unit).toBe(record.unit);
      expect(result.regression).toBeNull();
    }
  });
});

describe("TC-285 incompatible identities are non-comparisons", () => {
  // Trace: FR-044-AC-3
  // Trace: FR-044-CON-2
  it.each([
    [
      "definition-version",
      { ...latency, plan: { ...latency.plan, definitionVersion: "next" } },
    ],
    ["unit", { ...latency, unit: "seconds" }],
    ["tool", { ...latency, tool: { ...latency.tool, version: "next" } }],
    [
      "configuration",
      {
        ...latency,
        tool: {
          ...latency.tool,
          configurationDigest: `sha256:${"f".repeat(64)}`,
        },
      },
    ],
    [
      "environment",
      { ...latency, environment: { ...latency.environment, id: "other" } },
    ],
    [
      "environment",
      {
        ...latency,
        environment: {
          ...latency.environment,
          attributes: { ...latency.environment.attributes, runner: "other" },
        },
      },
    ],
    ["sampling", { ...latency, sampling: { id: "other", sampleCount: 30 } }],
  ])("reports changed %s identity before arithmetic", (reason, candidate) => {
    const result = compareMeasurementSets(
      [latency],
      [observed(candidate as MeasurementRecord, "e".repeat(40), 99)],
      lowerIsBetter,
    );
    expect(result.state).toBe("incompatible");
    if (result.state !== "incompatible") return;
    expect(result.incompatibilities.map((issue) => issue.reason)).toContain(
      reason,
    );
    expect(result.ratchet.baselineApplied).toBe(false);
    expect(result).not.toHaveProperty("deltas");
  });
});

describe("TC-286 absent, partial, and ambiguous collections stay distinct", () => {
  // Trace: FR-044-AC-4
  // Trace: FR-044-CON-2
  it("distinguishes missing baseline, empty candidate, and partial population", () => {
    expect(compareMeasurementSets([], [latency]).state).toBe(
      "missing-baseline",
    );
    expect(compareMeasurementSets([latency], []).state).toBe(
      "empty-population",
    );
    expect(compareMeasurementSets([], []).state).toBe("empty-population");

    const extra = {
      ...observed(complexity, "e".repeat(40), 7),
      plan: latency.plan,
      unit: latency.unit,
      environment: latency.environment,
      sampling: latency.sampling,
    };
    const partial = compareMeasurementSets(
      [latency],
      [observed(latency, "e".repeat(40), 42), extra],
    );
    expect(partial.state).toBe("partial-collection");
    if (partial.state === "partial-collection") {
      expect(partial.missingFromBaseline).toHaveLength(1);
      expect(partial.missingFromCandidate).toEqual([]);
      expect(partial.samplingCountMismatches).toEqual([]);
    }

    const missingCandidate = compareMeasurementSets(
      [latency, { ...extra, sourceRevision: latency.sourceRevision }],
      [observed(latency, "e".repeat(40), 42)],
    );
    expect(missingCandidate.state).toBe("partial-collection");
    if (missingCandidate.state === "partial-collection") {
      expect(missingCandidate.missingFromCandidate).toHaveLength(1);
    }

    const sampledPartially = compareMeasurementSets(
      [latency],
      [
        observed(
          {
            ...latency,
            sampling: { ...latency.sampling!, sampleCount: 12 },
          },
          "e".repeat(40),
          42,
        ),
      ],
    );
    expect(sampledPartially.state).toBe("partial-collection");
    if (sampledPartially.state === "partial-collection") {
      expect(sampledPartially.samplingCountMismatches).toEqual([
        expect.objectContaining({ baseline: 30, candidate: 12 }),
      ]);
    }
  });

  it("reports mixed revisions and duplicate subjects as separate reasons", () => {
    const mixed = compareMeasurementSets(
      [latency, observed(latency, "e".repeat(40), 42)],
      [observed(latency, "f".repeat(40), 43)],
    );
    expect(mixed.state).toBe("incompatible");
    if (mixed.state !== "incompatible") return;
    expect(mixed.incompatibilities.map((issue) => issue.reason)).toEqual([
      "mixed-revision",
      "duplicate-subject",
    ]);
  });
});

describe("TC-287 ratchet policy is explicit and applied only to compatible data", () => {
  // Trace: FR-044-AC-5
  // Trace: FR-044-CON-1
  it("separates a delta from a caller-requested regression verdict", () => {
    const candidate = observed(latency, "e".repeat(40), latency.value + 2);
    const observedOnly = compareMeasurementSets([latency], [candidate]);
    expect(observedOnly.state).toBe("comparable");
    expect(observedOnly.ratchet).toEqual({
      requested: false,
      baselineApplied: false,
    });

    const judged = compareMeasurementSets(
      [latency],
      [candidate],
      lowerIsBetter,
    );
    expect(judged.state).toBe("comparable");
    if (judged.state !== "comparable") return;
    expect(judged.regression).toBe(true);
    expect(judged.ratchet).toEqual({ requested: true, baselineApplied: true });
  });

  it("refuses invalid tolerance and relative policy over a zero baseline", () => {
    expect(() =>
      compareMeasurementSets([latency], [latency], {
        ...lowerIsBetter,
        tolerance: { kind: "absolute", value: Number.NaN },
      }),
    ).toThrow(/finite and non-negative/);
    expect(() =>
      compareMeasurementSets([latency], [latency], {
        ...lowerIsBetter,
        tolerance: { kind: "absolute", value: -1 },
      }),
    ).toThrow(/finite and non-negative/);

    const zero = { ...latency, value: 0 };
    const result = compareMeasurementSets([zero], [zero], {
      ...lowerIsBetter,
      tolerance: { kind: "relative", value: 0.1 },
    });
    expect(result.state).toBe("incompatible");
    if (result.state === "incompatible") {
      expect(result.incompatibilities[0].reason).toBe(
        "zero-baseline-for-relative-policy",
      );
    }
  });

  it("supports relative tolerance and a caller-declared adverse decrease", () => {
    const relative = compareMeasurementSets(
      [latency],
      [observed(latency, "e".repeat(40), latency.value * 1.05)],
      {
        direction: "increase-is-regression",
        tolerance: { kind: "relative", value: 0.1 },
      },
    );
    expect(relative.state).toBe("comparable");
    if (relative.state === "comparable") {
      expect(relative.regression).toBe(false);
    }

    const decrease = compareMeasurementSets(
      [complexity],
      [observed(complexity, "e".repeat(40), complexity.value - 1)],
      {
        direction: "decrease-is-regression",
        tolerance: { kind: "absolute", value: 0 },
      },
    );
    expect(decrease.state).toBe("comparable");
    if (decrease.state === "comparable") {
      expect(decrease.regression).toBe(true);
    }

    const zeroWithoutPolicy = compareMeasurementSets(
      [{ ...complexity, value: 0 }],
      [observed({ ...complexity, value: 0 }, "e".repeat(40), 1)],
    );
    expect(zeroWithoutPolicy.state).toBe("comparable");
    if (zeroWithoutPolicy.state === "comparable") {
      expect(zeroWithoutPolicy.deltas[0].relativeDelta).toBeNull();
    }
  });
});

describe("TC-288 trends order observations and expose discontinuities", () => {
  // Trace: FR-044-AC-6
  it("sorts by time and marks a definition change", () => {
    const first = observed(latency, "a".repeat(40), 40, "2026-08-21T17:00:00Z");
    const second = observed(
      latency,
      "b".repeat(40),
      41,
      "2026-08-21T18:00:00Z",
    );
    const third = {
      ...observed(latency, "c".repeat(40), 42, "2026-08-21T19:00:00Z"),
      plan: { ...latency.plan, definitionVersion: "3" },
    };
    const trend = measurementTrend([third, second, first]);
    expect(trend.state).toBe("trend");
    if (trend.state !== "trend") return;
    expect(trend.points.map((point) => point.continuity)).toEqual([
      "start",
      "continuous",
      "discontinuity",
    ]);
    expect(trend.points[2].discontinuities).toContain("definition-version");
  });

  it("distinguishes empty and mixed-subject trends", () => {
    expect(measurementTrend([]).state).toBe("empty-population");
    const mixed = measurementTrend([latency, complexity]);
    expect(mixed.state).toBe("incompatible");
    if (mixed.state === "incompatible") {
      expect(mixed.incompatibilities[0].reason).toBe("mixed-population");
    }
  });
});

describe("TC-289 renderers are deterministic and the engine is measure-neutral", () => {
  // Trace: FR-044-AC-7
  // Trace: FR-044-CON-1
  // Trace: FR-044-CON-4
  it("renders byte-identical JSON and Markdown", () => {
    const result = compareMeasurementSets(
      [latency],
      [observed(latency, "e".repeat(40), latency.value + 2)],
      lowerIsBetter,
    );
    expect(renderMeasurementComparisonJson(result)).toBe(
      renderMeasurementComparisonJson(result),
    );
    const markdown = renderMeasurementComparisonMarkdown(result);
    expect(markdown).toBe(renderMeasurementComparisonMarkdown(result));
    expect(markdown).toContain("Baseline applied: yes");
    expect(markdown).toContain("| Subject/scope key |");

    const unjudged = compareMeasurementSets(
      [complexity],
      [observed(complexity, "e".repeat(40), complexity.value)],
    );
    expect(renderMeasurementComparisonMarkdown(unjudged)).toContain(
      "Regression: not evaluated",
    );

    const noRegression = compareMeasurementSets(
      [latency],
      [observed(latency, "e".repeat(40), latency.value - 1)],
      lowerIsBetter,
    );
    expect(renderMeasurementComparisonMarkdown(noRegression)).toContain(
      "Regression: no",
    );

    const zeroBaseline = compareMeasurementSets(
      [{ ...complexity, value: 0 }],
      [observed({ ...complexity, value: 0 }, "e".repeat(40), 1)],
    );
    expect(renderMeasurementComparisonMarkdown(zeroBaseline)).toContain(
      "| n/a |",
    );

    const partial = compareMeasurementSets(
      [latency],
      [
        observed(
          {
            ...latency,
            sampling: { ...latency.sampling!, sampleCount: 12 },
          },
          "e".repeat(40),
          42,
        ),
      ],
    );
    expect(renderMeasurementComparisonMarkdown(partial)).toContain(
      "Sampling-count mismatches:",
    );
    const subjectPartial = compareMeasurementSets(
      [latency],
      [
        observed(latency, "e".repeat(40), 42),
        {
          ...observed(complexity, "e".repeat(40), 7),
          plan: latency.plan,
          unit: latency.unit,
          environment: latency.environment,
          sampling: latency.sampling,
        },
      ],
    );
    expect(renderMeasurementComparisonMarkdown(subjectPartial)).toContain(
      "Sampling-count mismatches: none",
    );

    const incompatible = compareMeasurementSets(
      [latency],
      [
        {
          ...observed(latency, "e".repeat(40), 42),
          plan: { ...latency.plan, id: "OTHER-PLAN" },
          unit: "other",
          environment: { ...latency.environment, id: "other" },
        },
      ],
    );
    expect(renderMeasurementComparisonMarkdown(incompatible)).toContain(
      ": unit",
    );
    expect(
      renderMeasurementComparisonMarkdown(compareMeasurementSets([], [])),
    ).toContain("State: `empty-population`");
  });

  it("contains no analyzer, known measure, or universal-threshold branch", () => {
    const source = readFileSync(
      join(here, "..", "src", "evidence", "comparison.ts"),
      "utf8",
    ).toLowerCase();
    for (const forbidden of [
      "hyperfine",
      "cyclomatic",
      "latency-ms",
      "mutation-score",
      "universal threshold",
    ]) {
      expect(source).not.toContain(forbidden);
    }
  });
});
