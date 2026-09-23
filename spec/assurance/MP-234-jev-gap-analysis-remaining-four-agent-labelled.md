---
id: MP-234
title: Gap-analysis battery's remaining four questions against agent-labelled truth on the shipped corpus
type: MeasurementPlan
status: active
owner: quoin-maintainers
stage: branch-comparison
metric: jev.remaining_four_agent_label_agreement
definition_version: jev.remaining-four-v1
ground_truth_kind: agent-labelled
relationships: []
---

# Gap-analysis battery's remaining four questions against agent-labelled truth on the shipped corpus

## Decision Use

Decide whether the gap-analysis lens's answers to `code_implements_intent`,
`code_exceeds_requirement`, `divergence_kind` and `severity` carry signal on
the 29 triples as they actually ship. Every live call asks all four. MP-232
graded `assertion_vacuous` only. MP-233 graded these four against targeted
mutants of a sampled subset. Nothing has graded what the lens says about the
shipped triples themselves (PLAT-1014). This plan measures that, and assigns
no GO/NO-GO for wiring: MP-230 governs that decision and is not changed here.

## Population

The 29 triples of `rust/crates/quoin-jev/tests/fixtures/gap-semantic-corpus.json`
(27 real, 2 constructed: `GAP-28`, `GAP-29`), one request per triple, shown the
shipped (unmutated) state. Per question:

- `code_implements_intent`: all 29.
- `code_exceeds_requirement`: 28. `GAP-01` is excluded by the stated rule:
  FR-099's architectural SHALL neither states nor excludes the specific
  projection `quire.coverage` makes, so neither answer can be scored.
- `divergence_kind`: all 29.
- `severity`: all 29.

**What this population makes the questions measure.** 18 of the 29 triples
carry a `Trace:` tag naming a requirement about a different subsystem or a
process step: 16 FR-101 rows, `GAP-02` (FR-052) and `GAP-27` (FR-096). The
corpus's own `note` fields recorded this before PLAT-839's first live call. On
those rows the labels are `code_implements_intent: no`,
`code_exceeds_requirement: yes`, `divergence_kind: code_exceeds_requirement`
and `severity: high`. So on this corpus all four questions are dominated by one
skill: noticing that the cited requirement is not about this code. The 11
correctly-traced rows are reported separately.

## Measure Definition

Agreement between the lens's answer and the agent-written label, definition
`jev.remaining-four-v1`, computed by the shared maths in
`rust/crates/quoin-jev/tests/support/grading.rs` (the same functions PLAT-917
and PLAT-838 use):

- `noul` answers are thresholded at 0.5, as in MP-232 and MP-233.
- `divergence_kind` is graded as asked, against the six-way label.
- `severity`'s score is rounded to the nearest rubric level (`none`, `low`,
  `medium`, `high`) and graded at the verdict PLAT-839's rubric maps it to:
  `high` -> `FAIL`, `medium`/`low` -> `CONDITIONAL`, anything else -> `PASS`.
- A label may carry contested alternatives, recorded where a second reading is
  defensible. An answer matching one counts as agreement, and the
  constant-predictor baseline gets the same credit, as in the
  criterion-strength and EARS corpora.
- An unanswered or out-of-space answer stays in the denominator.

## Collection Procedure

The labels are in
`rust/crates/quoin-jev/tests/fixtures/gap-semantic-remaining4-labels.json`,
with the labelling rule per question and a rationale per row. They were written
before the first live call that grades against them.
`rust/crates/quoin-jev/tests/live_gap_remaining4.rs` (`live-api` feature)
sends one `FullBatteryV1` request and one `FullBattery` request per triple
through `quoin_jev::client::production` and grades both. The offline test
`gap_remaining4_labels.rs` runs in the default gate. It checks that the key
covers exactly the corpus, that every label is a legal answer, that the
provenance sentence is still present, and that a lens answering every label
exactly scores 100%.

## Environment and Sampling

The whole corpus, not a sample. One pass per variant per run. The live service
chooses the model and the run records which model answered. Repeat runs are
reported as replications and are not pooled.

## Interpretation

**Every label is agent-labelled, not human ground truth.** One Claude agent
session wrote them by reading each triple. Nothing was checked against a
compiler or a test runner, unlike MP-232's mutation truth. A restatement of any
number below that drops this sentence is a misreport.

The largest judgment is the trace-mismatch rule. It labels `code_exceeds_requirement`
against the requirement the lens is SHOWN, which is the only one it sees.
Another FR in this repository may well state the behaviour. This rule departs
from MP-233's P3, which assumed the shipped FR-101 bodies exceed nothing, and
that departure is deliberate: MP-233 stated its assumption as unverified, and a
row-by-row reading does not support it for mismatched traces.

A pass here shows the lens can see that a cited requirement is not about the
code in front of it. It does not show that the lens judges code-vs-requirement
semantics on correctly-traced triples. The correctly-traced subset answers
that, and at n=11 it is too small to gate.

## Comparison and Enforcement

**Bars, pre-registered in `live_gap_remaining4.rs`'s module doc and committed
before its first live call.** Each question is gated on its own, over its own
population, `FullBatteryV1` variant:

1. **Bar A, MP-222 margin > 0 pp:** agreement exceeds the best constant
   predictor that population admits, computed from the labels.
2. **Bar B, MP-223 defect recall > 0%:** of the rows labelled with a defect, at
   least one is flagged as some defect.
3. **Bar C, MP-224 no-defect recall > 0%:** of the rows labelled with the
   no-defect answer, at least one is cleared.

The defect and no-defect classes are: `code_implements_intent` `no` / `yes`;
`code_exceeds_requirement` `yes` / `no`; `divergence_kind` anything but
`aligned` / `aligned`; `severity` anything but `PASS` / `PASS`.

These are reported and not gated: `divergence_kind` derived from the same
response's `noul` answers by MP-233's rule (picking the better of two
predictors after seeing both would be a free point); `severity` at its exact
rubric level; the `FullBattery` replication; the correctly-traced and
uncontested subsets.

Compare only like variant, corpus revision and label revision. An edit to the
labels file is a new definition version.
