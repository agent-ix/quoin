/**
 * Eval quality dimensions (quoin#201, FR-043-AC-2/AC-4/AC-5).
 *
 * The harness already recorded tokens, latency and tool calls — so an eval run
 * answered "was it cheap" and could not answer "was it right". A cheap run
 * producing wrong findings scored better than an expensive run producing right
 * ones, and nothing in the report said so.
 */

import {
  scoreActionability,
  scoreCost,
  scoreFindings,
  scoreScenario,
} from "../evals/lib/quality.mjs";

const labels = [
  { id: "MM-1", family: "marker-form-mismatch", findable: true },
  { id: "VP-1", family: "vacuous-under-guard", findable: true },
  { id: "GG-1", family: "gate-that-gates-nothing", findable: false },
];

describe("finding precision and recall", () => {
  it("TC-941 is reported per family, because an average hides a hole", () => {
    // TC-941
    // A tool that finds every marker mismatch and no vacuous suite has a
    // respectable average — and the average is exactly what hides it.
    const { families } = scoreFindings(
      [{ family: "marker-form-mismatch", rowId: "TC-001" }],
      labels,
    );
    const byFamily = Object.fromEntries(families.map((f) => [f.family, f]));
    expect(byFamily["marker-form-mismatch"].recall).toBe(1);
    expect(byFamily["vacuous-under-guard"].recall).toBe(0);
    expect(byFamily["vacuous-under-guard"].misses).toBe(1);
  });

  it("TC-942 counts a finding matching no label as a false positive", () => {
    // TC-942
    const { families } = scoreFindings(
      [
        { family: "marker-form-mismatch", rowId: "TC-001" },
        { family: "marker-form-mismatch", rowId: "TC-002" },
      ],
      labels,
    );
    const mm = families.find((f) => f.family === "marker-form-mismatch")!;
    expect(mm.truePositives).toBe(1);
    expect(mm.falsePositives).toBe(1);
    expect(mm.precision).toBe(0.5);
  });

  it("excludes an unfindable label from the denominator AND reports it", () => {
    // FR-043-AC-7's whole reason for existing: a scored miss must be
    // distinguishable from a defect nobody claimed was findable. An excluded
    // denominator nobody sees is a denominator nobody can question.
    const { families, excluded } = scoreFindings([], labels);
    expect(excluded).toEqual(["GG-1"]);
    expect(families.some((f) => f.family === "gate-that-gates-nothing")).toBe(
      false,
    );
  });

  it("TC-943 reports null, not zero, when a family has no denominator", () => {
    // TC-943
    // 0/0 is not 0%. A precision of 0 claims the run was wrong; null says it
    // emitted nothing to be right or wrong about.
    const { families } = scoreFindings([], labels);
    const vp = families.find((f) => f.family === "vacuous-under-guard")!;
    expect(vp.precision).toBeNull();
    expect(vp.recall).toBe(0);
  });
});

describe("actionability", () => {
  it("TC-944 counts findings that name where, which is what 15 of 496 measured", () => {
    // TC-944
    // Pass 2: 481 findings named neither the row they came from nor a line
    // that distinguished them. A finding you cannot act on is a finding nobody
    // acts on, whatever its precision.
    const scored = scoreActionability([
      { family: "x", rowId: "TC-001" },
      { family: "x", line: 42 },
      { family: "x" },
      { family: "x", rowId: "  " },
      { family: "x", line: 0 },
    ]);
    expect(scored.actionable).toBe(2);
    expect(scored.total).toBe(5);
    expect(scored.rate).toBe(0.4);
  });

  it("has no rate when there were no findings", () => {
    expect(scoreActionability([]).rate).toBeNull();
  });
});

describe("cost per confirmed insight", () => {
  it("TC-945 reports tokens AND tool calls, which are different costs", () => {
    // TC-945
    // Tokens are the context budget; tool calls are wall-clock and blast
    // radius. A run that reads the corpus once and one that greps it forty
    // times can spend the same tokens.
    const cost = scoreCost({ tokenUsage: { total: 90000 }, toolCalls: 12 }, 3);
    expect(cost.tokensPer).toBe(30000);
    expect(cost.toolCallsPer).toBe(4);
  });

  it("divides by CONFIRMED findings, not by findings emitted", () => {
    // Dividing by emitted output rewards a run for producing more of it,
    // which is precisely backwards.
    const cheapAndWrong = scoreCost(
      { tokenUsage: { total: 10000 }, toolCalls: 2 },
      1,
    );
    const dearAndRight = scoreCost(
      { tokenUsage: { total: 30000 }, toolCalls: 6 },
      5,
    );
    expect(dearAndRight.tokensPer).toBeLessThan(cheapAndWrong.tokensPer!);
  });

  it("reports null per-insight cost when nothing was confirmed", () => {
    // Not Infinity and not 0 — a run that confirmed nothing has no cost per
    // insight, and either number would be a claim the run does not support.
    const cost = scoreCost({ tokenUsage: { total: 50000 }, toolCalls: 9 }, 0);
    expect(cost.tokens).toBe(50000);
    expect(cost.tokensPer).toBeNull();
    expect(cost.toolCallsPer).toBeNull();
  });
});

describe("scoreScenario", () => {
  it("carries all three dimensions beside the cost columns", () => {
    const scored = scoreScenario({
      found: [{ family: "marker-form-mismatch", rowId: "TC-001" }],
      labels,
      metrics: { tokenUsage: { total: 20000 }, toolCalls: 8 },
    });
    expect(scored.findings.families.length).toBeGreaterThan(0);
    expect(scored.actionability.rate).toBe(1);
    expect(scored.cost.tokensPer).toBe(20000);
  });

  it("survives an empty run without inventing numbers", () => {
    const scored = scoreScenario({});
    expect(scored.findings.families).toEqual([]);
    expect(scored.actionability.rate).toBeNull();
    expect(scored.cost.tokensPer).toBeNull();
  });
});
