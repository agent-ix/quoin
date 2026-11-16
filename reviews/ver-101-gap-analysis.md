---
id: SR-20261116075206
title: "Gap analysis — VER-101 formal rule approval state and workflow (quoin PR 950)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quoin; spec/functional/FR-188-formal-rule-approval-state-from-signed-commits.md, spec/functional/FR-189-formalize-requirement-workflow.md, spec/functional/FR-190-formal-rule-analysis-claim-selection.md, rust/crates/quoin-cli/src/formal_rules.rs, rust/crates/quoin-cli/src/flow/provider.rs, rust/crates/quoin-quire/src/formal_rules.rs, skills/formalize-requirement/"
review_set: subset
relationships:
  - target: "ix://agent-ix/quoin/FR-188"
    type: reviews
  - target: "ix://agent-ix/quoin/FR-189"
    type: reviews
  - target: "ix://agent-ix/quoin/FR-190"
    type: reviews
---

## Summary

Ticket: VER-101. Reviewed PR `agent-ix/quoin#950` at `15244a674e8a29f226c3f04549b51f181ef000b3` against freshly fetched `origin/main`. Plan completion: not assessed. `quoin matrix --repo . --json` reports zero binders for all 49 acceptance criteria across FR-188 (23), FR-189 (15), and FR-190 (11); all test criteria are `untagged`, and the two inspection criteria per FR are `method-without-symbol`. New tests in the changed status, extraction, and provider paths carry no matching AC trace attributes, so the implementation cannot be traced through the computed Test Matrix. `quire trace` does find a provider test tagged FR-189-AC-4, but the computed matrix has no binder for it; direct tags do not discharge this matrix gap.

## Verdict

**FAIL** — the matrix has untagged acceptance criteria, which is a high-severity gap-analysis finding under the method contract.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | The computed matrix has zero binders for every FR-188, FR-189, and FR-190 acceptance criterion; the new tests are not trace-linked to the contract, leaving all 49 criteria unverified by the matrix. | rust/crates/quoin-cli/src/formal_rules.rs:481-700; rust/crates/quoin-quire/src/formal_rules.rs:170-260; rust/crates/quoin-cli/src/flow/provider/tests.rs:180-330 |

## Detail

### FND-001 — no matrix-backed test evidence for the formal-rule contract

The computed matrix reports `untagged` for FR-188-AC-1..15 and AC-17..23, `method-without-symbol` for AC-16; FR-189-AC-1..5 and AC-8..15 are `untagged`, AC-6..7 are `method-without-symbol`; and FR-190-AC-1..11 are `untagged`. Every row has zero binders. The new unit tests around decision selection and QSL extraction are ordinary `#[test]` functions without trace annotations, and their assertions are not discoverable as evidence for the relevant criteria. The workflow/provider tests similarly do not supply matrix binders. Add criterion tags to tests that actually exercise each behavior and satisfy the inspection criterion trace model, then rerun the computed matrix. The direct `quire trace` result for FR-189-AC-4 points at a pre-existing provider test attribute, but `quoin matrix` still reports zero binders and does not count it as discharged evidence.

## Coverage

Plan completion: not assessed.

- Matrix: `quoin matrix --repo . --json`; FR-188, FR-189, FR-190 acceptance-criterion rows inspected, with binders and static status filtered from the computed output.
- Trace cross-check: `quire trace --scope . --id FR-189-AC-4 --json`; one direct claim exists in `rust/crates/quoin-cli/src/flow/provider/tests.rs`, while the matrix still has no binder.
- Semantic review: skipped; not requested.
- Reverse-gap and stub review: reviewed changed files; no additional standalone finding beyond the matrix traceability gap and code-review findings above.
