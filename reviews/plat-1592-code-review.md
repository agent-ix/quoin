---
id: SR-20261113041802
title: "Code review of quoin PR 842 (PLAT-1592 claimed signing time)"
type: SpecReview
analysis: base
scope: "agent-ix/quoin@6df276b1; rust/crates/quoin-change-assurance/src/model/approval.rs, rust/crates/quoin-change-assurance/src/model/approval_time.rs, rust/crates/quoin-change-assurance/schemas/verification-receipt-v2.schema.json, rust/crates/quoin-cli/src/change_assurance.rs, rust/crates/quoin-cli/tests/fixtures/retained-command-help.json, tests touched by the PR"
review_set: subset
---

# Code review of quoin PR 842

## Summary

Ticket: PLAT-1592 (the quoin half; the Engineering Assurance half, PR 218, is merged).
This review includes the rust-review lane, applied with quoin's own
`.claude/skills/rust-style/SKILL.md`, which outranks the ecosystem skill in this repository.
Scope is `git diff origin/main...HEAD` at 6df276b1.

Engineering Assurance FR-041 was read from `origin/main` (45ff0b8 plus 30ec8bf), together
with `src/approval_verify.rs`. EA's `SignedAtSource` enum has exactly one variant,
`TaggerDate`.

Checks run:

- The focused tests were run through `locked-build.sh` (debug=0, jobs 4) and all pass:
  `tc_1511_claimed_approval` 11, `tc_1511_record_and_payload` 10, `tc_455_boundary` 7,
  `tc_503_schema_assets` 7, `tc_1511_change_assurance_claimed_approval` 6,
  `tc_retained_help_capture` 6, `tc_457_change_assurance_boundary` 13 and
  `tc_3003_nfr_populations` 13.
- Clippy `-D warnings` over `quoin-change-assurance` and `quoin-cli` (all targets, all
  features) is clean, and so is `cargo fmt --check`.
- The schema asset is byte-identical to the FR-065 JSON block. There is no new dependency
  and no `Cargo.toml` change.

## Verdict

**Changes requested.** There is one medium finding and three low findings. The rest is sound:

- Field names, the exactly-one rule, approved-only and closed objects match FR-041. A time on
  a non-`approved` status fails `exact(&base)`. Both forms, neither form, half of the instant
  form, a lowercase `t`/`z`, an offset, fractional seconds, `:60` and hour 24 are all refused.
- The renderer never puts another instant in place of a missing time. Its fallback prints
  `time missing`, and the receipt carries no clock-derived member.
- The wording stays a claim: the block is headed "the event writer's claim and not verified
  by Quoin", and the time is "claimed by the signer".
- The `signed_at` allowlist entry is minimal. It strips only the underscore identifier. A
  prose "signed at" is still split into `signed` and still censused.
- No git, network or identity operation is added.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | `signed_at_source: transparency_log` is accepted by the reader (`TimeSource::TransparencyLog`), the receipt schema enum and the tests, but EA FR-041 says it "is reported only after a gitsign release documents how it exposes the integrated time and this requirement is amended", and EA's `SignedAtSource` has only `TaggerDate`, so no producer can emit it. Accepting it pre-commits quoin to a shape and to wording that would be wrong for it: every rendered time reads "signing time claimed by the signer", and FR-063/FR-065 call every time "the signer's own claim", which a log-integrated time is not. Fix: remove the variant and the enum value from schema, FR-065 block and FR-063 row; make `transparency_log` a refused-source case in the tests; add it when EA amends FR-041 | rust/crates/quoin-change-assurance/src/model/approval_time.rs:26-47; rust/crates/quoin-change-assurance/schemas/verification-receipt-v2.schema.json:214-217 |
| FND-002 | low | `is_utc_instant` is documented as checking "every field in its calendar range", but it accepts day 31 for every month, so `2026-02-31T00:00:00Z` and `2026-04-31T00:00:00Z` are accepted as a "UTC instant". The schema pattern does the same. Fix: reword the doc comment (and state in FR-063 that the form check does not check days per month), or check days per month with leap years in the reader and say so in the spec | rust/crates/quoin-change-assurance/src/model/approval_time.rs:156-197 |
| FND-003 | low | `ClaimedTime::Reported { instant: String }` is a public variant with a bare `String`, so any caller can build a `ClaimedApproval` whose instant never passed `is_utc_instant`, and `to_json` writes it out. The crate validates its other wire strings with newtypes (`NonEmptyText`, `Identity`). Fix: a `SignedAt` newtype with a private field and a parsing constructor | rust/crates/quoin-change-assurance/src/model/approval_time.rs:79-91 |
| FND-004 | low | `ClaimedTime::to_json` is documented "for tests and callers that want the form alone" but has no caller in `src/` or `tests/`. Fix: delete it | rust/crates/quoin-change-assurance/src/model/approval_time.rs:147-151 |

## Examined scope

```yaml
clean: false
scope:
- {id: approval_time.rs, path: rust/crates/quoin-change-assurance/src/model/approval_time.rs, role: examined, excerpt: "ClaimedTime::{members_for, read, members, to_json}; TimeSource; TimeMissing; is_utc_instant"}
- {id: approval.rs, path: rust/crates/quoin-change-assurance/src/model/approval.rs, role: examined, excerpt: "allowed.extend(ClaimedTime::members_for(fields)); time: ClaimedTime::read(fields, subject)?"}
- {id: verification-receipt-v2.schema.json, path: rust/crates/quoin-change-assurance/schemas/verification-receipt-v2.schema.json, role: examined, excerpt: "signed_at pattern; signed_at_source enum [tagger_date, transparency_log]; signed_at_missing enum; oneOf"}
- {id: change_assurance.rs, path: rust/crates/quoin-cli/src/change_assurance.rs, role: examined, excerpt: "receipt_text claimed approval block; claimed_time_line"}
- {id: retained-command-help.json, path: rust/crates/quoin-cli/tests/fixtures/retained-command-help.json, role: examined, excerpt: "receipt --help: same approver, meaning, signature kind and signing time ... never replaced by another time, the time of verification included."}
- {id: tc_455_boundary.rs, path: rust/crates/quoin-change-assurance/tests/tc_455_boundary.rs, role: examined, excerpt: "WIRE_IDENTIFIERS adds signed_at"}
- {id: EA FR-041, path: engineering-assurance origin/main spec/functional/FR-041-signed-tag-approval-of-a-change-assurance-record.md, role: context_only, excerpt: "The source value transparency_log is reserved: it is reported only after a gitsign release documents how it exposes the integrated time and this requirement is amended to parse it."}
```
