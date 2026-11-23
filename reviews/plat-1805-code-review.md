---
id: SR-20261123204925
title: "code review of FR-211 evidence resolution and approval verification in clause discharge"
type: SpecReview
analysis: code-review
scope: "agent-ix/quoin; PR #1034 at 495c6707 over origin/main 86313ad5; rust/crates/quoin-assurance/src/discharge/{verify,store,build,rmm,pack_rmm,render,report}.rs, rust/crates/quoin-core/src/{capabilities,runtime,dispatch}.rs, rust/crates/quoin-core/src/ops/{assurance.rs,evidence/discharge.rs,evidence/verification.rs}, rust/crates/quoin-cli/src/{discharge,core_runtime,assurance,dossier}.rs, rust/crates/quoin-cli/src/evidence/mapping_facts.rs, rust/crates/quoin-quire/src/clauses.rs, rust/crates/quoin-quire-types/src/clauses.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quoin/FR-211
    type: references
  - target: ix://agent-ix/quoin/FR-135
    type: references
  - target: ix://agent-ix/quoin/FR-173
    type: references
  - target: ix://agent-ix/quoin/FR-195
    type: references
---

# Code review of FR-211 evidence resolution and approval verification in clause discharge

## Summary

Ticket: PLAT-1805. PR agent-ix/quoin#1034, head 495c6707 (9 commits on origin/main
86313ad5). Code review with the repository rust-style (which outranks the ecosystem
rust-review) applied as the Rust lane. Read: the whole of `discharge/verify.rs`, the
diffs of every other changed production file, the engineering-assurance
`approval-verify` adapter on its origin/main (6ea59921) for the wire contract, and the
`quoin-evidence` path builders for the store layout. Focused test runs and a mutation
pass back the verdict (see Verdict). Tool versions: quoin 0.36.8, quire 0.52.0
(engine 0.62.2).

## Examined scope

- FR-211 Evidence references (ref grammar, families, folded scenarios, retained kind,
  `producers`, `kind-not-expected`, `no-producer`): `verify.rs` `check_reference`,
  `resolve_reference`.
- FR-211 Approval verification (approval-missing, request members, record_id and
  RFC 8785 digest, invalid-record-identity, record-id-too-long, approved plus exactly
  one meaning equal to a role, attester-not-approver, refusal mapping exit 2 and 3):
  `verify.rs` `verify_approval`, `record_id`, `record_digest`; `runtime.rs`
  `ProcessApprovalVerifier`; `core_runtime.rs` `invoke_verifying`.
- FR-211 Customer-owned clauses: `verify.rs` `check_direct`, `check_disposition`;
  `quoin-quire/src/clauses.rs` `set_policy`.
- FR-211 When resolution and verification run: `--facts` with `--repo`,
  `render_discharge` and `build_authored_argument` explicit inputs
  (`verification.rs` `check_supplied_report`), record-discharge refusing the whole
  list (`store.rs` `record_discharge_facts`), offline dossier RMM (`dossier.rs`,
  `offline: true`).
- FR-211 Approval subject: `verify.rs` `approval_subject`,
  `ops/evidence/discharge.rs` `approval_subject`, CLI `approval-subject`.
- FR-211-CON-1 and CON-2: no score, threshold or acceptance decision anywhere in the
  diff; no signature code (only the EA result is read).
- FR-135 first-match precedence: `rmm.rs` `row`.
- FR-173 `--evidence`: `mapping_facts.rs` `read_evidence`, `propose`.
- Breaking API changes: `ModuleHost::clause_policies`, `Capabilities::with_hosts`
  eighth argument, `assurance.render_discharge` request shape, removed
  `*_with_approval_requirements` and `inward_completeness_from_store*` exports.

## Verdict

FAIL. Not mergeable until FND-001 and FND-002 are fixed; FND-003 and FND-004 should be
fixed in the same round.

