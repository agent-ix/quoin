---
id: SR-967175026302197768900474181941935084789
title: "EARS review — report and self-test requirements"
type: SpecReview
analysis: ears-conformance
scope: "agent-ix/quoin; PLAT-1638; FR-217 and FR-235"
review_set: subset
---

## Summary

The edited FR-217 and FR-235 statements are concrete and attributable to the Quoin CLI, report generator, suite, or runner. Targeted Quire validation reported no grammar finding in either document, and semantic review found no EARS mismatch.

## Verdict

**PASS** — no EARS-conformance findings.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Coverage

Targeted Quire 0.53.1 validation covered both requirement documents. No changed statement uses a non-canonical trigger, vague response, or ambiguous actor/response pattern that changes the built behavior.

## Scope

- **FR-217 description** — `spec/functional/FR-217-validation-report.md` (examined): The quoin CLI SHALL generate a per-release test report from retained normal suite run records and requirement evidence, so a reader can see the exact tool version tested, test outcomes, requirement coverage, uncovered requirements, and known defects.
- **FR-217 environment behavior** — `spec/functional/FR-217-validation-report.md` (examined): For each suite run, the report environment SHALL identify the required sibling checkouts. For each checkout, it SHALL record the canonical repository identity, observed commit revision, and resolved absolute checkout path.
- **FR-235 description** — `spec/functional/FR-235-quoin-selftest.md` (examined): When a user requests `quoin selftest --list` or `quoin selftest --run`, the Quoin CLI shall list or run command-level self-test runners associated with the installed release, so the user can exercise the exact installed binary without checking out the source repository.
- **FR-235 boundary behavior** — `spec/functional/FR-235-quoin-selftest.md` (examined): Customer self-test execution SHALL require only the installed command and its matching release runner. It SHALL NOT require a checkout of `spec-objects-safety`, `spec-objects-security`, or any other source repository.
