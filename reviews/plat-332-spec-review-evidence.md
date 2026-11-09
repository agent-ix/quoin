---
id: SR-16324
title: "Evidence review of PLAT-332 trace-mark spec"
type: SpecReview
analysis: evidence
scope: "agent-ix/quoin; spec/adrs/ADR-001-execution-evidence-is-emitted-by-the-run.md; spec/functional/FR-120-trace-marks-in-test-result-output.md; spec/functional/FR-121-emitter-mechanism-constraints.md; spec/non-functional/NFR-028-inert-and-low-cost-emission.md; spec/non-functional/NFR-029-self-contained-result-evidence.md; spec/stakeholder/StR-009-accepted-software-verified-by-tests-that-ran.md; spec/usecase/US-023-record-a-run-that-names-its-own-requirements.md; spec/adrs/index.md; spec/*/index.md; spec/spec.md"
review_set: subset
---

## Summary

Ticket: PLAT-332. PR agent-ix/quoin#773 (spec-only, head f1662cc3), scoped to `git diff origin/main...HEAD`.
The evidence lens examined ADR-001, StR-009, US-023, FR-120, FR-121, NFR-028 and NFR-029, reading FR-033 and FR-032
for context. It records 4 findings (0 high).

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Emitter-behaviour ACs are verified by 'Test' in quoin's spec, but the emitters live outside quoin, so no quoin test can carry them | spec/functional/FR-120-trace-marks-in-test-result-output.md:85 |
| FND-002 | medium | FR-121-AC-1 'same result output' never holds literally (JUnit time/timestamp), and 'installed' and the compiled-artifact baseline are undefined | spec/functional/FR-121-emitter-mechanism-constraints.md:36 |
| FND-003 | low | FR-121-AC-5 is a design choice verified by 'Test', and 'equivalent harness' is undefined | spec/functional/FR-121-emitter-mechanism-constraints.md:40 |
| FND-004 | low | FR-120-AC-6 'the obligation it names is not discharged' is false whenever another passing test binds the same obligation | spec/functional/FR-120-trace-marks-in-test-result-output.md:86 |

## Detail

- **FND-001** (FR-120-AC-5): FR-120-AC-5, FR-121-AC-1..5 and NFR-028 describe the pytest, vitest/jest and Rust emitters (PLAT-333 and its siblings), not quoin. With Verification 'Test' in quoin's spec, quoin's computed matrix gets rows that no quoin test can ever tag. Fix: allocate these criteria to the emitter-owning repos' specs, or give quoin a fixture harness that exercises each emitter and say so.

- **FND-002** (FR-121-AC-1): JUnit output carries per-run time and timestamp attributes, so the result output of two runs is never the same. The AC needs a normalization (for example, ignoring timing attributes). 'Same compiled artifacts' also needs a baseline showing that two no-emitter builds are identical, and a definition of 'installed' (dependency added, plugin on the path, feature declared).

- **FND-003** (FR-121-AC-5): Which harness the Rust emitter uses is a design decision, so it should be verified by Inspection. 'Or an equivalent harness' does not say what counts as equivalent, for example stable JUnit with per-case properties.

- **FND-004** (FR-120-AC-6): An obligation bound by two tests, one passing and one failing, may be discharged by the passing one. The AC should say that the failing entry discharges nothing, not that the obligation is undischarged.

## Verdict

No blocking findings from this lens.
Clean units (examined, no finding from this lens): every other unit listed in the reviewer comment scope.
