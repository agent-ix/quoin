---
id: US-018
title: "Inspect evidence-graph concentration and change exposure"
type: US
relationships:
  - target: "ix://agent-ix/quoin/StR-004"
    type: "traces_to"
---
# US-018: Inspect evidence-graph concentration and change exposure

## Story

**As an** assurance owner reviewing a change
**I want** read-only fan-out, change-impact, and reaffirmation-churn views over the governed evidence graph
**So that** I can find concentrated evidence, affected obligations, and repeatedly reaffirmed statements without collecting new evidence or treating an incomplete graph as complete.

## Context

The evidence store already relates obligations to suites, records explicit
reaffirmations, and retains the auditor's binding verdicts. Quire owns the
artifact relationship graph. Today those facts can be inspected separately,
but there is no deterministic view that joins them and keeps unresolved or
unavailable inputs visible.

## Acceptance Examples (Illustrative)

### US-018-EX-1: One suite carries many obligations

- **Given** a suite bound to several distinct live obligations
- **When** the owner opens the fan-out view
- **Then** the suite appears once with the exact obligation set and count

### US-018-EX-2: A requirement change exposes dependent evidence

- **Given** a changed requirement with transitive dependents
- **When** the owner opens the change-impact view
- **Then** each reached requirement, obligation, suite, and relationship path is shown without replacing the auditor's verdict

### US-018-EX-3: Reaffirmation history is not multiplied by suites

- **Given** one reaffirmation copied to several bindings for the same obligation
- **When** the owner opens the churn view
- **Then** it counts as one reaffirmation event and the affected suites remain listed separately

## Constraints (Contextual)

The views read validated Quire exports and Quoin's retained store. They run no
producer, write no record, and make no trust, quality, or instability verdict.

## Dependencies (Contextual)

Upstream: [FR-030](../functional/FR-030-evidence-store.md),
[FR-032](../functional/FR-032-evidence-auditor.md), and Quire's stable
assurance export. Downstream: the portfolio reporting work in
`agent-ix/quoin#281`.

## Priority and Risk (Informative)

Priority is P1. The principal risk is a plausible partial answer: an absent
store, unresolved binding, or incomplete relationship projection must remain
visible rather than becoming a zero.
