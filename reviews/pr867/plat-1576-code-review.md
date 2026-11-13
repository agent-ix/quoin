---
id: SR-20261113125829
title: "Code review and Rust review of quoin risk-summary (PR 867, FR-169, FR-170)"
type: SpecReview
analysis: code-review
review_set: subset
scope: "agent-ix/quoin; PR #867 (git diff origin/main...HEAD at 6fd570a5): rust/crates/quoin-risk-advice/src/{summary.rs,lib.rs,records.rs}, rust/crates/quoin-risk-advice/tests/{tc_169_risk_summary.rs,snapshots/risk_summary.json,snapshots/risk_summary.md}, rust/crates/quoin-cli/src/{main.rs,risk.rs,help.rs,tests.rs}, rust/crates/quoin-cli/tests/{tc_169_risk_summary_command.rs,tc_shell_contract.rs,fixtures/retained-command-help.json}, spec/assurance/{MP-250,MP-251}"
relationships:
  - {target: "ix://agent-ix/quoin/FR-169", type: references}
  - {target: "ix://agent-ix/quoin/FR-170", type: references}
---

# Code review and Rust review of quoin risk-summary (PR 867)

## Summary

Ticket: PLAT-1576. One code-review file, with the rust-review lane folded in. quoin's own
`rust/` rust-style skill was applied first, then the ecosystem rust-review and rust-style
checklists.

What was checked and holds:

- JSON is one object with exactly `attribution` and `rows`. `unreadable` is `serde(skip)`.
  Row keys, estimate-entry keys, control-entry keys and residual-entry keys equal the FR-169
  tables exactly. Every key is always present (`null` or `[]`).
- Row order is hazard id, then situation id. A hazard with no situation gets one
  `situation: null` row. The `hazard: null` rows come last, ordered by situation. Every list
  comes from a BTreeMap or BTreeSet, so it is ascending by id. Findings sit in a
  `BTreeSet<RowFinding>`. The `RowToken` enum is declared in FR-170 table order, so the derived
  `Ord` gives table order, then `names`. Output is deterministic.
- `policy_region` uses the shared `Links::policies_of` and `policy::lookup`, the two calls
  `policy::cells` makes for `quoin risk advise`. Residual-risk findings come from
  `discrepancies::build` and are mapped token for token, so they are not reimplemented.
  Control `requirements`/`verifications` come from the shared `Links` maps. `summarize` uses
  the shared `load()` with `records::CHECKED` and `refuse_unreadable = false`, the same call
  `advise` makes. So the error codes match `risk advise`, and an unreadable file is a
  diagnostic, not a stop.
- Read list: `Records::read` keeps every document in `others`, with its `satisfies` and
  `verifies` edges, whatever its type. A retained-evidence record of any type therefore counts
  as a verification. Edges come from frontmatter `relationships` only.
- `decided_by` (coder's call 5): the safety manifest declares a `Decided by` column in the
  `Determination` table. FR-103 makes the schema require a non-empty `decided_by` for
  `accepted`/`not_accepted` and reject one for `undecided`. FR-103's "when and by whom from
  version control" rules out typed history and dates, not this declared column. Reading the cell
  as written, with an empty cell as `null`, is consistent with FR-169 and FR-103.
- Exit code branch (coder's call 1): `usage_refused` matches the routed family name
  `risk-summary` exactly. Only Clap parse errors that are not help, invalid-subcommand or
  missing-required take it. Probed with the built binary: `risk-summary --bogus` exits 2,
  `risk advise --bogus` still exits 3. `--format xml` exits 2 with an empty stdout
  (tc_169 command test). No other family's exit behaviour changes.
- MP-250/251 and root text (coder's call 2): the NFR-003 probe lists cases from the
  top-level help, so the new family adds exactly one `risk-summary:unknown-flag` case. The root
  catalogue lines in help.rs, tests.rs, tc_shell_contract.rs and the retained root-help fixture
  are the same one-word addition. This is legitimate and minimal. The census walker that skips
  nested commands is the known, unrelated PLAT-1683.
- Residual risk on a non-row estimate (coder's call 3): the FR-169 row table restricts
  `residual_risks` to records whose `estimate` edge names a `post_control` estimate of the
  situation. FR-169 makes skipped estimates silent. FR-103 rejects a residual risk on a
  pre-control estimate at validation. Showing such a record in no row is consistent with FR-169.
- Read-only: nothing opens a file for writing (AC-10 tests compare the tree before and after).
  No score/rank/verdict/count key. The stem "accept" appears only in the attribution, the
  tokens and copied values.
- Snapshots: 8 lines of Markdown and 165 lines of JSON, compared byte for byte with no update
  path, so the test can fail. They are a legitimate functional artifact (FR-169-AC-19). Their
  coverage gap is in the gap-analysis SR.
- Rust: no `unwrap`, `expect`, `panic` or indexing in production code. Public items are
  documented. `let _ = writeln!(String, ..)` is the crate's infallible-write idiom.
  `cargo fmt --check` is clean.

Focused tests were run at 6fd570a5 against a temporary spec-objects-safety origin/main worktree
(bf44b49): quoin-risk-advice tc_169 (18), quoin-cli tc_169 command (4) and tc_shell_contract
(3), all passed. The full gate was not rerun. The coder's results on this head are rust-lint 0,
rust-deny 0, workflow-test 0, and the workspace tests with only the known unrelated failures.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Markdown cells print copied values unescaped, so a pipe character in a value splits the row (coder's call 4). Reproduced: a residual risk whose `Decided by` cell is authored as the escaped pipe form of "Pat, pipe, Lee" is read back with a bare pipe. The row then prints `decided_by=Pat` and `Lee` as two cells, which gives 9 cells under an 8-column header. This breaks FR-169's "one table whose columns are the row keys". FR-169 is silent on escaping. Escape a pipe as backslash-pipe in every copied value (ids, levels, decider, region names), add a test, and add one FR-169 line saying so. | rust/crates/quoin-risk-advice/src/summary.rs:436-503 |
| FND-002 | low | FR-170 says "the command computes the finding with the code those two commands use, so one fact has one implementation". summary.rs restates three rules from evidence.rs: the `kind == control` filter, `post-control-without-control` when no control is assumed, and `control-without-requirement`/`-verification` when the shared lists are empty. They agree today. Factor the per-control evidence and the assumed-control filter out of evidence.rs and call them from both. | rust/crates/quoin-risk-advice/src/summary.rs:328-366, rust/crates/quoin-risk-advice/src/evidence.rs:84-131 |

## Verdict

Not mergeable as is: one medium finding and one low. The command implements the FR-169/170
contract, and reuse of the reader, the lookup and the discrepancy rules is real. The exit-code
branch is narrow and tested. The MP and root-text edits are minimal. The Markdown pipe defect is
the one behaviour fix needed.
