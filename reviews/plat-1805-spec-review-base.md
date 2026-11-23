---
id: SR-17092
title: "base review of FR-211 discharge evidence resolution and approval verification"
type: SpecReview
analysis: base
scope: "agent-ix/quoin; spec/functional/FR-211-discharge-resolves-evidence-and-approvals.md, spec/functional/FR-046-clause-discharge.md, spec/functional/FR-208-module-specific-discharge-approval.md, spec/functional/index.md, spec/spec.md"
review_set: subset
---

## Summary

Ticket: PLAT-1805. PR agent-ix/quoin#1026 at 559743f. Base checklist review of new
FR-211 and the amended FR-046 Constraints and FR-208 bullets/AC-2/AC-5, checked
against the real code: `quoin-assurance/src/discharge/{parse,store,report}.rs`,
`quoin-evidence/src/{paths.rs,assurance_records.rs,store/gc.rs}`, quire-rs
`src/clauses.rs` (origin/main ba503da), and engineering-assurance FR-041 and the
`approval-verify`/`approval-tag` commands on its origin/main. The ref grammar and
retained-kind rule match the store layout and the `approval-verify` statuses match
EA FR-041, but two gaps let an unverified approval be reported as verified, and the
approver has no way to produce a tag whose digest matches.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | high | "Admitted" is not tied to an entry point: `quoin discharge --facts` (FR-046-AC-6), report documents read off stdin, and fact files committed straight into `discharge/` all reach classification without approval-verify or ref resolution, so an approval-class clause is discharged by an unverified non-blank `approvalRef` (FR-208 bullet 1 only checks blankness), contradicting FR-211-AC-9 | FR-211, FR-211-AC-9, FR-046-AC-6, FR-208-AC-1 | missing-requirement |
| FND-002 | high | The report must show the "verified approver" (FR-211-AC-4, FR-208-AC-5), but the store keeps no verification result and a read never re-verifies; for a disposition fact the approver is stored nowhere, and for a direct fact a hand-committed file's `attestedBy` would be labelled verified | FR-211-AC-4, FR-208-AC-5, FR-133 | wrong-requirement |
| FND-003 | medium | `record_digest` is "SHA-256 of the canonical JSON of the fact" without naming the canonicalization (RFC 8785 via FR-098 vs the boundary `canonical_json`), and no command gives the approver the `record_id`/`record_digest` to put in the tag (EA `approval-tag` takes them as input), so the approval workflow cannot be completed and two implementations would disagree | FR-211, FR-211-AC-4, FR-098 | missing-requirement |
| FND-004 | medium | A direct fact with no `approvalRef` on an approval-class clause: FR-208-AC-1 stores it and leaves the clause open, while FR-211 says admission invokes approval-verify with `tag` = `approvalRef`; whether recording such a fact is refused, stored-open, or calls EA is undefined | FR-211, FR-208-AC-1 | missing-requirement |
| FND-005 | medium | `quoin evidence gc` (FR-030-AC-8/AC-19) deletes non-latest runs, scans and their analysis-report projections; a fact admitted citing one keeps a dangling `evidence://` ref that is never re-resolved, so "retained evidence" stops being true with no open reason | FR-211, FR-030-AC-8 | missing-requirement |
| FND-006 | medium | EA invocation contract gaps: the request's `repository` member is unnamed; every EA refusal maps to the FR-188 unavailable status although FR-188 maps `allowed_signers_invalid` to usage exit 2; `record_id` from four ids of up to 64 bytes reaches 259 bytes, over EA's 256-byte limit, so a valid fact is reported as "unavailable" | FR-211, FR-188, EA FR-041 | wrong-requirement |
| FND-007 | medium | `producers` is not a member of quire-rs `ExpectedOutput {kind, description}`, which has no `deny_unknown_fields` and drops it silently; FR-211 declares no relationship to the quire-rs requirement that must add it (FR-073), and until it lands every direct fact on a clause with non-empty `expectedOutputs` is refused, so FR-211-AC-1 cannot pass | FR-211, FR-211-AC-1, quire-rs FR-073 | missing-requirement |
| FND-008 | low | `scenario_path` lowercases the scenario file name; the grammar does not say whether `scenarios/<integration>/<scenario>` is matched case-folded, so an uppercase ref is either unresolved or aliases | FR-211 | missing-requirement |
| FND-009 | low | "(FR-030, FR-133 lookup)" cites a lookup FR-133 does not define; FR-133 covers discharge facts only | FR-211 | wrong-requirement |
| FND-010 | low | FR-211-AC-9 names GDPR and AI-Act pack clauses in a generic engine criterion, restates AC-4..AC-6, and couples a quoin test to pack content | FR-211-AC-9 | wrong-requirement |

