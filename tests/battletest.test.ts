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

import {
  chmodSync,
  mkdirSync,
  mkdtempSync,
  readFileSync,
  writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

import {
  collectTier2Sources,
  diff,
  render,
  scoreAgainstKey,
  scoreAgainstSources,
} from "../scripts/battletest.mjs";
import {
  compareTier2Baseline,
  createTier2Baseline,
} from "../scripts/lib/tier2-baseline.mjs";

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
    // This is a runner-mapping state, not a claim that no production detector
    // exists. Keep the semantic test independent of today's answer key.
    const score = scoreAgainstKey({}, { findings: [{ id: "AK-999" }] });
    expect(score.notMechanized).toEqual(["AK-999"]);
    expect(score.missed).toEqual([]);
    expect(score.detected).toEqual([]);
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

  it("TC-1072 scores an evaluated production source with no finding as a miss", () => {
    const score = scoreAgainstSources(
      { "quoin.validate": { ok: true, payload: { findings: [] } } },
      {
        findings: [
          {
            id: "AK-999",
            source: "quoin.validate",
            expect_finding: "gate-that-gates-nothing",
          },
        ],
      },
    );
    expect(score.missed).toEqual(["AK-999"]);
    expect(score.notEvaluated).toEqual([]);
  });

  it("TC-1073 names an unavailable source and does not coerce it to a miss", () => {
    const score = scoreAgainstSources(
      {
        "quoin.evidence-audit": {
          ok: false,
          reason: "no suite-to-obligation join exists",
        },
      },
      {
        findings: [
          {
            id: "AK-998",
            source: "quoin.evidence-audit",
            expect_finding: "mocked-confirmation",
          },
        ],
      },
    );
    expect(score.missed).toEqual([]);
    expect(score.notMechanized).toEqual(["AK-998"]);
    expect(render(score, null)).toContain(
      "NOT EVALUATED AK-998 via quoin.evidence-audit: no suite-to-obligation join exists",
    );
  });

  it("TC-1074 executes the declared Tier-2 source registry", () => {
    const root = mkdtempSync(join(tmpdir(), "quoin-tier2-sources-"));
    const corpus = join(root, "corpus");
    mkdirSync(corpus);
    const quire = join(root, "quire");
    writeFileSync(
      quire,
      `#!/bin/sh\nprintf '%s' '{"diagnostics":[],"suspicions":[],"metrics":[]}'\n`,
    );
    chmodSync(quire, 0o755);
    const quoin = join(root, "quoin.mjs");
    writeFileSync(
      quoin,
      `process.stdout.write(JSON.stringify({findings:[{kind:"gate-that-gates-nothing"}]}));\n`,
    );

    const sources = collectTier2Sources({ quire, quoin, corpus });
    expect(sources["quire.coverage"].ok).toBe(true);
    expect(sources["quoin.validate"].payload.findings).toEqual([
      { kind: "gate-that-gates-nothing" },
    ]);
    expect(sources["quoin.evidence-audit"]).toMatchObject({
      ok: false,
      reason: expect.stringContaining("bindings.json is absent"),
    });
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

describe("retained multi-source Tier-2 baseline", () => {
  const provenance = {
    answer_key_digest: "sha256:key",
    corpus: { revision: "fc5d644" },
    declaration: { revision: "module-sha" },
    tools: {
      quire: { digest: "sha256:quire" },
      quoin: { digest: "sha256:quoin" },
    },
    environment: { node: "v22", platform: "linux", arch: "x64" },
  };

  it("TC-1101 retains raw and normalized output for every supported producer", () => {
    const record = createTier2Baseline({
      provenance,
      sources: {
        "quire.coverage": {
          ok: true,
          state: "evaluated",
          command: { executable: "QUIRE", args: ["coverage"] },
          payload: {
            diagnostics: [
              { reason: "no-symbol-bound", path: "spec/FR-001.md" },
            ],
            suspicions: [],
            metrics: [{ name: "coverage.backed", value: 0 }],
          },
        },
        "quoin.validate": {
          ok: true,
          state: "evaluated",
          command: { executable: "NODE", args: ["QUOIN", "validate"] },
          payload: {
            findings: [
              {
                kind: "gate-that-gates-nothing",
                path: "Makefile",
                line: 5,
              },
            ],
          },
        },
        "quoin.evidence-audit": {
          ok: false,
          state: "unavailable",
          command: { executable: "NODE", args: ["QUOIN", "evidence"] },
          reason: "no suite-to-obligation join exists",
        },
      },
      score: {
        detected: ["AK-001"],
        missed: ["AK-007"],
        notMechanized: ["AK-006"],
        notEvaluated: [{ id: "AK-006", source: "quoin.evidence-audit" }],
        recall: 0.5,
      },
    });
    expect(record.sources["quire.coverage"].raw.diagnostics).toHaveLength(1);
    expect(
      record.sources["quire.coverage"].normalized.findings[0].schemaVersion,
    ).toBe("finding-envelope-v2");
    expect(record.sources["quoin.validate"].normalized.findings).toHaveLength(
      1,
    );
    expect(record.sources["quoin.evidence-audit"]).toMatchObject({
      state: "unavailable",
      raw: null,
      normalized: {
        state: "unavailable",
        reason: "no suite-to-obligation join exists",
      },
    });
  });

  it("TC-1102 keeps expected unavailability outside clean and missed states", () => {
    const baseline = createTier2Baseline({
      provenance,
      sources: {
        evidence: {
          ok: false,
          state: "unavailable",
          command: { executable: "NODE", args: [] },
          reason: "bindings absent",
        },
      },
      score: {
        detected: [],
        missed: [],
        notMechanized: ["AK-006"],
        notEvaluated: [{ id: "AK-006", source: "evidence" }],
        recall: null,
      },
    });
    const snapshot = structuredClone(baseline);
    const comparison = compareTier2Baseline(
      baseline,
      structuredClone(baseline),
    );
    expect(comparison.comparable).toBe(true);
    expect(comparison.lost).toEqual([]);
    expect(comparison.source_regressions).toEqual([]);
    expect(comparison.source_changes).toEqual([]);
    expect(baseline).toEqual(snapshot);
  });

  it("TC-1103 candidate comparison reports unavailable regressions and never rewrites the baseline", () => {
    const evaluated = createTier2Baseline({
      provenance,
      sources: {
        evidence: {
          ok: true,
          state: "evaluated",
          command: { executable: "NODE", args: [] },
          payload: { findings: [] },
        },
      },
      score: {
        detected: ["AK-006"],
        missed: [],
        notMechanized: [],
        notEvaluated: [],
        recall: 1,
      },
    });
    const unavailable = createTier2Baseline({
      provenance,
      sources: {
        evidence: {
          ok: false,
          state: "unavailable",
          command: { executable: "NODE", args: [] },
          reason: "bindings absent",
        },
      },
      score: {
        detected: [],
        missed: [],
        notMechanized: ["AK-006"],
        notEvaluated: [{ id: "AK-006", source: "evidence" }],
        recall: null,
      },
    });
    const before = JSON.stringify(evaluated);
    const comparison = compareTier2Baseline(evaluated, unavailable);
    expect(comparison.lost).toEqual(["AK-006"]);
    expect(comparison.source_regressions).toEqual([
      {
        source: "evidence",
        before: "evaluated",
        after: "unavailable",
        reason: "bindings absent",
      },
    ]);
    expect(JSON.stringify(evaluated)).toBe(before);
  });
});
