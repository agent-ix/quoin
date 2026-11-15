---
id: SR-20261114002440
title: "Integrity analysis — PLAT-1615 M2 dossier requirements (quoin#929)"
type: SpecReview
analysis: integrity
date: 2026-11-15
scope: "agent-ix/quoin; spec/functional/FR-191, FR-196..FR-201, spec/non-functional/NFR-040, NFR-041"
review_set: subset
relationships:
  - { target: "ix://agent-ix/quoin/FR-198", type: references }
  - { target: "ix://agent-ix/quoin/NFR-041", type: references }
---

## Summary

Ticket: PLAT-1615. Checked completeness, consistency and atomicity of the structural changes: the extended `gap_kind` list and its subject rules, the registration model in FR-191, the NFR-041 / NFR-040 split, and whether each content FR reuses or restates its source's rules.

## Verdict

**FAIL** — FR-198 restates FR-164's unjustified-impacting rule in a narrower form, and NFR-041's link to NFR-040 is prose only. The extended `gap_kind` list is closed and ordered, every new kind used in FR-196..FR-201 appears in it, and every kind in it is used by one of those FRs.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-198 triggers `anomaly_no_release_rationale` on "empty source `releaseRationale`", but the source rule (FR-164 `unjustified-impacting`) uses entries that count as rationale and covers closed `wont-fix`; the dossier restates the rule and diverges. | spec/functional/FR-198-dossier-unresolved-anomalies-content.md:55-56; rust/crates/quoin-anomaly/src/report.rs:308-311 |
| FND-002 | low | NFR-041 says it "Extends NFR-040" in prose only; there is no relationship edge, and NFR-040 still constrains only FR-191..FR-195, so its determinism and no-clock rules do not formally cover FR-196..FR-201. | spec/non-functional/NFR-041-m2-dossier-states-only-recorded-values.md:37; spec/non-functional/NFR-040-dossier-is-deterministic-and-records-only.md:6-16 |
| FND-003 | low | FR-191's M2 subject rule lets `no_acceptability_policy` name "a policy or document"; a gap for an absent policy has no policy to name, so the subject is ambiguous. | spec/functional/FR-191-dossier-command.md:102 |

## Coverage

- Examined: FR-191 gap_kind list and both subject rules, registration and source-list bullets; FR-196..FR-201 gap-producing Behavior bullets; NFR-041 Scope; NFR-040 relationships (context).
- Gap-kind closure: 16 new kinds in FR-191; each used by exactly one M2 FR, except `no_acceptability_policy` (FR-199 and FR-200), which is intended.

## Dispositions

Disposition round 1, checked against PR #929 head 958f582d (fix commit 958f582d over review base b0a203c0).

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | FR-198 now derives `anomaly_no_release_rationale` from source `gate.failures` with mode `unjustified-impacting` (report.rs:33,166) and states it does not restate FR-164's rules; an FR-164 edge was added. |
| FND-002 | fixed | NFR-041 now has a `depends_on` edge to NFR-040, and NFR-040 `constrains` FR-196..FR-201. |
| FND-003 | fixed | FR-191 now names "the document for absent `no_acceptability_policy` and `no_plan_record`". |