## Verdict

FAIL. Not mergeable until FND-001 and FND-002 are resolved; FND-003 to FND-007 should
be fixed in the same round.

Clean (examined, no finding): the `evidence://` family list matches `quoin-evidence`
`paths.rs` (`runs`, `scans`, `mock-inspections`, `analysis-reports` as
`<family>/<SUITE-N>/<stem>`; `experiments`/`operational` as `<family>/<uuid>`;
`scenarios/<integration>/<scenario>`); `subject.kind` exists on assurance records;
FR-211-AC-5's status list is exactly EA FR-041's non-approved statuses; the exact-one
`meaning` rule and the `attestedBy` == approver rule are testable against EA output;
FR-211-CON-1/CON-2 and FR-211-AC-10 keep risk acceptance with people and add no
signature code (consistent with FR-188-CON-1 and FR-208-CON-2); GDPR and the AI Act
are named, never quoted, and are freely published law, so no paywalled text is
reproduced; `record_digest` is a canonical identity digest binding a signature to the
exact fact content and is never stored, so it is not a tracking antipattern.
Version note: `quoin --version` resolved 0.36.8 and `quire` 0.52.0 (engine 0.62.2).

## New findings (disposition pass 1)

Re-checked against fix commit f64ad1d.

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-011 | high | FR-211 requires `quoin discharge --facts` to resolve refs against the evidence store and verify approvals with `--repo` (FR-211 Inputs, FR-211-AC-13), but FR-133 says `--repo` reads stored facts "instead of `--facts`" and the CLI refuses "--facts and --repo cannot be combined"; so on the `--facts` path there is no store and no repository, and no direct fact or approval-class clause can ever discharge | FR-211-AC-13, FR-211, FR-133, FR-046-AC-6 | wrong-requirement |
| FND-012 | medium | Re-running both checks over a report given on stdin (`render_discharge`, `build_authored_argument`) needs the evidence store, `--repo`, the engineering-assurance flags and the installed module set (for `producers`, `approvalRoles` and customer-owned clauses); none of these is an input of those operations, and FR-047 (which says the discharge report "already arrives" as input) is not amended | FR-211, FR-211-AC-13, FR-047 | missing-requirement |
| FND-013 | medium | The dossier RMM is built from `quoin discharge --repo --rmm` (FR-195), which now calls engineering-assurance on every build. FR-191/FR-195 pass no engineering-assurance flags, so every approval-class clause renders `approval-verifier-unavailable`. With the flags, gitsign verification uses Sigstore trust services (EA FR-041-AC-26), against NFR-040's "SHALL NOT read the wall clock or the network". The RMM compliance value for the new reasons (`unresolved`, `kind-not-expected`, `no-producer`, `approval-*`) is not stated (FR-195-AC-5) | FR-211, FR-195, NFR-040, FR-195-AC-5 | missing-requirement |
| FND-014 | low | `record-id-too-long` covers the length limit, but a clause-set authority or id with a leading `-` or a control character still yields EA `approval_verify_request_invalid`, which FR-211 maps to exit 3 / `approval-verifier-unavailable` | FR-211, FR-211-AC-10 | missing-requirement |
| FND-015 | low | When some of a clause's `expectedOutputs` entries carry `producers` and others do not, a ref matching none of them could be reported as `no-producer` or as `kind-not-expected`; the "Until the catalog schema carries `producers`" bullet is transitional prose rather than a rule about an entry that lacks `producers` | FR-211, FR-211-AC-3, FR-211-AC-5 | wrong-requirement |

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | FR-211 "When resolution and verification run" names every report path, and AC-13 tests each; the store-path conflict on `--facts` is recorded separately as FND-011 |
| FND-002 | fixed | Verification runs on every report build; the report shows only the current verification's approver and never labels a stored `attestedBy` verified (AC-16) |
| FND-003 | fixed | RFC 8785 via FR-098, lowercase hex, and the `quoin evidence approval-subject` command (AC-17, AC-18) |
| FND-004 | fixed | A missing or blank `approvalRef` gives `approval-missing` with no engineering-assurance call (AC-9) |
| FND-005 | fixed | Refs are re-resolved on every report build, and a ref removed by gc leaves the clause open (AC-14) |
| FND-006 | fixed | `repository` is `--repo`, refusals map as in FR-188, and `record-id-too-long` is checked before the call (AC-10, AC-11); the residual cause is recorded as FND-014 |
| FND-007 | fixed | depends_on quire-rs FR-073 added, and a missing `producers` member gives `no-producer` (AC-5) |
| FND-008 | fixed | Refs are compared in the store's folded form (AC-4) |
| FND-009 | fixed | The FR-133 lookup citation is replaced by the quoin-evidence store reader |
| FND-010 | fixed | The GDPR/AI-Act criterion is removed |
| FND-011 | fixed | Round 2 (aaae0c3): FR-133 allows `--repo` beside `--facts` as the store, and FR-211-AC-21 tests it |
| FND-012 | fixed | Round 2: FR-047 Inputs and FR-211 name the store, the engineering-assurance flags and the module set for report-document operations, with FR-211-AC-22 |
| FND-013 | still-open | Round 2: the network and NFR-040 conflict is gone, but the `--rmm` compliance value of a row with an FR-211 reason (`unresolved`, `kind-not-expected`, `no-producer`, `approval-*`, `not-verified-offline`) is still unstated, while FR-195's gap rule keys on compliance values; the offline rule introduced FND-016 |
| FND-014 | fixed | Round 2: `invalid-identifier` is checked before the call and never reported as unavailable (FR-211-AC-20); the wording issues are FND-017 |
| FND-015 | fixed | Round 2: `no-producer` applies only when no entry carries `producers` (FR-211-AC-23) |
| FND-013 | still-open | Round 3 (1b4687c): FR-195 now gives every FR-211 reason `compliance` `undischarged`, but FR-135 (the `quoin discharge --rmm` view that FR-195 says its rows equal) is not amended. FR-135 defines `Not compliant - undischarged` as "no fact exists" and `attestation not current` as "a fact exists and its attestation has expired...", and `rmm.rs` maps every open clause that has a fact to `AttestationNotCurrent`. So the source command renders a current fact with an unresolved ref as "attestation not current", while FR-195 requires `undischarged` and equal rows. FR-135 needs the FR-211 reasons mapped to `undischarged` |
| FND-016 | fixed | Round 3: the RMM resolves refs locally, so direct rows equal the source rows; only approval-class rows differ (FR-195-AC-1). The FR-191 sentence is recorded as FND-019 |
| FND-017 | fixed | Round 3: `invalid-record-identity` is distinct from FR-133's `invalid-identifier`, is tested on the joined `record_id`, and applies to approval-class clauses only (FR-211-AC-20) |
| FND-018 | fixed | Round 3: FR-046-AC-6 and AC-9 state that a direct fact discharges only with a store (`--repo`) |
| FND-013 | fixed | Round 4 (c0393fe): FR-135 maps every FR-211 reason and `not-verified-offline` to `Not compliant - undischarged` with the reason shown (FR-135-AC-3, AC-8), matching FR-195; the regressions that wording introduces are FND-020 and FND-021 |
| FND-019 | fixed | Round 4: FR-191 adds "except as FR-195 states for approval-class clauses" |
| FND-020 | fixed | Round 5 (77360f2): FR-135 lists `Not compliant - temporary exception expired` and puts it first in an explicit precedence, matching FR-174-AC-6, TC-004, FR-195 and rmm.rs |
| FND-021 | fixed | Round 5: FR-135's precedence puts attestation currency ahead of FR-211 reasons, and AC-3 and AC-8 follow it |
| FND-022 | fixed | Round 6 (bcf05e3): the FR-195 bullet, FR-195-AC-1 and the FR-211 matrix bullet all say approval-class rows are `undischarged`/`not-verified-offline` "subject to FR-135's precedence"; no remaining contradiction |

