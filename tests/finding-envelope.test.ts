import { describe, expect, it } from "vitest";

import {
  available,
  normalizeExternalObservation,
  normalizeQuireFinding,
  normalizeQuoinFinding,
  notApplicable,
  unavailable,
  validateFindingEnvelope,
} from "../evals/lib/finding-envelope.mjs";
import {
  scoreActionability,
  scoreActionabilityV2,
  scoreFindings,
} from "../evals/lib/quality.mjs";

describe("finding-envelope-v2", () => {
  it("TC-1080 normalizes all three producer classes without inventing evidence", () => {
    const quireRaw = {
      reason: "status-column-matches-nothing",
      path: "spec/tests.md",
      line: 30,
    };
    const quire = normalizeQuireFinding(quireRaw, {
      producer: "quire",
      channel: "coverage.diagnostics",
      family: "status-column-matches-nothing",
    });
    const quoin = normalizeQuoinFinding(
      { kind: "mocked-confirmation", path: "tests/a.ts", line: 7 },
      { producer: "quoin", channel: "evidence.audit" },
    );
    const external = normalizeExternalObservation(
      {
        kind: "manual-review",
        subject: "FR-001-AC-1",
        causalEvidence: "reviewer observed the wrong subject",
        changeTarget: notApplicable("observation records behavior only"),
        nextMove: notApplicable("observation records behavior only"),
      },
      { producer: "reviewer", channel: "retained-ruling" },
    );

    expect(quire.source.class).toBe("quire");
    expect(quire.locus).toEqual(available({ path: "spec/tests.md", line: 30 }));
    expect(quire.causalEvidence).toEqual(
      unavailable("producer emitted no causal evidence"),
    );
    expect(quire.raw).toBe(quireRaw);
    expect(quoin.source.class).toBe("quoin");
    expect(external.source.class).toBe("external-observation");
    expect(external.changeTarget.state).toBe("not_applicable");
  });

  it("TC-1081 rejects missing, malformed, and unknown-version records", () => {
    expect(() =>
      normalizeQuireFinding({ reason: "x" }, { channel: "diagnostic" }),
    ).toThrow(/producer is required/);
    const valid = normalizeQuireFinding(
      { reason: "x" },
      { producer: "quire", channel: "diagnostic" },
    );
    expect(() =>
      validateFindingEnvelope({ ...valid, causalEvidence: { state: "lost" } }),
    ).toThrow(/invalid availability state/);
    expect(() =>
      validateFindingEnvelope({
        ...valid,
        schemaVersion: "finding-envelope-v3",
      }),
    ).toThrow(/unsupported schemaVersion/);
  });

  it("TC-1082 scores positive, negative, unavailable, and not-applicable records explicitly", () => {
    const full = normalizeQuireFinding(
      {
        reason: "configured-column-missing",
        subject: "functional-coverage table",
        path: "spec/tests.md",
        line: 30,
        causalEvidence: "configured Status; observed Coverage Status",
        changeTarget: "status.column or the table header",
        nextDiagnosticStep: "compare the configured and observed columns",
      },
      { producer: "quire", channel: "coverage.diagnostics", family: "column" },
    );
    const missing = normalizeQuoinFinding(
      { kind: "validation", path: "spec/a.md", line: 2 },
      { producer: "quoin", channel: "validate", family: "validation" },
    );
    const unavailableEvidence = normalizeQuoinFinding(
      {
        kind: "validation",
        subject: "FR-002",
        causalEvidence: unavailable("producer redacted protected evidence"),
        changeTarget: "FR-002",
        remedy: "supply reviewable evidence",
      },
      { producer: "quoin", channel: "validate", family: "validation" },
    );
    const excluded = normalizeExternalObservation(
      {
        kind: "observation",
        subject: notApplicable("population-level observation"),
        locus: notApplicable("population-level observation"),
        causalEvidence: "retained reviewer ruling",
        changeTarget: notApplicable("no product repair is asserted"),
        nextMove: notApplicable("no product repair is asserted"),
      },
      { producer: "reviewer", channel: "ruling", family: "observation" },
    );

    const scored = scoreActionabilityV2([
      full,
      missing,
      unavailableEvidence,
      excluded,
    ]);
    expect(scored).toMatchObject({
      numerator: 1,
      denominator: 3,
      rate: 0.333,
    });
    expect(scored.namedMisses).toHaveLength(2);
    expect(
      scored.namedMisses[0].missing.map((m: { field: string }) => m.field),
    ).toEqual(["causal_evidence", "change_target", "next_move"]);
    expect(scored.exclusions).toHaveLength(1);
    expect(scored.partitions).toEqual([
      expect.objectContaining({
        id: "external-observation/reviewer/ruling/observation",
        numerator: 0,
        denominator: 0,
        rate: null,
        exclusions: [expect.objectContaining({ id: expect.any(String) })],
      }),
      expect.objectContaining({
        id: "quire/quire/coverage.diagnostics/column",
        numerator: 1,
        denominator: 1,
        rate: 1,
      }),
      expect.objectContaining({
        id: "quoin/quoin/validate/validation",
        numerator: 0,
        denominator: 2,
        rate: 0,
        namedMisses: [
          expect.objectContaining({ id: expect.any(String) }),
          expect.objectContaining({ id: expect.any(String) }),
        ],
      }),
    ]);
  });

  it("TC-1083 keeps v1 reproducible while L1/L2/L3-family scoring consumes envelopes", () => {
    const normalized = normalizeQuireFinding(
      { reason: "located", path: "spec/a.md", line: 7 },
      {
        producer: "quire",
        channel: "coverage.diagnostics",
        family: "located",
        corpus: "case-a",
      },
    );
    expect(scoreActionability([normalized])).toMatchObject({
      definitionVersion: "finding.actionability-v1",
      actionable: 1,
      total: 1,
      rate: 1,
    });
    expect(
      scoreFindings(
        [normalized],
        [
          {
            id: "label-a",
            family: "located",
            corpus: "case-a",
            location: "spec/a.md:7",
          },
        ],
      ),
    ).toMatchObject({ positional: 1, families: [{ truePositives: 1 }] });
  });
});
