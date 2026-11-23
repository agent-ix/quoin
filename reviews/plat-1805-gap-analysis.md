---
id: SR-20261123204926
title: "gap analysis of FR-211 evidence resolution and approval verification in clause discharge"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quoin; PR #1034 at 495c6707; spec/functional/FR-211-discharge-resolves-evidence-and-approvals.md (AC-1..23, CON-1/2), FR-046, FR-047, FR-133, FR-135 (AC-3, AC-8), FR-173 (--evidence), FR-191, FR-195, FR-208; tests in rust/crates/quoin-assurance/src/discharge/{verify,store,rmm}.rs, rust/crates/quoin-core/src/ops/{assurance.rs,evidence/tests.rs}, rust/crates/quoin-cli/tests/{tc_2606_discharge_store_command,tc_1488_pack_requirements_report,tc_173_facts_from_mappings}.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quoin/FR-211
    type: references
  - target: ix://agent-ix/quoin/FR-195
    type: references
  - target: ix://agent-ix/quoin/FR-135
    type: references
---

# Gap analysis of FR-211 evidence resolution and approval verification

## Summary

Ticket: PLAT-1805. This is a manual check of each acceptance criterion against its
tests, as the dispatching brief allowed, scoped to the criteria the ticket names. The
full planless `quoin:gap-analysis` matrix and reverse-gap sweep over the repository
was not run, so this is not a full gap audit. Plan completion: not assessed. The coder
wrote the tests after the code, so each FR-211 oracle was checked by mutation: one
wrong behaviour was applied to the production code at a time and the focused tests
were run (results in the code-review SR of this PR and in the Verdict below).

## Examined scope

- FR-211-AC-1, AC-2, AC-3, AC-4, AC-5, AC-23: `verify.rs` unit tests over a memory
  store; AC-1 also in `ops/evidence/tests.rs` and `tc_2606`.
- FR-211-AC-6, AC-7, AC-8, AC-9, AC-10, AC-16, AC-20: `verify.rs` unit tests with a
  recording stub verifier, which also prove that no call is made.
- FR-211-AC-11: unit test, `ops/evidence/tests.rs`, and `tc_1488` with a refusing EA
  script (exit 2) and an absent EA (exit 3), with no fact written.
- FR-211-AC-12: customer-owned direct and disposition unit test.
- FR-211-AC-13, AC-22: disposition verification, offline, and supplied-report re-check
  (`ops/assurance.rs` forged `verifiedApprover` and direct entry).
- FR-211-AC-14: an emptied store reopens the clause (unit) and an unresolved ref in
  `ops/evidence/tests.rs`.
- FR-211-AC-15: `ops/evidence/tests.rs` and `tc_173` (record-discharge refuses the
  list, writes nothing).
- FR-211-AC-17, AC-18: digest unit test with the RFC 8785 text written out, op test,
  and CLI `tc_2606`.
- FR-211-AC-19: Inspection; there is no score or acceptance wording in `render.rs` or
  `rmm.rs`.
- FR-211-AC-21: `tc_2606` `--facts` beside `--repo`, alone, and stored facts ignored.
- FR-135-AC-3, AC-8: `rmm.rs` precedence test.
- FR-173-AC-1, AC-2, AC-5, AC-7: `tc_173` `--evidence` tests.
- FR-195 `not-verified-offline` (AC-1, AC-5) and FR-191 dossier RMM.

## Verdict

FAIL on three medium gaps. Every FR-211 criterion carries a `#[trace]` tag (AC-19 is
Inspection). Mutation results: 14 mutants. Killed (12): attester check dropped (killed by an_attester_other_than_the_verified_approver_is_refused); meaning accepted when it merely contains a role (every_other_status_and_meaning_leaves_the_clause_open); customer-owned direct fact discharged (a_customer_owned_clause_takes_only_a_delegated_disposition); ref resolution reverted to accept anything (three tests including a_record_the_store_no_longer_holds_reopens_the_clause); leading '-' identity check removed (the_record_identity_is_checked_as_engineering_assurance_checks_it); status check removed (every_other_status_and_meaning_leaves_the_clause_open); no-producer branch removed (two tests); offline flag ignored (an_offline_run_never_calls_the_verifier_for_an_approval_class_clause); old FR-135 precedence restored (an_fr_211_reason_never_outranks_a_currency_failure_or_an_expired_exception); disposition approval not verified (a_disposition_on_an_approval_class_clause_is_verified_too); record-discharge writing despite failures (store unit test and record_discharge_refuses_the_whole_list_when_a_ref_does_not_resolve); render_discharge skipping the re-check (a_supplied_report_without_a_store_leaves_a_direct_fact_open). Survived (2): build_authored_argument skipping the re-check of its `discharge` input (no test supplies a discharge report to that operation or to `quoin assurance authored`); settle no longer clearing a supplied `verifiedApprover` on an entry that then fails and moves to open (survives the quoin-assurance unit tests; the quoin-core render test that asserts `mallory` is absent targets this case but was not run against that mutant).

