---
id: SR-20261111200815
title: "Code review of quoin PR 814 (requirements mapping matrix view)"
type: SpecReview
analysis: base
scope: "agent-ix/quoin@6ed10483; rust/crates/quoin-assurance/src/discharge/{rmm,mod}.rs, rust/crates/quoin-assurance/src/lib.rs, rust/crates/quoin-cli/src/discharge.rs, rust/crates/quoin-cli/tests/tc_3206_requirements_matrix_command.rs, rust/crates/quoin-cli/tests/fixtures/retained-command-help.json, rust/crates/quoin-core/src/dispatch.rs, rust/crates/quoin-core/src/ops/evidence/{discharge,mod,tests}.rs, rust/crates/quoin-core/tests/tc_458_evidence_boundary.rs"
review_set: subset
---

# Code review of quoin PR 814

## Summary

Ticket: PLAT-307. Scoped to `git diff origin/main...HEAD` at 6ed10483. The Rust lane
(`rust-review`, with the repo's `rust-style`) is folded in. Builds went through
locked-build.sh with debug=0 and jobs=4. Builds ran in a fresh worktree at the PR head, 6ed10483.
`cargo test --locked -p quoin-assurance -p quoin-core -p quoin-cli` passed 537 tests
and failed 0. That includes the rmm unit tests, tc_3205, tc_3206, tc_1650 retained
help and route census, tc_447 operation census and tc_90 module sizes.
`cargo clippy -p quoin-assurance -p quoin-core -p quoin-cli --all-targets
--all-features -D warnings` is clean, and so is `cargo fmt --all --check`.

These parts are clean. Rows iterate `binding.clauses`, so row order is the binding
report's order. A doubled clause is refused by `build_discharge_report` and by
`unique_clauses` before any row is built (tc_3205 asserts exit 3). An expired or
not-yet-valid fact lands in `binding.open` with `fact: Some`, which maps to
`AttestationNotCurrent`. Evidence and tailoring cells are filled only when the
compliance is `Compliant` or `Tailored`. So an expired, undischarged or unresolved
clause can never render as Compliant. Stored facts are re-parsed through `parse_fact` on
every read (`discharge_report_from_store`), so a disposition with an empty or
whitespace-only rationale refuses the whole operation (CORE_BAD_REQUEST). It never
produces a Tailored row without a rationale. The new op is in `OPERATIONS` and in the
exhaustive dispatch `match`. Its request is the `deny_unknown_fields`
`DischargeReportRequest`, which is bounded by `MAX_DISCHARGE_BYTES` (the tc_412 bound
table and the no-host table are both extended). The payload is a `Serialize` struct.
The retained-help fixture edit matches the new clap help byte for byte. rmm.rs is 472
lines, under the 700-line ceiling in `tc_90_module_sizes`.

Output size is bounded only by what FR-133 already bounds. Rows are at most the clauses
of a request of 8 MiB or less. Each cell's source fact is 1 MiB or less, and there are
at most 10,000 stored facts. The matrix adds roughly a 2x copy (struct plus render). It
introduces no new unbounded accumulator.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Rendered Markdown escapes only `\|` and CR/LF inside table cells. Untrusted text from stored facts and the binding report passes through as live Markdown and HTML (images or tracking pixels, links labelled "Compliant", raw `<img>`). The title line and the unused-facts list interpolate clause-set fields and fact clause ids unescaped, so a newline there injects arbitrary blocks, including a fake table. | rust/crates/quoin-assurance/src/discharge/rmm.rs:225-258, 291-302 |
| FND-002 | low | The fallback row for a binding clause missing from the report hardcodes Applicability `Applicable` whatever `clause.outcome` says. With a mismatched (binding, report) pair passed to the public fn, the row mislabels a not-binding or unresolved clause. Report entries absent from the binding are dropped silently. | rust/crates/quoin-assurance/src/discharge/rmm.rs:147-162 |

## Verdict

**MERGEABLE AFTER FND-001 IS FIXED.** Fix: one inline-escape function applied to every
interpolated string (title fields, each cell, unused-fact clause ids). It maps every
control character, including `\n` and `\r`, to a space and backslash-escapes every
CommonMark ASCII punctuation character that can open markup: `` \ ` * _ [ ] ( ) < > ! # | & ~ ``.
Escape the backslash first so `\|` cannot be re-paired. Drop the backtick code span
around the unused-fact clause id, or escape inside it. Extend TC-3204 with a rationale
of `![p](https://x/pixel) <img src=x> [Compliant](https://e) a\|b`, a newline in
`clauseSet.authority`, and an unknown-clause fact id containing a backtick and a newline.
Assert the literal escaped row and title bytes.

Probe: markdown-it (GFM tables) rendered a rationale cell from the current escaping as
`<img src="https://attacker.example/pixel">`, raw `<img src=x>`, and
`<a href="https://e">Compliant</a>`. The `\|` input rendered unsplit but lost its
backslash (`X\|A` displays `X|A`). The row did not split, so that is not a row-split
bypass. FND-002 should derive the fallback applicability from `clause.outcome`
(or refuse a mismatched pair); it is low because that case cannot occur through the op.
