---
id: SR-20261113052920
title: Failure-domain review of the Grype advisory adapter and join (FR-149, FR-142, FR-144)
type: SpecReview
analysis: failure-domain
review_set: subset
scope: 'agent-ix/quoin; PR #846 (gh pr diff 846 at 888cbecb): spec/functional/FR-149-grype-advisory-report-adapter.md, spec/functional/FR-142-advisory-join-onto-soup-records.md, spec/functional/FR-144-soup-report.md, spec/functional/index.md, spec/spec.md'
---

# SR-20261113052920: Failure-domain review of the Grype advisory adapter and join (FR-149, FR-142, FR-144)

## Summary

Ticket: PLAT-1601. Unstated failure modes in the grype path of PR #846: refused and failed runs, suppressed matches, malformed matches and identity across purl types. Covered: a non-JSON, truncated or envelope-less report is refused (FR-149-AC-3); a refused report leaves no record and reads as not run (FR-142-AC-22); a PyPI and an npm package of one name do not cross (FR-142-AC-19); a spelling difference is unmatched, not dropped (FR-142-AC-23).

## Verdict

FAIL on three medium findings.

## Findings

| ID | Severity | Summary | Refs |
|----|----------|---------|------|
| FND-001 | medium | The join reads only retained FindingRecords and has no input carrying an EA producer result, so 'If the result of the producer run ... was not `completed`, then ... the join SHALL report grype as not run' cannot be implemented. With an older grype scan retained, a later failed run leaves the join reading the older scan as current and possibly clean, which is the 'failed scan read as clean' NFR-030 forbids. FR-142-AC-22 passes only in a store with no prior scan. Fix: restate as the observable case (a failed run records nothing, so the newest retained scan is unchanged), and either accept stale reads explicitly or report the scan's age (the recorded database build time and timestamp) so the reader can see it; add an AC with a prior clean scan plus a failed run. | spec/functional/FR-142-advisory-join-onto-soup-records.md:92 |
| FND-002 | medium | A match with no `vulnerability.id` is skipped and 'counted', but FR-142 never reads that count, so a record whose only match was skipped is reported free of grype advisories: a dropped diagnostic read as clean. cargo-deny keeps an id-less diagnostic as `unclassified` (FR-034-AC-26) rather than skipping it. Fix: refuse such a report, or record the match with a placeholder rule id so FR-142 lists it unmatched or attached, and add a join AC. | spec/functional/FR-149-grype-advisory-report-adapter.md:62, 78 |
| FND-003 | medium | FR-149 asserts that an empty configuration yields no ignored match, so any `ignoredMatches` entry 'marks a run that departed from it'. Grype applies built-in ignore rules regardless of the config file (for example kernel-headers packages matched through their upstream kernel), which report into `ignoredMatches`; if so, every container scan holding such a package is refused and grype reads as not run forever for that image. This was not reproduced against a Grype binary. Fix: verify against the Grype version FR-044 runs; if defaults populate it, either list the built-in rules that are accepted or record ignored matches as findings marked ignored rather than refusing. | spec/functional/FR-149-grype-advisory-report-adapter.md:38-41, 64 |
