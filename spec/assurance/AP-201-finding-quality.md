---
id: AP-201
title: Quoin finding quality and measurement reporting profile
type: AssuranceProfile
status: active
owner: quoin-maintainers
scope: finding scoring, measurement persistence, comparison, gates, and report rendering
impact_assessments:
  - id: concealed-measurement-gap
    scenario: a green summary is accepted while a defect family has zero recall or most findings lack a repair locus
    severity: material
    verifiability:
      class: probabilistic
      stochastic_dependency: verifier
    detect_before_harm:
      expected: true
      control_ref: ix://agent-ix/quoin/FR-043
review_policy:
  mode: require
  operations: [code-review, gap-analysis]
relationships: []
---

# Quoin finding quality and measurement reporting profile

## Decision Boundary

This profile governs Quoin's quality benchmark, measurement store, comparison
rules, gates, and deterministic report views. It does not make `/gap-analysis`
or any agent review a universal executable gate.

## Impact Scenarios

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

## Evidence Policy

Retain the active plans, source and corpus revisions, tool and engine identity,
configuration digest, full raw payload, per-family counts, gap count, comparison
reasons, and tickets created from confirmed findings.

## Measurement Ownership

Quoin owns MP-201 through MP-212 and the `bench-tier1` producer that records
them under `spec/evidence/measurements`. It does not store Quire engine-health
or qa-corpus inventory measurements on their behalf; the portfolio joins those
repositories without collapsing their producer or population boundaries.

## Normalized Finding Envelope

`finding-envelope-v2` is the evaluation boundary shared by Quire findings,
Quoin findings, and retained external observations. It does not change a
producer's public payload. Every envelope records the producer class, producer,
channel and optional version; kind and evaluation identity; subject, locus,
causal evidence, change target, and next move; and the unmodified producer
record beside the normalized fields.

Each finding field is explicitly `available` with its producer-supplied value,
`unavailable` with a reason, or `not_applicable` with a reason. Adapters may copy
explicit fields and coordinates. They do not derive causal evidence from a
kind, turn a locus into a change target, or attribute an external observation
to Quire or Quoin.

Adding optional fields is backward-compatible within v2. Removing or
reinterpreting a field, state, or scoring meaning requires a new envelope and
metric definition version. Readers reject unknown versions and malformed
availability slots.

## Tool Reliance and Independence

Quoin's own tests establish conformance, not semantic correctness. Corpus labels,
healthy controls, independent readers, and human adjudication provide distinct
evidence paths. Agent reviews supplement but do not replace deterministic gates.

## Exceptions

Missing plans or incomparable inputs are refusals. New thresholds begin advisory;
promotion to a blocking gate requires an active owner-approved plan and a tested
appeal path.
