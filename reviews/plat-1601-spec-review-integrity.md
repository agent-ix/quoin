---
id: SR-20261113052924
title: "Integrity review of PLAT-1601 OSV-Scanner adapter and advisory join"
type: SpecReview
analysis: integrity
scope: "agent-ix/quoin; PR #881; spec/functional/FR-142-advisory-join-onto-soup-records.md, spec/functional/FR-149-osv-scanner-advisory-report-adapter.md, spec/functional/FR-144-soup-report.md"
review_set: all
---

# Integrity review of PLAT-1601 OSV-Scanner adapter and advisory join

## Summary

Reviewed completeness, consistency, and testability of the changed adapter and join obligations and their cross-references. The acceptance criteria map to the intended report behavior, but the adapter's dependency on fixture and producer artifacts is not aligned with the stated source requirement.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-149-AC-10 names the npm and Python fixtures from engineering-assurance FR-039 and says the Go fixture is added by PLAT-1601, while the companion producer requirement defines its own advisory-scan fixtures. The criterion does not identify the actual Go SBOM/report source that FR-149 consumes. Reference the producer fixtures in engineering-assurance FR-044, and distinguish those from FR-039's existing SBOM fixtures. | spec/functional/FR-149-osv-scanner-advisory-report-adapter.md:119; spec/functional/FR-149-osv-scanner-advisory-report-adapter.md:124 |

## Integrity checks

The new IDs are sequential within FR-149 and the new FR-142 / FR-144 criteria are in their existing tables. Relative links resolve. The ticket's container OS-package outcome and exact-version pin are recorded in the base review; the old Grype file is removed as part of the adapter replacement.
