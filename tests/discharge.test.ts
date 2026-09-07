/** FR-046 — explicit clause discharge accounting (TC-1125..TC-1129). */

import { describe, expect, it } from "vitest";

import {
  buildDischargeReport,
  renderDischargeReport,
  type DischargeFact,
} from "../src/assurance/index.js";
import {
  parseClauseBinding,
  type ClauseBindingReport,
} from "../src/quire/index.js";

const digest = `sha256:${"a".repeat(64)}`;

const binding: ClauseBindingReport = {
  schemaVersion: "clause-binding-v1",
  clauseSet: {
    authority: "example.invalid",
    id: "synthetic-widget-rules",
    version: "1.0.0",
  },
  clauseSetDigest: digest,
  context: { product: "widget", deployment: "test" },
  clauses: [
    {
      clauseId: "SYN-001",
      force: "mandatory",
      outcome: "binding",
      reasons: [],
      expectedOutputs: ["test-result"],
    },
    {
      clauseId: "SYN-002",
      force: "recommended",
      outcome: "binding",
      reasons: [],
      expectedOutputs: ["review-record"],
    },
    {
      clauseId: "SYN-003",
      force: "mandatory",
      outcome: "binding",
      reasons: [],
      expectedOutputs: ["decision-record"],
    },
    {
      clauseId: "SYN-004",
      force: "mandatory",
      outcome: "unresolved",
      reasons: [
        {
          code: "missing-context",
          dimension: "environment",
          message: "environment is not known",
        },
      ],
      expectedOutputs: ["environment-record"],
    },
    {
      clauseId: "SYN-005",
      force: "permitted",
      outcome: "not_binding",
      reasons: [],
      expectedOutputs: [],
    },
  ],
};

const attestation = {
  attestedBy: "reviewer-1",
  authority: "quality-lead",
  attestedAt: "2026-08-01T00:00:00.000Z",
  expiresAt: "2026-09-01T00:00:00.000Z",
  sourceRevision: "0123456789abcdef",
  evidenceDigest: digest,
};

const direct: DischargeFact = {
  kind: "direct",
  clauseId: "SYN-001",
  evidenceRefs: ["evidence://run/one"],
  attestation,
};

const disposition: DischargeFact = {
  kind: "disposition",
  clauseId: "SYN-002",
  decision: "temporary_exception",
  rationale: "Synthetic exception for the bounded test window.",
  approvalRef: "decision://synthetic/one",
  attestation,
};

describe("clause discharge", () => {
  // Trace: FR-046-AC-1
  // TC-1125
  it("accepts only a validated clause-binding-v1 report", () => {
    const parsed = parseClauseBinding(JSON.stringify(binding));
    expect(parsed).toEqual({ ok: true, value: binding });

    const invalid = parseClauseBinding(
      JSON.stringify({ ...binding, clauseSetDigest: "not-a-digest" }),
    );
    expect(invalid.ok).toBe(false);
  });

  // Trace: FR-046-AC-2
  // TC-1126
  it("partitions every binding clause into direct, disposition, or open", () => {
    const report = buildDischargeReport({
      binding,
      facts: [direct, disposition],
      asOf: "2026-08-15T00:00:00.000Z",
    });

    expect(report.binding.direct.map((entry) => entry.clauseId)).toEqual([
      "SYN-001",
    ]);
    expect(report.binding.dispositions.map((entry) => entry.clauseId)).toEqual([
      "SYN-002",
    ]);
    expect(report.binding.open.map((entry) => entry.clauseId)).toEqual([
      "SYN-003",
    ]);
    expect(report).not.toHaveProperty("score");
  });

  // Trace: FR-046-AC-3
  // TC-1127
  it("keeps unresolved applicability separate and refuses to spend a fact on it", () => {
    const unresolvedFact: DischargeFact = {
      ...direct,
      clauseId: "SYN-004",
    };
    const report = buildDischargeReport({
      binding,
      facts: [unresolvedFact],
      asOf: "2026-08-15T00:00:00.000Z",
    });

    expect(report.unresolved).toEqual([
      expect.objectContaining({
        clauseId: "SYN-004",
        state: "unresolved",
        reason: "environment is not known",
      }),
    ]);
    expect(report.unusedFacts).toContainEqual({
      clauseId: "SYN-004",
      kind: "direct",
      reason: "unresolved",
    });
  });

  // Trace: FR-046-AC-4
  // TC-1128
  it("reopens a binding clause when its attestation is expired", () => {
    const expired: DischargeFact = {
      ...direct,
      attestation: {
        ...attestation,
        expiresAt: "2026-08-10T00:00:00.000Z",
      },
    };
    const report = buildDischargeReport({
      binding,
      facts: [expired],
      asOf: "2026-08-15T00:00:00.000Z",
    });

    expect(report.binding.open[0]).toMatchObject({
      clauseId: "SYN-001",
      reason: "discharge fact is expired",
    });
  });

  // Trace: FR-046-AC-5
  // TC-1129
  it("rejects duplicate facts and incomplete attestations", () => {
    expect(() =>
      buildDischargeReport({
        binding,
        facts: [direct, direct],
        asOf: "2026-08-15T00:00:00.000Z",
      }),
    ).toThrow("duplicate discharge fact");

    expect(() =>
      buildDischargeReport({
        binding,
        facts: [
          {
            ...direct,
            attestation: { ...attestation, authority: "" },
          },
        ],
        asOf: "2026-08-15T00:00:00.000Z",
      }),
    ).toThrow("authority must not be empty");

    expect(() =>
      buildDischargeReport({
        binding,
        facts: [
          {
            ...direct,
            kind: "invented",
          } as unknown as DischargeFact,
        ],
        asOf: "2026-08-15T00:00:00.000Z",
      }),
    ).toThrow("kind must be one of direct, disposition");

    expect(() =>
      buildDischargeReport({
        binding,
        facts: [
          {
            ...direct,
            inventedScore: 100,
          } as unknown as DischargeFact,
        ],
        asOf: "2026-08-15T00:00:00.000Z",
      }),
    ).toThrow("unknown field inventedScore");
  });

  // Trace: FR-046-AC-6
  // TC-1130
  it("renders every population without inventing a score", () => {
    const report = buildDischargeReport({
      binding,
      facts: [direct, disposition],
      asOf: "2026-08-15T00:00:00.000Z",
    });
    const rendered = renderDischargeReport(report);
    expect(rendered).toContain("## Direct evidence");
    expect(rendered).toContain("## Open binding clauses");
    expect(rendered).toContain("## Unresolved applicability");
    expect(rendered).not.toContain("Score");
  });
});
