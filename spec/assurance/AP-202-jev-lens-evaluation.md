---
id: AP-202
title: Jev semantic lens evaluation profile
type: AssuranceProfile
status: active
owner: quoin-maintainers
profile_version: 0.2
profile_kind: general
scope: live evaluation of Jev (typesafe.ai System One) semantic lenses before any lens is wired into a skill
impact_assessments:
  - id: unproven-lens-integrated
    severity: material
    scenario: a semantic lens is wired into a review skill on the strength of an agreement percentage that a constant answer would match, and its findings are then read as evidence
    verifiability:
      class: probabilistic
      stochastic_dependency: verifier
    detect_before_harm:
      expected: true
      control_ref: ix://agent-ix/quoin/MP-222
review_policy:
  mode: require
  operations: [spec-review, code-review, gap-analysis]
relationships: []
---

# Jev semantic lens evaluation profile

## Decision Boundary

This profile governs one decision per lens: GO or NO-GO for wiring that lens
into a skill. It covers the criterion-strength lens (PLAT-917), the EARS
conformance lens (PLAT-838) and the gap-analysis lens (PLAT-839). It does not
govern the skills themselves, and no lens is wired in while its gate plan
reports NO-GO or not-computed.

## Applicability

Apply it to every live run whose numbers are cited as evidence for or against
a lens. Each lens is decided on its own evidence; one lens failing does not
hold another.

## Impact Scenarios

The material scenario is `unproven-lens-integrated`. Measured on the
criterion-strength corpus: answering `sound` to every criterion agrees with a
recorded reading on 81.8% of the eleven weakness fixtures, and the first live
run scored 46.7% while never once answering `sound`. A raw agreement
percentage therefore cannot separate a working lens from a constant one.

## Assurance Concerns

Margin over the best constant predictor, recall of real defects, recall of the
no-defect answer, stability under repetition, calibration, and cost. Each is
measured against the context the lens's ticket specifies. A starved input
makes the number a statement about the corpus, not the lens.

## Selected Practices

- Every gate bar is written into the lens's gate plan and committed before the
  first live call it judges. A bar chosen after seeing a result is not a bar.
- At most two variants per lens beyond the first run, each pre-registered, and
  every variant is reported.
- Ground truth is named by kind: human-labelled, agent-labelled, or mechanical
  (a compiler, a test runner or the deterministic engine). Agent labels are
  never presented as human ground truth.
- Live runs sit behind the non-default `live-api` feature of `quoin-jev`, so the
  default gate stays socketless and key-free.

## Evidence Policy

Retain the raw service responses, the question set, the corpus and context
revision, the classifier identity the service reports, token counts, and the
timestamp, as measurement collections under `spec/evidence/measurements`.

## Measurement Ownership

Quoin owns MP-222 through MP-232. MP-222 to MP-227 are shared metric plans,
dimensioned by `lens` and `variant`. The gate plans are MP-228
(criterion-strength), MP-229 (EARS) and MP-230 (gap-analysis). MP-231 (EARS
engine/semantic delta) and MP-232 (vacuous-assertion agreement with mutation)
are lens-specific metrics.

## Exceptions

A lens whose gate plan is missing, or whose bars were committed after the run
they judge, is NO-GO by refusal, not by measurement. A GO is advisory evidence
for wiring the lens in. It does not make the lens a blocking CI gate; that
promotion is a separate, measured, user-gated decision.
