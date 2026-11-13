---
id: SR-20261113052919
title: Integrity review of the Grype advisory adapter and join (FR-149, FR-142, FR-144)
type: SpecReview
analysis: integrity
review_set: subset
scope: 'agent-ix/quoin; PR #846 (gh pr diff 846 at 888cbecb): spec/functional/FR-149-grype-advisory-report-adapter.md, spec/functional/FR-142-advisory-join-onto-soup-records.md, spec/functional/FR-144-soup-report.md, spec/functional/index.md, spec/spec.md'
---

# SR-20261113052919: Integrity review of the Grype advisory adapter and join (FR-149, FR-142, FR-144)

## Summary

Ticket: PLAT-1601. Consistency and completeness review of the PR #846 diff against the merged FR-034, FR-142..FR-144 and NFR-030. Consistent: one newest scan per tool across suites (FR-142 lines 82, 85), purl-equality join with qualifiers and subpath ignored and no name normalisation, so a spelling difference is unmatched and not dropped (FR-142-AC-23), `disposition_clause` as the only resolution path (FR-142-AC-24), no severity mapping (FR-149 line 68), unknown stays unknown for unsupported purl types (FR-142-AC-20). Inconsistencies below.

## Verdict

FAIL on two medium findings.

## Findings

| ID | Severity | Summary | Refs |
|----|----------|---------|------|
| FND-001 | medium | The Description says a record 'assessed only by tools whose scans were not read' is stated `unsupported for the purl type' (lines 58-60), but Behavior (line 93) makes a record `unsupported` only when its purl type is assessed by no tool, and FR-142-AC-21 has an npm record with no grype scan reported as 'grype not run'. Two implementers produce different output for an npm record with grype missing, and the Description's label is wrong for it (the type is supported; the scan is absent). Fix: drop the 'or is assessed only by tools whose scans were not read' clause from the Description and say such a record is not free of advisories because its tool is not run. | spec/functional/FR-142-advisory-join-onto-soup-records.md:57-60, 93 |
| FND-002 | medium | 'Free of advisories' for a record as a whole is never defined: FR-142 defines only 'free of a tool's advisories', and for an npm record cargo-audit never assesses it (FR-142-AC-25). FR-142 line 94 and FR-144 line 35 forbid the whole-record state for unsupported records, and FR-144-AC-17 requires 'in no clean group', but FR-144's ordering has no clean group (unresolved, past end of support, the rest), so an unsupported record sorts among clean ones and the AC has no observable. Fix: define a record free of advisories as free of the advisories of every tool that assesses its purl type (at least one), and either add an `advisories not assessed` group to the FR-144 ordering or restate AC-17 on the row text alone. | spec/functional/FR-144-soup-report.md:35, 40, 74 |
| FND-003 | low | The Description's envelope includes 'a `descriptor` naming Grype and its version' (lines 33-34), but the refusal statement (line 63) and FR-149-AC-3 do not refuse a descriptor with no version. Fix: add the version to line 63 and AC-3, or drop it from the Description. | spec/functional/FR-149-grype-advisory-report-adapter.md:33-34, 63 |
| FND-004 | low | NFR-030 ('Failed or suppressed scans read as clean: 0') constrains FR-141..FR-144 but not FR-149, which is where a suppressed grype scan (`ignoredMatches`) is refused, and NFR-030-AC-2 names only cargo-deny. Fix: add a `constrains` edge from NFR-030 to FR-149 and a grype refused/failed case to NFR-030-AC-2, or state why the FR-142-AC-22 coverage suffices. | spec/functional/FR-149-grype-advisory-report-adapter.md:5-17 |
