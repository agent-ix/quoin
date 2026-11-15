---
id: SR-210
title: "Evidence review of PLAT-1436 dossier spec (quoin PR #924)"
type: SpecReview
analysis: evidence
scope: "agent-ix/quoin; PR #924 @ ab633975: verification methods of FR-188..FR-192 and NFR-039 criteria and metrics"
review_set: subset
relationships:
  - target: "ix://agent-ix/quoin/NFR-039"
    type: references
---

## Summary

Ticket: PLAT-1436. I ran `quoin advise --repo . --json` (quoin 0.34.0) and
filtered the result to this PR's 36 obligations: FR-188-AC-1..11, FR-189-AC-1..5,
FR-190-AC-1..5, FR-191-AC-1..5, FR-192-AC-1..6 and NFR-039-AC-1..4.

- Every one is authored `Test`.
- None is a mismatch, and every authored value is catalogued.
- One is inconclusive: FR-191-AC-1, where no rule matched.
- The recommendations are within class `Test`:
  - unit or bdd-spec-by-example for the example-shaped ACs;
  - property-based-testing for the ordering and gap ACs;
  - golden-approval-testing as an option for NFR-039-AC-1.

The authored methods are consistent with the advice. The defect is one
criterion whose stated method cannot produce the evidence it claims.

## Verdict

PASS with one medium finding.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | NFR-039-AC-2 and its metric "Clock or network reads during generation: Test" ask for a run "with the clock and network unavailable". A test cannot make the wall clock unavailable to a Rust process, and an offline run that gives the same bytes does not show that no read happened. The no-clock, no-network property should be verified by Inspection or static analysis of the dossier code path: no `SystemTime`, `Instant` or `now()` call, and no network crate in its dependency graph. That is the pattern quire-cli FR-034-AC-6 uses with `cargo deny` and `cargo tree -i`. | spec/non-functional/NFR-039-dossier-is-deterministic-and-records-only.md:45; spec/non-functional/NFR-039-dossier-is-deterministic-and-records-only.md:53 |
