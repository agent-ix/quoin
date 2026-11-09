---
id: SR-16391
title: "EARS review of PLAT-334 FR-120 E-4 and AC-5 amendment"
type: SpecReview
analysis: ears-conformance
scope: "agent-ix/quoin; spec/functional/FR-120-trace-marks-in-test-result-output.md"
review_set: subset
relationships:
  - target: "ix://agent-ix/quoin/FR-120"
    type: "reviews"
---

## Summary

Ticket: PLAT-334. PR agent-ix/quoin#776, scoped to `git diff origin/main...HEAD`. The EARS lens examined the amended
Emitter contract rule E-4 and FR-120-AC-5. One finding (low).

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | E-4's new sentence is compound, puts its trigger last, names an undefined "trace call", and states store behaviour as an emitter duty | spec/functional/FR-120-trace-marks-in-test-result-output.md:123 |

## Detail

- **FND-001** (FR-120 E-4): The added sentence "An emitter whose ids are only known once the test body reaches its trace
  call SHALL still record the case ... when the call is never reached ...; that entry carries no `trace` property and is
  non-binding: it binds no obligation and discharges nothing" has three obligations: record the case, omit the
  property, be non-binding. The event trigger comes after the response, not in EARS form "When <trigger>, the
  <system> shall". "Trace call" is a vitest-helper mechanism that the runtime-neutral contract does not define. "Binds
  no obligation and discharges nothing" is what the store does with an entry that has no ids (FR-030). An emitter cannot
  meet or break it, so it reads as an obligation no emitter test can check. Suggested shape: "Where an emitter learns a
  test's ids only when the test body calls its trace function, when that call is not reached (the test is skipped or
  throws first), the emitter SHALL record the case with the runner's outcome and no `trace` property." The binding
  effect then follows from FR-030 and AC-5 and can be cited.

## Verdict

Low only. The AC-5 wording is reviewed under base (SR-16390 FND-001) and is not repeated here. The unchanged E-4 first
sentence ("SHALL NOT write an outcome the runner did not report") stays a correct Ubiquitous negative.
