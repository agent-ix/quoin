---
id: SR-20261113051914
title: "Base spec review of quoin risk advise (FR-145..FR-148, NFR-031)"
type: SpecReview
analysis: base
review_set: subset
scope: "agent-ix/quoin; PR #845 (git diff origin/main...HEAD at 96566e29): spec/functional/FR-145..FR-148, spec/non-functional/NFR-031, spec/functional/index.md, spec/non-functional/index.md, spec/spec.md"
---

# Base spec review of quoin risk advise (FR-145..FR-148, NFR-031)

## Summary

Ticket: PLAT-1651. Checklist review of the read-only `quoin risk advise` report against
spec-objects-safety FR-101..107, FR-201..205 and FR-301..305 on `origin/main`.

The read-only and no-verdict contract is sound and consistent with safety FR-104: nothing is
written or stored, the attribution matches FR-104's report clause, no region is assigned outside
the policy, "no gap" is never worded as sound, and anomalies, impacts and decisions are copied as
entered. No paywalled standard text or name appears in the diff; the vocabulary is the safety
module's own. The FR-145..148 / NFR-031 range is unused on `origin/main`, on every remote branch,
and by quoin PR #844 (which touches FR-143, FR-144, MP-250, MP-251 only). No hash, digest or
pin antipattern is introduced.

Open question (1), whether FR-147 should flag a control whose verifying run did not pass: agree
with the ruling, no. FR-302 tests that the edge exists and says so (FR-302-AC-7), FR-205 leaves
pass/fail to the evidence and the person, and the Test Matrix is read by its own command. A
pass/fail gap would make this report disagree with `risk-summary` on the same control and read
as an effectiveness verdict. FR-147-AC-6 already pins this. See the integrity review for the
one place (the retained-evidence route) where "bound" could smuggle pass status back in.

Open question (2), anomalies matched to the hazard only: too narrow (FND-003). Open question
(4), trace parent: FND-009.

Applicable AssuranceProfiles: spec/assurance/AP-201-finding-quality.md (scope: finding scoring,
measurement persistence, comparison, gates and report rendering) and AP-202-jev-lens-evaluation.md
(Jev lens evaluation) were inspected; neither scope covers a safety risk report, so neither
applies and no enforced run is required.

