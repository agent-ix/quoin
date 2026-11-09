---
id: SR-17045
title: "Code review of quoin PR 781 (FR-071-AC-3..AC-8 over quire-fixtures)"
type: SpecReview
analysis: code-review
scope: "agent-ix/quoin@f17b3c2d; rust/crates/quoin-semantic/tests/tc_278_mapping_expectations.rs"
review_set: subset
---

# Code review of quoin PR 781

## Summary

Ticket: PLAT-278. Scoped to `git diff origin/main...HEAD` at f17b3c2d (one commit). The Rust
review lane (rust-review, with the repo's `.claude/skills/rust-style`) is folded in here.
Tools: quoin 0.29.1.

Checks run: `cargo test -p quoin-semantic --test tc_278_mapping_expectations` passed 6 of 6,
built against `@agent-ix/semantic-core` 0.3.2, the version `package.json` pins. The PR author
measured against a stale 0.3.0 install. `cargo clippy -p quoin-semantic --tests -- -D warnings`
and `cargo fmt --check` are clean. Fixtures are read only through `quire_fixtures::mapping_json`
and `mapping_fixture`, which panic when a file is absent, so every test fails without the
fixture. No fixture is copied into quoin. Each assertion compares a fixture value to a literal,
so none of them is vacuous. The file-level `#![allow]` carries the repo's base set with a
`reason`, the SPDX header is correct, and every test carries a `/// Trace:` line with one
criterion id.

## Verdict

**PASS WITH FINDINGS (one medium, two low).** The tests are sound. The module doc states the
wrong cause for the AC-1 blocker. That is the same misdiagnosis the FR-071 note carries (see the
spec-review artifact for this ticket).

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The module doc calls the tagged `{kind, value}` shape a "fixture-owner defect" that holds until `quire-fixtures` is regenerated. That is wrong. The shape is semantic-core's current adjacent wire form: filament-core-data IR-994, PR #310, commit 7e6f209e. On that commit `TypeRefTarget.json` and `ConstraintDecl.json` are `oneOf` over `{kind, value}`. quoin's pin (0.3.2 in `package.json`, not "0.3.0") predates it, and the pin is the blocker. The doc also calls the npm-installed schemas "vendored" | rust/crates/quoin-semantic/tests/tc_278_mapping_expectations.rs:13-19 |
| FND-002 | low | The module header says "FR-071-AC-1 and AC-3 to AC-8, criterion by criterion", but no test in the file traces FR-071-AC-1 | rust/crates/quoin-semantic/tests/tc_278_mapping_expectations.rs:4 |
| FND-003 | low | `semantic_core_validator` rebuilds the semantic-core path by hand (`schema_dir().join("semantic-core")`), although `quoin_semantic::contract::semantic_core_dir` already exists. It also clones every schema document twice: once into `resources`, then again into `borrowed` | rust/crates/quoin-semantic/tests/tc_278_mapping_expectations.rs:39-57 |

## Examined scope

```yaml
clean: false
scope:
- {id: tc_278_003_both_forms_fails_at_the_second_forms_locus, path: rust/crates/quoin-semantic/tests/tc_278_mapping_expectations.rs, role: examined, excerpt: "Trace: FR-071-AC-3"}
- {id: tc_278_004_type_cells_resolve_in_the_specified_order, path: rust/crates/quoin-semantic/tests/tc_278_mapping_expectations.rs, role: examined, excerpt: "Trace: FR-071-AC-4"}
- {id: tc_278_005_multiplicity_cells_map_and_invalid_cells_are_errors, path: rust/crates/quoin-semantic/tests/tc_278_mapping_expectations.rs, role: examined, excerpt: "Trace: FR-071-AC-5"}
- {id: tc_278_006_constraint_cells_map_to_the_closed_vocabulary, path: rust/crates/quoin-semantic/tests/tc_278_mapping_expectations.rs, role: examined, excerpt: "Trace: FR-071-AC-6"}
- {id: tc_278_007_fence_lines_outside_the_subset_are_errors_with_locus, path: rust/crates/quoin-semantic/tests/tc_278_mapping_expectations.rs, role: examined, excerpt: "Trace: FR-071-AC-7"}
- {id: tc_278_008_a_reader_rule_violation_is_reported_at_the_row, path: rust/crates/quoin-semantic/tests/tc_278_mapping_expectations.rs, role: examined, excerpt: "Trace: FR-071-AC-8"}
- {id: tests/common/mod.rs, path: rust/crates/quoin-semantic/tests/common/mod.rs, role: context_only, excerpt: "pub fn mapping_json(name: &str) -> Value { quire_fixtures::mapping_json(name) }"}
```
