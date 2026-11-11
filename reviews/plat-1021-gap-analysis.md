---
id: SR-20261111193747
title: "Gap analysis of quoin PR 811 (graded run measurement record)"
type: SpecReview
analysis: base
scope: "agent-ix/quoin@07cd5722; spec/functional/FR-134-graded-run-measurement-record.md, spec/functional/FR-108-measurement-verdict-checker.md (AC-15..AC-23), spec/matrix.md (FR-108, FR-134, TC-2013..2022, TC-2800, TC-2801), tagged tests"
review_set: subset
---

# Gap analysis of quoin PR 811

## Summary

Ticket: PLAT-1021. Planless, scoped to the PR diff. Plan completion: not assessed.
`quoin matrix --json` (quoin 0.30.0) shows that `tc_1021_001` tags FR-134-AC-1 and
`tc_1021_101` tags FR-134-AC-2. Both are undischarged because no run evidence exists yet,
which is normal before merge. No criterion is tagged outside a row. FR-134 and
TC-2800/2801 do not occur on origin/main or in open PRs #807, #809 or #810. #808 (FR-132,
TC-2500..2506) has merged. The branch is one commit behind main, with no textual
conflict. The real gap is the committed spec for this ticket: FR-108-AC-15..AC-23 and
TC-2013..TC-2022 still have no tagged test, and every TC row still reads 🚧.

## Verdict

**FAIL (one high).** The PR backs its own new FR-134 rather than the ACs this ticket was
specced against. AC-21 (the live write) and AC-23 (the offline mapping over the committed
corpus) have no test of any kind.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | No test tags FR-108-AC-15..AC-23 (TC-2013..TC-2022), which specify this ticket's producer. The matrix FR-108 row still says "has not yet built". FR-134-AC-1 and AC-2 cover a subset (aggregate counts, one row per item, empty refusal, record and verify end to end) under a duplicate id. Retag the tests to the FR-108 ACs, add the missing ones, and remove FR-134. See SR-20261111193746 FND-001 | spec/functional/FR-108-measurement-verdict-checker.md:190-198, spec/matrix.md:406-415 |
| FND-002 | medium | FR-108-AC-23 is untested. `constant_predictor_items` (the offline mapping) has no default-path test over the committed fixture corpus checking 15 items split 11 `weakness_kind` and 4 `coverage`, or that the result passes `validate`. The e2e test uses a hand-authored fixture instead | rust/crates/quoin-jev/tests/support/grading.rs:373-392 |
| FND-003 | medium | FR-108-AC-21 is neither implemented nor tested as specified. It calls for a write through `write_measurement_collection` when the environment names a measurement repository, placed before every assertion, and nothing written otherwise. The PR writes an input file after the unanswered and unrecognized assertions, and no test covers the hook | rust/crates/quoin-jev/tests/live_criterion_strength.rs:768-804,859-862 |

## Examined scope

```yaml
clean: false
scope:
  - {id: FR-134-AC-1, path: spec/functional/FR-134-graded-run-measurement-record.md, role: examined, excerpt: "record_input over graded items yields a record whose aggregate has examined equal to the item count and matched equal to the agreeing items, plus exactly one item row per graded item under the derived item metric; an empty run is refused as CollectionInvalid."}
  - {id: FR-134-AC-2, path: spec/functional/FR-134-graded-run-measurement-record.md, role: examined, excerpt: "A record built by record_input is accepted by quoin measurement record, and quoin measurement verify on a constant-predictor plan then decides against the recomputed baseline"}
  - {id: FR-108-AC-21, path: spec/functional/FR-108-measurement-verdict-checker.md, role: examined, excerpt: "The write comes before every assertion of the test, including the unanswered and unrecognized checks and the bars"}
  - {id: FR-108-AC-23, path: spec/functional/FR-108-measurement-verdict-checker.md, role: examined, excerpt: "it yields 15 items in two families, weakness_kind 11 and coverage 4"}
```
