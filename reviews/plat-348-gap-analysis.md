---
id: SR-20261111191352
title: "Gap analysis of quoin PR 809 (cargo-deny adapter, SBOM --discharges)"
type: SpecReview
analysis: base
scope: "agent-ix/quoin@df41e4a3; spec/functional/FR-034-finding-shaped-evidence.md (AC-21..24), spec/functional/FR-041-sbom-inventory-evidence.md (AC-7, AC-8), spec/matrix.md TC-2700..2705, their tagged tests and the code they cover"
review_set: subset
---

# Gap analysis of quoin PR 809

## Summary

Tickets: PLAT-348, PLAT-347. Planless; plan completion was not assessed. Scoped to the
criteria the PR adds and the code it changes. `quoin matrix` (quoin 0.30.0) binds every new
criterion to a tagged test, with no `criterion-tagged-outside-row` and no
`trace-id-outside-marker`. Every new test passes. Spec-review was not run as a separate
method; per the brief, the spec checks are folded in here.

## Verdict

**Gaps found (one high, two medium).** AC-21, AC-22 and AC-24 are asserted against real
captured output, and AC-7 is asserted both with and without `--discharges`. The gaps are
places where the spec states a property the code does not have. An AC is missing for the
summary-only failed run. AC-8 is untested on its empty edge. And the run-path behaviour
change is specified for SBOM only.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | FR-034's new prose says "The summary is the proof the scan executed" and that a summary with no diagnostic is a clean scan. Real cargo-deny 0.19.8 prints a summary carrying `errors: 1` and no diagnostic when run with `--log-level off` (exit 4). The spec claim is false, and no AC requires the summary's counts to agree with the diagnostics read. Add an AC (for example FR-034-AC-25: a summary whose error/warning counts exceed the diagnostics read is refused) and a test using that real line | spec/functional/FR-034-finding-shaped-evidence.md:77-83 |
| FND-002 | medium | FR-041-AC-8 is tested only with a non-empty inventory. With zero components an unknown id is silently dropped (`unmatched: []`, verified by probe), so the AC fails on its edge and no test covers it | spec/functional/FR-041-sbom-inventory-evidence.md:95 |
| FND-003 | medium | The run-path `--discharges` injection applies to every run adapter, but only FR-041 (SBOM) states it. No requirement covers `--discharges` on a junit, entries or agent-eval recording, where the flag was ignored before this PR and now binds the stated obligation to every passing entry. Either scope the behaviour to sbom in code, or state it (and its red-suite consequence) in the FR that owns `evidence record` and add an AC and test | rust/crates/quoin-core/src/ops/evidence/records.rs:210-226 |
