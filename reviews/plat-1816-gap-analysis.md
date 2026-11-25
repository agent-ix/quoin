---
id: SR-2026112016223475846355045
title: "PLAT-1816 gap-analysis of Quoin repository"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quoin at PR head 85ffa442c4eb17f102d93ea1503c5502d699e26f; whole repository matrix, trace tags, source and test surfaces"
review_set: subset
relationships:
  - {target: "ix://agent-ix/quoin/FR-191", type: references}
---
## Summary
The engine-backed `quoin matrix --repo . --json` found 673 live untagged criteria and 5 criteria bound only by ignored tests across the repository. The changed FR-191-AC-31 is statically tagged to `generated_documents_carry_the_running_quoin_version`; no run evidence is stored in the repository's evidence store. A source marker scan across Rust crates found no `todo!()` or `unimplemented!()` production body; textual placeholder matches were comments, test fixtures, or domain behavior. The changed producer path is fully specified. The manual reverse-gap inventory was limited to touched producer routes, so repository-wide untraced behavior is not claimed as exhaustively counted.

## Verdict
FAIL — the repository-wide matrix contains untagged and ignored-only criteria (pre-existing beyond this PR); this is a planless repository assurance result, not a finding that the changed acceptance criterion is untested.

## Findings
| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | 673 live acceptance criteria are untagged; 5 additional criteria are bound only by ignored tests. | `quoin matrix --repo . --json`; `quire coverage --scope . --json` |

## Coverage
Plan completion: not assessed. Reconciliation: engine-backed `quoin matrix` (quoin 0.36.8, quire 0.52.0). Whole-repository population: 2,542 criteria; 1,659 tagged, 673 untagged, 5 tagged-by-ignored-test, 169 method-without-symbol, 36 retired. Evidence status is `no run evidence` for all 2,542. The changed AC-31 has a binder in `rust/crates/quoin-cli/tests/tc_1614_dossier_command.rs::generated_documents_carry_the_running_quoin_version`. Coverage diagnostics include broad legacy untracked ids; none identify the new criterion as unbound in the static matrix. Reverse inspection traced touched generator routes through `engine::render_for` and the shared writer; no stub or untraced behavior was found there. A repository-wide public API candidate scan found 2,375 exported declarations; the broad stub-marker scan found 0 actual `todo!()`/`unimplemented!()` bodies. A complete semantic mapping of that entire API inventory was not established, so no repository-wide reverse-gap count is claimed. Semantic intent review was skipped because it was not opted in. The matrix gaps are repository-wide and not attributed to this PR.
