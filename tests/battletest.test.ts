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
  assertPromotionDisposition,
  diff,
  render,
  scoreAgainstCohorts,
  scoreAgainstKey,
  scoreAgainstRetainedCohorts,
  scoreAgainstRetainedSources,
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
  it("pins the promotion disposition without relabeling historical keys", () => {
    expect(() =>
      assertPromotionDisposition({
        detected: ["AK-001", "AK-002", "AK-003", "AK-004", "AK-005"],
        missed: [],
        unavailable: [{ id: "AK-006" }],
        invalidAnswerKey: [{ id: "AK-007" }],
        controlFailures: [],
        notEvaluated: [],
      }),
    ).not.toThrow();
    expect(() =>
      assertPromotionDisposition({
        detected: ["AK-001", "AK-002", "AK-003", "AK-004", "AK-007"],
        unavailable: [{ id: "AK-006" }],
        invalidAnswerKey: [],
        controlFailures: [],
      }),
    ).toThrow(/detected/);
  });
  it("counts a finding as detected when the payload carries its signal", () => {
    const payload = {
      diagnostics: [
        { reason: "no-symbol-bound" },
        { reason: "hollow-denominator" },
      ],
      suspicions: [
        {
          kind: "vacuous-under-guard",
          path: "crates/filament-shell/tests/property_suite.rs",
          line: 91,
        },
      ],
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
    expect(
      sources["quoin.validate"],
      JSON.stringify(sources["quoin.validate"]),
    ).toMatchObject({
      ok: true,
      state: "evaluated",
    });
    expect(sources["quoin.validate"].payload.findings).toEqual([
      { kind: "gate-that-gates-nothing" },
    ]);
    expect(sources["quoin.evidence-audit"]).toMatchObject({
      ok: false,
      reason: expect.stringContaining("bindings.json is absent"),
    });
  });

  it("TC-1104 scores retained finding envelopes rather than raw producer arrays", () => {
    const retained = createTier2Baseline({
      provenance: {},
      cohorts: {
        defect: {
          provenance: {},
          sources: {
            "quoin.validate": {
              ok: true,
              state: "evaluated",
              command: { executable: "NODE", args: [] },
              payload: {
                findings: [
                  { kind: "gate-that-gates-nothing", path: "Makefile" },
                ],
              },
            },
          },
        },
      },
      score: {},
    }).cohorts.defect.sources;
    const answerKey = {
      findings: [
        {
          id: "AK-NORMALIZED",
          source: "quoin.validate",
          expect_finding: "gate-that-gates-nothing",
        },
      ],
    };

    retained["quoin.validate"].raw.findings[0].kind = "raw-only-value";
    expect(scoreAgainstRetainedSources(retained, answerKey).detected).toEqual([
      "AK-NORMALIZED",
    ]);

    retained["quoin.validate"].normalized.findings = [];
    expect(scoreAgainstRetainedSources(retained, answerKey).missed).toEqual([
      "AK-NORMALIZED",
    ]);
  });

  it("TC-1116 requires an exact defect locus and a clean pinned control", () => {
    const payload = (line: number | null) => ({
      diagnostics: [],
      metrics: [],
      suspicions:
        line === null
          ? []
          : [
              {
                kind: "oracle-resembles-implementation",
                path: "tests/property_suite.rs",
                line,
              },
            ],
    });
    const answerKey = {
      findings: [
        {
          id: "AK-EXACT",
          cohort: "defect",
          source: "quire.coverage",
          declaration: { state: "pinned-by-cohort" },
          evidence_sidecar: { state: "not-required" },
          reproduction_command: { executable: "QUIRE", args: [] },
          locus: { state: "exact" },
          expected_locus: { path: "tests/property_suite.rs", line: 72 },
          healthy_control: {
            state: "pinned",
            cohort: "control",
            source: "quire.coverage",
          },
          expect_suspicion: "oracle-resembles-implementation",
        },
      ],
    };
    const score = scoreAgainstCohorts(
      {
        defect: {
          sources: { "quire.coverage": { ok: true, payload: payload(72) } },
        },
        control: {
          sources: { "quire.coverage": { ok: true, payload: payload(null) } },
        },
      },
      answerKey,
    );
    expect(score.detected).toEqual(["AK-EXACT"]);
    expect(score.controlFailures).toEqual([]);
  });

  it("TC-1117 distinguishes unavailable, invalid-key, and healthy-control failure", () => {
    const retained = (findings: object[]) =>
      createTier2Baseline({
        provenance: {},
        cohorts: {
          one: {
            provenance: {},
            sources: {
              "quoin.validate": {
                ok: true,
                state: "evaluated",
                command: { executable: "NODE", args: [] },
                payload: { findings },
              },
              "quoin.evidence-audit": {
                ok: false,
                state: "unavailable",
                command: { executable: "NODE", args: [] },
                reason: "immutable binding absent",
              },
            },
          },
          control: {
            provenance: {},
            sources: {
              "quoin.validate": {
                ok: true,
                state: "evaluated",
                command: { executable: "NODE", args: [] },
                payload: { findings },
              },
            },
          },
        },
        score: {},
      }).cohorts;
    const fields = {
      declaration: { state: "pinned-by-cohort" },
      evidence_sidecar: { state: "not-required" },
      reproduction_command: { executable: "NODE", args: [] },
      locus: { state: "exact" },
    };
    const score = scoreAgainstRetainedCohorts(
      retained([{ kind: "gate-that-gates-nothing", path: "Makefile" }]),
      {
        findings: [
          {
            id: "AK-CONTROL",
            cohort: "one",
            source: "quoin.validate",
            ...fields,
            expect_finding: "gate-that-gates-nothing",
            healthy_control: { state: "pinned", cohort: "control" },
          },
          {
            id: "AK-UNAVAILABLE",
            cohort: "one",
            source: "quoin.evidence-audit",
            ...fields,
            expect_finding: "mocked-confirmation",
            healthy_control: { state: "unavailable" },
          },
          {
            id: "AK-NOT-EVALUATED",
            cohort: "one",
            source: "producer.without-retained-run",
            ...fields,
            expect_finding: "some-signal",
            healthy_control: { state: "not-required" },
          },
          {
            id: "AK-INVALID",
            answer_key_state: "invalid",
            invalid_reason: "exact snapshot absent",
          },
        ],
      },
    );
    expect(score.detected).toEqual([]);
    expect(score.missed).toEqual(["AK-CONTROL"]);
    expect(score.unavailable).toEqual([
      expect.objectContaining({ id: "AK-UNAVAILABLE" }),
    ]);
    expect(score.notEvaluated).toEqual([
      expect.objectContaining({ id: "AK-NOT-EVALUATED" }),
    ]);
    expect(score.invalidAnswerKey).toEqual([
      { id: "AK-INVALID", reason: "exact snapshot absent" },
    ]);
    expect(score.controlFailures).toEqual([
      expect.objectContaining({ id: "AK-CONTROL", cohort: "control" }),
    ]);
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
    cohort_manifest_digest: "sha256:cohorts",
    tools: {
      quire: { digest: "sha256:quire" },
      quoin: { digest: "sha256:quoin" },
    },
    environment: { node: "v22", platform: "linux", arch: "x64" },
  };
  const cohort = (sources: object) => ({
    provenance: {
      corpus: { revision: "corpus-sha" },
      declaration: { revision: "module-sha" },
    },
    sources,
  });

  it("TC-1119 retains the committed v2 cohort states and exact score buckets", () => {
    const baseline = JSON.parse(
      readFileSync(
        join(__dirname, "..", "bench", "battletest-baseline.json"),
        "utf8",
      ),
    );
    expect(baseline.schema_version).toBe("tier2-finding-quality-v2");
    expect(baseline.score).toMatchObject({
      detected: ["AK-004", "AK-005"],
      missed: ["AK-001", "AK-002", "AK-003"],
      notMechanized: ["AK-006"],
      unavailable: [expect.objectContaining({ id: "AK-006" })],
      notEvaluated: [],
      invalidAnswerKey: [expect.objectContaining({ id: "AK-007" })],
      controlFailures: [],
      recall: 0.4,
    });
    expect(
      baseline.cohorts["semantic-review-defect"].sources["quire.coverage"]
        .state,
    ).toBe("evaluated");
    expect(
      baseline.cohorts["semantic-review-control"].sources["quire.coverage"]
        .state,
    ).toBe("evaluated");
    expect(
      baseline.cohorts["semantic-review-defect"].sources["quoin.evidence-audit"]
        .state,
    ).toBe("unavailable");
    expect(
      compareTier2Baseline(baseline, structuredClone(baseline)),
    ).toMatchObject({
      comparable: true,
      lost: [],
      source_regressions: [],
      source_changes: [],
    });
  });

  it("TC-1101 retains raw and normalized output for every supported producer", () => {
    const record = createTier2Baseline({
      provenance,
      cohorts: {
        defect: cohort({
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
        }),
      },
      score: {
        detected: ["AK-001"],
        missed: ["AK-007"],
        notMechanized: ["AK-006"],
        notEvaluated: [{ id: "AK-006", source: "quoin.evidence-audit" }],
        recall: 0.5,
      },
    });
    const sources = record.cohorts.defect.sources;
    expect(sources["quire.coverage"].raw.diagnostics).toHaveLength(1);
    expect(sources["quire.coverage"].normalized.findings[0].schemaVersion).toBe(
      "finding-envelope-v2",
    );
    expect(sources["quoin.validate"].normalized.findings).toHaveLength(1);
    expect(sources["quoin.evidence-audit"]).toMatchObject({
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
      cohorts: {
        defect: cohort({
          evidence: {
            ok: false,
            state: "unavailable",
            command: { executable: "NODE", args: [] },
            reason: "bindings absent",
          },
        }),
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
      cohorts: {
        defect: cohort({
          evidence: {
            ok: true,
            state: "evaluated",
            command: { executable: "NODE", args: [] },
            payload: { findings: [] },
          },
        }),
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
      cohorts: {
        defect: cohort({
          evidence: {
            ok: false,
            state: "unavailable",
            command: { executable: "NODE", args: [] },
            reason: "bindings absent",
          },
        }),
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
        cohort: "defect",
        source: "evidence",
        before: "evaluated",
        after: "unavailable",
        reason: "bindings absent",
      },
    ]);
    expect(JSON.stringify(evaluated)).toBe(before);
  });
});