Clean (examined, no finding): the twelve killed mutants above are each caught by the
test that names the criterion. Their bindings to FR-211 AC-1 to AC-23 (except AC-19)
are correct, apart from the AC-4 case noted next. The AC-4 test oracle is wrong
(it asserts an uppercase integration segment is unresolved, against the store's
layout); that is filed as FND-002 of the code-review SR, not repeated here.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | medium | Nothing tests the production `ProcessApprovalVerifier` request on the wire. Every CLI stub engineering-assurance (`stub_verifier`, the refusing script in `tc_1488`) runs `cat >/dev/null` and answers a canned line, so a wrong `protocol`, a missing `record_digest` or `repository`, or a misspelt gitsign member passes every test. FR-211-AC-17 requires that a tag created from the `approval-subject` output verifies as `approved` (AC-6), and no test creates one. The real-EA harness already exists (`tc_plat_1789_signed_approval_chain.rs`, `QUOIN_E2E_EA`, `make e2e-signing`). Add one ignored-by-default case there: approval-subject, then EA `approval-tag`, then `quoin discharge --repo` with the flags gives `approved` and names the verified approver. Also have one stub assert the request members it receives | rust/crates/quoin-cli/tests/tc_1488_pack_requirements_report.rs:282-299, rust/crates/quoin-core/src/runtime.rs:581-603 | correct-requirement-no-evidence |
| FND-002 | medium | FR-195 `not-verified-offline` is tested only at the unit level (`verify.rs` offline test). No dossier test asserts that an approval-class row reads `undischarged` with reason `not-verified-offline` or that the RMM runs no engineering-assurance (FR-195-AC-1, AC-5). Removing `"offline": true` from both requests in `dossier.rs` leaves every test passing: the row then reads `approval-verifier-unavailable`, against FR-195 | rust/crates/quoin-cli/src/dossier.rs:600, rust/crates/quoin-cli/src/dossier.rs:613 | correct-requirement-no-evidence |
| FND-003 | low | FR-135-AC-8's third case is "a temporary_exception past decisionExpiresAt with a current attestation and an unverifiable approval". The test runs with an empty `SetPolicy`, so no approval is involved and the "unverifiable approval" condition is never exercised | rust/crates/quoin-assurance/src/discharge/rmm.rs:642-692 | correct-requirement-no-evidence |
| FND-004 | low | FR-211-AC-18's third case (an `approval-subject` `record_id` over 256 bytes exits non-zero with empty stdout) and FR-211-AC-14's "the stored fact file is unchanged" after `quoin evidence gc` have no test. AC-14 is shown with an emptied memory store, not the gc command, and the file contents are not compared | rust/crates/quoin-assurance/src/discharge/verify.rs:1130-1191, rust/crates/quoin-cli/tests/tc_2606_discharge_store_command.rs:248-301 | correct-requirement-no-evidence |
| FND-005 | medium | FR-211-AC-13 and AC-22 tag the supplied-report re-check for both `render_discharge` and `build_authored_argument`, but only `render_discharge` is tested. A mutant that skips `check_supplied_report` on `build_authored_argument`'s `discharge` input survives every focused test, so an approval-class fact with an unverified `approvalRef` in a `--discharge` report given to `quoin assurance authored` could return to `supported` with nothing failing. Add a test that passes a report holding such a fact and asserts the view is open with the reason | rust/crates/quoin-core/src/ops/assurance.rs:276-285, rust/crates/quoin-core/src/ops/assurance.rs:583-626 | correct-requirement-no-evidence |