Clean (examined, no finding): the approval decision is right as written. `approved`
plus `meaning` of exactly one value byte-equal to a role, `attestedBy` equal to the
verified approver for direct facts only, `approval-missing` before any call,
`invalid-record-identity` and `record-id-too-long` before any call and never reported
as unavailable, and an allowed-signers refusal stopping the run with exit 2 while any
other refusal reads `approval-verifier-unavailable` (exit 3 from record-discharge).
The request carries exactly EA's `deny_unknown_fields` members (`protocol`,
`repository`, `tag`, `record_id`, `record_digest`, `allowed_signers`, `gitsign` with
`certificate_identity`, `certificate_oidc_issuer`, `principal`), `--request -` is EA's
CLI, the error object is on EA's stdout, and `approval_verify_allowed_signers_invalid`
is EA's code. `record_digest` is SHA-256 over `quoin_store::canonical_bytes` (the
`quire-canonical` encoder) of the parsed fact without `approvalRef`, and approval-subject
and verification use the same function, so the two always agree. A supplied
`verifiedApprover` is discarded before checks run. `record-discharge` stages every
fact, checks them all, and writes nothing when any fails. `--facts` with `--repo`
reads the facts from the file and resolves against the store; without `--repo` a
direct fact is `no-store`. FR-135 precedence in `rmm.rs` is expired exception, then
attestation not current (even with an FR-211 reason), then undischarged with the
reason. FR-173 `--evidence` refusals name the file and the key. quoin never verifies a
signature and never scores risk (CON-1, CON-2). No consumer outside the quoin
workspace depends on `quoin-core`, `quoin-assurance`, `quoin-quire` or
`quoin-quire-types` (top-level repositories under the dev root checked read-only;
`verification-quoin` and `review-scratch` are copies of quoin itself), and the
workspace builds and its focused tests pass, so the breaking API changes leave nothing
broken. `no-producer` until quire-rs carries `producers`: `set_policy` always yields
empty `producers`, so every direct fact on a real clause with non-empty
`expectedOutputs` is open with `no-producer`, as FR-211 states; quire-rs is not edited.

Focused runs at 495c6707, all passing: quoin-assurance `discharge::` unit tests (51),
quoin-core `ops::` unit tests (171), and the integration tests `tc_2606` (5), `tc_1488`
(20), `tc_173` (25), `tc_3206` (1), `tc_447` (8), `tc_458` (24) and quoin-assurance
`tests/discharge.rs` (14). With `SPEC_OBJECTS_SECURITY_DIR` pointing at a
spec-objects-security checkout, `tc_173_scope_exclusion_decision_schema_and_facts_are_strict`
passes; it fails only when that checkout is not supplied. Clippy over the changed crates (`-D warnings`) did not reach them: it stops at 13
pre-existing errors in `quoin-evidence`, a dependency this PR does not touch, so the
lint status of the changed files was not measured.

