---
id: SR-16392
title: "Integrity review of PLAT-334 FR-120 E-4 and AC-5 amendment"
type: SpecReview
analysis: integrity
scope: "agent-ix/quoin; spec/functional/FR-120-trace-marks-in-test-result-output.md"
review_set: subset
relationships:
  - target: "ix://agent-ix/quoin/FR-120"
    type: "reviews"
---

## Summary

Ticket: PLAT-334. PR agent-ix/quoin#776, scoped to `git diff origin/main...HEAD`. The integrity lens checked the amended
E-4 and FR-120-AC-5 for consistency with FR-120 (AC-1..6, E-1..E-10), NFR-029, US-023, FR-030, FR-032 and ADR-100. Two
findings (1 high, 1 medium).

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | E-4 now allows a tagged case with no trace ids, but NFR-029 (unamended) requires every tagged case's ids in the artifact, with a zero threshold | spec/functional/FR-120-trace-marks-in-test-result-output.md:123; spec/non-functional/NFR-029-self-contained-result-evidence.md:17-19,35,40-42 |
| FND-002 | medium | E-4's second sentence "with its ids when they are known" is not scoped, so an emitter with static ids may drop them from a failing test | spec/functional/FR-120-trace-marks-in-test-result-output.md:123 |

## Detail

- **FND-001** (FR-120 E-4): NFR-029 constrains FR-120. Its statement says every result artifact an emitter produces
  SHALL carry "each tagged test's symbol, file path, outcome and trace ids". Its metric has target 0 and threshold 0
  for "Tagged cases whose ... ids are not all in the artifact". Its Verification asserts "every tagged case is recorded
  with its ... ids". The amended E-4 requires a vitest/jest emitter to write a tagged case (skipped, or thrown before
  the trace call) with no `trace` property. A conforming emitter therefore fails NFR-029 by construction. Fix: amend
  NFR-029 in this PR so ids are required on every tagged case that reached the point where its ids are known, with the
  E-4 non-binding entry named as the exception. The alternative is to drop the exception from E-4.
- **FND-002** (FR-120 E-4): Before this PR, a panicking or throwing test was "recorded `fail` with its ids, never
  absent" for every emitter. The amended second sentence reads "with its ids when they are known" and is not limited to
  the deferred-id emitter of the third sentence. The Rust and pytest emitters know their ids statically from the
  attribute or marker. Under one reading they may still record a failing case without ids and conform. US-023-EX-2 and
  E-10's failing variant expect a failing tagged test to carry its ids, so the matrix can show a red test against its
  criterion. Fix: keep "with its ids" unconditional in the second sentence and put the exception only in the third
  sentence, limited to emitters whose ids are known only at the trace call.

## Verdict

Not mergeable as written: FND-001 is high (two requirements contradict). Clean: the amendment keeps "never fabricate an
outcome the runner did not report". The id-less entry carries only the runner's outcome, and its non-binding effect
agrees with FR-030's binding rule (trace id on a passing entry only) and with FR-030-AC-18 (the symbol join never binds).
FR-032 checks entries by symbol, so these entries still feed its vacuity and stale-run checks. ADR-100 is unaffected.
