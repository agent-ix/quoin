---
id: SR-036
title: "Gap analysis — intervention-experiment evidence (Quoin #270)"
type: SpecReview
analysis: gap-analysis
scope: "PLAN-002, US-015, FR-056, FR-057, FR-058, intervention implementation, retained agent-eval evidence, and tests/intervention.test.ts"
review_set: subset
---

# SR-036: Gap analysis — intervention-experiment evidence (Quoin #270)

## Summary

Targeted post-implementation gap analysis for Quoin #270 and PLAN-002. All five
plan tasks are complete. The implementation retains the exact baseline and
treatment runner reports, adapts them without executing an experiment, validates
and stores the resulting intervention record, and renders claims, evidence,
counterevidence, gaps, owner, and actions without inventing a causal result.

The retained runs both passed one of two samples, so the first real producer
correctly records a zero pass-rate effect with `cause_not_established`, `none`
attribution confidence, explicit confounders, and follow-up actions. That result
is evidence that the producer works; it is not evidence that the evaluated
sentinel change improved agent performance.

## Verdict

**PASS** — PLAN-002 has no incomplete task, all 27 target matrix rows are backed
by real test symbols, and every implementation surface added by the plan has an
owning requirement. No target gap or unowned implementation was found.

## Findings

| ID      | Severity | Summary                                                                                                                                                                                                               | Refs             |
| ------- | -------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------- |
| FND-001 | low      | Repository-wide coverage remains a separate backlog: the full census reports 461/924 rows backed plus pre-existing parser and status-column diagnostics outside #270. None names a target requirement, task, or test. | `spec/matrix.md` |

FND-001 is an out-of-scope census observation, not a target gap, so it does not
alter this targeted verdict.

## Plan completion

| Task     | Result   | Evidence                                                                                                                                              |
| -------- | -------- | ----------------------------------------------------------------------------------------------------------------------------------------------------- |
| TASK-006 | complete | Versioned record types, JSON Schema, and semantic validation in `src/measurement/intervention-types.ts` and `src/measurement/intervention-schema.ts`. |
| TASK-007 | complete | Governed raw-byte-verified, atomic, idempotent intake in `src/measurement/intervention.ts` and the measurement record command.                        |
| TASK-008 | complete | Deterministic claim-centered human and JSON projection in `src/measurement/intervention-report.ts` and `src/measurement/report.ts`.                   |
| TASK-009 | complete | First-party retained-report adapter in `src/measurement/agent-eval-intervention.ts`.                                                                  |
| TASK-010 | complete | Real retained-run E2E, no-process boundary, compatibility checks, clean test gate, and this review.                                                   |

PLAN-002 is marked `complete`; TASK-006 through TASK-010 are marked `done`.

## Matrix verification

Quire 0.31.0 coverage reconciliation, using the locked
`spec-artifacts-process@61a20e0` vocabulary, reports:

| Requirement | Backed obligations | Test cases       |
| ----------- | -----------------: | ---------------- |
| FR-056      |                9/9 | TC-1195..TC-1203 |
| FR-057      |              14/14 | TC-1204..TC-1216 |
| FR-058      |                5/5 | TC-1217..TC-1221 |

The FR-057 range contains thirteen unique test cases: eleven acceptance
criteria and three constraints, with TC-1212 covering both an acceptance
criterion and a constraint. All are bound to real tests in
`tests/intervention.test.ts`; none relies on a matrix checkbox alone.

The clean inner repository gate passed 65 test files and 746 tests. TypeScript
typechecking, targeted ESLint and Prettier checks, the production build, and
Quire document validation also passed for the implementation work. Validation
continues to emit pre-existing repository-wide grammar warnings outside this
reviewed subset.

After replay onto Quoin main `ab73aadfb7796e9a30ec89e42f87afe09fe7f1fd`,
the current inner gate passed the production build, document validation, version
agreement, 67/67 test files, and 815/815 tests using Quire CLI
`bcface2714a958a328f3427714650ab2df71030f`, engine
`ca7362d4dacecb96f01d74d1d971327118c25917`, and the locked
`spec-artifacts-process@61a20e0` vocabulary. The broader verification-stack gate
stopped before governed execution because its locked Filament revision
`546e7943ee5a8fe552242cbb19d12aa902536652` is no longer reachable from any
current remote-tracking ref; this provenance limitation remains visible rather
than being bypassed with a fabricated ref.

## Underspecified-code trace

- `intervention-types.ts` and `intervention-schema.ts` implement FR-056's
  versioned record, design, treatment, effect, attribution, confounder, and raw
  evidence obligations.
- `intervention.ts` and `commands/measurement/intervention.ts` implement
  FR-057's governing-definition, validation, collision, atomic-write, raw-byte,
  and query obligations.
- `intervention-report.ts` and `report.ts` implement FR-057's claims, evidence,
  counterevidence, gaps, owner, actions, determinism, and no-aggregate-score
  obligations.
- `agent-eval-intervention.ts` implements FR-058's two-report reconciliation,
  exact digest derivation, refusal paths, and conservative causal conclusion.
- `measurement/index.ts`, `commands/measurement/record.ts`, and `vite.config.ts`
  expose the governed surfaces and preserve compatibility under FR-057.

No added production symbol lacks one of those owners, and no stub, placeholder,
or unimplemented branch remains in the reviewed diff.

## Retained operational evidence

The baseline report digest is
`sha256:741ab150c107d1f5551a9be2092081cc0439f85e804d23f81e3923dfab8fe076`
(11,305 bytes). The treatment report digest is
`sha256:baade4ab1c2e8f3447b4c585c95634884cb2ed1c69a9cf8819d0b495c7f77963`
(14,261 bytes). TC-1221 consumes those exact checked-in bytes and persists the
canonical record through the FR-057 path without spawning a process.

## Semantic review

Not run. The optional semantic-review expansion was not requested; this review
performed the required plan, matrix, and code-ownership checks only.
