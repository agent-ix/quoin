---
id: SR-20261113181014
title: "Specification integrity — PLAT-1525 PR #882 dispositions"
type: SpecReview
analysis: integrity
scope: "spec/functional/FR-046-clause-discharge.md, FR-173-facts-from-mappings.md, FR-174-per-clause-compliance-from-mappings.md"
review_set: subset
---

## Summary

The updated criteria cover the main disposition choices and refusal combinations, but they omit the runtime state transition at a temporary exception's authored expiry. The success report also has a state that cannot pass the specified validation and refusal rules.

## Verdict

**FAIL** — the temporary-exception lifecycle is not observable end to end, and the needs-disposition report condition conflicts with the input contract.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | No criterion requires a temporary exception to become open at its authored expiry | FR-046-AC-7, FR-173-AC-10, FR-173-AC-12, FR-174-AC-2 |
| FND-002 | medium | The needs-disposition success output is unreachable for a validated corpus | FR-173 |

### FND-001 — No criterion requires a temporary exception to become open at its authored expiry

The temporary decision has a required authored expiry and is bounded by it, but the specified run-time currentness rule and tests only mention the attestation interval. The mapping criterion checks that the authored expiry does not exceed the attestation expiry; the end-to-end criterion checks persistence and the three rendered labels without crossing the authored expiry. The criteria do not define or observe the boundary where a temporary decision stops counting. Add an explicit before/at-or-after boundary rule and a criterion verifying the stored decision no longer discharges the clause after its authored expiry.

### FND-002 — The needs-disposition success output is unreachable for a validated corpus

FR-173 lists needs-disposition under reports emitted on success, but its condition is missing disposition metadata. FR-016 makes those fields required; FR-173 refuses missing or malformed disposition values and refuses a --spec that fails validation. The report state is therefore contradictory or unreachable. Remove it, or define a separate refusal diagnostic and test it.

## Analysis notes

- No new dependency relationships were added; existing FR-173/FR-174 relationships remain consistent.
- Full spec/**/*.md validation exits 3 on an unrelated existing row-width error in spec/functional/FR-113-portfolio-priority-ranking.md:108. The three changed FR files validate and are grammar-clean.
