---
id: SR-16321
title: "Base checklist review of PLAT-332 trace-mark spec"
type: SpecReview
analysis: base
scope: "agent-ix/quoin; spec/adrs/ADR-001-execution-evidence-is-emitted-by-the-run.md; spec/functional/FR-120-trace-marks-in-test-result-output.md; spec/functional/FR-121-emitter-mechanism-constraints.md; spec/non-functional/NFR-028-inert-and-low-cost-emission.md; spec/non-functional/NFR-029-self-contained-result-evidence.md; spec/stakeholder/StR-009-accepted-software-verified-by-tests-that-ran.md; spec/usecase/US-023-record-a-run-that-names-its-own-requirements.md; spec/adrs/index.md; spec/*/index.md; spec/spec.md"
review_set: subset
---

## Summary

Ticket: PLAT-332. PR agent-ix/quoin#773 (spec-only, head f1662cc3), scoped to `git diff origin/main...HEAD`.
The base lens examined ADR-001, StR-009, US-023, FR-120, FR-121, NFR-028 and NFR-029, reading FR-033 and FR-032
for context. It records 3 findings (1 high).

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | FR-120-AC-7 reintroduces requirement-edit staleness that FR-032-AC-2 withdrew because bindings carry no hash | spec/functional/FR-120-trace-marks-in-test-result-output.md:87 |
| FND-002 | low | FR-120 constraint rows are out of order (CON-1, CON-3, CON-2) | spec/functional/FR-120-trace-marks-in-test-result-output.md:94-96 |
| FND-003 | low | New spec/adrs/ADR-001 numbering is ambiguous next to the repo's docs ADR 0001/0002 and cited Quire ADR-0011 | spec/adrs/ADR-001-execution-evidence-is-emitted-by-the-run.md:1-3 |

## Detail

- **FND-001** (FR-120-AC-7): FR-120-AC-7 requires that editing a bound requirement makes the binding stale in the auditor report. FR-032-AC-2 (withdrawn 2026-10-29) removed exactly this suspect-link check because bindings carry no statement hash. Implementing AC-7 needs a stamped requirement hash or revision on every binding, which is the hash/tracking-record pattern the checklist forbids, and it is auditor behaviour outside the mark contract FR-120 owns. Fix: delete FR-120-AC-7.

- **FND-002** (FR-120-CON-3): The FR-120 Constraints table lists FR-120-CON-1, then CON-3, then CON-2. IDs should be in sequence.

- **FND-003** (ADR-001): quoin already has ADRs at docs/semantic-module-architecture/adr/0001-authority-by-concern.md and 0002-..., and its FRs cite Quire's ADR-0011. A new spec/adrs/ADR-001 starts a second sequence, so 'ADR-001' and 'ADR-0001' name different decisions in one repo. Pick one ADR location or give the new one a distinct number.

## Verdict

Not mergeable as written: high findings open.
Clean units (examined, no finding from this lens): every other unit listed in the reviewer comment scope.
