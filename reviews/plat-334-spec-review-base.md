---
id: SR-16390
title: "Base review of PLAT-334 FR-120 E-4 and AC-5 amendment"
type: SpecReview
analysis: base
scope: "agent-ix/quoin; spec/functional/FR-120-trace-marks-in-test-result-output.md"
review_set: subset
relationships:
  - target: "ix://agent-ix/quoin/FR-120"
    type: "reviews"
---

## Summary

Ticket: PLAT-334. PR agent-ix/quoin#776 (spec-only, head b882899d), scoped to `git diff origin/main...HEAD`: FR-120-AC-5
and Emitter contract rule E-4. The base checklist examined the two amended units against the rest of FR-120 (AC-1..6,
E-1..E-10), FR-030 (AC-18 and the binding rules), FR-032 (vacuity and stale-run checks), ADR-100, US-023 and NFR-029,
and checked the behaviour against `rust/crates/quoin-evidence/src/record.rs` and
`rust/crates/quoin-auditor/src/audit/ladder.rs`. No assurance profile applies (AP-201 and AP-202 cover finding scoring
and Jev lenses, not evidence recording). One finding (medium).

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-120-AC-5's new clause conditions on emitter state quoin cannot observe and gives an entry a `trace` property it does not have | spec/functional/FR-120-trace-marks-in-test-result-output.md:102 |

## Detail

- **FND-001** (FR-120-AC-5): AC-5 is a quoin criterion verified by Test, while the Emitter contract says "Quoin tests
  none of it". The amended AC conditions on "a case whose ids are not known at record time" and on "its ids when they
  are known". That is the emitter's state when it wrote the file. `quoin evidence record` sees only a `<testcase>` with
  or without a `trace` property, so a quoin test cannot set up or check the condition. The AC also says the *entry* has
  "no `trace` property". `trace` is a property of the JUnit `<testcase>`; the recorded entry carries `traceIds`. Fix:
  state the quoin-observable behaviour, for example "A `<testcase>` with no `trace` property records an entry with the
  case's outcome and no trace ids, which binds no obligation." Keep the emitter-side condition in E-4 only.

## Verdict

Mergeable after FND-001 and the integrity findings (SR-16392) are fixed. Clean: the amendment does not weaken "SHALL NOT
write an outcome the runner did not report"; the first sentence of E-4 is unchanged and the id-less entry carries only
the runner's outcome. It is consistent with FR-030: the run record keeps every entry, binding is by trace id on a
passing entry only (record.rs:118-134), so an entry with no ids binds nothing, and FR-030-AC-18's symbol join never
binds. It also helps FR-032, whose vacuity and stale-run checks match entries by symbol (ladder.rs:273-280), so a
skipped or failed id-less entry is still seen. E-2, E-3, E-5 and E-10 are not contradicted.
