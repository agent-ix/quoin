---
id: MP-212
title: Finding actionability v2 rate
type: MeasurementPlan
status: active
owner: quoin-maintainers
stage: baseline
metric: actionability_v2_rate
definition_version: finding.actionability-v2
relationships: []
---

# Finding actionability v2 rate

## Decision Objective

Determine whether a finding says what is affected, why it fired, and what a
reader can safely change or inspect next.

## Population and Scope

Consume only `finding-envelope-v2` records. Include every finding for which
actionability is applicable. Retain records carrying a `not_applicable` field
as named exclusions; never fold them into either side of the ratio.

## Measure Definition

The numerator is findings carrying all of:

- an available affected subject or locus;
- available causal evidence;
- an available concrete change target; and
- an available remedy or safe next diagnostic step.

An `unavailable` required field is a named miss and stays in the denominator.
Presence alone does not establish correctness; span correctness is measured
separately. Report the same numerator, denominator, rate, exclusions, and named
misses for every producer/channel/family partition so an overall value cannot
hide a weak producer or family.

## Collection and Provenance

Retain the raw producer record beside its normalized envelope, producer class,
channel and version, counts, exclusions, and each named miss. Record scorer,
tool, corpus, configuration, and source revisions with the collection.

## Environment and Sampling

Report controlled Tier-1 and pinned Tier-2 populations separately.

## Interpretation and Limitations

This metric grades whether the producer supplied actionable material. It does
not infer missing evidence from prose and does not claim that supplied advice
is correct merely because it exists.

Concrete examples:

- **Positive:** a diagnostic names `spec/tests.md:30`, says configured `Status`
  did not match observed `Coverage Status`, names `status.column` or the table
  header as the change target, and tells the reader to compare the two before
  choosing which to edit.
- **Negative:** a finding names a line but supplies no causal evidence, change
  target, or next move. It remains in the denominator with all three named as
  misses.
- **Unavailable:** protected evidence is explicitly unavailable with the
  producer's reason. It remains a named miss; absence does not shrink the
  denominator.
- **Not applicable:** a population-level retained observation that asserts no
  product repair marks subject/locus and repair fields `not_applicable` with a
  reason and remains a named exclusion.

## Comparison and Enforcement

Compare only like populations and `finding.actionability-v2` definitions. The
accepted baseline is a one-way non-regression ratchet. The Epic #261 completion
gate requires 100% in every applicable producer/channel/family partition and
requires every exclusion to remain enumerated. Keep historical
`finding.actionability-v1` observations under their original locus-presence
definition.
