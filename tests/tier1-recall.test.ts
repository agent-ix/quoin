import { describe, expect, it } from "vitest";

import { normalizeQuireFinding } from "../evals/lib/finding-envelope.mjs";
import {
  detectionRecall,
  localityMissInventory,
  recallGateFails,
  recallVerdicts,
} from "../scripts/lib/tier1-recall.mjs";

const corpus = (
  name: string,
  language: string,
  defect: Record<string, unknown>,
) => ({
  name,
  mode: "minting",
  language,
  kind: "failure",
  findable: true,
  defects: [{ id: `${name}-1`, family: "section", findable: true, ...defect }],
});

describe("detection.recall", () => {
  it("keeps level, mode, and language independent and repeats the GAP count", () => {
    const corpora = [
      corpus("rust-deep", "rust", {
        location: "spec/tests.md:9",
        actionable_fragments: ["Test Case Summary"],
      }),
      corpus("python-shallow", "python", {
        location: null,
        actionable_fragments: [],
      }),
    ];
    const findings = [
      {
        corpus: "rust-deep",
        family: "section",
        path: "spec/tests.md",
        line: 9,
        message: "declared Test Case Summary was not found",
      },
      { corpus: "python-shallow", family: "section", path: null, line: null },
    ];

    const rows = detectionRecall(corpora, findings, 31);
    const normalizedRows = detectionRecall(
      corpora,
      findings.map((finding) =>
        normalizeQuireFinding(
          { reason: "section", ...finding },
          {
            producer: "quire",
            channel: "coverage.diagnostics",
            family: finding.family,
            corpus: finding.corpus,
          },
        ),
      ),
      31,
    );
    expect(normalizedRows).toEqual(rows);
    expect(rows).toEqual([
      expect.objectContaining({
        language: "python",
        level: "L1",
        rate: 1,
        gap_count: 31,
      }),
      expect.objectContaining({
        language: "python",
        level: "L2",
        rate: 0,
        gap_count: 31,
      }),
      expect.objectContaining({
        language: "python",
        level: "L3",
        rate: 0,
        gap_count: 31,
      }),
      expect.objectContaining({
        language: "rust",
        level: "L1",
        rate: 1,
        gap_count: 31,
      }),
      expect.objectContaining({
        language: "rust",
        level: "L2",
        rate: 1,
        gap_count: 31,
      }),
      expect.objectContaining({
        language: "rust",
        level: "L3",
        rate: 1,
        gap_count: 31,
      }),
    ]);
  });

  it("reports the exact missed case instead of turning no finding into zero without a reason", () => {
    const rows = detectionRecall(
      [corpus("silent", "rust", { location: "src/lib.rs:5" })],
      [],
      2,
    );
    expect(rows.every((row) => row.rate === 0)).toBe(true);
    expect(rows.every((row) => row.misses.includes("silent"))).toBe(true);
  });

  it("ratchets families independently inside one mode-language partition", () => {
    const first = corpus("first", "rust", {
      family: "section",
      location: "spec/tests.md:9",
      actionable_fragments: ["section"],
    });
    const second = corpus("second", "rust", {
      family: "column",
      location: "spec/tests.md:12",
      actionable_fragments: ["column"],
    });
    const rows = detectionRecall(
      [first, second],
      [
        {
          corpus: "first",
          family: "section",
          path: "spec/tests.md",
          line: 9,
          message: "section",
        },
      ],
      0,
    );

    expect(rows).toHaveLength(6);
    expect(
      rows.filter((row) => row.family === "section").map((row) => row.rate),
    ).toEqual([1, 1, 1]);
    expect(
      rows.filter((row) => row.family === "column").map((row) => row.rate),
    ).toEqual([0, 0, 0]);
  });

  it("retains every locality miss with producer, loci, cause, and disposition", () => {
    const corpora = [
      corpus("wrong-place", "rust", {
        location: "spec/tests.md:9",
        actionable_fragments: ["repair target"],
      }),
      corpus("silent", "rust", {
        location: "spec/tests.md:12",
        actionable_fragments: ["repair target"],
      }),
    ];
    const findings = [
      normalizeQuireFinding(
        {
          reason: "section",
          path: "spec/other.md",
          line: 4,
          message: "section did not match",
        },
        {
          producer: "quire",
          channel: "coverage.diagnostics",
          family: "section",
          corpus: "wrong-place",
        },
      ),
    ];

    expect(localityMissInventory(corpora, findings)).toEqual([
      expect.objectContaining({
        case: "silent",
        family: "section",
        producer: [],
        expectedLocus: "spec/tests.md:12",
        observedLocus: [],
        missingLevels: ["L1", "L2", "L3"],
        rootCause: "the producer emitted no finding in the expected family",
        disposition: expect.objectContaining({ state: "deferred" }),
      }),
      expect.objectContaining({
        case: "wrong-place",
        family: "section",
        producer: ["quire:coverage.diagnostics"],
        expectedLocus: "spec/tests.md:9",
        observedLocus: ["spec/other.md:4"],
        missingLevels: ["L2", "L3"],
        rootCause:
          "the family finding did not match the controlled expected locus",
        disposition: expect.objectContaining({ state: "deferred" }),
      }),
    ]);
  });

  it("counts a findable case with no detector label as a miss, not an omitted denominator", () => {
    const rows = detectionRecall(
      [
        {
          name: "undetected",
          mode: "skeptic",
          language: "rust",
          kind: "failure",
          findable: true,
          defects: [],
        },
      ],
      [],
      4,
    );
    expect(rows).toHaveLength(3);
    expect(rows.every((row) => row.population === 1 && row.reached === 0)).toBe(
      true,
    );
  });

  it("counts exact located payload observations when no diagnostic family exists", () => {
    const observed = {
      name: "unminted",
      mode: "join",
      language: "python",
      kind: "failure",
      findable: true,
      defects: [],
      observations: {
        untracked_symbols: [
          { symbol: "test_warns", trace_id: "TC-999", path: "src/test.py" },
        ],
      },
    };
    const rows = detectionRecall([observed], [], 2, [
      {
        name: "unminted",
        untrackedSymbols: [
          {
            symbol: "test_warns",
            trace_id: "TC-999",
            path: "src/test.py",
            line: 4,
          },
        ],
      },
    ]);

    expect(rows.map((row) => [row.level, row.rate])).toEqual([
      ["L1", 1],
      ["L2", 1],
      ["L3", 0],
    ]);
    expect(
      localityMissInventory(
        [observed],
        [],
        [
          {
            name: "unminted",
            untrackedSymbols: [
              {
                symbol: "test_warns",
                trace_id: "TC-999",
                path: "src/test.py",
                line: 4,
              },
            ],
          },
        ],
      ),
    ).toEqual([
      expect.objectContaining({
        case: "unminted",
        family: "direct-observation",
        missingLevels: ["L3"],
        rootCause:
          "the direct observation has no controlled actionable-fragment contract",
        disposition: expect.objectContaining({ state: "deferred" }),
      }),
    ]);

    const wrongLocus = detectionRecall([observed], [], 2, [
      {
        name: "unminted",
        untrackedSymbols: [
          {
            symbol: "test_warns",
            trace_id: "TC-999",
            path: "src/other.py",
          },
        ],
      },
    ]);
    expect(wrongLocus.map((row) => [row.level, row.rate])).toEqual([
      ["L1", 1],
      ["L2", 0],
      ["L3", 0],
    ]);
  });

  it("ratchets one partition without averaging it into another", () => {
    const baseline = [
      {
        mode: "minting",
        language: "rust",
        level: "L1",
        population: 2,
        rate: 1,
      },
      {
        mode: "minting",
        language: "python",
        level: "L1",
        population: 2,
        rate: 0.5,
      },
    ];
    const rows = [
      { ...baseline[0], reached: 1, rate: 0.5, gap_count: 0, misses: ["r"] },
      { ...baseline[1], reached: 2, rate: 1, gap_count: 0, misses: [] },
    ];
    expect(recallVerdicts(rows, baseline).map((row) => row.verdict)).toEqual([
      "regressed",
      "improved",
    ]);
  });

  it("refuses a delta when the partition population moved", () => {
    const row = {
      mode: "join",
      language: "typescript",
      level: "L2",
      population: 3,
      reached: 2,
      rate: 0.667,
      gap_count: 1,
      misses: ["x"],
    };
    expect(
      recallVerdicts([row], [{ ...row, population: 2, rate: 0.5 }])[0],
    ).toMatchObject({
      verdict: "incomparable",
      why: expect.stringContaining("population moved"),
    });
  });

  it("requires every new or changed partition to be retained", () => {
    expect(recallGateFails([{ verdict: "held" }])).toBe(false);
    for (const verdict of ["new", "improved", "regressed", "incomparable"]) {
      expect(recallGateFails([{ verdict }])).toBe(true);
    }
  });
});
