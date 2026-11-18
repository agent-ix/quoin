---
id: SR-20261117175255
title: "Code review — retained assurance help"
type: SpecReview
analysis: code-review
scope: "agent-ix/quoin; rust/crates/quoin-cli/tests/fixtures/retained-command-help.json, rust/crates/quoin-cli/tests/tc_retained_help_capture.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quoin/FR-205
    type: references
  - target: ix://agent-ix/quoin/FR-102
    type: references
---

## Summary

Reviewed the retained assurance help update and its subprocess regression against FR-205 and the retained command-surface contract. The fixture change matches the CLI's existing option, but the new test is traced to a criterion it does not exercise.

## Verdict

**CONDITIONAL** — fix the inaccurate trace binding before merge.

## Review Scope

- **Examined FR-205-AC-1:** “Given a file with exactly one valid FR-071 record for each selected Quire criterion, `quoin assurance --formalization-records <FILE>` presents each matched record under its owning requirement without changing the record value.”
- **Examined FR-102-AC-1:** “Native fixture tests replay the retained command surface before shell replacement.”
- The `--formalization-records` option already exists in the Clap parser and the updated retained-help capture accurately describes the FR-071 input.

## Test Evidence

- `cargo fmt --all --check`: passed.
- `cargo test -p quoin-cli --test tc_retained_help_capture tc_205_assurance_help_exposes_formalization_records --locked`: passed (1 test).
- The complete `tc_retained_help_capture` target failed 3 of 7 tests: two root-help replay assertions and the `not-a-command` stderr replay. The candidate changes only the assurance-help capture and its new assertion; the failing root-help and unknown-command capture entries are unchanged from the PR base and appear pre-existing.
- `cargo clippy -p quoin-cli --all-targets --all-features --locked -- -D warnings`: passed.
- `cargo deny check`: unavailable; it failed to acquire `/home/peter/.cargo/advisory-dbs/db.lock` because that path is read-only in the review environment. The PR changes no dependencies.
- Inspected the computed Quire matrix for FR-205-AC-1. It lists the new help-only test as a binder, alongside the existing record-rendering test.
- Full gap-analysis was not applicable to this diff: it changes only a retained-help fixture and a test, with no production implementation.

## Findings

| ID      | Severity | Summary                                                                                                                        | Refs                                                        |
| ------- | -------- | ------------------------------------------------------------------------------------------------------------------------------ | ----------------------------------------------------------- |
| FND-001 | low      | The help-only test is tagged as evidence for FR-205-AC-1, which requires consuming and presenting valid records.               | rust/crates/quoin-cli/tests/tc_retained_help_capture.rs:224 |
