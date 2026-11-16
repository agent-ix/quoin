---
id: SR-20261116004345
title: "Gap analysis of PLAT-1656 risk management plan dossier"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quoin; PLAT-1656 PR #948 patch against peter/plat-1619-anomalies: spec/functional/FR-200-dossier-risk-management-plan-content.md, spec/functional/FR-191-dossier-command.md, rust/crates/quoin-cli/src/dossier.rs, rust/crates/quoin-dossier/src/gap.rs, rust/crates/quoin-dossier/src/lib.rs, rust/crates/quoin-dossier/src/risk_plan.rs, rust/crates/quoin-dossier/src/sources.rs, rust/crates/quoin-dossier/tests/tc_1437_generators.rs, rust/crates/quoin-dossier/tests/tc_1617_verification_report.rs, rust/crates/quoin-dossier/tests/tc_1618_soup.rs, rust/crates/quoin-dossier/tests/tc_1619_anomalies.rs"
review_set: subset
relationships:
  - target: "ix://agent-ix/quoin/FR-200"
    type: references
  - target: "ix://agent-ix/quoin/FR-191"
    type: references
---

## Summary

Audited the PLAT-1656 patch against its repository requirements, tests and source behavior; plan completion was not assessed. The diff adds no unrelated user-visible behavior, but the new gap order conflicts with FR-191 and the specific malformed-policy acceptance path lacks end-to-end evidence.

## Verdict

**CONDITIONAL** — the patch has two medium findings; the computed matrix also cannot confirm its trace bindings because of a known engine classification issue.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | `no_plan_record` precedes `no_acceptability_policy`, contrary to the closed gap-kind order in FR-191. | rust/crates/quoin-dossier/src/gap.rs:60-63,90-91; FR-191 |
| FND-002 | medium | The AC-4 test bypasses the typed record reader and command engine, so it does not prove source exit 3 with no selected output file. | rust/crates/quoin-dossier/src/risk_plan.rs:262-267; FR-200-AC-4 |

## Coverage

- Reconciliation: `quoin matrix --repo /home/peter/dev/worktrees/plat1656-review --json` (Quoin 0.35.0) and `quire matrix --scope /home/peter/dev/worktrees/plat1656-review --format json` (Quire engine 0.60.0); for FR-200, 0 tagged and 6 untagged criteria, all with no run evidence.
- Matrix limitation: the six tests carry matching `#[trace("FR-200-AC-N")]` attributes, while `quoin matrix` reports each as `tagged-without-citing-matrix-row` and no binders. `quire coverage` reports 3,792 repository diagnostics and 290 untracked symbols but no FR-200 untracked symbol. This matches the previously recorded PLAT-1767 marker/row classification limitation; no hand-authored matrix or trace workaround was introduced.
- Plan completion: not assessed.
- Criteria examined: FR-200-AC-1 through FR-200-AC-6; the implementation and tests are traced to FR-200. The gap-kind order is also checked against FR-191's closed list.
- Changed production behaviors inventoried: policy-source selection, typed read and source-error mapping; policy table validation and deterministic rendering; policy-only and missing-plan gaps; generator registration and CLI wiring. Untraced behaviors: 0.
- Source stubs: 0. Test stubs: 0. AC-4's test does not cover the full reader/command failure behavior stated by the criterion.
- Semantic review: skipped; not requested.
