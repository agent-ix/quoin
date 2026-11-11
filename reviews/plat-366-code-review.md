---
id: SR-20261111195247
title: "Code review of quoin PR 812 (PLAT-366 audit-script tool identity, FR-036 AC-7/AC-8 checks)"
type: SpecReview
analysis: base
scope: "agent-ix/quoin@3a2c4228; rust/crates/quoin-evidence/src/adapters/audit_script.rs, rust/crates/quoin-core/src/ops/evidence/tests.rs, rust/crates/quoin-core/tests/tc_2901_process_executables.rs, rust/crates/quoin-core/tests/tc_workspace_dependency_graph_is_acyclic.rs"
review_set: subset
---

# Code review of quoin PR 812

## Summary

Ticket: PLAT-366. This review covers `git diff origin/main...HEAD` at 3a2c4228 only. It is a
code-review with the rust-review lane folded in. quoin's repository rust-style skill and the
dev-tools rust-style baseline were both loaded. Tools: quoin 0.30.0. `cargo clippy -p quoin-core
-p quoin-evidence --all-targets --all-features -D warnings` is clean. `cargo test -p quoin-core -p
quoin-evidence --all-features` passes every test the PR adds. Three `tc_446_*` reconcile tests fail
locally. That is an environment fault: the shared node_modules has semantic-core 0.3.0 but the lock
needs 0.4.3. Those tests are unrelated to this diff.

`tool: None` from the audit-script reader: the recorded scan's `tool` is always `request.tool`
(records.rs:133). `result.tool` is read in only two places, the identity check (records.rs:110) and
the `parse_results` payload (documents.rs:171, an `Option`). The auditor uses `scan.tool` only as a
display label (`quoin-auditor/src/audit/ladder.rs:243`). Mutation-score scoping filters on the
entry's `metric` and never on the tool (`audit/scores.rs:17-25`). Catalog `tooling` is advise-only.
No reader treats `None` as a wildcard. No binding, freshness or ladder decision keys on the tool.

Arbitrary `--tool` through `--adapter audit-script`: a probe added `("semgrep 1.2.3",
Some("audit-script"))` and `("cargo-deny", Some("audit-script"))` to TC-2900. Both now record. This
is accepted. The operator chose the adapter explicitly. A SARIF document with no driver name already
behaves the same way (sarif.rs:113). The stored tool is the operator's declaration, and nothing
downstream credits evidence by tool name. Without `--adapter`, `--tool semgrep` still selects SARIF,
which is held to its own identity.

## Verdict

**PASS WITH FINDINGS (one medium, two low).** The audit-script change is minimal and correct, and the
identity check is left as it was. TC-202 runs `cargo metadata --offline --locked` and is
deterministic. Both new checks have injected-violation tests. TC-203 catches the common regression, a
new `Command::new("x")` in a file that imports `std::process::Command`. However, it silently skips
whole classes of production code (FND-001). TC-202 can pass vacuously when the checkout path contains
`quoin-cli` (FND-002). Skipping `quoin-suite-trace` and `quoin-workflow-check` is principled: both are
Makefile-run developer tools and neither ships in the `quoin` binary.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | TC-203's `executables_in` silently drops production executors instead of failing on them. (a) `source.split("#[cfg(test)]")` cuts off everything after the first textual occurrence of `#[cfg(test)]`, wherever it appears. In real sources that means quoin-cli/src/help.rs from line 14 (a cfg(test) import), quoin-auditor/src/catalog/source.rs from line 225 (a doc comment that mentions it) and quoin-core/src/ops/evidence/records.rs from line 464 (a cfg(test) method). (b) `use std::process::{Stdio, Command};` is not recognised as importing `Command`, because only `{Command,` is matched. (c) Any alias other than the hard-coded `ProcessCommand` is missed, for example `as Cmd`. A probe of `executables_in` returned `{}` for all four cases with `Command::new("curl")`. FR-036 states that a check which silently ignores what it cannot read is asserting over a set it cannot see. AC-8 claims the whole set. Fix: skip only `mod tests` blocks or cfg(test) items, not the rest of the file. Treat any `Command::new` in a file that imports anything from `std::process` as a process command. Report any unrecognised `Command::new` site as unreadable rather than dropping it | rust/crates/quoin-core/tests/tc_2901_process_executables.rs:45-56 |
| FND-002 | low | `cli_dependents` identifies the CLI with `id.contains("quoin-cli")` over cargo package ids, which embed the absolute checkout path (`path+file:///…/rust/crates/quoin-core#…`). In a checkout whose path contains `quoin-cli`, such as a worktree named `quoin-cli-fix`, every crate counts as the CLI. The check then returns `[]` and passes vacuously, and so does the "walk must reach the CLI crate" guard. A probe confirmed this. Fix: match the package name from `packages[].name`, or the id's final path segment before `#` | rust/crates/quoin-core/tests/tc_workspace_dependency_graph_is_acyclic.rs:133 |
| FND-003 | low | The new file is named `tc_2901_process_executables.rs` but holds TC-203. No TC-2901 row or tag exists. The name points at an unrelated id, which invites a collision or confusion when TC-2901 is later allocated. The repo's rust-style skill also names `tc_source_conventions.rs` as the legitimate home for source-inspection tests. Fix: rename the file (for example `tc_process_executables.rs`) or move the test into `tc_source_conventions.rs` | rust/crates/quoin-core/tests/tc_2901_process_executables.rs:1 |
