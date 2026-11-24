---
id: SR-17100
title: "Base spec review of the PLAT-1853 customer-artifact retained kind"
type: SpecReview
analysis: base
scope: "agent-ix/quoin; spec/functional/FR-030-evidence-store.md, spec/functional/FR-133-discharge-facts-in-the-evidence-store.md, spec/functional/FR-211-discharge-resolves-evidence-and-approvals.md (PR #1045, git diff origin/main...e0476e21)"
review_set: subset
---

# Base spec review of the PLAT-1853 customer-artifact retained kind

## Summary

Ticket: PLAT-1853. I reviewed the PR #1045 diff (FR-030, FR-133 and FR-211) against
the owner's option (a) decision. The decision says discharge resolves every
`evidenceRefs` entry against retained evidence whose kind is in the catalog
`producers`, only all-`procedure` clauses are customer-owned, and a new retained
kind lets record and document outputs discharge. Every part of the decision has
an AC: AC-36 to AC-41, CON-4 and CON-5 in FR-030, AC-8 in FR-133, and AC-25 to
AC-30 in FR-211. The change adds no content identity or digest. CON-4, AC-39,
FR-133-AC-8 and FR-211-AC-29 forbid one. I found three defects. The new
`delegated` rule names no reason and conflicts with the disposition partition.
The rule for a stored `artifactId` against a lowercased path is ambiguous.
AC-26 contradicts itself.

`quire validate --scope . <the three FRs>` exited 0 with quire 0.52.0 (engine
0.62.2) and quoin 0.36.8.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | medium | FR-211-AC-27 and its Behavior bullet leave a `delegated` disposition on a record or document clause "open" but give no reason code, while every other open state in FR-211 has one. They also do not say whether `record-discharge` refuses that disposition. They contradict FR-133-AC-5 and FR-174, which count a recorded `delegated` disposition under dispositions by decision, not as open. The rule covers only `record` and `document` outputs. It does not state the general rule that `delegated` discharges only a customer-owned clause. | FR-211-AC-27; spec/functional/FR-211-discharge-resolves-evidence-and-approvals.md:137-141; FR-133-AC-5; FR-174 | wrong-requirement |
| FND-002 | medium | FR-030-AC-41 and its Behavior bullet treat a stored file "whose `artifactId` differs from its path" as an error. However, the store keeps `artifactId` as submitted (any case, AC-36) and lowercases only the path. Read literally, every valid mixed-case record is corrupt. The comparison needs to be stated as the folded id against the file stem. | FR-030-AC-41; spec/functional/FR-030-evidence-store.md:130-132; FR-030-AC-36 | wrong-requirement |
| FND-003 | low | FR-211-AC-26 opens with "The same clause" (the AC-25 clause, whose output lists `producers: [customer-artifact]`). It then expects `no-producer` for "a clause whose output catalog entry lists no `producers`", which cannot be that clause. That case is already AC-5 and should be dropped or stated as a separate clause. | FR-211-AC-26; FR-211-AC-5 | wrong-requirement |

## Verdict

**Changes requested.** FND-001 and FND-002 are medium. The high-severity structural defect is in SR-17101
(integrity): FR-211-AC-25 to AC-30 are not minted.

Owner questions:

- **(a) Any customer artifact satisfies any clause that lists the kind.** I accept
  this, and it is not a finding. It matches the existing rule that any retained
  `run` satisfies any output listing `run` (FR-211-AC-1). The direct fact's
  attester vouches for the match. On an approval-class clause, `record_digest`
  covers the fact's refs, so the signed approval covers the choice of artifact.
  Per FR-211 Description and CON-1, people accept risk and the tool assigns no
  acceptability. A stricter match, such as typing each artifact to a catalog
  output, is a speculative feature that no decision bullet asks for. It fails
  the value test.
