---
id: SR-20261113000902
title: "Integrity review of SOUP promotion, advisory join and report (FR-141..FR-144, NFR-030)"
type: SpecReview
analysis: integrity
review_set: subset
scope: "agent-ix/quoin; spec/functional/FR-141..FR-144, spec/non-functional/NFR-030, spec/functional/index.md, spec/non-functional/index.md, spec/spec.md (PR #837 diff, git diff origin/main...HEAD)"
---
# SR-20261113000902: Integrity review of SOUP promotion, advisory join and report (FR-141..FR-144, NFR-030)

## Summary

Ticket: PLAT-1513. Checks the new requirements for consistency, completeness and atomicity against each other and against the merged spec-objects-security FR-009..FR-012 and NFR-001. Structural defects in the join and promotion are recorded in the base review (SR-20261113000901) and not repeated here.

Clean: the IDs run FR-141..FR-144 and NFR-030 with no gap or duplicate, and both indexes and `spec/spec.md` list all five. The record-authoritative rule (FR-143 line 33), the disagreement flag and the 'unknown stays unknown' rule agree with FR-010 and NFR-001. FR-144's duplicate-purl refusal agrees with FR-009-AC-3. NFR-030's metrics each have a test AC.

## Verdict

PASS with findings. FND-001 and FND-002 are medium and fixable in this PR.

## Findings

| ID | Severity | Summary | Refs |
|----|----------|---------|------|
| FND-001 | medium | 'A version-less entry ... matches' the record's purl is not defined. The spec does not say whether the record purl is compared with `@version`, qualifiers and subpath removed, or whether namespace and type must be equal. Duplicate detection on line 32 ('the same purl') has the same gap: `pkg:cargo/a` and `pkg:cargo/a@1` are different text but can both match one record. Fix: state the match as type+namespace+name equal, with the version equal or absent in the entry (and say how qualifiers are handled), and use that relation for precedence and for the duplicate check. | spec/functional/FR-143-support-status-and-end-of-support-input.md:24-26 |
| FND-002 | medium | `asOf` is an ISO-8601 instant (FR-046 and `quoin discharge --as-of`) and `end_of_support` is a calendar date, with no time zone and no rule for comparing them. With `--as-of 2026-05-31T12:00:00Z` and an end of support of 2026-05-31, one implementer flags the component and another does not. AC-5 tests only a date-only `--as-of`. Fix: state the rule, for example 'past end of support when the UTC calendar date of asOf is after end_of_support', and add an AC with an instant inside the end-of-support day. | spec/functional/FR-143-support-status-and-end-of-support-input.md:40 |
| FND-003 | low | Validation of an entry checks `purl`, `support_status` and `source`, but not the `end_of_support` value. An entry with `end_of_support: "soon"` passes and is shown as a supplied date. Fix: refuse an entry whose `end_of_support` is neither a calendar date nor `unknown`, as FR-010 does for the record, and add that case to AC-1. | spec/functional/FR-143-support-status-and-end-of-support-input.md:31 |
| FND-004 | low | The absent-versus-empty rule is stated for `serves` only. `known_anomalies` has the same two states (FR-010-AC-2), and 'state `unknown` or `not stated` as the record does' leaves its rendering open. Fix: state `not stated` for an absent `known_anomalies` and `none` for an empty one, and extend AC-2 to cover it. | spec/functional/FR-144-soup-report.md:30-31 |
