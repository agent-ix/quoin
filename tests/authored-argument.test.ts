/** FR-047 — authored assurance arguments (TC-1131..TC-1136). */

import { describe, expect, it } from "vitest";

import {
  buildAuthoredArgumentView,
  parseAssuranceArgument,
  renderAuthoredArgument,
  type AssuranceArgumentDefinition,
  type SufficiencyDecision,
} from "../src/assurance/index.js";

const digest = `sha256:${"b".repeat(64)}`;
const criterion = "Every binding synthetic clause has a current disposition.";

const argument: AssuranceArgumentDefinition = {
  id: "AA-900",
  title: "Synthetic widget release decision",
  type: "AssuranceArgument",
  status: "active",
  owner: "release-owner",
  profile: "ix://example.invalid/widget/AP-900",
  top_claim: {
    id: "CLAIM-900",
    statement: "The bounded synthetic widget change is acceptable.",
    subject: "widget revision 0123456789abcdef",
  },
  reasoning: [
    {
      id: "ARG-900",
      statement: "Argue from the explicitly reviewed clause disposition.",
      supports: "CLAIM-900",
      sufficiency_criteria: [criterion],
    },
  ],
  assumptions: [
    {
      id: "ASM-900",
      statement: "The test environment represents the bounded target.",
      owner: "release-owner",
      status: "accepted",
      review_by: "2026-09-01T00:00:00.000Z",
    },
  ],
  participants: [
    {
      id: "reviewer-900",
      role: "decision reviewer",
      authority: "may accept or reject this synthetic release",
      independence: "did not produce the implementation evidence",
    },
  ],
  challenges: [
    {
      id: "CH-900",
      target: "CLAIM-900",
      statement: "A bounded recovery case needed review.",
      status: "resolved",
      owner: "release-owner",
      resolution_refs: ["evidence://experiment/recovery-900"],
    },
  ],
  relationships: [
    {
      target: "ix://example.invalid/widget/AP-900",
      type: "references",
    },
  ],
};

const decision: SufficiencyDecision = {
  reasoningId: "ARG-900",
  criterion,
  state: "satisfied",
  evidenceRefs: ["evidence://discharge/widget-900"],
  decidedBy: "reviewer-900",
  authority: "may accept or reject this synthetic release",
  decidedAt: "2026-08-01T00:00:00.000Z",
  expiresAt: "2026-09-01T00:00:00.000Z",
  sourceRevision: "0123456789abcdef",
  evidenceDigest: digest,
};

describe("authored assurance arguments", () => {
  // Trace: FR-047-AC-1
  // TC-1131
  it("preserves the authored claim, authority, and independence", () => {
    const view = buildAuthoredArgumentView({
      argument,
      decisions: [decision],
      asOf: "2026-08-15T00:00:00.000Z",
    });
    expect(view.topClaim).toMatchObject({
      statement: argument.top_claim.statement,
      subject: argument.top_claim.subject,
      status: "supported",
    });
    expect(view.participants).toEqual(argument.participants);
    expect(view).not.toHaveProperty("score");
  });

  // Trace: FR-047-AC-2
  // TC-1132
  it("leaves an undecided criterion open instead of inferring from evidence", () => {
    const view = buildAuthoredArgumentView({
      argument,
      decisions: [],
      asOf: "2026-08-15T00:00:00.000Z",
    });
    expect(view.reasoning[0].criteria[0]).toMatchObject({
      status: "open",
      reason: "no sufficiency decision",
    });
    expect(view.topClaim.status).toBe("open");
  });

  // Trace: FR-047-AC-3
  // TC-1133
  it("reopens expired decisions and assumptions due for review", () => {
    const view = buildAuthoredArgumentView({
      argument: {
        ...argument,
        assumptions: [
          { ...argument.assumptions[0], review_by: "2026-08-10T00:00:00.000Z" },
        ],
      },
      decisions: [{ ...decision, expiresAt: "2026-08-10T00:00:00.000Z" }],
      asOf: "2026-08-15T00:00:00.000Z",
    });
    expect(view.reasoning[0].criteria[0].reason).toBe("decision is expired");
    expect(view.assumptions[0].reason).toBe("assumption review is due");
    expect(view.topClaim.status).toBe("open");
  });

  // Trace: FR-047-AC-4
  // TC-1134
  it("requires resolution evidence and a current expiry for accepted risk", () => {
    const view = buildAuthoredArgumentView({
      argument: {
        ...argument,
        challenges: [
          {
            ...argument.challenges[0],
            status: "accepted-risk",
            resolution_refs: ["decision://risk/900"],
          },
        ],
      },
      decisions: [decision],
      asOf: "2026-08-15T00:00:00.000Z",
    });
    expect(view.challenges[0]).toMatchObject({
      status: "open",
      reason: "accepted risk has no expiry",
    });
  });

  // Trace: FR-047-AC-5
  // TC-1135
  it("validates the closed authored contract and rejects duplicate decisions", () => {
    expect(parseAssuranceArgument(argument)).toEqual(argument);
    expect(() =>
      parseAssuranceArgument({ ...argument, invented_score: 100 }),
    ).toThrow("unknown field invented_score");
    expect(() =>
      buildAuthoredArgumentView({
        argument,
        decisions: [decision, decision],
        asOf: "2026-08-15T00:00:00.000Z",
      }),
    ).toThrow("duplicate sufficiency decision");
    expect(() =>
      buildAuthoredArgumentView({
        argument,
        decisions: [{ ...decision, authority: "self-declared authority" }],
        asOf: "2026-08-15T00:00:00.000Z",
      }),
    ).toThrow("does not match the authored participant");
  });

  // Trace: FR-047-AC-6
  // TC-1136
  it("renders open reasons and explicit decision state deterministically", () => {
    const view = buildAuthoredArgumentView({
      argument,
      decisions: [],
      asOf: "2026-08-15T00:00:00.000Z",
    });
    const first = renderAuthoredArgument(view);
    expect(first).toBe(renderAuthoredArgument(view));
    expect(first).toContain("OPEN");
    expect(first).toContain("no sufficiency decision");
    expect(first).toContain("Participants and authority");
  });
});
