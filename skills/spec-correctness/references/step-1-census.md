# Step 1: Census

**Goal**: state what the classifier found, as counts, so the rest of the run has a
denominator. Nothing in this step is a judgement.

## Bucket

Cross-tabulate the records two ways:

- `property` × `extraction` — this drives routing and volume.
- `archetype` × `extraction` — this tells you where the properties live (FR? StR? NFR?).

Report the totals plainly:

```
Criteria: 118
  extraction    extractable 51 · candidate 0 · not-extractable 67
  property      universal 43 · example 59 · error-case 9 · unclassified 5 · ordering 2
  archetype     FR 84 · NFR 21 · StR 13
  spans present domain 4 · precondition 2 · oracle 5
```

## What this step must never emit

- A threshold, a target, a percentage framed as a score, or a grade of any kind.
- A pass/fail, a verdict, or a severity. The classification carries no severity key and no
  promotion path by design (quire-rs FR-052-CON-1).
- A suggestion to reword, split, or "improve" any criterion. Not as a recommendation, not
  as a note, not as a parenthetical. If a criterion is a concrete example, the correct
  response is an example-based test, not an edit to the spec.
- A comparison of this repo against another repo's ratio.

Phrase counts as facts: "59 criteria are concrete examples" — not "only 43% are
extractable" and never "consider rewriting the 59 example criteria as properties."

## What is worth flagging (as information)

- Records with `row_id: null` — they cannot be tagged, so they cannot be reconciled. Route
  recorded as a finding with reason `no-row-id`.
- Documents that produced zero criteria — usually the archetype binds no `ac` grammar
  (US and IT are unbound), which is expected, not a gap.
- A `candidate` count above zero — rare (0.4% of the corpus). Worth naming because every
  one of them is review-gated by construction.

## Archetype note

Do **not** down-weight StR. The measured ranking is StR 29.1% > NFR 28.9% > FR 19.2%
(quire-rs CR-029, which falsified the earlier CR-020 prediction that StR would score low).
Stakeholder criteria produce good properties.

## Output of this step

The census tables, plus the working set for step 2: every record with a `row_id`.
