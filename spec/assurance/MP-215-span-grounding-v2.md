---
id: MP-215
title: Labeled property span outcomes
type: MeasurementPlan
status: active
owner: quoin-maintainers
stage: ratchet
metric: span_grounding_v2_rate
definition_version: property.span-grounding-v2
relationships: []
---

# Labeled property span outcomes

## Decision Objective

Determine whether every criterion in the retained labeled population produces
exact statement-relative property boundaries or an explicit, justified refusal
when the statement does not supply enough information to name those boundaries.

## Population and Scope

Include only specific-shape criteria matched by exact statement text and
property shape in `bench/span-grounding-v2-labels.json`. Each rule pins its
expected multiplicity. An unmatched specific-shape criterion is retained as
`not_applicable`; a missing, duplicated, or changed labeled population makes the
run malformed rather than changing the denominator.

## Measure Definition

Count a labeled criterion when all expected domain, precondition, and oracle
texts and coordinates match exactly, including expected null fields, or when it
emits no spans and carries the rule's exact structured safe-refusal signal.
Report exact spans and safe refusals separately. Wrong spans, unexpected
refusals, unjustified refusals, unsafe emissions, and malformed multiplicity are
named failures. Definition `property.span-grounding-v2`.

## Collection and Provenance

Retain label identities and multiplicities, raw `quire properties --json`
payloads, producer versions, exact outcome counts, named misses, exclusions,
corpus and declaration revisions, scorer revision, and timestamp. Preserve the
historical `property.span-grounding-v1` observation unchanged beside v2.

## Environment and Sampling

Run the full pinned Tier-1 corpus with its pinned declaration modules and one
identified producer binary. Do not sample or fuzzy-match statements.

## Interpretation and Limitations

A safe refusal is correct only for the reason pinned by the label. It prevents
invented boundaries; it does not claim that the producer extracted spans. This
controlled labeled population is a regression contract, not an ecosystem-wide
estimate.

## Comparison and Enforcement

The retained Tier-1 baseline is a one-way non-regression ratchet. Compare only
like label definitions, multiplicities, corpus, declaration, and producer
inputs. Epic #261's completion gate requires 100% exact boundaries or justified
safe refusals, zero named misses, and zero malformed labels on this population.
