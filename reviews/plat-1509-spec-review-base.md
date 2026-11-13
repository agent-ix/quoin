---
id: SR-201
title: "Base spec review of quoin PR 836 (signed approvals in change assurance)"
type: SpecReview
analysis: base
scope: "agent-ix/quoin@33a5d210; spec/functional/FR-063-change-assurance-record-integrity.md, spec/functional/FR-065-change-assurance-verification.md, spec/functional/FR-068-change-assurance-cli-surface.md, spec/usecase/US-017-verify-change-assurance-evidence.md"
review_set: subset
---

# Base spec review of quoin PR 836

## Summary

Ticket: PLAT-1509. Scoped to `git diff origin/main...HEAD` at 33a5d210 and reviewed
together with agent-ix/engineering-assurance PR 214 (FR-041). Tools: quoin 0.30.1, quire
0.44.0. `quire validate` passes on the four changed files; its only warnings (FR-068 lines
79, 105, 108) are on lines this PR does not change. Assurance profiles AP-201 (finding
quality) and AP-202 (Jev lens evaluation) are active but govern measurement and Jev
work, not change assurance, so neither applies.

Cross-repo shape: the seven status names, the `tag`/`record_id`/`record_digest` bounds
(256 bytes), when `approver`/`meaning`/`signature_kind` are present, and empty-string
meaning values (FR-041-AC-5) all agree with FR-041. Each of the seven statuses maps to
exactly one FR-065 rule. schema_version 2 for the payload and the receipt, with no
compatibility reader, is intended (prerelease). The record digest (sha256 of the stored
bytes of the deterministic RFC 8785 serialization) is stable across writes. It is the
legitimate kind of digest, a canonical identity digest that binds a signature to the
exact content approved, not tracking ceremony. The FR-063 "no hash is involved" wording
is correctly narrowed to "no digest is stored in the record".

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | medium | FR-063 says `verified_approval` carries "the retained result of `approval-verify` and nothing else", yet its closed table omits FR-041's `protocol`, `signature` and `binding`. The real EA result would be refused as undeclared. Fix: call it a projection, list the mapping, and keep `protocol` (`engineering-assurance.approval-verify-result/v1`) so a shape change in EA is detected. | FR-063 ix-flow decision contract | wrong-requirement |
| FND-002 | medium | FR-098 says "No digest is computed or compared anywhere in the port", which contradicts the sha256 record digest that quoin-core now computes and compares (FR-063, FR-065 rule 1). Fix: amend FR-098 to except the FR-063 approval-binding record digest. | spec/functional/FR-098-semantic-and-identity-parity.md:17; FR-065 Signed approval | wrong-requirement |
| FND-003 | medium | The receipt schema now has `schema_version` const 2, but its `$id` (`verification-receipt-v1.schema.json`), `title` (`VerificationReceiptV1`) and the FR-068 asset name still say v1. Its `$comment` still says verify_receipt re-verifies pre-PLAT-1015 receipts, a compatibility reader that contradicts "version 1 receipts are not read". Fix: rename to v2 and delete that `$comment` clause. | FR-065 Schema; FR-068-AC-8 | wrong-requirement |
| FND-004 | low | FR-065 checks record mismatch (rule 1) before the signature statuses, which is the reverse of FR-041's signature > binding order. A `signed_bad` or `unsigned` claim with a stale digest therefore reports `approval_record_mismatch`. Fix: state why, or align the order with FR-041. | FR-065 Signed approval rules 1-5 | wrong-requirement |
| FND-005 | low | The digest covers the "exact stored bytes", but FR-063 fixes neither a trailing newline nor end-of-line handling. A checkout with eol conversion changes the bytes, and every approval then reads `approval_record_mismatch`. Fix: fix the stored bytes exactly (RFC 8785 output, newline rule) and mark `records/**` `-text`. | FR-063 record digest paragraph; FR-063-AC-15 | missing-requirement |
| FND-006 | low | No command shows an approver the digest to write into `Approved-Record-Digest`; FR-068 is unchanged. Fix: have FR-068 report the record digest (for example from the record or receipt command), or document the exact `sha256sum` procedure. | FR-068; FR-063 record digest paragraph | missing-requirement |

## Verdict

Not mergeable: the high finding in the failure-domain review (SR-203 FND-001) and the
medium findings here need fixing in this PR.
