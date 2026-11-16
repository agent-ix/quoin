---
id: SR-20261116004343
title: "Gap analysis of PLAT-1619 unresolved anomalies dossier"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quoin; PLAT-1619 PR #947 patch against peter/plat-1618-soup-document: spec/functional/FR-198-dossier-unresolved-anomalies-content.md, rust/crates/quoin-dossier/Cargo.toml, rust/crates/quoin-dossier/src/anomalies.rs, rust/crates/quoin-dossier/src/gap.rs, rust/crates/quoin-dossier/src/lib.rs, rust/crates/quoin-dossier/tests/tc_1619_anomalies.rs, rust/Cargo.lock"
review_set: subset
relationships:
  - target: "ix://agent-ix/quoin/FR-198"
    type: references
  - target: "ix://agent-ix/quoin/FR-191"
    type: references
---

## Summary

Audited the FR-198 criteria, changed implementation and tests using the computed Quoin and Quire matrices. The diff contains tests aimed at all eight criteria and no unimplemented FR-198 behavior or stub was observed. The installed matrix tools classify all eight criteria as untagged even though the Rust test/source functions carry `#[trace(...)]` attributes. This matches the previously verified clean-main marker/row classification limitation recorded under PLAT-1767; it is not evidence of a missing test in this patch. No workaround or hand-authored matrix row was added.

## Verdict

**CONDITIONAL** — no PR-specific gap finding; computed trace coverage cannot be confirmed because the installed matrix tooling does not recognize these trace bindings, matching the known clean-main baseline.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Coverage

- Reconciliation: `quoin matrix --repo /home/peter/dev/worktrees/plat1619-anomalies --json` (Quoin 0.35.0) and `quire matrix --scope /home/peter/dev/worktrees/plat1619-anomalies --format json` (engine 0.60.0); all eight FR-198 criteria are reported untagged. `quire coverage --scope ... --json` reports the `#[trace(...)]` bindings outside computed rows and 3,785 diagnostics across the scope.
- Plan completion: not assessed.
- Criteria examined: FR-198-AC-1 through FR-198-AC-8. Each has a corresponding changed integration or unit test; the source carries explicit trace attributes.
- Changed production behavior: anomaly selection/rendering, source failure, grouped and ungrouped records, and both FR-198 gap kinds are covered by the changed tests. No stub or unrelated untraced behavior was observed.
- Semantic review: skipped; not requested by the applicable scope.
- Matrix limitation: the same marker/row classification mismatch was previously reproduced on clean main and recorded as PLAT-1767. The computed status is preserved as a tooling limitation, not treated as a PR-specific defect.