- **(b) quire-rs FR-073 `producers`.** quire-rs `origin/main` (b6beef4) has
  `producers` as of 80d1f15 (PLAT-1831, quire-rs#714, 2026-11-23). FR-073 says
  `producers` is an open set of kind strings: Quire validates its shape only
  (non-empty, distinct, no blank or padded string). FR-073-AC-10 accepts a kind
  that no list defines (`vendor-experiment`). `src/clauses.rs`
  `validate_producers` has no allowlist. `customer-artifact` is therefore
  admitted today, and no quire-rs spec or schema change is needed. 80d1f15 is
  not yet in any quire-rs tag. The installed quire (engine 0.62.2) predates it,
  so packs can use the member only after a quire-rs release. That is release
  routing, not a defect in this PR. The FR-211 bullet "Until the catalog schema
  carries `producers`" is pre-existing and still true for released engines.
- **(c) Store limits.** The 64-byte ASCII `artifactId` with `case-collision`
  and `invalid-identifier` follows FR-130 (scenario ids) and FR-133 (fact
  ids). The 256-byte limit on `title` and `suppliedBy` and the 2048-byte limit
  on `location` have no FR-130 or FR-133 precedent. Those FRs bound only ids
  and rely on each operation's request ceiling (FR-133-AC-6) for free text.
  The 256 figure matches only the unrelated FR-211 `record_id` limit. The
  limits are cheap, testable, untrusted-input bounds on a committed file and
  do no harm, so I raise no finding. Keeping them or dropping them in favour
  of the request ceiling are both acceptable.

Other checks: all new ids (FR-030-AC-36 to AC-41, FR-030-CON-4 and CON-5,
FR-133-AC-8, FR-211-AC-25 to AC-30) are new. `git log --all -S<id>` finds each
only in e0476e21. The diff has no paywalled standard text and no compatibility
layer, migration or fallback. The `schemaVersion` member follows the store's
existing `STORE_SCHEMA_VERSION` convention. Append-only with an identical-replay
no-op and `RecordIntegrity` on a different document matches FR-030-AC-22 and
FR-093-AC-8. `gc` never collecting the kind is stated (AC-40) and exercised in
discharge (FR-211-AC-30).

## Examined

FR-030 Outputs, the new Customer artifacts bullets, CON-4, CON-5, AC-36 to
AC-41 and the CR note. FR-133 new Behavior bullet and AC-8. FR-211 Evidence
references, the new producers bullet, Customer-owned clauses, AC-25 to AC-30
and Dependencies. Context: FR-211-AC-1, AC-5 and AC-12; FR-133-AC-3, AC-5 and
AC-6; FR-130 identifier rules and AC-10 and AC-11; FR-174; FR-046-AC-7;
quire-rs FR-073 and `src/clauses.rs` on origin/main.

## New findings (disposition pass 1)

Fix commit d64c733b. The lead's ruling makes `delegated` discharge only a customer-owned clause, and `record-discharge` now refuses it on any other clause. Today's code accepts that case: `quoin-assurance/src/discharge/verify.rs` `check_disposition` returns success for a non-approval-class clause that is not customer-owned and verifies the approval for an approval-class one. So this is a behaviour change, and the FR-211 SHALL and AC-27 state it clearly; the code ticket must add the refusal. The ruling introduces the two findings below.

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-004 | medium | The new FR-211 rule now conflicts with FR-173 and FR-174. FR-173 emits a `delegated` disposition for every binding clause whose `scope_exclusion` authors `delegated`, whatever the clause's output kinds. FR-173-AC-12 says that array is accepted by `record-discharge` when its refs resolve and its approvals verify. FR-174-AC-1 records every supported disposition on synthetic clauses of unstated kind. For a `delegated` exclusion on a clause that is not customer-owned, FR-211 now makes `record-discharge` refuse the whole list, so FR-173's own output cannot be recorded. FR-173 must refuse or report such an exclusion (or say why it cannot), and FR-173-AC-12 and FR-174-AC-1 must name a customer-owned clause for `delegated`. | FR-211 Customer-owned clauses; FR-211-AC-27; FR-173 (line 61); FR-173-AC-4; FR-173-AC-12; FR-174-AC-1 | wrong-requirement |
| FND-005 | low | The refusal reuses the reason `customer-owned` for a clause that is not customer-owned, so the code states the opposite of the clause's property. Existing codes name the condition at fault (`no-store`, `approval-missing`, `kind-not-expected`), and no existing code fits. A new code `not-customer-owned` would be accurate. The prose tail in `verify.rs` softens the problem, so no behaviour is wrong. The new FR-211 sentence also gives the reason only for `record-discharge`, not for a report built from `--facts`. | FR-211 Customer-owned clauses; FR-211-AC-27; rust/crates/quoin-assurance/src/discharge/verify.rs:338-347 | wrong-requirement |

## Dispositions

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | d64c733b states once that `delegated` discharges only a customer-owned clause and that `record-discharge` refuses it on any other clause, names a reason, and removes the "open" wording. AC-27 is now an instance of that rule. The cross-FR conflict the rule creates with FR-173 and FR-174 is FND-004. |
| FND-002 | fixed | FR-030 Behavior and AC-41 now compare the lowercased `artifactId` with the file name, and AC-41 states that a mixed-case id in a lowercase file name is valid. |
| FND-003 | fixed | FR-211-AC-26 no longer carries the `no-producer` case, which AC-5 covers. |
