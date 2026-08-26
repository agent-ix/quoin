import { describe, expect, it } from "vitest";

import {
  detectionRecall,
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
