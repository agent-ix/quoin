---
id: SR-20261112201001
title: "Code review of quoin PR 833 (pack-level requirements mapping report)"
type: SpecReview
analysis: code-review
scope: "agent-ix/quoin@4e7ad271; rust/crates/quoin-assurance/src/discharge/{pack_rmm,rmm,mod}.rs, rust/crates/quoin-assurance/src/lib.rs, rust/crates/quoin-cli/src/{core_runtime,discharge}.rs, rust/crates/quoin-cli/Cargo.toml, rust/crates/quoin-cli/tests/{tc_1488_pack_requirements_report.rs,fixtures/retained-command-help.json}, rust/crates/quoin-core/src/{capabilities,dispatch,runtime}.rs, rust/crates/quoin-core/src/ops/evidence/{discharge,mod}.rs, rust/crates/quoin-core/src/ops/modules/support.rs, rust/crates/quoin-core/tests/tc_458_evidence_boundary.rs, rust/crates/quoin-quire/src/clauses.rs"
review_set: subset
---

# Code review of quoin PR 833

## Summary

Ticket: PLAT-1488. The Rust review lane (rust-review with the repo's `.claude/skills/rust-style`)
is folded in. Tools: quoin 0.30.1. Focused tests on one target dir via locked-build.sh (deleted
after): tc_1488_pack_requirements_report (12 pass), quoin-cli discharge unit tests,
tc_retained_help_capture, tc_3206, tc_2606, quoin-core tc_458_evidence_boundary (23) and
tc_447_operation_census (8) all pass. The retained `discharge --help` fixture is byte-identical to
the built binary's `--help`. A manual probe installed a licensed sentinel pack and broke its
`text`, `title` and set `title` fifteen ways (quoted-scalar EOF, mapping-in-scalar, sequence,
duplicate field, unknown anchor, `!!int`/`!!float`/`!!bool`/custom tags, rights `none`, missing
set file). No sentinel reached stdout or stderr; the engine errors carry file path, serde field
path and YAML position only. Only a non-licensed enum value (`force`) is echoed.

## Verdict

Correct on every main path. Pack-ness is only the FR-137 `is_pack` derivation; no rights rule is
restated. Each matrix is built and rendered by the FR-135 `build_requirements_mapping` /
`render_requirements_mapping` verbatim, and `escape` widened only to `pub(super)`. Sets are sorted
byte-wise on `authority/id@version`. `clause_set_refs` and `evaluate_clause_sets` propagate every
error through `collect::<Result<_>>`, and a missing or malformed set file refuses the whole report
with exit 2 and empty stdout, so nothing fails open. Non-restricted sets in a pack are included.
Refusals reach stderr only. The flag checks moved into code exit 1, as FR-135-AC-7 does. Nothing
is written (AC-6 oracle with a real control). tc_1488_100 is a real wire-name probe that the
existing source-reading census requires, not a hand-kept list. Four low defects, all in tests or
efficiency.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | tc_1488_013 is tagged with a `/// Trace: FR-140-AC-12` doc comment, not `#[trace(...)]`, and only checks that each flag name occurs in the fixture. tc_1650 already compares the fixture byte for byte with the real `--help`, and tc_1488_012 covers AC-12. Delete it, or tag it with `#[trace]` | rust/crates/quoin-cli/src/discharge.rs:420-440 |
| FND-002 | low | Sentinels are keyed on `clause.len()`, so Z-1/Z-2, A-1/A-2 and M-1 share one text and one title sentinel. AC-5 asks for a unique sentinel per field. The search still finds a leak, but cannot tell which clause leaked. Key them on the clause index | rust/crates/quoin-cli/tests/tc_1488_pack_requirements_report.rs:73-79 |
| FND-003 | low | The blank-rationale leg of tc_1488_009 asserts only non-success and a `rationale` substring. It does not pin the FR-135-AC-6 refusal (exit 1 and the same refusal kind as the single-set path), so a different failure would pass. Assert the status and compare with the single-set refusal | rust/crates/quoin-cli/tests/tc_1488_pack_requirements_report.rs:691-700 |
| FND-004 | low | `evaluate_clause_sets` loads the module strictly once in `clause_set_refs`, then again inside `clauses::evaluate` for every set (N+1 strict loads), after `is_pack` has already loaded it. Evaluate every set from the one loaded registry | rust/crates/quoin-core/src/runtime.rs:658-684 |
