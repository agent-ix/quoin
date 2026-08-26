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
  it("TC-946 scores by location when both sides name one", () => {
    // TC-946
    // The failure SR-014 FND-001 describes: two defects of ONE family seeded
    // at different places, and a tool reporting the right family twice at the
    // SAME place. Matching on family alone scored both true -- precision 1.00
    // where the truth is 0.50 -- which is the overstatement FR-043-AC-2 exists
    // to prevent.
    const twoOfAFamily = [
      {
        id: "MM-1",
        family: "marker-form-mismatch",
        findable: true,
        location: "src/lib.rs:5",
      },
      {
        id: "MM-2",
        family: "marker-form-mismatch",
        findable: true,
        location: "src/other.rs:40",
      },
    ];
    const { families, positional } = scoreFindings(
      [
        { family: "marker-form-mismatch", path: "src/lib.rs", line: 5 },
        { family: "marker-form-mismatch", path: "src/lib.rs", line: 5 },
      ],
      twoOfAFamily,
    );
    const mm = families.find((f) => f.family === "marker-form-mismatch")!;
    expect(mm.truePositives).toBe(1);
    expect(mm.falsePositives).toBe(1);
    expect(mm.precision).toBe(0.5);
    expect(mm.misses).toBe(1);
    expect(positional).toBe(1);

    // Both findings in the right places score 1.00 honestly, so the test
    // measures the location rule rather than a cap on true positives.
    const honest = scoreFindings(
      [
        { family: "marker-form-mismatch", path: "src/lib.rs", line: 5 },
        { family: "marker-form-mismatch", path: "src/other.rs", line: 40 },
      ],
      twoOfAFamily,
    );
    const both = honest.families.find(
      (f) => f.family === "marker-form-mismatch",
    )!;
    expect(both.precision).toBe(1);
    expect(both.recall).toBe(1);
    expect(honest.positional).toBe(2);

    // A label naming a FILE with no line is a claim about the file: a finding
    // in it matches without inventing a line the label never asserted.
    const fileOnly = scoreFindings(
      [{ family: "missing-usecase", path: "spec/FR-001.md", line: 12 }],
      [
        {
          id: "MU-1",
          family: "missing-usecase",
          findable: true,
          location: "spec/FR-001.md",
        },
      ],
    );
    expect(fileOnly.families[0].truePositives).toBe(1);
  });

  it("TC-952 declared collateral is set aside, reported, and spent once", () => {
    // TC-952
    // The behaviour, not the declaration's shape (TC-950 covers that). A
    // seeded `hollow-denominator` necessarily also produces `no-symbol-bound`,
    // and the engine cannot separate the two causes — `coverage.backed` is the
    // only ratio metric that can go hollow and its one cause already has a
    // bespoke diagnostic. Scoring that second, CORRECT finding as a false
    // positive punishes the toolchain for being right.
    const seeded = [
      {
        id: "HD-1",
        family: "hollow-denominator",
        findable: true,
        location: "src/lib.rs",
        collateral: [
          { family: "marker-form-mismatch", reason: "no-symbol-bound" },
        ],
      },
    ];

    const one = scoreFindings(
      [
        { family: "hollow-denominator", reason: "hollow-denominator" },
        { family: "marker-form-mismatch", reason: "no-symbol-bound" },
      ],
      seeded,
    );
    // The seeded defect scores; the consequence is neither a true nor a false
    // positive, and `marker-form-mismatch` gets no row at all — this corpus
    // makes no claim about that family.
    expect(one.families).toEqual([
      {
        family: "hollow-denominator",
        truePositives: 1,
        falsePositives: 0,
        misses: 0,
        precision: 1,
        recall: 1,
        // Every row now carries the shape it was scored under, defaulting to
        // `defect`. An ADVISORY family reports no precision — it fires on
        // nearly every corpus by construction, so the defect-shaped false
        // positive definition counts each correct firing against it
        // (agent-ix/quoin#234).
        shape: "defect",
      },
    ]);
    // Reported by name, never silently dropped: an unscored finding nobody
    // sees is a finding nobody can question.
    expect(one.collateral).toEqual([
      { family: "marker-form-mismatch", reason: "no-symbol-bound" },
    ]);

    // A declaration is SPENT once, like a label. Otherwise one declaration
    // absorbs every matching finding, and a toolchain reporting the same
    // consequence three times scores identically to one reporting it once --
    // the duplicate laundering CR-098's positional pairing exists to stop,
    // re-entering through a side door.
    const three = scoreFindings(
      [
        { family: "hollow-denominator", reason: "hollow-denominator" },
        { family: "marker-form-mismatch", reason: "no-symbol-bound" },
        { family: "marker-form-mismatch", reason: "no-symbol-bound" },
        { family: "marker-form-mismatch", reason: "no-symbol-bound" },
      ],
      seeded,
    );
    expect(three.collateral).toHaveLength(1);
    const mm = three.families.find((f) => f.family === "marker-form-mismatch");
    expect(mm?.falsePositives).toBe(2);

    // The reason is load-bearing. A declaration naming only the family would
    // absorb any finding of that family, including one at a place no defect
    // was seeded.
    const wrongReason = scoreFindings(
      [{ family: "marker-form-mismatch", reason: "stale-name-correct-trace" }],
      seeded,
    );
    expect(wrongReason.collateral).toEqual([]);
    expect(
      wrongReason.families.find((f) => f.family === "marker-form-mismatch")
        ?.falsePositives,
    ).toBe(1);
  });

  it("TC-967 a declaration on one case cannot absorb another case's finding", () => {
    // TC-967
    // agent-ix/quoin#238. Spending a declaration ONCE (TC-952) was not enough:
    // the pairing was on family and reason only, so a collateral declaration
    // on case A consumed a correct finding from case B and that finding
    // vanished from scoring entirely — neither true positive nor false
    // positive.
    //
    // Measured over the 34-case tier-1 corpus: five cases declared
    // `hollow-denominator` collateral, the run emitted exactly five
    // `hollow-denominator` findings, and one of them was the SEEDED, labelled
    // defect of a sixth case. All five were absorbed. That family scored
    // recall 0.00 for the whole run while the per-language cut of the same run
    // scored 1.00 — two contradictory numbers out of one score.
    const labels = [
      {
        id: "MF-1",
        family: "marker-form-mismatch",
        corpus: "marker-case",
        findable: true,
        location: "src/lib.rs",
        collateral: [
          { family: "hollow-denominator", reason: "hollow-denominator" },
        ],
      },
      {
        id: "HD-1",
        family: "hollow-denominator",
        corpus: "hollow-case",
        findable: true,
        location: null,
      },
    ];
    const score = scoreFindings(
      [
        {
          family: "marker-form-mismatch",
          reason: "no-symbol-bound",
          corpus: "marker-case",
          path: "src/lib.rs",
        },
        {
          family: "hollow-denominator",
          reason: "hollow-denominator",
          corpus: "marker-case",
        },
        {
          family: "hollow-denominator",
          reason: "hollow-denominator",
          corpus: "hollow-case",
        },
      ],
      labels,
    );
    // Only the marker case's own consequence is set aside.
    expect(score.collateral).toHaveLength(1);
    // And the hollow case's seeded defect still scores.
    const hollow = score.families.find(
      (f) => f.family === "hollow-denominator",
    );
    expect(hollow?.truePositives).toBe(1);
    expect(hollow?.misses).toBe(0);
    expect(hollow?.recall).toBe(1);
  });

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

  it("TC-982 a SCOPED ruling governs only the declaration it names", () => {
    // TC-982
    // THE FABRICATED NUMBER THIS STOPS. `agent-ix/quire-rs#304` made
    // `archetype-matches-nothing` fire for several declarations at once, so on
    // a three-file fixture it fires correctly for `inspection`, `suite`,
    // `nfr-acceptance-criterion` and `stakeholder-validation-criterion` — none
    // of which the fixture is about. The control therefore rules on
    // `test-case/archetype-matches-nothing`, SCOPED.
    //
    // The first implementation stripped the scope on the way to the family, so
    // those four correct firings were read as violations of a ruling that never
    // covered them, and the run published precision 0.556. Measured, caught
    // before it reached a baseline, and pinned here (agent-ix/quoin#245).
    const findings = [
      { family: "adv", corpus: "control", declaration: "inspection" },
      { family: "adv", corpus: "control", declaration: "suite" },
      { family: "adv", corpus: "control", declaration: "test-case" },
    ];
    const { families } = scoreFindings(
      findings,
      [],
      { adv: "advisory" },
      {
        adv: {
          present: [],
          absent: [{ corpus: "control", scope: "test-case" }],
        },
      },
    );
    const adv = families.find((f) => f.family === "adv")!;
    // ONE false positive — the `test-case` firing the control ruled out. The
    // other two are outside the ruling's scope and unknown, not wrong.
    expect(adv.precision_basis!.falsePositives).toBe(1);
    expect(adv.precision_basis!.truePositives).toBe(0);
    expect(adv.precision_basis!.unadjudicated).toBe(2);
    expect(adv.precision).toBe(0);
  });

  it("TC-983 an UNSCOPED ruling governs every firing of its family on that case", () => {
    // TC-983
    // The counterpart to TC-982: a fixture that writes the bare token means the
    // whole family, and scoping the match by declaration must not quietly turn
    // that into a ruling on nothing.
    const findings = [
      { family: "adv", corpus: "c", declaration: "suite" },
      { family: "adv", corpus: "c", declaration: "test-case" },
    ];
    const { families } = scoreFindings(
      findings,
      [],
      { adv: "advisory" },
      { adv: { present: [{ corpus: "c", scope: null }], absent: [] } },
    );
    const adv = families.find((f) => f.family === "adv")!;
    expect(adv.precision_basis!.truePositives).toBe(2);
    expect(adv.precision_basis!.unadjudicated).toBe(0);
  });

  it("TC-1001 a STANDING ruling covers only the declarations it names", () => {
    // TC-1001
    // The first measurement of `unadjudicated` read 316 of 319 for
    // `archetype-matches-nothing`, and the obvious way to move that number is
    // wrong: adding the token to seventy-six `expect.yaml` files would be 300
    // edits made to satisfy a counter. The fact is ONE fact — this corpus binds
    // the real ecosystem declaration, which declares `Inspections`,
    // `SuiteRegistry`, `NFR` and `StR` targets that no three-file fixture
    // carries — so `corpus.yaml` states it once and the runner reads it.
    //
    // SCOPED, never a wildcard. `test-case` is deliberately outside the ruling:
    // it is the declaration quire-rs#304 is about, so a firing there is still
    // something somebody has to rule on, and must stay counted.
    const findings = [
      { family: "adv", corpus: "a", declaration: "suite" },
      { family: "adv", corpus: "a", declaration: "inspection" },
      { family: "adv", corpus: "a", declaration: "test-case" },
    ];
    const standing = {
      adv: {
        present: [
          { corpus: "a", scope: "suite" },
          { corpus: "a", scope: "inspection" },
        ],
        absent: [],
      },
    };
    const { families } = scoreFindings(
      findings,
      [],
      { adv: "advisory" },
      standing,
    );
    const adv = families.find((f) => f.family === "adv")!;
    expect(adv.precision_basis!.truePositives).toBe(2);
    expect(adv.precision_basis!.falsePositives).toBe(0);
    // The `test-case` firing survives the ruling and stays counted.
    expect(adv.precision_basis!.unadjudicated).toBe(1);
  });

  it("TC-1002 a standing ruling is counted separately from a per-case one", () => {
    // TC-1002
    // Different strengths of evidence. The first run after
    // `archetype-matches-nothing` gained a standing ruling read precision 1.00
    // over 323 firings — of which 3 were the fixtures' own `expect.yaml` and
    // 320 came from ONE sentence in `corpus.yaml`. A single figure over both
    // reads as "right 323 times", which is not the claim the ruling makes.
    const { families } = scoreFindings(
      [
        { family: "adv", corpus: "a", declaration: "suite" },
        { family: "adv", corpus: "a", declaration: "test-case" },
      ],
      [],
      { adv: "advisory" },
      {
        adv: {
          present: [
            { corpus: "a", scope: "suite", standing: true },
            { corpus: "a", scope: "test-case" },
          ],
          absent: [],
        },
      },
    );
    const basis = families.find((f) => f.family === "adv")!.precision_basis!;
    expect(basis.truePositives).toBe(2);
    expect(basis.byStanding).toBe(1);
  });

  it("TC-987 a firing nobody ruled on is counted and published, never folded into the null", () => {
    // TC-987
    // What #234 shipped was a bare `null`, which reads as "nothing to see". The
    // state it was actually describing is 316 firings on which this benchmark
    // holds no opinion. Not-measured and zero are different claims, and so are
    // not-measured and not-asked.
    const { families } = scoreFindings(
      [
        { family: "adv", corpus: "a", declaration: "suite" },
        { family: "adv", corpus: "b", declaration: "suite" },
      ],
      [],
      { adv: "advisory" },
      {},
    );
    const adv = families.find((f) => f.family === "adv")!;
    expect(adv.precision).toBeNull();
    expect(adv.precision_basis!.unadjudicated).toBe(2);
    expect(adv.precision_basis!.rulings).toBe(0);
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

  it("TC-994 an absent cost is null, never 0, and the half it knows is still reported", () => {
    // TC-994
    // `?? 0` published "0 tokens" for tier 1, which calls no model at all — a
    // measurement claiming the run was free. And an all-or-nothing return threw
    // away the half it COULD report: tier 1 knows its subprocess count exactly.
    // Absent and zero are different claims (agent-ix/quoin#243).
    const tierOne = scoreCost({ toolCalls: 240 }, 29);
    expect(tierOne.tokens).toBeNull();
    expect(tierOne.tokensPer).toBeNull();
    expect(tierOne.toolCalls).toBe(240);
    expect(tierOne.toolCallsPer).toBe(8.28);
    expect(tierOne.truePositives).toBe(29);
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
