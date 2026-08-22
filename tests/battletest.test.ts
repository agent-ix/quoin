/**
 * The committed battletest runner (quoin#203).
 *
 * A battletest was a manual session whose findings were transcribed into prose
 * and ad-hoc frozen into unit tests. This makes pass 3 a command.
 *
 * It does NOT replace the human pass — every conclusion-changing finding of
 * pass 2 came from somebody reading code, and a runner claiming otherwise
 * would be the overclaim this programme exists to end. It replaces the
 * RE-RUN: checking whether what was found before is still found.
 */

import { readFileSync } from "node:fs";
import { join } from "node:path";

import { diff, render, scoreAgainstKey } from "../scripts/battletest.mjs";

const key = JSON.parse(
  readFileSync(join(__dirname, "..", "bench", "answer-key.json"), "utf8"),
);

describe("scoring against the adjudicated answer key", () => {
  it("counts a finding as detected when the payload carries its signal", () => {
    const payload = {
      diagnostics: [
        { reason: "no-symbol-bound" },
        { reason: "hollow-denominator" },
      ],
      suspicions: [{ kind: "vacuous-under-guard" }],
      metrics: [{ name: "coverage.specific_shaped", value: 78 }],
    };
    const score = scoreAgainstKey(payload, key);
    expect(score.detected).toContain("AK-001"); // no-symbol-bound
    expect(score.detected).toContain("AK-002"); // hollow-denominator
    expect(score.detected).toContain("AK-003"); // specific_shaped === 0
    expect(score.detected).toContain("AK-004"); // vacuous-under-guard
  });

  it("does not count an unmechanized finding as a miss", () => {
    // AK-006 and AK-007 have no detector yet. Scoring them as failures would
    // make the number move when nothing changed, and would understate recall
    // over the set the tools can actually be judged on.
    const score = scoreAgainstKey({}, key);
    expect(score.notMechanized.length).toBeGreaterThan(0);
    for (const id of score.notMechanized) {
      expect(score.missed).not.toContain(id);
      expect(score.detected).not.toContain(id);
    }
  });

  it("reports recall over the mechanized set, and null when there is none", () => {
    const none = scoreAgainstKey({}, key);
    expect(none.recall).toBe(0);
    // 0/0 is not 0% — a key with nothing mechanized has no recall to report.
    const empty = scoreAgainstKey({}, { findings: [{ id: "AK-999" }] });
    expect(empty.recall).toBeNull();
  });

  it("requires the metric to match its expected VALUE, not merely exist", () => {
    // A metric present but wrong is not a detection. The key carries the
    // ADJUDICATED figure (78 of 951); a different number is a different
    // corpus state, not evidence of the finding.
    const wrong = scoreAgainstKey(
      { metrics: [{ name: "coverage.specific_shaped", value: 5 }] },
      key,
    );
    expect(wrong.detected).not.toContain("AK-003");
    expect(wrong.missed).toContain("AK-003");
  });
});

describe("diffing against the baseline", () => {
  it("treats a LOST finding as the regression, not a changed total", () => {
    // The question a re-run answers is "does the toolchain still surface what
    // it surfaced before". A finding gained is good news; a finding lost is
    // the failure, and netting them would hide it.
    const delta = diff(
      { detected: ["AK-001", "AK-002"], recall: 1 },
      {
        detected: ["AK-002", "AK-003"],
        missed: [],
        notMechanized: [],
        recall: 1,
      },
    );
    expect(delta.lost).toEqual(["AK-001"]);
    expect(delta.gained).toEqual(["AK-003"]);
    expect(
      render(
        {
          detected: ["AK-002", "AK-003"],
          missed: [],
          notMechanized: [],
          recall: 1,
        },
        delta,
      ),
    ).toContain("LOST");
  });

  it("handles a first run with no baseline", () => {
    const delta = diff(null, {
      detected: ["AK-001"],
      missed: [],
      notMechanized: [],
      recall: 1,
    });
    expect(delta.lost).toEqual([]);
    expect(delta.gained).toEqual(["AK-001"]);
    expect(delta.recallBefore).toBeNull();
  });

  it("says so when nothing moved", () => {
    const score = {
      detected: ["AK-001"],
      missed: [],
      notMechanized: [],
      recall: 1,
    };
    expect(
      render(score, diff({ detected: ["AK-001"], recall: 1 }, score)),
    ).toContain("no change against the baseline");
  });
});
