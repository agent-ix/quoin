---
id: SR-17080
title: "Gap analysis of quoin PR 816 (PLAT-1403 FR-131-AC-13 and AC-14)"
type: SpecReview
analysis: base
scope: "agent-ix/quoin@a0b62b2c; spec/functional/FR-131 (AC-13, AC-14), spec/matrix.md (FR-131 row, TC-3300..TC-3302), rust/crates/quoin-cli/tests/{tc_3003_nfr_populations.rs,nfr_populations/span_breadth.rs}, rust/crates/quoin-jev/tests/{tc_3302_gate_observations.rs,support/gate.rs}"
review_set: subset
date: '2026-11-11'
---

# Gap analysis of quoin PR 816

## Summary

Ticket: PLAT-1403. Plan completion was not assessed. I ran `quoin matrix --repo . --json` with quoin 0.30.0 on the PR head. The command wrote nothing to stderr and reported no criterion tagged outside a row.

- **FR-131-AC-13** is `tagged`. Its binders are the two TC-3300 functions and TC-3301. The matrix marks the TC-3301 binder `ignored: true`.
- **FR-131-AC-14** is `tagged`. Its binders are the two TC-3302 functions.
- **Evidence.** Both criteria read "no run evidence", like every other criterion in this repository's static matrix. An ignored test therefore does not count as evidence for AC-13. Its row in spec/matrix.md states that it needs `QUIRE_RS_CHECKOUT`, which is honest.
- **Id collisions.** TC-3300..3302 and FR-131-AC-13/14 are absent from origin/main, and #816 is the only open PR, so nothing collides. The PLAT-1404 range TC-3500..3599 is not touched.
- **Size census.** The module-size census reads `src/` only, and this PR changes only tests.
- **Command surface.** The PR adds no operation or route, so TC-1650 and the census are unaffected.

## Verdict

Both new criteria are bound to tests that would fail under the main regressions:

- **AC-13.** TC-3300's negative controls cover an invented precondition, a drifted statement, a wrong refusal and a withdrawn row. Its refusal test checks the case where the quire-rs checkout is missing.
- **AC-14.** TC-3302 covers a skipped bar, an invented bar and an unmet bar.

There are two gaps:

- **AC-14's "validates before it is kept"** has no test (FND-001).
- **AC-13's "statement unchanged"** is never tested against the label normalization the real file uses (FND-002). That gap is how code-review FND-001 shipped.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-131-AC-14 says the producer validates the observation before it is kept. `gate::record`, the only function the live gates call, admits the observation and then writes it under `QUOIN_GATE_COLLECTION_OUT`. No default-gate test calls it, because TC-3302 calls `collection` and `admit` directly. A `record` that wrote before validating, or never validated, would pass every default test. Add a TC-3302 case that calls `record` with a temp `QUOIN_GATE_COLLECTION_OUT`: it should write for a valid run and panic without writing for a skipped bar | rust/crates/quoin-jev/tests/support/gate.rs:101-115, rust/crates/quoin-jev/tests/tc_3302_gate_observations.rs:40-86 |
| FND-002 | medium | FR-131-AC-13 matches only when "the statement is unchanged". Every TC-3300 fixture label sets `normalizedStatement` to the plain lowercase form of the statement, so the test never covers a label whose stored normalization differs, as the real `quire-rs-FR-039-AC-5` does. The resulting false non-match (code-review FND-001) passes every test. Add a fixture label whose statement contains a character that the labeller's normalization folds, and assert it matches | rust/crates/quoin-cli/tests/tc_3003_nfr_populations.rs:96-174 |