Examined (all role examined): FR-145..FR-148 statements and all their criteria, NFR-031
statement and NFR-031-AC-1..AC-7. Context only: spec-objects-safety FR-101..107, FR-202..205,
FR-301..305 on origin/main, quoin FR-136, FR-144, StR-007, StR-009.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-147 coins control-not-implemented, control-not-verified and no-control-assumed for the facts safety FR-302 (control-without-requirement, control-without-verification) and FR-204 (post-control-without-control) already name and that risk-summary (FR-305) already reports, and its verified rule adds a retained-evidence route FR-302 lacks, so the two reports can disagree on the same control; reuse the safety tokens and FR-302's rule, or state why they differ. | FR-147:37-41 |
| FND-002 | medium | FR-305 already shows policy_region per pre- and post-control estimate, so FR-145's claim that this report adds the policy cell and repeats no chain column is not true; and FR-146 reads --policy while FR-148 and FR-102 read the residual risk's own policy, so one estimate can show two regions in one report and a third in risk-summary. State the overlap with FR-305 and which policy wins for a post-control estimate. | FR-146:28-35, FR-148:30-31 |
| FND-003 | medium | Anomalies are matched to the hazard only, but an anomaly assessment row of Kind Requirement names an FR, NFR or acceptance criterion (spec-artifacts-process FR-015), and an open anomaly against the FR that satisfies a control of the estimate is the most direct evidence gap; also list open anomalies naming an implementing FR/NFR or one of its criteria. | FR-147:39 |
| FND-004 | medium | The decision queue lists undecided residual_risk records only, but safety FR-106 benefit_risk and FR-107 overall_residual_risk_evaluation also carry an undecided decision a named person must make (FR-105 reports benefit-risk-undecided); either queue them or state in the Description that they are out of scope and why. | FR-148:28 |
| FND-005 | medium | "equal the set in this requirement exactly" has no set to compare against: FR-146 names the entry's contents in prose (estimate id, phase, levels, policy, cell, region, flag, finding, reason) but never its JSON key names, unlike FR-305's key table; add an entry key table. | FR-146:29-35, FR-146-AC-7 |
| FND-006 | medium | Same defect as FND-005 for the queue entry and the no_record group: no JSON key names are given, so the exact-set check cannot be written; FR-147's entry likewise has no key table. | FR-148:29, FR-148-AC-8 |
| FND-007 | medium | The second clause, "no region line is worded as a property of the risk on the tool's authority", is a judgement no test can fail; replace it with the fixed region wording of FR-146 (every region line matches "the policy marks this cell as <region>"). | NFR-031-AC-2 |
| FND-008 | low | "the name or text of a paywalled standard" names no list of names to inspect for and no way to recognise text; name the list (or the repo's existing licensed-text check) the inspection uses. | NFR-031-AC-7 |
| FND-009 | low | No requirement in the set traces to a stakeholder need. Not StR-009 (tests that ran and passed; FR-147 deliberately tests link existence) and not StR-007 (graph evidence portfolio). Recommend traces_to ix://agent-ix/spec-objects-safety/StR-001, the parent of its sibling report FR-305, on FR-145 (FR-146..148 inherit through depends_on FR-145). | FR-145:4-14 |
| FND-010 | medium | Ordering the queue by region-list position gives the list order a priority meaning safety FR-101 never assigns it (FR-101 defines only the outside_acceptable flag), so the order reads as a prioritisation nobody authored; either have FR-101 state that region-list order is authored meaning, or order the queue by residual-risk id within each policy. | FR-148:32-34 |

## Verdict

Not mergeable as written: no high finding here, but seven medium findings, two of which
(FND-001, FND-002) are duplication with safety FR-302 and FR-305 that the PR body says it avoids.
All are spec-text fixes inside this PR.

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-011 | medium | Runs `quoin risk-summary`, which does not exist in quoin (no source, PLAT-1576 in Backlog), and FR-305 is only a `references` edge; the AC cannot run when FR-146 is built. Either depend on FR-305 and block on PLAT-1576, or assert the region against the FR-102 lookup of the fixture directly. | FR-146-AC-10 |
| FND-012 | medium | Same defect as FND-011: compares against `quoin risk-summary`, which is not built (PLAT-1576 Backlog), so the AC cannot run when FR-147 is built; assert the tokens against FR-302's rule over the fixture instead (see FND-001). | FR-147-AC-4 |

## Dispositions

Round 1, against commit 9a653e9d.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | still-open | Tokens are now FR-302/FR-204/FR-203/FR-301's own, but the verified rule still adds the FR-205 route (a retained record bound to a criterion of the implementing FR) that FR-302's rule lacks (FR-302: an incoming verifies edge from any document). The Description now claims "the same rule" and FR-147-AC-4 asserts token equality with risk-summary, so a control verified only through that route gets no token here and control-without-verification under FR-302: AC-2 and AC-4 cannot both pass. Either use FR-302's rule exactly, or say the route is FR-205's, exempt it from AC-4, and raise the FR-302 gap with spec-objects-safety. |
| FND-002 | fixed | - |
| FND-003 | fixed | - |
| FND-004 | fixed | - |
| FND-005 | fixed | - |
| FND-006 | fixed | - |
| FND-007 | fixed | - |
| FND-008 | fixed | - |
| FND-009 | fixed | - |
| FND-010 | fixed | - |

Round 2, against commit 329ec3ac.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | - |
| FND-011 | fixed | - |
| FND-012 | fixed | - |
