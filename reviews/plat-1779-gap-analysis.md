---
id: SR-20261117175249
title: "Gap analysis — PLAT-1779 clause approval module set"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quoin; spec/functional/FR-181-source-profile-evaluation.md, spec/functional/FR-208-module-specific-discharge-approval.md, rust/crates/quoin-cli/tests/tc_1488_pack_requirements_report.rs, rust/crates/quoin-core/src/capabilities.rs, rust/crates/quoin-core/src/ops/evidence/discharge.rs, rust/crates/quoin-core/src/ops/modules/support.rs, rust/crates/quoin-core/src/runtime.rs, rust/crates/quoin-quire/src/clauses.rs"
review_set: subset
relationships:
  - {target: "ix://agent-ix/quoin/FR-181", type: references}
  - {target: "ix://agent-ix/quoin/FR-208", type: references}
---

## Summary

Ticket: PLAT-1779. Planless subset audit of the module-set approval path found that its cross-module validation requirement is only explicit for profile evaluation, and the new discharge regression is bound to an unrelated approval criterion.

## Verdict

**CONDITIONAL** — the changed behavior has one missing requirement and one inflated test-to-criterion binding; no new matrix gaps were introduced beyond the base branch.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | medium | FR-208 does not specify that record-discharge approval metadata is validated against the full installed module vocabulary. | FR-208; rust/crates/quoin-quire/src/clauses.rs::approval_required_clauses_in_module_set | missing-requirement |
| FND-002 | medium | The new positive cross-vocabulary test is tagged to FR-208-AC-1, which requires absent or blank approval references to remain open. | rust/crates/quoin-cli/tests/tc_1488_pack_requirements_report.rs::tc_208_005_record_discharge_reads_approval_metadata_with_the_full_module_set | wrong-requirement |

## Coverage

- Reconciliation: `quoin matrix --repo <review-worktree> --json` (Quoin 0.36.5, Quire 0.51.0, engine 0.62.2); full repository matrix read. The same static counts occur on `origin/main`: 2,195 criteria; 1,386 tagged, 623 untagged, 146 method-without-symbol, 37 retired, and 3 tagged-by-ignored-test. All 2,195 have no run evidence. No matrix status delta was introduced by this PR; the changed test adds a binder to FR-208-AC-1 without asserting that criterion.
- Plan completion: not assessed
- Criteria in changed behavior: FR-181-AC-11 and FR-208-AC-1..5 examined; FR-181-AC-11 remains backed by existing profile-evaluation tests, and FR-208-AC-1..4 remain backed by existing tests.
- Changed behavior inventory: 4; untraced behavior: 1 (full-installed-vocabulary validation of approval metadata in record-discharge); source stubs: 0; test stubs: 0.
- Semantic review: skipped; not requested.

## Scope

| Unit | Path | Role | Excerpt |
| --- | --- | --- | --- |
| `FR-181-AC-11` | `spec/functional/FR-181-source-profile-evaluation.md` | examined | “When a selected set's declaring module uses an edge type registered by another installed module, the set is reported as `evaluated` and no `UnknownEdgeType` failure is returned.” |
| `FR-208-AC-1` | `spec/functional/FR-208-module-specific-discharge-approval.md` | examined | “A binding clause with non-empty `approvalRoles` remains open when its direct fact has no `approvalRef`, or has an empty or whitespace-only reference.” |
| `FR-208-AC-2` | `spec/functional/FR-208-module-specific-discharge-approval.md` | examined | “A binding clause with non-empty `approvalRoles` that require a named human approver is direct only when its direct fact has a non-blank `approvalRef` identifying that person; the report retains the reference without changing the direct classification.” |
| `FR-208-AC-3` | `spec/functional/FR-208-module-specific-discharge-approval.md` | examined | “A binding clause with empty `approvalRoles` continues to accept a direct fact without `approvalRef` as specified by FR-046.” |
| `FR-208-AC-4` | `spec/functional/FR-208-module-specific-discharge-approval.md` | examined | “When multiple installed modules declare the same exact clause-set identity, discharge fails without selecting either module's approval policy.” |
| `FR-208` | `spec/functional/FR-208-module-specific-discharge-approval.md` | examined | “For each binding clause, Quoin SHALL read its `approvalRoles` metadata from the uniquely resolved clause set.” |
| `approval_required_clauses_in_module_set` | `rust/crates/quoin-quire/src/clauses.rs` | examined | “The approval requirements declared by `module`, validating its clause sets against the complete installed module vocabulary. The returned keys remain attributable to `module`; other installed modules contribute vocabulary but do not contribute approval policies to this result.” |
| `tc_208_005_record_discharge_reads_approval_metadata_with_the_full_module_set` | `rust/crates/quoin-cli/tests/tc_1488_pack_requirements_report.rs` | examined | “The regression installs a clause-set module using `references`, installs a provider declaring `references`, supplies a valid approvalRef, then asserts record-discharge succeeds and stdout contains `C-1`.” |

No source or test stub was observed in the changed files. Repository-wide untagged criteria and missing run evidence are unchanged from the selected base and are not attributed to this diff.

## Validation

- `quoin matrix --repo <review-worktree> --json` and the same command on detached `origin/main` completed; static status totals match exactly.
- The changed regression passed independently through `locked-build.sh` after running the repository's frozen pnpm install for the required semantic schema build inputs.
- This is a planless subset audit; plan completion was not assessed. Optional semantic review was skipped.

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | FR-208 now explicitly owns the complete installed vocabulary and provider-policy boundaries for record-discharge, with ACs for valid external vocabulary, provider attribution, and unknown vocabulary. after_excerpt: `| FR-208-AC-6 | When the uniquely resolved clause set uses an edge type declared by another installed module, `record-discharge` validates the clause set using vocabulary from all installed modules. | Test |` |
| FND-002 | fixed | The new test is tagged to FR-208-AC-6, which directly specifies cross-module approval-metadata loading. after_excerpt: `#[trace("FR-208-AC-6")]` |
