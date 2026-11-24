---
id: SR-20261124021651
title: "gap analysis of the FR-169 risk-summary contract and FR-199 DOS-RISK content criteria"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quoin; PR #1041 at 0078ebcb; spec/functional/FR-169-risk-summary-command.md (AC-1, AC-2, AC-29..35), spec/functional/FR-199-dossier-risk-management-content.md (AC-1, AC-9..16), FR-191 gap list; tests in rust/crates/quoin-risk-advice/tests/tc_169_risk_summary.rs, rust/crates/quoin-dossier/tests/tc_1622_risk_management.rs, rust/crates/quoin-dossier/src/risk_management.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quoin/FR-169
    type: references
  - target: ix://agent-ix/quoin/FR-199
    type: references
  - target: ix://agent-ix/quoin/FR-191
    type: references
---

# Gap analysis of the FR-169 and FR-199 criteria in PLAT-1810

## Summary

Ticket: PLAT-1810. `quire matrix --scope . --format json` (quire 0.52.0, engine 0.62.2)
over the PR head: every FR-169 and FR-199 criterion has at least one tagged test. The
reverse-gap check was limited to the PR diff: every new item maps to FR-169 or FR-199,
and there are no stubs. Each tagged test was then checked for whether it asserts the
criterion, using the mutation pass recorded in the code-review SR of this PR. The
semantic review was not run. Plan completion: not assessed.

## Examined scope

- FR-169-AC-1, AC-2: four top-level keys and closed entry key sets. Tagged and killed
  by the extra-key mutants.
- FR-169-AC-29: titles and texts, harm_texts ascending, `null`. Tagged and killed.
- FR-169-AC-30: rationale text, then `null` for an empty cell. Only the text half is
  tested (FND-001).
- FR-169-AC-31, AC-32: benefit_risks and overall_evaluations entries, ascending, `[]`.
  Tagged and killed.
- FR-169-AC-33: three tables, header only when empty. Tagged and killed.
- FR-169-AC-34: the overall evaluation contract check. Tagged and killed.
- FR-169-AC-35: `<br>` with JSON keeping line breaks. Tagged; the coder's mutation
  killed it.
- FR-199-AC-1, AC-9, AC-11, AC-12, AC-13, AC-15, AC-16: tagged and killed.
- FR-199-AC-10: authored rationale, then `not stated`. Only the first half is tested
  (FND-001).
- FR-199-AC-14: tagged; the decided half survives (code-review FND-001).
- FR-199-AC-2, AC-3, AC-4, AC-6, AC-7: bound only to `risk_management.rs` lib tests,
  which do not compile on origin/main or on this branch (FND-002).
- FR-191 closed gap-kind order: context only (FND-003).

## Verdict

CONDITIONAL. No criterion is untagged. Three gaps remain: two criteria whose null /
`not stated` half has no test, five FR-199 criteria whose only tests cannot currently
build, and a pre-existing FR-191 order drift outside this PR.

Brief item (a): the null / `not stated` path is not vacuous. The shipped
spec-objects-safety manifest requires `Rationale`, `Benefit` and `Product` to match
`^[^<>\s][^<>]*$`. An empty cell therefore exits 2 through the loader, so these cells can
never be null from a valid corpus. But `--module` can select another declaring module,
and the code branch exists. `Decided by` (`^[^<>]*$`) can be empty and its null path is
tested. So the AC wording needs no fix. What is missing is unit-level tests:
`records.rs` `choice()` with an empty Rationale, Benefit and Product cell gives `None`,
and a `risk_management.rs` render test whose Summary has `rationale: None`,
`product: None` and `benefit: None` asserts `not stated`. The existing
`decided_evaluation()` fixture already has `rationale: None` but nothing asserts on it.
Optionally the AC can say "where the declaring module allows an empty cell".

