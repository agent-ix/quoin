---
id: SR-16323
title: "EARS conformance review of PLAT-332 trace-mark spec"
type: SpecReview
analysis: ears-conformance
scope: "agent-ix/quoin; spec/adrs/ADR-001-execution-evidence-is-emitted-by-the-run.md; spec/functional/FR-120-trace-marks-in-test-result-output.md; spec/functional/FR-121-emitter-mechanism-constraints.md; spec/non-functional/NFR-028-inert-and-low-cost-emission.md; spec/non-functional/NFR-029-self-contained-result-evidence.md; spec/stakeholder/StR-009-accepted-software-verified-by-tests-that-ran.md; spec/usecase/US-023-record-a-run-that-names-its-own-requirements.md; spec/adrs/index.md; spec/*/index.md; spec/spec.md"
review_set: subset
---

## Summary

Ticket: PLAT-332. PR agent-ix/quoin#773 (spec-only, head f1662cc3), scoped to `git diff origin/main...HEAD`.
The ears-conformance lens examined ADR-001, StR-009, US-023, FR-120, FR-121, NFR-028 and NFR-029, reading FR-033 and FR-032
for context. It records 4 findings (0 high).

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | FR-120 statement is compound with two subjects (emitter SHALL write; quoin evidence record SHALL read) | spec/functional/FR-120-trace-marks-in-test-result-output.md:18-20 |
| FND-002 | low | NFR-028 statement is compound and 'measurably' conflicts with the table's 5% threshold | spec/non-functional/NFR-028-inert-and-low-cost-emission.md:17-18 |
| FND-003 | low | FR-121 statement 'SHALL hold the constraints below' has no response of its own | spec/functional/FR-121-emitter-mechanism-constraints.md:16-23 |
| FND-004 | low | StR-009-VC-1 bundles three independent conditions | spec/stakeholder/StR-009-accepted-software-verified-by-tests-that-ran.md:40 |

## Detail

- **FND-001** (FR-120): One SHALL statement binds two different systems: the emitter (out of repo) and `quoin evidence record`. Split it, or state the quoin obligation alone and move the emitter obligation to the contract table.

- **FND-002** (NFR-028): The statement joins byte-identity and speed in one SHALL, and 'SHALL NOT measurably slow' contradicts the Measurement table, which accepts up to 5% overhead. State the bound in the statement.

- **FND-003** (FR-121): The statement delegates entirely to the AC table and names no system response, so it is not a conforming EARS requirement. Name the subject and the response (for example: the emitter SHALL record each tagged test once with its ids and outcome, with emission off by default).

- **FND-004** (StR-009-VC-1): Binding only via emitted ids, failing tests discharging nothing, and self-containment are three checks that pass or fail independently. Split them into separate VCs.

## Verdict

No blocking findings from this lens.
Clean units (examined, no finding from this lens): every other unit listed in the reviewer comment scope.
