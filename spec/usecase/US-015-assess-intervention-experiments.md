---
id: US-015
title: "Assess intervention experiments without overstating causality"
type: US
relationships:
  - target: "ix://agent-ix/quoin/StR-004"
    type: "traces_to"
  - target: "ix://agent-ix/quoin/FR-056"
    type: "traces_to"
  - target: "ix://agent-ix/quoin/FR-057"
    type: "traces_to"
  - target: "ix://agent-ix/quoin/FR-058"
    type: "traces_to"
---

# US-015: Assess intervention experiments without overstating causality

## Story

**As an** assurance practitioner evaluating deployed-system behavior
**I want** experiment evidence to retain what changed, what stayed fixed, what was
observed, and what remains uncertain
**So that** I can distinguish an observed effect from a causal conclusion and act on
failed or inconclusive work without treating it as missing evidence.

## Context

Execution traces establish that software ran, but they do not establish why an
outcome changed. A causal claim needs the experiment design, baseline and treatment,
sampling and repetition conditions, potential interactions and confounders, and a
conclusion whose confidence is explicit. Negative outcomes are useful evidence too:
an honest `cause_not_established` record prevents the same unsupported claim from
quietly returning in a later report.

The practitioner needs a claim-centered report, not a synthetic score. Evidence,
counterevidence, gaps, ownership, and the next action must remain inspectable beside
the conclusion that they qualify.

## Priority and Risk (Informative)

- **Priority:** P0. Quoin must not admit or amplify an unsupported causal claim.
- **Primary risk:** ambiguous treatment attribution or unverifiable raw evidence can
  make a technically valid record materially misleading.
- **Mitigation:** require treatment-linked observations, reproducible randomized
  assignment, explicit attribution limits, and digest verification at intake.

## Acceptance Examples (Illustrative)

### US-015-EX-1: A repeated experiment supports a causal conclusion

- **Given** a repeated baseline/treatment experiment with pinned inputs and no known
  confounder
- **When** the practitioner inspects its retained record
- **Then** the observed effect, causal conclusion, attribution confidence, and raw
  evidence are visible together

### US-015-EX-2: An inconclusive experiment remains first-class evidence

- **Given** an experiment whose samples do not establish the source of an observed
  change
- **When** its producer records the outcome
- **Then** the record remains available with `cause_not_established`, the confounders
  and gaps, an owner, and a next action

### US-015-EX-3: A report does not turn uncertainty into a score

- **Given** experiments containing supporting effects, counterevidence, and open gaps
- **When** the practitioner renders the report
- **Then** those sections remain separate and no overall trust or quality score is
  shown

### US-015-EX-4: A real evaluation comparison remains causally modest

- **Given** retained baseline and treatment agent-evaluation reports with matching
  scenarios but insufficient attribution control
- **When** the first-party producer records their observed difference
- **Then** it retains the computed effects and raw reports with
  `cause_not_established` rather than inventing a causal conclusion

## Dependencies

- **Upstream**: [StR-004](../stakeholder/StR-004-governed-workflows.md) requires
  assurance work to preserve explicit governance boundaries.
- **Downstream**: [FR-056](../functional/FR-056-intervention-experiment-record.md)
  defines the record contract; [FR-057](../functional/FR-057-intervention-experiment-intake-report.md)
  defines evidence-store intake and reporting; and
  [FR-058](../functional/FR-058-agent-eval-intervention-producer.md) supplies the
  first real producer without executing experiments inside Quoin.
