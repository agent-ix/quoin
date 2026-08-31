---
id: SR-080
title: "Code review — operational evidence and combined #270/#271 stack"
type: SpecReview
analysis: code-review
scope: "PLAN-003; US-016; FR-059..FR-061; operational implementation; tests/operational.test.ts; combined ab73aad..a5502a6 stack"
review_set: subset
relationships:
  - target: "ix://agent-ix/quoin/PLAN-003"
    type: reviews
  - target: "ix://agent-ix/quoin/TM-001"
    type: references
---

# SR-080: Code review — operational evidence and combined #270/#271 stack

## Summary

Reviewed Quoin #271 incrementally from #270
`26bb17f0dff8e4207a6d5e4fc78a591652dc9280` through implementation tip
`a5502a64d639c12f9c238829e1f9a41e83c64a27`, then reviewed the combined stack
from current-main base `ab73aadfb7796e9a30ec89e42f87afe09fe7f1fd` through the
same tip. The review covered operational schemas and semantics, pair atomicity,
clock discharge, deterministic reporting, retained GitHub Actions adaptation,
compatibility with #270, and all TC-1223..TC-1248 tests. Two implementation
defects and one common-gate defect found by the rerun were fixed; no unresolved
defect remains.

## Verdict

**PASS** — #271 and the combined #270/#271 stack preserve engine boundaries,
offline producer behavior, atomic persistence, adverse-state visibility, and
spec-to-test-to-code alignment. Quoin #286 and `filament-ide-rs` were not touched.

## Findings

| ID      | Severity | Summary                                                                                                                                                                                                      | Refs                                                  |
| ------- | -------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | ----------------------------------------------------- |
| FND-001 | high     | Fixed: the producer accepted a selected release job without proving its run id, source revision, and attempt matched the workflow-run export, so unrelated jobs evidence could mint an exercise.             | FR-061-AC-1; FR-061-AC-2; TC-1244; TC-1245; `7edcdc4` |
| FND-002 | high     | Fixed: FR-059 admits `/` in an identity, but the storage path rejected a schema-valid record id instead of persisting it; slash-bearing ids now use an injective filename encoding.                          | FR-059-AC-1; FR-060-AC-1; TC-1223; TC-1232; `a5502a6` |
| FND-003 | medium   | Fixed: the full gate selected a stale development-module schema ahead of the active installed contract; the reusable Quoin gate now prefers active module roots and keeps the checkout as the last fallback. | `tests/skill-contracts.test.ts`; `e3d3d3b`            |

## Review Method

- Checked schema closure, standing/exercise discrimination, temporal and clock
  derivation, typed pins, linkage, adverse outcomes, and raw-evidence integrity.
- Checked pair intake validates both records and their relationship before one
  canonical pair file is atomically renamed; injected second-record failures
  cannot leave a first record behind.
- Checked discharge requires kind, subject, scope, accepted mode, successful
  outcome, and timestamp-derived `met` state.
- Checked human/JSON reports share one deterministic projection and keep unknown,
  unavailable, adverse, missed, open, and incomplete states out of claims.
- Checked the GitHub adapter only reads retained workflow/run/jobs bytes, joins
  the selected job to the run by run id, source revision, and attempt, derives
  path, event, actor, times, outcome, and digests, and exposes no API, dispatch,
  publication, process, or operational-control path.
- Rechecked the combined intervention and operational exports, command wiring,
  persistence collections, reporting, and legacy-store compatibility.

## Validation Evidence

The governed inner gate passes typecheck, ESLint, Prettier, production build,
Quire validation, version agreement, and 827/827 tests across 68 files with
Quire 0.31.0. Quire reports 9/9 FR-059, 10/10 FR-060, and 5/5 FR-061 acceptance
criteria backed, with zero target unbacked rows or status lies. The incremental
and combined diffs contain no target skip, placeholder, TODO/FIXME/XXX, unsafe
execution surface, or aggregate trust score.

A direct `quoin report` rerun over the retained v0.22.5 pair renders the
standing-capability claim and the successful clock-met exercise as separate
records. Both human and JSON views expose claims, evidence, counterevidence,
gaps, owner, and actions; the retained npm-receipt gap remains visible.

The outer verification stack remains correctly unable to authenticate this
unpromoted topic history against a lock that names pre-feature Quoin code. That
is a promotion/provenance boundary, not an inner implementation failure; no
synthetic remote ref or weakened check was introduced.

## Semantic Review

The optional broad gap-analysis semantic expansion was not run. This code review
did perform required spec-faithfulness and code-test-alignment checks.
