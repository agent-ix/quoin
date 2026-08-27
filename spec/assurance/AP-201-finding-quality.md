---
id: AP-201
title: Quoin finding quality and measurement reporting profile
type: AssuranceProfile
status: active
owner: quoin-maintainers
scope: finding scoring, measurement persistence, comparison, gates, and report rendering
impact: material assurance-decision impact across repositories using Quoin
impact_assessments:
  - concern: an aggregate or report conceals an unmeasured family, changed population, or unusable finding
    tier: material
    scenario: a green summary is accepted while a defect family has zero recall or most findings lack a repair locus
    dimensions:
      consequence: repository work is prioritized from incomplete or unactionable evidence
      reversibility: a decision can be corrected after records and raw evidence are recovered
      scope_of_effect: every repository and review consuming the report
      detectability: low when gaps, provenance, and not-computed states are aggregated away
      recovery: invalidate the comparison, restore the raw record, and issue a corrected report
    rationale: Quoin turns tool observations into the information used for QA decisions
    uncertainty: controlled-corpus scores do not establish precision on an unadjudicated live repository
review_selection:
  mode: require
  analyses: [evidence, integrity, scope-boundary]
  rationale: changes can alter collection, scoring, comparability, or what readers infer
lifecycle: [development, review, release, maintenance]
relationships: []
---

# Quoin finding quality and measurement reporting profile

## Purpose and Scope

This profile governs Quoin's quality benchmark, measurement store, comparison
rules, gates, and deterministic report views. It does not make `/gap-analysis`
or any agent review a universal executable gate.

## Applicability and Impact

Apply it whenever code changes what is collected, scored, persisted, compared,
or rendered. These outputs direct testing and specification work, so a plausible
but incomplete report has material impact.

## Assurance Concerns

Prioritize per-family recall, adjudication coverage, population integrity,
machine-readable loci and remediation, explicit gaps and not-computed states,
exact provenance, atomic persistence, and refusal of incomparable deltas.

## Selected Practices

Run `make gate` with an identified Quire binary, the controlled corpus, schema
and comparison mutation tests, and byte-identity report tests. Sample findings
against source before changing a rule.

## Evidence Expectations

Retain the active plans, source and corpus revisions, tool and engine identity,
configuration digest, full raw payload, per-family counts, gap count, comparison
reasons, and tickets created from confirmed findings.

## Measurement Ownership

Quoin owns MP-201 through MP-211 and the `bench-tier1` producer that records
them under `spec/evidence/measurements`. It does not store Quire engine-health
or qa-corpus inventory measurements on their behalf; the portfolio joins those
repositories without collapsing their producer or population boundaries.

## Tool Reliance and Independence

Quoin's own tests establish conformance, not semantic correctness. Corpus labels,
healthy controls, independent readers, and human adjudication provide distinct
evidence paths. Agent reviews supplement but do not replace deterministic gates.

## Exceptions and Escalation

Missing plans or incomparable inputs are refusals. New thresholds begin advisory;
promotion to a blocking gate requires an active owner-approved plan and a tested
appeal path.
