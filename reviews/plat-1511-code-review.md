---
id: SR-20261113012816
title: "Code review of quoin PR 839 (PLAT-1511 claimed signed approvals)"
type: SpecReview
analysis: base
scope: "agent-ix/quoin@3ba037d1; rust/crates/quoin-change-assurance, rust/crates/quoin-core/src/ops/change_assurance, rust/crates/quoin-cli/src/change_assurance.rs, rust/crates/quoin-cli/tests/fixtures/retained-command-help.json, tests/fixtures/change-assurance-cli"
review_set: subset
---

# Code review of quoin PR 839

## Summary

Ticket: PLAT-1511. Scoped to `git diff origin/main...HEAD` at 3ba037d1, 41 files. The Rust
lane (`rust-review`, with the repo `rust-style` skill taking precedence) is folded in here.
Focused tests ran through locked-build.sh with a worktree-local target and debug=0:
`cargo test -p quoin-change-assurance -p quoin-cli -p quoin-core -p quoin-store`. 72 test
binaries passed. One binary failed: `tc_136_anomaly_command` (5 of 5 tests). The PR does not
touch it. It panics on a path-existence precondition (line 30) before any code under test
runs, because the sibling `spec-artifacts-process` checkout is at 28c8d2c (#137) and has no
`tests/fixtures/anomaly`, which needs #140 or later. It fails the same way on origin/main.

Hard checks, all confirmed:

- **No Git, network, IdP or process.** `verify/approval.rs` and `model/approval.rs` name no
  `std` item. `record_digest` reads one file through the host store.
- **No trust taken from the claim, record or revision.** `judge_approval` never reads
  `claim.trust`, and the record policy names no trust input.
- **Rule order.** The rules run in this order: status, then policy narrowing (only when
  `required` is true and the status is `approved`), then record binding (also on
  `meaning_missing`, as FR-065 rule 3 says), then meaning. All six reason codes are reachable
  and distinct, and the incomplete flags are right.
- **Record digest.** It is computed once from the bytes `store.record()` returned, the same
  buffer that was parsed. There is no TOCTOU and no re-serialization, the digest is never
  stored, and `record-digest` writes nothing.
- **Schema assets.** Both v2 assets are byte-identical to the FR-063 and FR-065 schema
  blocks. The attestation stays v1, and there is no v1 compat reader.
- **Forged claims.** A forged claim reaches `valid` only where the spec says it should. Closed
  shape, wrong protocol, an extra or out-of-row member, a claim on rejected or revise, and
  duplicates are each refused or `decision_mismatch`.

Judgement calls:

- **(5b) Hand-edited help fixture.** The fixture is the help source: `help.rs` does
  `include_str!` on it. There is no other "real" output to drift from, so editing it is
  authoring, not drift. Only its shape differs (FND-006).
- **(5c) MP-250/251 pairs.** These are correct. The nfr003 population test passes against
  the binary.
- **(5d) The `#[allow(clippy::too_many_lines)]` on `dispatch()`.** This is acceptable. It
  carries a reason, it covers a routing table, and the repo has precedent.
- **(5e) Unsorted policy arrays.** This is a real inconsistency (FND-001).
- **(5f) Binding check on `meaning_missing`.** This is spec-conformant.
- **(5a) Census widening.** It has a reason that holds independent of the count, but it is
  broader than it needs to be (FND-002).

## Verdict

**PASS WITH FINDINGS. No high findings. Three medium (FND-001..003) to fix in-PR, plus four
low.**

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | `read_signed_approval` does not require `signature_kinds`/`approvers` sorted, while every other record array is `require_sorted` on read. A stored revision with unsorted policy arrays, which the writer never produces, is accepted, and its digest differs from the canonical one for the same policy. Add `require_sorted` for both arrays. The prepare path normalizes first, so writes are unaffected. Add a refused-unsorted case to the FR-063-AC-16 test | rust/crates/quoin-change-assurance/src/model/record/read.rs:338-367 |
| FND-002 | medium | The claim-word census now keeps `_` inside words, which exempts every snake_case token, not only the spec's wire names. Measured on the head: 40 lines fail the strict rule, all of them identifiers, and 0 fail the widened rule, so nothing is hidden today, but any future `verify_signature`-style identifier or `signed_by_x` in prose passes unseen. Keep `_` as a separator and skip only an explicit allowlist of the spec-declared identifiers (`signed_approval`, `signature_kind`, `signature_kinds`, `signed_bad`) | rust/crates/quoin-change-assurance/tests/tc_455_boundary.rs:51-61 |
| FND-003 | medium | The plain `receipt` output labels the claim as unverified and names approval-verify, but it omits what FR-068-AC-13 requires of the output: that trust comes from the relying party, not from the claim's `trust`, the record or a revision it names. It also does not say that a claim under `required: false` is not relied on. Extend the second line and assert it in `tc_1511_a_receipt_labels_the_claimed_approval_as_an_unverified_claim` | rust/crates/quoin-cli/src/change_assurance.rs:510-532 |
| FND-004 | low | The receipt help tells the relying party to re-run with "the claim's `tag`, `record_id` and `record_digest`", but a receipt's `claimed_approval` has no `record_id` member (FR-065 schema). Say "the claim's `tag` and `record_digest` and the receipt's `record_id`" | rust/crates/quoin-cli/tests/fixtures/retained-command-help.json:111-117 |
| FND-005 | low | Stale comments. decision.rs still says the comparison is "the oracle's `=== 1`" now that it compares 2, and the records.rs module doc still says the input is validated "against the v1 schema" | rust/crates/quoin-change-assurance/src/model/decision.rs:305, rust/crates/quoin-change-assurance/src/records.rs:8 |
| FND-006 | low | The new `record-digest --help` fixture entry differs in shape from the other 60 help pages. Its keys are ordered stderr-then-stdout, and its page ends with one newline instead of the blank line every oclif-shaped page carries (only `update --help` shares this). Match the sibling shape | rust/crates/quoin-cli/tests/fixtures/retained-command-help.json:122-130 |
| FND-007 | low | FR-063 requires a repository holding records to mark `records/**` as `-text`, or a CRLF checkout changes every digest and breaks every claim. Neither `write-record` nor `record-digest` help tells the user so. Add one sentence to both pages | rust/crates/quoin-cli/tests/fixtures/retained-command-help.json:122-130, rust/crates/quoin-cli/tests/fixtures/retained-command-help.json:166-172 |

## Examined scope

```yaml
clean: false
scope:
- {id: verify/approval.rs, path: rust/crates/quoin-change-assurance/src/verify/approval.rs, role: examined, excerpt: "judge_approval: status, then outside_policy (required && approved), then record_id/digest/wrong_record, then meaning_missing"}
- {id: model/approval.rs, path: rust/crates/quoin-change-assurance/src/model/approval.rs, role: examined, excerpt: "VerifiedApproval::from_json closed per status row; ClaimedApproval receipt form"}
- {id: model/decision.rs, path: rust/crates/quoin-change-assurance/src/model/decision.rs, role: examined, excerpt: "schema_version 2; verified_approval on non-approved returns None"}
- {id: record/read.rs, path: rust/crates/quoin-change-assurance/src/model/record/read.rs, role: examined, excerpt: "read_signed_approval"}
- {id: records.rs, path: rust/crates/quoin-change-assurance/src/records.rs, role: examined, excerpt: "record_digest(stored_bytes) = sha256: + sha256_hex"}
- {id: intake/mod.rs, path: rust/crates/quoin-change-assurance/src/intake/mod.rs, role: examined, excerpt: "read_change_record_with_digest hashes the bytes it parsed"}
- {id: ops/change_assurance, path: rust/crates/quoin-core/src/ops/change_assurance/mod.rs, role: examined, excerpt: "record_digest op; write_record reports record_digest"}
- {id: cli change_assurance.rs, path: rust/crates/quoin-cli/src/change_assurance.rs, role: examined, excerpt: "record-digest subcommand; receipt_text claimed approval line"}
- {id: schemas v2, path: rust/crates/quoin-change-assurance/schemas, role: examined, excerpt: "byte-identical to FR-063/FR-065 schema blocks"}
- {id: tc_455_boundary words(), path: rust/crates/quoin-change-assurance/tests/tc_455_boundary.rs, role: examined, excerpt: "split keeps '-' and '_'"}
- {id: retained-command-help.json, path: rust/crates/quoin-cli/tests/fixtures/retained-command-help.json, role: examined, excerpt: "help source via include_str! in help.rs"}
- {id: dispatch.rs, path: rust/crates/quoin-core/src/dispatch.rs, role: examined, excerpt: "#[allow(clippy::too_many_lines, reason = ...)]"}
```
