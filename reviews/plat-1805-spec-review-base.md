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
