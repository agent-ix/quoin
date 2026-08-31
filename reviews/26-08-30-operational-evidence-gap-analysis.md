---
id: SR-037
title: "Gap analysis — operational evidence (Quoin #271)"
type: SpecReview
analysis: gap-analysis
scope: "PLAN-003, US-016, FR-059, FR-060, FR-061, operational implementation, retained GitHub Actions evidence, and tests/operational.test.ts"
review_set: subset
---

# SR-037: Gap analysis — operational evidence (Quoin #271)

## Summary

Targeted post-implementation gap analysis for Quoin #271 and PLAN-003. All five
plan tasks are complete. The implementation represents standing capabilities and
actual or drill exercises, validates and atomically retains them, evaluates
clocked discharge, renders claim-centered operational reports, and supplies a
first-party offline producer for retained GitHub Actions release evidence.

The first real producer consumes Quoin release run 33280266874 at immutable
revision `a9808be18b61f8e4d44e3b74de27e90f17c5c76b`. It derives an available
manual release capability and a linked successful actual exercise whose Publish
job completed in 37 seconds, within the declared ten-minute clock.

## Verdict

**PASS** — PLAN-003 has no incomplete task, all 26 target matrix rows are backed
by real test symbols, and every implementation surface added by the plan has an
owning requirement. No target gap or unowned implementation was found.

## Findings

| ID      | Severity | Summary                                                                                                                                                                                                        | Refs             |
| ------- | -------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------- |
| FND-001 | low      | Repository-wide coverage remains a separate backlog: the full census reports 489/976 rows backed plus pre-existing parser and status-column diagnostics outside #271. None names a target requirement or test. | `spec/matrix.md` |

FND-001 is an out-of-scope census observation, not a target gap, so it does not
alter this targeted verdict.

## Plan completion

| Task     | Result   | Evidence                                                                                                                                        |
| -------- | -------- | ----------------------------------------------------------------------------------------------------------------------------------------------- |
| TASK-011 | complete | Versioned operational record types, JSON Schema, and semantic validation in `operational-types.ts` and `operational-schema.ts`.                 |
| TASK-012 | complete | Governed raw-byte-verified intake, pair atomicity, and clocked discharge in `operational.ts` and the measurement record command.                |
| TASK-013 | complete | Deterministic claim-centered human and JSON projection in `operational-report.ts` and `report.ts`.                                              |
| TASK-014 | complete | First-party retained-workflow/run/jobs adapter in `github-release-operational.ts` and its command.                                              |
| TASK-015 | complete | Real retained-run E2E, no-control boundary, compatibility and all-or-nothing checks, clean test gate, coverage reconciliation, and this review. |

PLAN-003 is marked `complete`; TASK-011 through TASK-015 are marked `done`.

## Matrix verification

Quire 0.31.0 coverage reconciliation, using the locked
`spec-artifacts-process@61a20e0` vocabulary, reports:

| Requirement | Backed obligations | Test cases       |
| ----------- | -----------------: | ---------------- |
| FR-059      |                9/9 | TC-1223..TC-1231 |
| FR-060      |              12/12 | TC-1232..TC-1243 |
| FR-061      |                5/5 | TC-1244..TC-1248 |

All 26 rows bind to real tests in `tests/operational.test.ts`; none relies on a
matrix checkbox alone. The real-run checks copy the exact retained artifacts to
an isolated repository and invoke only the offline Quoin producer.

The clean inner repository gate passed 66 test files and 758 tests. TypeScript
typechecking, targeted ESLint and Prettier checks, the production build, and
Quire document validation also passed. Validation continues to emit pre-existing
repository-wide grammar warnings outside this reviewed subset.

## Underspecified-code trace

- `operational-types.ts` and `operational-schema.ts` implement FR-059's envelope,
  shape distinction, control vocabulary, clocks, version pins, outcomes,
  governance, linkage, and raw-evidence obligations.
- `operational.ts` and `commands/measurement/record.ts` implement FR-060's
  definition gate, raw-byte gate, validation, atomic writes, idempotence,
  collision handling, pair integrity, query, and discharge obligations.
- `operational-report.ts` and `report.ts` implement FR-060's claims, evidence,
  counterevidence, gaps, owner, actions, determinism, and no-aggregate-score
  obligations.
- `github-release-operational.ts` and
  `commands/measurement/operational-release.ts` implement FR-061's structural
  workflow check, exact run/job reconciliation, source-derived fields, adverse
  outcome preservation, and offline pair submission.
- `measurement/index.ts` and `vite.config.ts` expose the governed surfaces and
  preserve compatibility under FR-060.

No added production symbol lacks one of those owners, and no stub, placeholder,
or unimplemented branch remains in the reviewed diff.

## Retained operational evidence

| Artifact                   |  Bytes | SHA-256                                                            |
| -------------------------- | -----: | ------------------------------------------------------------------ |
| v0.22.5 `release.yml`      |  6,651 | `5a867277a071c2dd8fe1ab86e22b4b3e580fff0b269756c3911c4dde2ed1cc78` |
| Workflow-run REST payload  | 11,898 | `a3a3eea43f6f17fc9851a50aa6853aa1d67a77ad91c6b4ddc1922aa03c4acaaf` |
| Workflow-jobs REST payload | 13,166 | `adb0e51b5212d9ff4f01238a26040f8ea893c75f6bc02cfbc4b7d98552182e92` |

The two REST digests were compared with fresh read-only API responses, and the
workflow digest was compared with the file at the run's immutable source
revision. TC-1244 and TC-1248 consume those exact checked-in bytes and persist
the canonical pair through the FR-060 path without network or process execution.

## Semantic review

Not run. The optional semantic-review expansion was not requested; this review
performed the required plan, matrix, and code-ownership checks only.
