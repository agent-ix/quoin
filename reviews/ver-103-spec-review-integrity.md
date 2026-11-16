---
id: SR-20261116004329
title: "Integrity review of VER-103 formalization coverage spec"
type: SpecReview
analysis: integrity
scope: "agent-ix/quoin; PR #936; FR-202 and FR-203 inputs, outputs, behavior and acceptance criteria"
review_set: all
---

## Summary

The two requirements are testable overall, but the external-state mapping is incomplete and a required draft-gap reason has no output obligation. The documented IR-96 boundary is respected; no local replacement record shape is introduced.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The mapping statement leaves `none` and `draft` unmapped to IR-96 (`eligible`/`not-applicable` and `authored` respectively), and does not map the non-qualified dispositions listed in FR-202 to a specific IR-96 state. Specify the complete mapping and which states this lane may emit. | spec/functional/FR-203-formalization-as-evidence-kind.md:35; spec/functional/FR-202-formalization-coverage-state.md:60 |
| FND-002 | medium | A draft item is required to appear as a gap, but neither the gap output nor its AC requires the approval/draft reason to appear. The requirement's own design says the reason is retained; make it observable in the gap output. | spec/functional/FR-203-formalization-as-evidence-kind.md:29,36,52 |
