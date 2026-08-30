---
id: US-016
title: "Assess operational controls and their exercises"
type: US
relationships:
  - target: "ix://agent-ix/quoin/StR-004"
    type: "traces_to"
  - target: "ix://agent-ix/quoin/FR-059"
    type: "traces_to"
  - target: "ix://agent-ix/quoin/FR-060"
    type: "traces_to"
---

# US-016: Assess operational controls and their exercises

## Story

**As an** assurance practitioner responsible for deployed-system behavior
**I want** operational controls and their actual exercises recorded as distinct
evidence
**So that** I can see which safeguards exist, whether they worked when used or
drilled, and whether clocked obligations were met without mistaking capability for
performance.

## Context

A runbook or configured switch can establish that a rollback, kill, override, or
fallback surface exists. It cannot establish that the surface still works, that an
authorized person can reach it, or that it completes inside a reporting or recovery
clock. Those are separate claims and need separate records.

The same distinction applies across releases, feature flags, canary and shadow
deployments, human override and appeal, abstention and safe fallback, and pinned
policy, prompt, model, tool, and data revisions. Operational reporting must retain
failed and partial exercises because removing them would erase the evidence that
most directly drives remediation.

## Acceptance Examples (Illustrative)

### US-016-EX-1: A standing capability remains a capability claim

- **Given** a deployed service with a documented kill switch
- **When** the practitioner inspects its standing-capability record
- **Then** the control surface, scope, authority, coverage, limitations, and clock
  support are visible without any claim that the switch was exercised

### US-016-EX-2: A drill carries its clock and outcome

- **Given** a rollback drill with a declared start event and deadline
- **When** the practitioner inspects its exercise record
- **Then** the actor, trigger, timing, observed result, resulting state, and retained
  output show whether the deadline was met

### US-016-EX-3: A failed control exercise remains visible

- **Given** an override or safe-fallback exercise that failed or completed only
  partially
- **When** the operational report is rendered
- **Then** the result appears as counterevidence or a gap beside its owner and next
  action rather than disappearing from the report

## Dependencies

- **Upstream**: [StR-004](../stakeholder/StR-004-governed-workflows.md) requires
  assurance decisions and their gates to remain explicit.
- **Downstream**: [FR-059](../functional/FR-059-operational-evidence-records.md)
  defines both record shapes; [FR-060](../functional/FR-060-operational-evidence-intake-report.md)
  defines storage, clocked discharge, and reporting.
