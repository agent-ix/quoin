---
id: SR-20261113052918
title: EARS conformance review of the Grype advisory adapter and join (FR-149, FR-142, FR-144)
type: SpecReview
analysis: ears-conformance
review_set: subset
scope: 'agent-ix/quoin; PR #846 (gh pr diff 846 at 888cbecb): spec/functional/FR-149-grype-advisory-report-adapter.md, spec/functional/FR-142-advisory-join-onto-soup-records.md, spec/functional/FR-144-soup-report.md, spec/functional/index.md, spec/spec.md'
---

# SR-20261113052918: EARS conformance review of the Grype advisory adapter and join (FR-149, FR-142, FR-144)

## Summary

Ticket: PLAT-1601. EARS grammar review of the Behavior statements added or changed by quoin PR #846: FR-142 lines 82, 84, 91-94, 101; FR-144 lines 33-35, 41; FR-149 lines 58-68. Every new statement has one SHALL or SHALL NOT, a ubiquitous, event (When) or unwanted-behaviour (If ... then) form, and a named system (the join, the adapter, the command, `quoin evidence record`). The paired positive/negative statements (FR-142 lines 93-94, FR-144 lines 34-35) follow the file's existing split style. `quire validate` raises no EARS warning on the three files.

## Verdict

PASS with one low finding.

## Findings

| ID | Severity | Summary | Refs |
|----|----------|---------|------|
| FND-001 | low | FR-142 line 92 embeds a factual claim inside the response clause ('then no report exists to retain, and the join SHALL ...'), so the statement is a compound of an assertion about EA FR-044 and a requirement on the join. Fix: keep only the requirement in the statement and move the rationale to the Description (see the failure-domain SR for the substantive problem with this statement). | spec/functional/FR-142-advisory-join-onto-soup-records.md:92 |
