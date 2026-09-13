/**
 * FR-046-AC-1 — `parseClauseBinding` accepts only a validated
 * clause-binding-v1 report (TC-1125).
 *
 * The rest of FR-046 — the direct/disposition/open partition, expiry, the
 * duplicate-fact and attestation refusals and the renderer — moved to
 * `rust/crates/quoin-assurance/` (quoin#447) and is asserted there. What is
 * left here is the one criterion whose subject, `parseClauseBinding` in
 * `src/quire/validate.ts`, is RETAINED TypeScript, and this is the
 * repository's only test of it.
 */

import { describe, expect, it } from "vitest";

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

describe("clause discharge", () => {
  // Trace: FR-046-AC-1
  it("accepts only a validated clause-binding-v1 report", () => {
    const parsed = parseClauseBinding(JSON.stringify(binding));
    expect(parsed).toEqual({ ok: true, value: binding });

    const invalid = parseClauseBinding(
      JSON.stringify({ ...binding, clauseSetDigest: "not-a-digest" }),
    );
    expect(invalid.ok).toBe(false);
  });
});
