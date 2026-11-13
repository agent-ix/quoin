---
id: SR-20261113052924
title: "Integrity review of licence and ban policy findings (FR-150; FR-142, FR-144 amendments)"
type: SpecReview
analysis: integrity
review_set: subset
scope: 'agent-ix/quoin; PR #847 (gh pr diff 847): spec/functional/FR-150-licence-and-ban-policy-findings.md, spec/functional/FR-041-sbom-inventory-evidence.md, spec/functional/FR-142-advisory-join-onto-soup-records.md, spec/functional/FR-144-soup-report.md, spec/functional/index.md, spec/spec.md'
---

# SR-20261113052924: Integrity review of licence and ban policy findings (FR-150; FR-142, FR-144 amendments)

## Summary

Ticket: PLAT-1671. Completeness, consistency and atomicity of the FR-150 evaluation rules and the FR-142/FR-144 policy-finding additions. The policy format, refusals, vacuity and the separation of policy findings from advisories (FR-142-AC-34) are complete and consistent with FR-034 and FR-149. FR-144's status vocabulary and FR-150's deny matching are not.

## Verdict

FAIL on two medium findings and one low finding.

## Findings

| ID | Severity | Summary | Refs |
|----|----------|---------|------|
| FND-001 | medium | FR-142 applies the vacuity and `unordered` rules to policy findings unchanged, but FR-144's policy status admits only `no policy findings in the scan read`, `policy findings not assessed: licence-policy not run`, or the attached findings. A record under a vacuous (FR-142-AC-33) or unordered licence-policy scan has no status a row may state, so an implementer either invents one or prints `not run`, which is false. The append line lists scans 'reported not run, refused or vacuous', but FR-142 never reports a policy scan `refused` (a refused input leaves no record and reads as not run) and omits `unordered`. Fix: give the policy status the same per-state wording advisories have (not run, unordered, vacuous) and make the append list match FR-142's states. | spec/functional/FR-144-soup-report.md:37, 39 |
| FND-002 | medium | `allow` accepts 'SPDX licence identifiers (or exact expression strings)' but `deny` is described as 'identifiers', and table row 1 matches only 'a declared licence id'. Whether a `license.name` or an `expression` equal to a deny entry is `licence-denied` or `licence-unknown` is undefined; two implementations will differ. The paragraph after the table ('any denied makes licence-denied, otherwise any not in allow makes licence-not-allowed') also contradicts row 4 for a component with one allowed id and one free-text name: by the paragraph it is `licence-not-allowed`, by row 4 nothing or `licence-unknown`. Fix: state that `allow` and `deny` entries match ids, names and expressions alike, verbatim and ASCII case-insensitively, and give the precedence explicitly over the four kinds (denied > not-allowed for an id > unknown for a name or expression), with an AC for the mixed case. | spec/functional/FR-150-licence-and-ban-policy-findings.md:47-48, 65-75 |
| FND-003 | low | 'The adapter SHALL record, for each finding, the licence text or the ban reason in the message.' A `licence-unknown` finding for a component with no licence has no licence text, a ban entry's `reason` is optional, and a component with several offending licences has several texts. The message for each case is unstated, and FR-142 carries it 'unchanged' into the report. Fix: name the message for each case (for example `no licence declared`, `banned by <entry purl>`, all offending texts joined). | spec/functional/FR-150-licence-and-ban-policy-findings.md:97, 51 |
