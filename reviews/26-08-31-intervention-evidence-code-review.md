---
id: SR-078
title: "Code review — intervention experiment evidence (Quoin #270)"
type: SpecReview
analysis: code-review
scope: "PLAN-002; US-015; FR-056..FR-058; src/measurement/intervention*; agent-eval producer; tests/intervention.test.ts"
review_set: subset
relationships:
  - target: "ix://agent-ix/quoin/PLAN-002"
    type: reviews
  - target: "ix://agent-ix/quoin/TM-001"
    type: references
---

# SR-078: Code review — intervention experiment evidence (Quoin #270)

## Summary

Reviewed Quoin #270 from current-main base
`ab73aadfb7796e9a30ec89e42f87afe09fe7f1fd` through
`8a1daeea83db03be7d88f5a825ddbbb27e35dabf` (including the preceding assurance-gate repairs). The review covered the intervention
schema and semantics, governed raw-byte intake, idempotence and collisions,
claim-centered reporting, the offline cli-agent-evals adapter, retained evidence,
and all TC-1195..TC-1221 tests. The retained baseline and treatment bytes now
regenerate the checked-in intervention record exactly. No unresolved correctness,
safety, boundary, or code-test-alignment finding remains.

## Verdict

**PASS** — the intervention implementation is complete and fail closed. It
derives evidence from retained reports without invoking an agent, harness,
subprocess, or network client and does not infer causality from inadequate data.

## Findings

| ID      | Severity | Summary                                                                                                                                                                                                                                | Refs                                                             | Escape Cause                        |
| ------- | -------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------- | ----------------------------------- |
| FND-001 | low      | Fixed: the reconciled historical architecture guard violated the current Prettier gate; formatting now matches the already-correct #271 stack without changing behavior.                                                               | `tests/semantic-module-architecture.test.ts`                     | implementation-bug-despite-evidence |
| FND-002 | high     | Fixed: FR-058 called immutable historical runner output self-versioned even though the governed producer definition supplies its schema contract; the spec, matrix, refusal test, and evidence wording now describe the real boundary. | FR-058-AC-1; FR-058-AC-2; TC-1217; TC-1218                       | wrong-requirement                   |
| FND-003 | high     | Fixed: the producer accepted a caller-authored observation time and the record validator accepted date-only and impossible timestamps; time now comes from the treatment report and is checked as an actual RFC 3339 date-time.        | FR-056-AC-1; FR-058-AC-1; FR-058-AC-3; TC-1195; TC-1217; TC-1219 | implementation-bug-despite-evidence |
| FND-004 | high     | Fixed: raw paths containing redundant `.` or separators passed despite the normalized-path invariant; intake now rejects every non-normalized path before reading it.                                                                  | FR-056-AC-9; FR-057-AC-11; TC-1203; TC-1216                      | implementation-bug-despite-evidence |
| FND-005 | medium   | Fixed: the runtime schema lacked its stable `$id`/public export and advertised BLAKE3 records that the SHA-256 verifier could never accept; the exported engine-neutral contract now matches executable intake.                        | FR-056-AC-1; FR-056-AC-9; TC-1195; TC-1203                       | implementation-bug-despite-evidence |
| FND-006 | medium   | Fixed: three matrix rows were represented only by range shorthand in test comments; TC-1211, TC-1212, and TC-1219 now have literal tracking tags.                                                                                      | TC-1211; TC-1212; TC-1219                                        | correct-requirement-no-evidence     |
| FND-007 | low      | Fixed: the skill vocabulary gate preferred a stale developer checkout over the active installed public module contract; active module paths now win deterministically.                                                                 | `tests/skill-contracts.test.ts`                                  | implementation-bug-despite-evidence |

## Review Method

- Checked schema closure, semantic invariants, safe record IDs and evidence
  paths, definition gating, raw byte size/digest checks, same-directory atomic
  rename, byte idempotence, and collision refusal.
- Checked that invalid input accumulates stable reasons and cannot leave a
  partial record, and that unknown/uncontrolled and negative results remain
  explicit rather than becoming a score or causal claim.
- Checked both report renderers derive from the same deterministically ordered
  projection and preserve counterevidence, gaps, owners, actions, and raw refs.
- Checked the agent-eval producer derives all decision-bearing values from two
  retained real reports and contains no execution or network surface. The governed
  definition supplies the input-schema version, and the treatment report supplies
  the observation clock.

## Validation Evidence

The governed inner gate passes TypeScript typecheck, ESLint, Prettier, production
build, Quire validation, version agreement, and 815/815 tests across 67 files using
the pinned Quire 0.30.2 binary. Quire reports zero target unbacked rows, status
lies, or no-symbol rows for FR-056..FR-058 and TC-1195..TC-1221. The changed source
and tests contain no target skip, placeholder, TODO/FIXME/XXX, or weak no-op
assertion.

## Semantic Review

The optional broad gap-analysis semantic expansion was not run. This code review
did perform required spec-faithfulness and code-test-alignment checks.
