---
id: SR-20261113052923
title: "EARS conformance review of licence and ban policy findings (FR-150; FR-142, FR-144 amendments)"
type: SpecReview
analysis: ears-conformance
review_set: subset
scope: 'agent-ix/quoin; PR #847 (gh pr diff 847): spec/functional/FR-150-licence-and-ban-policy-findings.md, spec/functional/FR-041-sbom-inventory-evidence.md, spec/functional/FR-142-advisory-join-onto-soup-records.md, spec/functional/FR-144-soup-report.md, spec/functional/index.md, spec/spec.md'
---

# SR-20261113052923: EARS conformance review of licence and ban policy findings (FR-150; FR-142, FR-144 amendments)

## Summary

Ticket: PLAT-1671. EARS pattern check over every new or edited Behavior statement in FR-150, FR-142 and FR-144. The FR-142 and FR-144 additions use ubiquitous, event-driven and unwanted-behaviour patterns with the system as subject; the FR-150 refusal statements are well-formed unwanted-behaviour statements. Two FR-150 statements depart from the grammar.

## Verdict

PASS with two low findings.

## Findings

| ID | Severity | Summary | Refs |
|----|----------|---------|------|
| FND-001 | low | 'A refused policy or SBOM SHALL NOT be recorded as a clean scan.' puts the input, not the system, as the subject, and 'The adapter SHALL be selected by `--adapter licence-policy`...' is passive (the command selects). Fix: 'The adapter SHALL NOT record a clean scan for a refused policy or SBOM.' and 'When `--adapter licence-policy` or `--tool licence-policy` is given, `quoin evidence record` SHALL select the licence-policy adapter.' | spec/functional/FR-150-licence-and-ban-policy-findings.md:87, 92 |
| FND-002 | low | 'The adapter SHALL record a FindingRecord of tool licence-policy with one finding per row of the table above that applies.' has its response defined by reference to a table, and rows 2 and 4 can both apply to one component (an id not allowed plus a free-text name), while the paragraph after the table says the component gets a single licence finding. The statement does not say whether a component gets one licence finding or one per applicable row. Fix: state the response directly, e.g. 'at most one licence finding per component, chosen denied > not-allowed > unknown, plus one package-banned finding per matching ban entry'. | spec/functional/FR-150-licence-and-ban-policy-findings.md:88, 65-75 |
