---
id: US-007
title: "Review a spec into validated per-analysis review docs"
type: US
relationships:
  - target: "ix://agent-ix/quoin/FR-020"
    type: "traces_to"
  - target: "ix://agent-ix/quoin/FR-021"
    type: "traces_to"
  - target: "ix://agent-ix/quoin/StR-004"
    type: "traces_to"
---

# US-007: Review a spec into validated per-analysis review docs

## Story

**As a** spec author preparing requirements for implementation
**I want** to run a review that produces one structured, validated review document per analysis
**So that** findings are consistent, severity-graded, individually addressable, and synced rather than buried in one freeform report.

## Context

The bundled `spec-review` workflow (launched via `quoin review`, driven by
`ix-flow`) lets the author choose a review set — `base`, `all`, or a `subset` of
the seven analyses — and produces one `SpecReview` document per selected analysis
under `spec/reviews/`. Each doc is a validated artifact: a `## Summary` plus a
`## Findings` table whose `Severity` column is constrained to `low|medium|high`.
The author keeps direct authoring, but two guardrails keep quality stable over
many uses: the agent fetches the template from quoin (`quoin write --types
SpecReview`) rather than inventing the format, and validates the output with
`quire`. A coverage gate stops acceptance until every selected analysis has a
recorded, validated doc.

## Acceptance Examples (Illustrative)

These examples clarify expectations; they are illustrative, not verification criteria.

### US-007-EX-1: Subset review yields one validated doc per analysis

- **Given** a spec directory and a choice of `subset` with `integrity` and `dependency`
- **When** the author runs the review
- **Then** `spec/reviews/integrity.md` and `spec/reviews/dependency.md` are authored as `SpecReview` artifacts and pass `quire validate`

### US-007-EX-2: Coverage gate blocks an incomplete review

- **Given** a `subset` of two analyses but only one rendered doc
- **When** the author tries to accept the review
- **Then** acceptance is blocked, naming the missing analysis, until its doc exists

### US-007-EX-3: Out-of-set severity is rejected

- **Given** a findings row whose `Severity` is `catastrophic`
- **When** the doc is validated
- **Then** validation fails because `Severity` must be one of `low|medium|high`

### US-007-EX-4: The all review set includes EARS conformance

- **Given** the author selects the `all` review set
- **When** the workflow expands and checks the selected analyses
- **Then** `ears-conformance` is required alongside the other six focused analyses and acceptance remains blocked until its validated review document is recorded

## Options (Exploratory)

Approaches discussed during discovery, none implying commitment: one doc per
analysis (parallel-safe, chosen) versus a single combined report; findings as a
table (chosen, extractable) versus prose subsections; enforcing the chosen set via
an ix-flow gate versus prose discipline alone.

## Constraints (Contextual)

- The review preserves human-authored requirements artifacts, writing its findings
  into separate `SpecReview` docs.
- `SpecReview` is its own type, separate from the freeform `Review` type, which it
  leaves unchanged.

## Dependencies

- **Upstream**: [FR-020](../functional/FR-020-resolve-workflow-skills.md),
  [FR-021](../functional/FR-021-launch-ix-flow-runs.md); spec-artifacts-process
  `ix://agent-ix/spec-artifacts-process/FR-002` (SpecReview archetype)
- **Downstream**: matrix and planning workflows that consume accepted reviews
