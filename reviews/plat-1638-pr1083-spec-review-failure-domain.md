---
id: SR-967175026302197768900474181941935084793
title: "Failure-domain review — report environment and self-test"
type: SpecReview
analysis: failure-domain
scope: "agent-ix/quoin; PLAT-1638; FR-217 and FR-235 environment and asset boundaries"
review_set: subset
---

## Summary

The requirements distinguish customer self-test from the normal suite, define fail-before-case behavior for unavailable runner assets/platforms, and preserve missing checkout or failed temp-write observations without changing test outcomes. No unstated identity or failure-path gap was found.

## Verdict

**PASS** — no failure-domain finding.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Coverage

- FR-217 attributes checkout identity/revision/path and writable-directory status to the run environment at suite start, not later report-generator inference.
- FR-217-AC-14 preserves unavailable checkout identity or `writable: false` separately from results and coverage.
- FR-235 runner matching and unsupported platform, absent asset, and fetch failure paths stop before a case starts; customer self-test requires no sibling checkout.

## Scope

- **FR-217-AC-14** — `spec/functional/FR-217-validation-report.md` (examined): Given either required checkout or the temporary-directory write check is unavailable, the report records the unavailable checkout identity or `writable: false`; it preserves test outcomes and requirement coverage as separate facts.
- **FR-235-AC-10** — `spec/functional/FR-235-quoin-selftest.md` (examined): On an unsupported platform, missing matching runner, or failed fetch, execution returns an error before a case starts and reports no case as passed.
- **FR-235-AC-13** — `spec/functional/FR-235-quoin-selftest.md` (examined): Given the matching runner asset but no sibling source/specification checkout, `quoin selftest --run` invokes the installed command without accessing those repositories.
