---
id: SR-20261113155333
title: "Integrity review of the FR-116 analysis-act producer definition (FR-184, FR-153 amendment)"
type: SpecReview
analysis: integrity
review_set: subset
scope: "agent-ix/quoin; PR #862 at 404cb32c: spec/functional/FR-184-analysis-act-producer-definition.md (AC-1 to AC-7, CON-1, CON-2), spec/functional/FR-153-analysis-result-counting-by-disposition.md (AC-2)"
---

# Integrity review of the FR-116 analysis-act producer definition (FR-184, FR-153 amendment)

## Summary

Ticket: VER-107. I checked FR-184's ACs and constraints, and the amended FR-153-AC-2, for atomicity,
testability and coverage of the normative text.

These are atomic and testable: AC-1, AC-2, AC-6 and AC-7, CON-1 and CON-2, and FR-153-AC-2. FR-153-AC-2
is two cases, but each has a named expected premise.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-184-AC-3 lumps seven cases together under "each with its FR-153 or FR-116 category" and names none of the categories. Two of them are ambiguous. A "superseded run" is `stale` when the later run fails or errors, and `absent` when the later run lacks the symbol (FR-153). A "non-CI execution association" is an evaluated refusal (`qualification-rejected`, FR-116), while a missing association is `unevaluated`. Split the AC, or name the expected category for each case. | FR-184-AC-3; spec/functional/FR-116-owner-qualified-quire-evidence.md:111, :117; FR-153 Behavior (stale/absent) |
| FND-002 | medium | FR-184-AC-4 ("The definition names ...") and FR-184-AC-5 ("The text states ...") inspect FR-184's own text. Once the file merges they cannot fail, and they verify nothing about the qualifier. Replace them with behavioural criteria, for example a report from a solver not started through `producer-execute` yields no positive decision, or delete them. | FR-184-AC-4; FR-184-AC-5 |
| FND-003 | low | No AC covers "a report from a solver started any other way is not an observation of it". The text also does not name which supplied observation lets the qualifier tell, under CON-2's rule of reading only supplied observations. Name the observation, or the premise (for example the execution association), and add a case. | spec/functional/FR-184-analysis-act-producer-definition.md:52-53; FR-184-CON-2 |

## Verdict

Fix FND-001 and FND-002 in this PR. Both are confined to the AC table and change no approved rule.

Examined (role examined): FR-184-AC-1 to AC-7, FR-184-CON-1 and CON-2, FR-153-AC-2, and the What runs
exclusion.

Context only: FR-116 result table, FR-153 Behavior.
