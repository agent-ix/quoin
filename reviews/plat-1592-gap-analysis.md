---
id: SR-20261113041803
title: "Gap analysis of quoin PR 842 (PLAT-1592 amended criteria)"
type: SpecReview
analysis: base
scope: "agent-ix/quoin@6df276b1; FR-063-AC-14, FR-063-AC-17, FR-065-AC-13, FR-065-AC-21, FR-065-AC-22, FR-068-AC-14"
review_set: subset
---

# Gap analysis of quoin PR 842

## Summary

Ticket: PLAT-1592. Planless. Plan completion: not assessed.

The matrix comes from `quoin matrix --json` run with the PR-head binary
`rust/target/debug/quoin` (0.31.0-8-g6df276b1). The installed quoin 0.30.1 cannot read
`#[trace]` attributes.

Every in-scope criterion is `tagged` and has at least one binder:

- FR-063-AC-17 and FR-063-AC-14 are both bound by
  `tc_1592_an_approved_claim_carries_exactly_one_time_form`.
- FR-065-AC-13 and FR-065-AC-21 are bound by
  `tc_1592_the_claims_time_reaches_the_receipt...` and
  `tc_1511_the_receipt_schema_and_reader_agree...`.
- FR-065-AC-22 is bound by `tc_1592_the_receipt_shows_person_time_and_meaning_together`.
- FR-068-AC-14 is bound by that test and by `tc_1592_the_receipt_help_describes_the_claimed_time`.

All binders pass at HEAD. Each binder was read against its criterion and can fail. Three
mutations are each caught by a named test:

- forcing `members_for` to the instant form: the missing-form case
- `is_utc_instant` always true: the offset and month-13 cases
- `TimeSource::parse` accepting anything: the `clock` case

Production code has no orphan: `approval_time.rs` is owned by FR-063 and FR-065, and
`claimed_time_line` by FR-068.

The MP-250/MP-251 populations are unchanged because no flag or argument was added;
`tc_3003_nfr_populations` passes.

## Verdict

**PASS WITH FINDINGS.** There are three low coverage findings. FR-068-AC-14's static clause,
which no binder asserts, is a spec-review finding (compound AC), so it is not repeated here.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | Only month 13 and hour 24 test the range bounds of `is_utc_instant`. Removing `second < 60`, `minute < 60` or the day range survives every test, so a leap second `23:59:60Z`, minute 60 and day `00`/`32` are not exercised in the reader or the schema. Fix: add these cases to `tc_1592_an_approved_claim_carries_exactly_one_time_form` and to the `tc_503` forged list | rust/crates/quoin-change-assurance/tests/tc_1511_record_and_payload.rs:528-535; rust/crates/quoin-change-assurance/tests/tc_503_schema_assets.rs:639-647 |
| FND-002 | low | FR-065-AC-21 refuses "an undeclared time member". The receipt reader `ClaimedApproval::from_json` is a separate code path from the event reader, and the `tc_503` forged list has no undeclared time member (for example `signed_at_zone`) and no missing form beside half of the instant form, so neither case is tested on the receipt side. Fix: add both to the forged list | rust/crates/quoin-change-assurance/tests/tc_503_schema_assets.rs:411-436 |
| FND-003 | low | `tc_1592_the_receipt_shows_person_time_and_meaning_together` proves "no other instant" with `!missing.contains("2026-")`. That check is tied to the year: a leak of the verification time from 2027 onward passes it. Fix: assert that the output has no `YYYY-MM-DDTHH:MM:SS`-shaped substring | rust/crates/quoin-cli/tests/tc_1511_change_assurance_claimed_approval.rs:419 |

## Examined scope

```yaml
clean: false
scope:
- {id: FR-063-AC-14, path: spec/functional/FR-063-change-assurance-record-integrity.md, role: examined, excerpt: "A v2 payload with a well-formed verified_approval on an approved decision is a candidate; each of an undeclared member ... is refused as a candidate."}
- {id: FR-063-AC-17, path: spec/functional/FR-063-change-assurance-record-integrity.md, role: examined, excerpt: "A well-formed approved verified_approval carries exactly one of signed_at with signed_at_source, or signed_at_missing ... any time member on a status other than approved make the event a refused candidate."}
- {id: FR-065-AC-13, path: spec/functional/FR-065-change-assurance-verification.md, role: examined, excerpt: "... the signature kind and the claim's time exactly as the claim states it."}
- {id: FR-065-AC-21, path: spec/functional/FR-065-change-assurance-verification.md, role: examined, excerpt: "The receipt schema and reader require a non-null claimed_approval to carry exactly one of ... or an undeclared time member is refused"}
- {id: FR-065-AC-22, path: spec/functional/FR-065-change-assurance-verification.md, role: examined, excerpt: "The plain receipt output shows ... a missing time shows time missing with its reason and no other instant"}
- {id: FR-068-AC-14, path: spec/functional/FR-068-change-assurance-cli-surface.md, role: examined, excerpt: "The receipt output for a non-null claimed_approval shows the approver, the time ... and no Git, network or identity operation is added."}
```
