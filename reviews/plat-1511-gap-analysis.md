---
id: SR-20261113012817
title: "Gap analysis of quoin PR 839 (PLAT-1511 FR-063/065/068 amended criteria)"
type: SpecReview
analysis: base
scope: "agent-ix/quoin@3ba037d1; FR-063-AC-1/10/11/14/15/16, FR-065-AC-1, FR-065-AC-12..20, FR-068-AC-11/12/13"
review_set: subset
---

# Gap analysis of quoin PR 839

## Summary

Ticket: PLAT-1511. Planless. Plan completion: not assessed.

The matrix came from `quoin matrix --json`, using the PR-head binary
`rust/target/debug/quoin` (0.31.0-4-g3ba037d1). The installed release
`/home/peter/.npm-global/bin/quoin` (0.30.1) reports only 7 criteria tagged repo-wide, against
661 for the head binary. It does not read the `#[trace(...)]` attribute, so it cannot measure
this repo.

On the head binary, every in-scope criterion has at least one binder. That is 15 `tagged`, and
FR-063-AC-11, FR-065-AC-12, FR-068-AC-11 and FR-068-AC-13 are `method-without-symbol`
(Analysis/Inspection).

Each binder was read against its AC to check that it asserts the AC and can fail. Rule-order
mutants are each caught by a named test:

- swapping status and binding: AC-20
- swapping narrowing and binding: `tc_1511_policy_narrowing_precedes_binding...`
- swapping binding and meaning: the same test
- ignoring `approvers`: the stranger case

Two more changes are each caught. Digesting a re-serialization instead of the stored bytes is
caught by the CRLF-append in `tc_1511_615`. Dropping `trust` from the receipt is caught by AC-13.

Production code has no orphan: `record-digest` is owned by FR-068, and the approval modules by
FR-063 and FR-065.

## Verdict

**PASS WITH FINDINGS.** One medium coverage-inflation binding and one low untested branch.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-068-AC-11 was unbound on origin/main. This PR binds it only to `tc_1511_the_help_pages_describe_the_claim_and_the_digest`, which asserts that some literals are present and never asserts AC-11's absence properties: no attested command executed, no Git or network work, one FR-096 process boundary, and no identity, authorization, non-repudiation or certification claim in output or help. That inflates coverage. Drop the AC-11 tag, or bind AC-11 to a test that asserts those absences over the change-assurance help pages and plain receipt output | rust/crates/quoin-cli/tests/tc_1511_change_assurance_claimed_approval.rs:225 |
| FND-002 | low | FR-063-AC-14 lists "a missing `trust` ... on a non-`approved` status". The test removes `trust` only from an `approved` claim. The out-of-row case it does cover is approver, meaning and kind on `signed_bad`, so a missing `trust` on a non-approved status is untested. Add that case. The AC wording ("missing ... on a non-approved status" where "present" is meant for approver, meaning and signature_kind) is itself ambiguous and should be reworded in the spec | rust/crates/quoin-change-assurance/tests/tc_1511_record_and_payload.rs:82-130 |

## Examined scope

```yaml
clean: false
scope:
- {id: FR-063-AC-1, path: spec/functional/FR-063-change-assurance-record-integrity.md, role: examined, excerpt: "Schema v2 requires ... review-workflow binding with its signed_approval policy; undeclared fields (including any digest member) are refused."}
- {id: FR-063-AC-10, path: spec/functional/FR-063-change-assurance-record-integrity.md, role: examined, excerpt: "... or a payload schema_version other than 2 cannot approve it."}
- {id: FR-063-AC-11, path: spec/functional/FR-063-change-assurance-record-integrity.md, role: examined, excerpt: "Static and golden-text checks prove writing a record runs nothing ... a signer appears only as a retained verified_approval described as a claim"}
- {id: FR-063-AC-14, path: spec/functional/FR-063-change-assurance-record-integrity.md, role: examined, excerpt: "A v2 payload with a well-formed verified_approval on an approved decision is a candidate; each of an undeclared member ... is refused as a candidate."}
- {id: FR-063-AC-15, path: spec/functional/FR-063-change-assurance-record-integrity.md, role: examined, excerpt: "The record digest of a stored revision equals sha256: plus the lowercase SHA-256 hex of its stored bytes ..."}
- {id: FR-063-AC-16, path: spec/functional/FR-063-change-assurance-record-integrity.md, role: examined, excerpt: "A signed_approval with required true and no signature_kinds ... is refused when the record is written, and the published schema refuses the same"}
- {id: FR-065-AC-1, path: spec/functional/FR-065-change-assurance-verification.md, role: examined, excerpt: "Receipt schema v1 records the exact record/candidate ... undeclared fields are refused."}
- {id: FR-065-AC-12, path: spec/functional/FR-065-change-assurance-verification.md, role: examined, excerpt: "Golden terminology describes actor labels only as recorded attribution ... and claimed_approval only as a claim to be re-verified"}
- {id: FR-065-AC-13, path: spec/functional/FR-065-change-assurance-verification.md, role: examined, excerpt: "A matching approved event whose verified_approval is approved and carries the record's id and the digest ... keeps the review check valid"}
- {id: FR-065-AC-14, path: spec/functional/FR-065-change-assurance-verification.md, role: examined, excerpt: "wrong_record, or approved with a digest that is not the digest of the stored revision ... invalid with approval_record_mismatch"}
- {id: FR-065-AC-15, path: spec/functional/FR-065-change-assurance-verification.md, role: examined, excerpt: "untrusted_signer ... signed_bad ... signature_kind outside signature_kinds, or an approver outside a non-empty approvers"}
- {id: FR-065-AC-16, path: spec/functional/FR-065-change-assurance-verification.md, role: examined, excerpt: "meaning_missing ... unsigned or unverifiable ... invalid still dominates"}
- {id: FR-065-AC-17, path: spec/functional/FR-065-change-assurance-verification.md, role: examined, excerpt: "Two matching approved events that each carry a verified_approval ... decision_mismatch"}
- {id: FR-065-AC-18, path: spec/functional/FR-065-change-assurance-verification.md, role: examined, excerpt: "no verified_approval under required false yields claimed_approval null ... version 1 payload or receipt is not accepted; static boundaries"}
- {id: FR-065-AC-19, path: spec/functional/FR-065-change-assurance-verification.md, role: examined, excerpt: "no verified_approval under required true ... incomplete with approval_missing"}
- {id: FR-065-AC-20, path: spec/functional/FR-065-change-assurance-verification.md, role: examined, excerpt: "signed_bad claim also carries a digest that differs ... approval_signature_bad and not approval_record_mismatch"}
- {id: FR-068-AC-11, path: spec/functional/FR-068-change-assurance-cli-surface.md, role: examined, excerpt: "Static boundaries prove no command executes an attested command or performs Git or network work ..."}
- {id: FR-068-AC-12, path: spec/functional/FR-068-change-assurance-cli-surface.md, role: examined, excerpt: "write-record and record-digest report the same digest ... record-digest of an unstored revision fails with exit 2 and writes nothing."}
- {id: FR-068-AC-13, path: spec/functional/FR-068-change-assurance-cli-surface.md, role: examined, excerpt: "The receipt output and help text name a non-null claimed_approval as a claim not verified by Quoin ..."}
```
