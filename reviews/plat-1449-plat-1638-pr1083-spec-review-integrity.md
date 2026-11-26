---
id: SR-967175026302197768900474181941935084790
title: "Integrity review — Quoin ratings and report requirements"
type: SpecReview
analysis: integrity
scope: "agent-ix/quoin; PLAT-1449, PLAT-1638; FR-217, FR-235, FRR-001, FRR-002"
review_set: subset
---

## Summary

The rating relationship sets are disjoint and exhaustive, all retained high items have an aligned rationale, and the report/self-test changes distinguish inputs and outcomes without reintroducing risk-selected test coverage. No integrity or interpretation defect was found.

## Verdict

**PASS** — no integrity findings.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Coverage

- Mechanically checked relationship counts and all 210 FR IDs: 50 high, 160 not-high, every FR exactly once.
- Matched all 50 high reason IDs one-to-one with retained high relationships; each reason is nonempty.
- Verified proposed status and absence of `rated_by` and score fields on both records.
- Reviewed FR-217-AC-13/-14 for captured suite environment and unavailable-check behavior, and FR-235-AC-13 for no sibling checkout during customer self-test.
- Targeted validator: 4/4 changed artifacts grammar-clean and no grammar findings.

## Scope

- **FRR-001** — `spec/validation/FRR-001-not-high-process-risk.md` (examined): The not-high relationships are unique and disjoint from the high list; status: proposed.
- **FRR-002** — `spec/validation/FRR-002-high-process-risk.md` (examined): Each retained high feature can itself falsely state that verification, traceability, discharge, or approval occurred; one feature-specific false-claim reason follows for each.
- **FR-217-AC-13** — `spec/functional/FR-217-validation-report.md` (examined): Given a run environment containing both required sibling checkouts, the report records each canonical repository identity, commit revision, and resolved absolute path, plus the resolved temporary-directory path and its captured writable status.
- **FR-217-AC-14** — `spec/functional/FR-217-validation-report.md` (examined): Given either required checkout or the temporary-directory write check is unavailable, the report records the unavailable checkout identity or `writable: false`; it preserves test outcomes and requirement coverage as separate facts.
- **FR-235-AC-13** — `spec/functional/FR-235-quoin-selftest.md` (examined): Given the matching runner release asset is available but no sibling source or specification repositories are checked out, `quoin selftest --run` invokes the installed command and records its result without attempting to access those repositories.
