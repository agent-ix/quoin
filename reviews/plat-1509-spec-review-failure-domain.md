---
id: SR-203
title: "Failure-domain review of quoin PR 836 (trust model of verified_approval)"
type: SpecReview
analysis: failure-domain
scope: "agent-ix/quoin@33a5d210; spec/functional/FR-063-change-assurance-record-integrity.md, spec/functional/FR-065-change-assurance-verification.md, spec/usecase/US-017-verify-change-assurance-evidence.md"
review_set: subset
---

# Failure-domain review of quoin PR 836

## Summary

Ticket: PLAT-1509. Examined whether a forged decision event can make quoin report a
signed approval no one made. It can. Any party that can append to the run's ix-flow log
writes the `verified_approval` payload, and ix-flow FR-013's chain is an unkeyed SHA-256
over the event content, so it can be recomputed. Take a hand-written `verified_approval`
with status `approved`, approver `alice@example.test`, any tag name, the record id, and
a `record_digest` that anyone can compute with sha256 over the stored file. It passes
every FR-065 rule, and the receipt reports `approval` with that approver. The tag need
not exist. Nothing in either PR names who runs `approval-verify` or how its result
reaches the payload, and quoin cannot check the signature (no git, by design). The claim
is therefore attribution again. M3's purpose, approvals bound to signed git objects, is
met by EA FR-041 but not carried through to quoin's receipt.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | high | The spec says the retained `verified_approval` is "Engineering Assurance's verified result" (FR-063), and that "that result ... is the evidence that the named approver signed" (FR-065). Nothing binds the payload to an EA run, though: the event writer supplies it, the ix-flow chain is unkeyed, and quoin cannot re-verify. A forged payload therefore yields a valid receipt that names an approver who signed nothing. Fix: (a) state that `verified_approval` is the event writer's claim, with the same standing as the actor label, until it is re-verified; (b) retain what re-verification needs (tag, record_id, record_digest, FR-041 `protocol`, and the trust input, the allowed-signers path or the gitsign identity and issuer); (c) require any consumer that relies on `approval` to re-run EA `approval-verify` from those members (an EA requirement or an FR-068 step); (d) rename or label the receipt member as a claimed, re-verifiable approval. | FR-063 ix-flow decision contract; FR-065 Signed approval; FR-065 receipt `approval` paragraph; FR-063-CON-3; US-017 | wrong-requirement |
| FND-002 | medium | `verified_approval` is optional and no policy can require it. Omitting it gives `approval` null and a valid review check, so every `approval_*` invalid outcome binds only writers who volunteer a claim, and no record can ever demand a signed approval. Fix: add a closed member to the reviewed record's `review_workflow` (covered by the digest), for example `signed_approval: {required, allowed_signers_path}`. When it is required and absent, the receipt is `incomplete` with a new reason `approval_missing`. A retained trust input that differs from the declared one is `approval_signer_untrusted`. | FR-065 Signed approval; FR-063 review_workflow | missing-requirement |