## New findings (disposition pass 2)

Re-checked against fix commit aaae0c3.

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-016 | high | FR-195 now marks every clause that "depends on a resolved ref" `not-verified-offline`, which covers every direct fact, even though resolving a ref is a local, deterministic store read. The dossier RMM therefore never shows a direct clause compliant, and its rows differ from `quoin discharge --repo --rmm --json`, contradicting FR-195-AC-1 ("rows equal the source rows") and FR-191 (a source "gives the same result as its named source command") | FR-195, FR-195-AC-1, FR-191, FR-211 | wrong-requirement |
| FND-017 | low | `invalid-identifier` reuses FR-133's store-refusal kind (1 to 64 bytes) for a different character rule. It applies the leading-`-` rule to every identifier, although EA checks only the start of the joined `record_id`, and it does not say whether it applies to clauses without `approvalRoles`, where no call is made | FR-211, FR-211-AC-20, FR-133 | wrong-requirement |
| FND-018 | low | With `no-store`, `quoin discharge --facts` without `--repo` never puts a direct fact in the direct population, but FR-046-AC-9 (TC-626) and FR-046-AC-6 still describe `--facts` alone as classifying a direct fact as direct; they need `--repo` stated | FR-046-AC-9, FR-046-AC-6, FR-211-AC-21 | wrong-requirement |

