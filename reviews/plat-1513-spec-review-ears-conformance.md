---
id: SR-20261113000904
title: "EARS conformance review of SOUP promotion, advisory join and report (FR-141..FR-144, NFR-030)"
type: SpecReview
analysis: ears-conformance
review_set: subset
scope: "agent-ix/quoin; spec/functional/FR-141..FR-144, spec/non-functional/NFR-030, spec/functional/index.md, spec/non-functional/index.md, spec/spec.md (PR #837 diff, git diff origin/main...HEAD)"
---
# SR-20261113000904: EARS conformance review of SOUP promotion, advisory join and report (FR-141..FR-144, NFR-030)

## Summary

Ticket: PLAT-1513. Checks every Behavior bullet of FR-141..FR-144 and the NFR-030 Statement against EARS forms. `quire validate` reports no `ears:` or `quality:` warning for the changed files.

Clean: every bullet is singular, names its subject ('the command', 'the join', 'the report'), and uses the event, unwanted-behaviour, optional-feature or ubiquitous form correctly. The compound unwanted-behaviour pairs are already split into one SHALL per bullet.

## Verdict

PASS with findings. Low severity only.

## Findings

| ID | Severity | Summary | Refs |
|----|----------|---------|------|
| FND-001 | low | 'other than the `disposition_clause` of the next requirement' points at the next bullet, not a requirement, and turns one SHALL NOT into a conditional exception. Fix: make it optional-feature form. 'Where `--discharges` is not given, the command SHALL NOT write `disposition_clause` on a record it creates', with the other four keys in their own unconditional SHALL NOT. | spec/functional/FR-141-promote-sbom-components-to-soup-records.md:41-42 |
| FND-002 | low | 'leave the key absent or write the literal `unknown`' leaves the response to the implementer, so two conforming builds write different records from one SBOM. Fix: pick one, for example leave the key absent (FR-009-AC-5 admits it and adds nothing), and tighten AC-3 to that. | spec/functional/FR-141-promote-sbom-components-to-soup-records.md:39 |
| FND-003 | low | The command cannot tell from a `source` string whether a feed is public, paid or private, so it cannot meet this as a Design constraint. It is a rule on the person who writes the `--support` file. Fix: retype it as a Process constraint on the support-file author, keep Inspection, and state what the command checks: that `source` is present and non-empty. | spec/functional/FR-143-support-status-and-end-of-support-input.md:62 |
