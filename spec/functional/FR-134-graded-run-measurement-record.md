---
id: FR-134
title: "A graded-corpus run records its aggregate and per-item rows together"
type: FR
relationships:
  - target: "ix://agent-ix/quoin/FR-108"
    type: "extends"
---

# FR-134: A graded-corpus run records its aggregate and per-item rows together

## Description

`quoin_jev::record_input` SHALL turn a graded corpus run into one `quoin measurement record` input: the governed aggregate agreement observation and the constant-predictor per-item rows (`item_observations`), both derived from the same graded items. The criterion-strength gate in `quoin-jev` writes that input when asked, so a real graded run reaches `quoin measurement verify` with a recomputable baseline.

## Rationale

FR-108 decides a `constant-predictor` plan from per-item rows in the stored collection. With no producer writing them, every real plan answered `constant_predictor_rows_absent`. Deriving the aggregate and the rows from one slice makes the rows cover exactly `population.examined`, the condition the checker requires.

## Behavior

- The aggregate carries `examined` = the number of graded items and `matched` = the items whose answer equals the primary or a contested reading.
- Each item row carries its `family`, so the families are the plan's `answer_families` when the caller grades by them.
- An empty run is refused rather than recorded as a rate over nothing.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-134-AC-1 | `record_input` over graded items yields a record whose aggregate has `examined` equal to the item count and `matched` equal to the agreeing items, plus exactly one item row per graded item under the derived item metric; an empty run is refused as `CollectionInvalid`. | Test (TC-2800) |
| FR-134-AC-2 | A record built by `record_input` is accepted by `quoin measurement record`, and `quoin measurement verify` on a `constant-predictor` plan then decides against the recomputed baseline: `accept` when the observed rate clears baseline plus margin, `reject` with `rule_not_met` when it does not, never `constant_predictor_rows_absent`. | Test (TC-2801) |

## Dependencies

- FR-108 supplies the constant-predictor baseline and its checker.