## New findings (disposition pass 3)

Re-checked against fix commits b638e6d and 1b4687c.

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-019 | low | FR-191 still says a dossier source "gives the same result as its named source command", but FR-195 now makes approval-class RMM rows differ from `quoin discharge --repo --rmm --json` (`not-verified-offline`); FR-191 needs the FR-195 exception named | FR-191, FR-195-AC-1 | wrong-requirement |

## New findings (disposition pass 4)

Re-checked against fix commit c0393fe.

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-020 | high | FR-135 now puts a clause whose fact "is open for any reason other than its attestation's currency" under `Not compliant - undischarged`. That covers an expired `temporary_exception` with a current attestation (reason `temporary exception expired`, FR-046-AC-8), which FR-174 and FR-174-AC-6, TC-004, FR-195's `temporary_exception_expired` value and `rmm.rs` `TemporaryExceptionExpired` all render as `Not compliant - temporary exception expired` | FR-135, FR-174-AC-6, FR-195, FR-046-AC-8 | wrong-requirement |
| FND-021 | medium | FR-135 now says `attestation not current` applies only when the attestation's currency is the only failure, so a fact with an expired attestation and an unresolved ref is `undischarged`; FR-135-AC-8 says "the same fact" (the one with the unresolved ref) with an expired attestation is `attestation not current`, and AC-3 omits the "only" qualifier | FR-135-AC-8, FR-135-AC-3, FR-135 | wrong-requirement |

## New findings (disposition pass 5)

Re-checked against fix commit 77360f2.

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-022 | low | FR-195 still says the RMM "SHALL always show an approval-class clause with `compliance` `undischarged` and reason `not-verified-offline`" (and FR-195-AC-1 repeats it), while FR-195's next bullet and AC-5 now defer to FR-135's precedence. An approval-class clause with an expired attestation, or an expired temporary exception, is therefore both `undischarged` and `attestation_not_current`/`temporary_exception_expired`. Either value is open and gets a `clause_open` gap; only the label differs | FR-195, FR-195-AC-1, FR-195-AC-5, FR-135 | wrong-requirement |
