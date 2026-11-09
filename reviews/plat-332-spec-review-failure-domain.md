---
id: SR-16325
title: "Failure-domain review of PLAT-332 trace-mark spec"
type: SpecReview
analysis: failure-domain
scope: "agent-ix/quoin; spec/adrs/ADR-001-execution-evidence-is-emitted-by-the-run.md; spec/functional/FR-120-trace-marks-in-test-result-output.md; spec/functional/FR-121-emitter-mechanism-constraints.md; spec/non-functional/NFR-028-inert-and-low-cost-emission.md; spec/non-functional/NFR-029-self-contained-result-evidence.md; spec/stakeholder/StR-009-accepted-software-verified-by-tests-that-ran.md; spec/usecase/US-023-record-a-run-that-names-its-own-requirements.md; spec/adrs/index.md; spec/*/index.md; spec/spec.md"
review_set: subset
---

## Summary

Ticket: PLAT-332. PR agent-ix/quoin#773 (spec-only, head f1662cc3), scoped to `git diff origin/main...HEAD`.
The failure-domain lens examined ADR-001, StR-009, US-023, FR-120, FR-121, NFR-028 and NFR-029, reading FR-033 and FR-032
for context. It records 3 findings (0 high).

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Parametrized-test rule ('parameter suffix removed') is undefined for rstest, proptest and vitest/jest .each interpolated titles | spec/functional/FR-120-trace-marks-in-test-result-output.md:62-64 |
| FND-002 | low | Rust symbol rule names only mod nesting; the extractor also qualifies by impl target type, struct/trait and enclosing fn | spec/functional/FR-120-trace-marks-in-test-result-output.md:56 |
| FND-003 | low | Outcome rule does not cover retried/flaky cases (nextest flakyFailure/rerunFailure children) | spec/functional/FR-120-trace-marks-in-test-result-output.md:41 |

## Detail

- **FND-001** (FR-120): The rule assumes a pytest-style [suffix]. rstest reports cases as name::case_N. vitest/jest it.each / test.each titles are templates ('%s', '$a') that the runner interpolates, while the quire-rs TypeScript extractor records the literal registered title. The emitter must report the source title, not the runtime one. The contract does not say this, so emitters will diverge on exactly the cases ADR-001 says must be settled once.

- **FND-002** (FR-120): quire-rs src/symbols/rust.rs qualifies through every container: mod, struct, enum, trait, impl target type (Foo::fmt) and enclosing fn (PLAT-845: a::helper). proptest!-declared tests qualify under their container. 'In-file mod nesting' understates this. Say 'the extractor's container chain joined by ::'.

- **FND-003** (FR-120): nextest writes <flakyFailure>/<rerunFailure> children for retried tests. The outcome table maps only error/failure/skipped, so a test that passed on retry reads as pass with no stated decision. State whether passed-on-retry counts as pass.

## Verdict

No blocking findings from this lens.
Clean units (examined, no finding from this lens): every other unit listed in the reviewer comment scope.
