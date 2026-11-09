---
id: SR-20261103118155
title: "Gap analysis of quoin PR 792 (FR-070, FR-071, FR-078 criteria after the 0.4.x pins)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quoin@3b70d3a1; spec/functional/FR-070, FR-071, FR-078; rust/crates/quoin-semantic/tests"
review_set: subset
---

# Gap analysis of quoin PR 792

## Summary

Ticket: PLAT-1377 (also PLAT-278 and PLAT-1221). The analysis is planless and scoped to the
criteria the diff changes or newly asserts. `quoin matrix --json` (quoin 0.29.4) reported "no
coverage_matrix criteria: the module declares no `obligations:` source", so the Quoin matrix
method was unavailable. I did a manual check from acceptance criteria to tests instead, from
`#[trace]` markers and test bodies.

| Criterion | Test | Asserts real behaviour |
| --- | --- | --- |
| FR-071-AC-1 | tc_278_001 | yes: each FR-006 expected `FieldDecl` validates against the published `FieldDecl.json`, and the list is non-empty |
| FR-071-AC-4 | tc_278_004 | yes: each `type-*` expectation validates as `TypeRef`, plus the existing value assertions |
| FR-071-AC-6 | tc_278_006 | yes: each `con-*` constraint validates as `ConstraintDecl`, plus the vocabulary assertions |
| FR-070-AC-2 | tc_452_611, tc_452_611b | partly (FND-001) |
| FR-070-AC-3 | tc_452_612 | yes: the admitted list no longer carries `semantic_core` |
| FR-078-AC-1, FR-078-CON-1 | tc_264_020 | yes: the exact eight-key set, and the raw block validates against the published schema |
| FR-075-AC-1 | tc_452_643 | yes: an exact six-target set, not loosened |
| FR-070-CON-1, NFR-017-AC-4 | tc_452_641 | weakened, see code-review SR-20261103118154 FND-001 |

These are real behaviour checks, not fixture shape. The `tc_278` instances come from quire-rs
fixtures and are validated by a compiled semantic-core validator. `tc_452_611b` goes through the
production `read_module_semantic` path with a malformed value (`7`) as well as a valid one, so
it would fail if the strip were removed or moved after validation.

Plan completion: not assessed

## Verdict

**CONDITIONAL.** The criteria the PR adds or edits are covered by tests that can fail. The
PLAT-278 limitation is properly closed: AC-1, AC-4 and AC-6 are now schema-checked, and both
the note and the test-header paragraph are gone. One weak assertion (FND-001). The weakened
`tc_452_641` guard is recorded once, in the code-review artifact, not duplicated here.
Semantic review was skipped (not opted in).

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | `tc_452_611b` filters to error-level diagnostics (`errors()`) and checks only `package`. FR-070-AC-2 says a manifest with the retired key "loads the same way". A new warning, or a changed `contract_version`, would still pass. Assert that `codes()` is empty and check `contract_version`, as `tc_452_611` does. | rust/crates/quoin-semantic/tests/tc_452_manifest_criteria.rs:444-459 |

## Coverage

- Criteria in scope: FR-070-AC-2, FR-070-AC-3, FR-070-CON-1, FR-071-AC-1, FR-071-AC-4,
  FR-071-AC-6, FR-075-AC-1, FR-078-AC-1, FR-078-CON-1 and NFR-017-AC-4.
- Untagged criteria introduced by the PR: none.
- Reverse gap: the one production change (the strip in `read_semantic_block`) is owned by
  FR-070 and exercised by `tc_452_611b`.
- Stubs: none.
- Semantic review: skipped.
- Plan completion: not assessed
