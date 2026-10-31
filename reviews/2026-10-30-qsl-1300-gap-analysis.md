---
id: SR-185
title: "Gap analysis — QSL-1300 graph-quality producer source revision"
type: SpecReview
analysis: gap-analysis
scope: "QSL-1300; FR-066-AC-6; graph-quality Producer, synthetic fixtures, parser test, and FR-066 wording"
review_set: subset
relationships:
  - target: "ix://agent-ix/quoin/FR-066"
    type: references
---

## Summary

Compared QSL-1300 and FR-066-AC-6 with the current graph-quality parser, its tests, and the computed Test Matrix. The removed producer field no longer appears in the changed parser, test, or synthetic cases; the existing invocation attestation remains unchanged.

## Verdict

**CONDITIONAL** — the QSL-1300 field-removal path remains tested, while the computed FR-066 matrix exposes existing criterion and constraint bindings outside this PR. QSL-1313 tracks that follow-up.

## Assurance Context

AP-201, [spec/assurance/AP-201-finding-quality.md](../spec/assurance/AP-201-finding-quality.md), applies to graph-quality evidence intake. The change removes only the duplicate producer field; it leaves the verification-stack source attestation intact. No QSL-1300 plan or linked evidence record was found; the audit used FR-066, source, tests, and computed matrix output. No active exceptions apply. AP-202 does not apply because no semantic-lens evaluation is in scope.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The computed matrix reports FR-066-AC-1 untagged and FR-066-CON-2/CON-4 untagged; FR-066-CON-1/CON-3 have `method-without-symbol` status. Module-level trace text does not produce a test-function binder. This predates QSL-1300 and is tracked in [QSL-1313](https://linear.app/agent-ix/issue/QSL-1313/fr-066-computed-matrix-has-unbound-graph-adapter-criteria-and). | `spec/functional/FR-066-graph-producer-adapters.md:117`; `spec/functional/FR-066-graph-producer-adapters.md:120`; `spec/functional/FR-066-graph-producer-adapters.md:126`; `rust/crates/quoin-measurement-graph/tests/tc_475_parity.rs:41` |

## Coverage

Method: `quoin matrix --repo . --json` with Quoin 0.29.1, then inspection of FR-066 matrix rows and QSL-1300 code/test paths. FR-066-AC-6 remains tagged to `tc_480_024`; removing `source_revision` from the required producer fields did not remove its test binding. Reverse-gap and stub checks were limited to changed graph-quality symbols; no unowned changed behavior or stubs were found. Optional semantic review was skipped. Plan completion: not assessed.

## Dispositions

| Finding | Outcome | Reason |
| --- | --- | --- |
| FND-001 | deferred | Follow-up tracked in QSL-1313; outside QSL-1300 implementation scope. |
