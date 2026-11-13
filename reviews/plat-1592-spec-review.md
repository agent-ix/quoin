---
id: SR-20261113041804
title: "Spec review of quoin PR 842 (PLAT-1592 FR-063/065/068/US-017 amendments)"
type: SpecReview
analysis: base
scope: "agent-ix/quoin@6df276b1; spec/functional/FR-063-change-assurance-record-integrity.md, spec/functional/FR-065-change-assurance-verification.md, spec/functional/FR-068-change-assurance-cli-surface.md, spec/usecase/US-017-verify-change-assurance-evidence.md"
review_set: subset
---

# Spec review of quoin PR 842

## Summary

Ticket: PLAT-1592. The amended text was checked against Engineering Assurance FR-041 on
`origin/main`. FR-041 defines:

- `signed_at` as `YYYY-MM-DDTHH:MM:SSZ`
- `signed_at_source` as `tagger_date`, with `transparency_log` reserved until an amendment
- `signed_at_missing` as `tagger_date_unreadable`
- exactly one form on `approved`, and none on any other status
- the time as the signer's claim, with no plausibility check

The review covered integrity, EARS, testability and the boundary scope.

The in-place amendment of closed v2 is sound and stated plainly. FR-063 and FR-065 both say
that the version stays 2 and that no reader for an untimed claim is kept. Nothing retained in
the wild is affected:

- No quoin tag contains PLAT-1511 (#839).
- No Engineering Assurance tag contains #217 without #218.

So no released tool ever wrote an untimed `approved` claim or receipt.

Requiring the relying party's re-run to match the time is sound:

- FR-041 reads the time from the tagger line of the signed tag object, so it is stable across
  re-runs whatever the git version, the trust and the verification time.
- It differs only when the tag object differs, and then not relying is correct.
- It binds the time against a writer forging it in the event.

## Verdict

**Changes requested.** There are two medium findings and two low findings. US-017 is clean.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The reliance rule says the re-run must yield "the same approver, meaning, signature kind and signing time", but "the same signing time" is not defined. Two readers differ on whether `signed_at_source` must match, whether two `signed_at_missing` match, and what a later Engineering Assurance reporting another source for the same tag means. Fix: define it as the same form, meaning equal `signed_at` and equal `signed_at_source`, or `signed_at_missing` with the same reason on both. State that any other difference means the claim is not relied on, and carry the same definition into the receipt help | spec/functional/FR-068-change-assurance-cli-surface.md:110-113 |
| FND-002 | medium | FR-063's `signed_at_source` row and FR-065 "The time" accept `transparency_log`. EA FR-041 says that value is "reported only after ... this requirement is amended", and no producer emits it. FR-063 calls every time "the signer's own claim", and FR-065 requires every reported time to be "labelled as claimed by the signer". Both statements would be false for a log-integrated time. Fix: accept only `tagger_date` until FR-041 is amended, then amend these rows together with the source's own wording | spec/functional/FR-063-change-assurance-record-integrity.md:388 |
| FND-003 | low | FR-068-AC-14 is compound. It bundles the rendering of the output (Integration), the help text (Inspection) and "no Git, network or identity operation is added". That last clause is a static boundary FR-068-AC-11 already owns, and neither of AC-14's binders asserts it. Fix: drop the clause from AC-14 | spec/functional/FR-068-change-assurance-cli-surface.md:156 |
| FND-004 | low | FR-065-AC-22 and FR-068-AC-14 declare `Integration`. `quoin matrix` (HEAD binary) reports that as `unknown-method`, which is neither a catalog method id nor a catalog class. The pattern already exists on 19 FR-065 ACs, and the new criteria extend it. Fix: declare `Test` for the new criteria | spec/functional/FR-065-change-assurance-verification.md:493 |

## Examined scope

```yaml
clean: false
scope:
- {id: FR-063-AC-17, path: spec/functional/FR-063-change-assurance-record-integrity.md, role: examined, excerpt: "A well-formed approved verified_approval carries exactly one of signed_at with signed_at_source, or signed_at_missing, and each is retained and reported unchanged; ..."}
- {id: FR-063, path: spec/functional/FR-063-change-assurance-record-integrity.md, role: examined, excerpt: "signed_at_source | tagger_date or transparency_log (the latter reserved by Engineering Assurance FR-041) ... It is the signer's own claim"}
- {id: FR-065-AC-13, path: spec/functional/FR-065-change-assurance-verification.md, role: examined, excerpt: "... the signature kind and the claim's time exactly as the claim states it."}
- {id: FR-065-AC-21, path: spec/functional/FR-065-change-assurance-verification.md, role: examined, excerpt: "The receipt schema and reader require a non-null claimed_approval to carry exactly one of ..."}
- {id: FR-065-AC-22, path: spec/functional/FR-065-change-assurance-verification.md, role: examined, excerpt: "The plain receipt output shows ... the receipt's own verification not being a time source."}
- {id: FR-068-AC-14, path: spec/functional/FR-068-change-assurance-cli-surface.md, role: examined, excerpt: "... the help names the time as the signer's own claim, never replaced by another time; and no Git, network or identity operation is added."}
- {id: FR-068, path: spec/functional/FR-068-change-assurance-cli-surface.md, role: examined, excerpt: "the claim is relied on only when the re-run yields approved with the same approver, meaning, signature kind and signing time"}
- {id: US-017, path: spec/usecase/US-017-verify-change-assurance-evidence.md, role: examined, excerpt: "The claim shows the approver, the time the signer claims and the meaning together; a missing time is shown as missing and never replaced by another time."}
```