Brief item (b): the FR-191 list order (`no_plan_record`, `release_not_verified`,
`no_release_verify_input`, `no_snapshot`) differs from the `GapKind` order (`NoPlanRecord`,
`NoSnapshot`, `NoReleaseVerifyInput`, `ReleaseNotVerified`). This predates this PR. The
spec has carried this order since #929 (ce2a4cb9). The code has used its order since
PLAT-1620 (#952, 607af7ee). The drift is observable: DOS-REL with neither `--snapshot`
nor `--release-verify` sorts `no_snapshot` before `no_release_verify_input`. The DOS-REL
owner (PLAT-1620) should reconcile it, by changing the code to the older spec order.

Red main, measured on origin/main 61c4dfdc without this PR:

- `tc_3604_release_snapshot`: 32 of 33 fail at `Repo::bare` → `binding()` →
  `evidence record-discharge` with `no-clause-policy: no installed module declares clause
  set example.invalid/soup/1.0.0` (CORE_BAD_REQUEST, discharge-facts-failed). The fixture
  records a discharge fact against a clause set that no module in its empty `IX_HOME`
  declares. `check_facts` and the `no-clause-policy` reason first appear in #1034
  (5c97d54b; `verify.rs` does not exist at 9c5417ef). #1034's own review fix round made
  record-discharge refuse such facts. #1034's base was 681af611, where tc_3604 already
  called record-discharge. So #1034 broke tc_3604 on its own base: this is not a merge
  race with #1035, and its CI runs only CLA checks. The owner is PLAT-1805. The fixture
  needs a module that declares the clause set, or an installed one.
- `tc_145_risk_advice::control_links_are_tested_by_their_edges_alone`: the advise
  control-entry keys now include `retired_requirements`, added by PLAT-1798 (#1006,
  66ffc567), but the test still expects four keys. The owner is PLAT-1798.
- `tc_1834_dossier_risk_plan` (both tests): NotFound on
  `skeletons/risk_management_plan.md`. The local spec-objects-safety checkout is at
  be3cdf1, behind its origin/main, where a5d0094 (PLAT-1834) adds the skeleton. This is
  an environment issue, not a code defect. Fast-forward the checkout.
- quoin-dossier lib tests do not compile on main for two reasons. `Summary.unusable` was
  added by #1035 (9c5417ef) without updating `risk_management.rs` tests. This PR and
  #1040 both fix that. Separately, `rmm.rs` calls the 3-argument
  `discharge_report_from_store` that #1034 made 4-argument. Only #1040 fixes that.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-169-AC-30 ("an empty cell gives null") and FR-199-AC-10 ("one with no rationale reads `not stated`") are tagged on tests that assert only the present-text half. The null and `not stated` path for Rationale, and likewise for Benefit and Product (FR-169-AC-31/32 field rules and FR-199 Behavior), has no test. It is unreachable through the shipped manifest but reachable by another declaring module. Add unit tests on `choice()` and on `render` with `None` values | rust/crates/quoin-risk-advice/tests/tc_169_risk_summary.rs:1230-1246, rust/crates/quoin-dossier/tests/tc_1622_risk_management.rs:282-310, rust/crates/quoin-risk-advice/src/records.rs:255-269 |
| FND-002 | medium | FR-199-AC-2, AC-3, AC-4, AC-6 and AC-7 are bound only to `risk_management.rs` lib tests. That lib test target does not compile on origin/main, and with this PR alone it still fails on `rmm.rs`'s 3-argument `discharge_report_from_store`. Those criteria have no runnable evidence until #1040, or an equivalent `rmm.rs` fix, lands | rust/crates/quoin-dossier/src/rmm.rs:215, rust/crates/quoin-dossier/src/risk_management.rs:486-760 |
| FND-003 | low | Pre-existing, outside this diff: FR-191's closed gap list orders `release_not_verified`, `no_release_verify_input`, `no_snapshot`, but `GapKind` orders `NoSnapshot`, `NoReleaseVerifyInput`, `ReleaseNotVerified` (since PLAT-1620 #952). DOS-REL can emit `no_release_verify_input` with `no_snapshot`, and `release_not_verified` with `no_snapshot`, in the opposite order to the spec. The DOS-REL owner should reconcile it | rust/crates/quoin-dossier/src/gap.rs:73-79, spec/functional/FR-191-dossier-command.md:113 |

## Dispositions

Round 1 was checked against head 8d58b5a2. Both mutants named below were killed in the
round-1 mutation batch.

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | The new `records.rs` unit test `an_empty_rationale_benefit_product_or_decider_cell_is_none` covers this, and dropping `non_blank` from Rationale is killed. The new `risk_management.rs` test `every_absent_authored_value_reads_not_stated` covers the `not stated` half for rationale, benefit, product and decider, and printing an empty rationale cell is killed |
| FND-002 | fixed | The branch is rebased onto #1040, which fixes the `rmm.rs` call, so the dossier lib tests compile and the FR-199-AC-2/3/4/6/7 tests run and pass (129) |
| FND-003 | deferred | Pre-existing outside this diff. The FR-191 order of the three release gap kinds belongs to the DOS-REL owner (PLAT-1620), as the review stated |