Mutation pass (each mutant applied alone, focused tests run, then reverted): 14 mutants. Killed (12): attester check dropped (killed by an_attester_other_than_the_verified_approver_is_refused); meaning accepted when it merely contains a role (every_other_status_and_meaning_leaves_the_clause_open); customer-owned direct fact discharged (a_customer_owned_clause_takes_only_a_delegated_disposition); ref resolution reverted to accept anything (three tests including a_record_the_store_no_longer_holds_reopens_the_clause); leading '-' identity check removed (the_record_identity_is_checked_as_engineering_assurance_checks_it); status check removed (every_other_status_and_meaning_leaves_the_clause_open); no-producer branch removed (two tests); offline flag ignored (an_offline_run_never_calls_the_verifier_for_an_approval_class_clause); old FR-135 precedence restored (an_fr_211_reason_never_outranks_a_currency_failure_or_an_expired_exception); disposition approval not verified (a_disposition_on_an_approval_class_clause_is_verified_too); record-discharge writing despite failures (store unit test and record_discharge_refuses_the_whole_list_when_a_ref_does_not_resolve); render_discharge skipping the re-check (a_supplied_report_without_a_store_leaves_a_direct_fact_open). Survived (2): build_authored_argument skipping the re-check of its `discharge` input (no test supplies a discharge report to that operation or to `quoin assurance authored`); settle no longer clearing a supplied `verifiedApprover` on an entry that then fails and moves to open (survives the quoin-assurance unit tests; the quoin-core render test that asserts `mallory` is absent targets this case but was not run against that mutant).

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | high | A clause whose clause set has no uniquely resolved installed owner gets `ClausePolicy::default()`: no approval roles and no outputs. A resolved ref of any kind then discharges it even when the binding lists non-empty `expectedOutputs`, and an approval-class clause discharges with no engineering-assurance call. Reachable through `quoin discharge --binding B --repo R` with any binding whose set is not installed, through a report on stdin whose `clauseSet` is edited to an uninstalled key (`render_discharge`, `build_authored_argument`), and through the pack path's `no_policy`. The store unit tests depend on it: SYN-001 lists `expectedOutputs: ["test-result"]`, runs with an empty `SetPolicy`, and is asserted direct | rust/crates/quoin-assurance/src/discharge/verify.rs:250-251, rust/crates/quoin-core/src/ops/evidence/verification.rs:171, rust/crates/quoin-assurance/src/discharge/pack_rmm.rs:134-141, rust/crates/quoin-assurance/src/discharge/store.rs:712-725 | implementation-bug-despite-evidence |
| FND-002 | high | `resolve_reference` requires the `integration` segment of a `scenarios/` ref to be lowercase, but the store keeps it as written and folds only the scenario file name (`quoin_evidence::paths::scenario_path`; `quoin-evidence` stores `scenarios/IT-002/it-002-sc-07a.json`). Every real scenario record (integration ids are `IT-NNN`) is therefore `unresolved`, whichever case the ref uses. The AC-4 test encodes the wrong rule by asserting `evidence://scenarios/GitHub/login` is unresolved. The FR-211 Behavior bullet that says both segments are compared folded is a spec defect against FR-030's layout; FR-211-AC-4 itself names only the scenario segment | rust/crates/quoin-assurance/src/discharge/verify.rs:498-504, rust/crates/quoin-evidence/src/paths.rs:91-95, rust/crates/quoin-assurance/src/discharge/verify.rs:740-748 | wrong-requirement |
| FND-003 | medium | `settle` keeps a `direct` or `dispositions` entry that carries no `fact` without any check, so a report document on stdin, or a `--discharge` file given to `quoin assurance authored`, can list a clause as discharged with no fact, no ref and no approval, and it renders under "Direct evidence". FR-211 says Quoin discharges a clause only on evidence and approval that resolve when the report is rendered. A factless entry in either discharged population should move to open. (A supplied report can also omit clauses outright, because render and authored-argument take no binding report. That is a limit of the FR-211 input set; record it rather than fix it here.) | rust/crates/quoin-assurance/src/discharge/verify.rs:196-201, rust/crates/quoin-assurance/src/authored/build.rs:380-387 | implementation-bug-despite-evidence |
| FND-004 | medium | `build_discharge_report` and `build_discharge_report_with_approval_requirements` stay public and re-exported from `quoin-assurance`, and they return a `DischargeReport` whose `direct` population has not been resolved or verified. FR-211 requires the library report builders to run both checks. No production caller remains in the workspace and no other repository links the crate, so the risk is a future library caller, not a live bypass. Make them `pub(crate)`, or rename them to a partition type that cannot be rendered as a report | rust/crates/quoin-assurance/src/discharge/mod.rs:89, rust/crates/quoin-assurance/src/lib.rs:71 | implementation-bug-despite-evidence |
| FND-005 | low | `ProcessApprovalVerifier` builds `meaning` with `filter_map` over string values, so a result `meaning: ["reviewer", 1]` reads as exactly one value and is accepted. FR-211 says the array holds exactly one value. A non-string member should make the result unsupported (unavailable), not be dropped | rust/crates/quoin-core/src/runtime.rs:656-665 | implementation-bug-despite-evidence |
| FND-006 | low | On a non-zero EA exit the reason is `<code>: <stderr>`, but EA writes its `message` to stdout and usually nothing to stderr, so a user sees `approval-verifier-unavailable: approval_verify_request_invalid: ` with no explanation. This is also the reason when the `approvalRef` is not a plain revision (for example it holds whitespace), where the verifier is in fact available. Include the EA `message` (as `formal_rules.rs` does). The EA invocation is also a second copy of `formal_rules.rs` `verify_requesting`; share one spawn-and-decode helper | rust/crates/quoin-core/src/runtime.rs:632-641, rust/crates/quoin-cli/src/formal_rules.rs:501-545 | implementation-bug-despite-evidence |
| FND-007 | low | `quoin discharge --facts F` with `--engineering-assurance`/`--allowed-signers` and no `--repo`: the `--json` path silently ignores the flags (`assurance.build_discharge` is sent no approval), while the text path fails in `render_discharge` with "approval inputs need a repository". Refuse the flags without `--repo` up front, the same way on both paths | rust/crates/quoin-cli/src/discharge.rs:232-256, rust/crates/quoin-core/src/ops/evidence/verification.rs:217-220 | implementation-bug-despite-evidence |
| FND-008 | low | `read_evidence` accepts a key without the `ix://` prefix (`strip_prefix("ix://").unwrap_or(key)`), but FR-173 keys `--evidence` by a control's `ix://` id. A bare `CTRL-A` key is accepted silently instead of being refused | rust/crates/quoin-cli/src/evidence/mapping_facts.rs:164 | implementation-bug-despite-evidence |
