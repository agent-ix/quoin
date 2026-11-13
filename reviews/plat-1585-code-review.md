---
id: SR-20261113104838
title: "Code review (Rust lane) — PLAT-1585/PLAT-1586 quoin anomaly report and --fail-on gate (quoin PR 865)"
type: SpecReview
analysis: code-review
scope: "agent-ix/quoin; rust/crates/quoin-anomaly/src/{check,derived,history,lib,record,report}.rs, rust/crates/quoin-anomaly/src/report/render.rs, rust/crates/quoin-anomaly/tests/{common/mod.rs,tc_136_anomaly.rs,tc_162_anomaly_report.rs}, rust/crates/quoin-cli/src/{anomaly,tests}.rs, rust/crates/quoin-cli/tests/{tc_162_anomaly_report_command.rs,fixtures/retained-command-help.json,snapshots/anomaly-report.json,snapshots/anomaly-report.md}, spec/assurance/MP-250-nfr003-actionable-failures.md, spec/assurance/MP-251-nfr003-silent-failures.md"
review_set: subset
relationships:
  - {target: "ix://agent-ix/quoin/FR-162", type: references}
  - {target: "ix://agent-ix/quoin/FR-163", type: references}
  - {target: "ix://agent-ix/quoin/FR-164", type: references}
  - {target: "ix://agent-ix/quoin/NFR-035", type: references}
  - {target: "ix://agent-ix/quoin/FR-136", type: references}
---

## Summary

Tickets: PLAT-1585 (report, FR-162, FR-163, NFR-035, FR-136 AC-13..16) and PLAT-1586 (gate,
FR-164). PR head `a64728e3`, two commits, diff `git diff origin/main...HEAD` (18 files). The Rust
lane (repo `.claude/skills/rust-style/SKILL.md`, then ecosystem rust-review) is folded in. Tools:
quoin 0.30.1, quire 0.44.0 (engine 0.59.1), sibling spec-artifacts-process at `ed574c9` (contains
merged PR #148).

Focused checks on this head, each through `locked-build.sh` with one target dir under the
worktree: `cargo test -p quoin-anomaly --test tc_162_anomaly_report --test tc_136_anomaly`
(27 + 15 passed), `cargo test -p quoin-cli --test tc_162_anomaly_report_command --test
tc_retained_help_capture --test tc_3003_nfr_populations --test tc_136_anomaly_command` (9 + 6 +
13 + 5 passed, 1 ignored needing a quire-rs checkout), `cargo test -p quoin-cli --bin quoin -p
quoin-anomaly --lib` (101 passed), `cargo clippy -p quoin-anomaly -p quoin-cli --all-targets
--all-features -D warnings` (clean), `cargo fmt --check` on both (clean). The built `quoin` binary was run over hand-built git
repositories holding the shipped fixtures.

FND-001 is reproduced. The repository `r1` added AN-004 (`rationale-resolved-with`), then AN-005,
then AN-006 (`closed-wont-fix-rationale`), one commit each. `git log main --first-parent --follow
--name-status -- spec/anomalies/AN-006.md` prints `C064 spec/anomalies/AN-004.md
spec/anomalies/AN-006.md` and then walks into AN-004's own add commit. The repository `r2`
committed `closed-fixed` as AN-001, then added `rationale-assessed-with` as AN-002.
`quoin anomaly check spec` then reports
`AN-002.md:5: error anomaly.illegal-transition ... closed (994d4a53) to assessed (53d38d28)`, a
move AN-002 never made.

## Verdict

Changes requested: one high and five low findings, all fixable inside this PR.
The report shape, selection, grouping, gate logic, exit taxonomy (Partial 1 carrying a full
payload; parser refusal 3 through the shared `EXIT_INVALID` arm) and the never-writes property are
correct. No `unsafe`, no production `unwrap`/`expect`/indexing, the test `#![allow]` blocks carry
reasons, every new `.rs` file has the AGPL-or-later header, and no digest, pin or count test is
added. Determinism holds by construction: `anomaly_documents` sorts paths, rows sort by
`(id, path)`, groups use a `BTreeMap`, gate modes a `BTreeSet`.

The tool never writes a rationale or a verdict (NFR-035). No write path exists. The help offers no
rationale-writing option. The empty-gate sentence ("No anomaly fails the evaluated modes: ...")
states no acceptance. The JSON carries no score, rank, verdict or priority key. The selection uses
only module vocabulary words (`closed`, `wont-fix`, `unassessed`, `impacting`, `Requirement`,
`Hazard`) and makes no archetype assumption beyond FR-162 to FR-164.

The MP-250/MP-251 population lines and the `NATIVE_OWNED_ROUTES` entry are required, not ceremony.
`tc_3003_the_nfr003_populations_are_stated_by_class` asserts that each plan declares exactly the
command x error-class population the code enumerates. `tc_1650_..._route_census...` asserts that
the census equals Clap's routes. Without the lines both fail. No compatibility layer was added.

Rulings on the coder's flagged ambiguities, against the merged spec text:

1. Group shape `{target, anomalies:[{anomaly, impact}]}`: conforms. FR-162 asks for "the target
   and one element `{ anomaly, impact }` for each listed anomaly that names it" and does not name
   the array key. Taking one element per anomaly, with the first row's impact, follows "one element
   for each listed anomaly". The spec does not say which impact wins when two rows name one target.
   The row's `affected` keeps both. This is not a deviation. A one-line spec note would close it.
2. JSON `unassessed` boolean and Markdown `Flag` column: conform. FR-162 requires the flag and
   AC-3 lists required keys, not exactly these keys. The flag restates an authored value and is no
   verdict under NFR-035-AC-3.
3. Path form: conforms. The spec does not fix it. Paths are as the scope was given, minus a
   leading `./`. An absolute scope gives absolute paths. That is still deterministic per tree.
4. `unreadable.code` values (`missing`, `columns`, `row-arity`, `min-rows`), and treating a missing
   `title` or a bad Release Rationale table as a mapping error: conform. spec-artifacts-process
   `mappings.yaml` maps `title` as `required: true` and `releaseRationale` as a typed-table
   `[Author, Rationale]`. FR-136's CR note counts a missing required key, a wrong header, wrong
   arity and an empty table as mapping errors. FR-162 Dependencies authorises widening the reader.
   This is not a deviation. FND-005 notes the one place the reader is looser than the module.
5. Non-git precedence (`not-applicable` over `absent`): conforms. FR-136 and FR-163 each give
   "not under git" its own clause, and it decides whether the derivation applies at all. Not a
   deviation.
6. `not stated` and `<br>` cells: conform. An absent record value reads `not stated` (FR-162 Row).
   A `-` Target is absent in the record. `<br>` joins multi-entry cells, which the spec leaves to
   presentation.
7. Gate modes deduplicated in canonical order: conforms. `gate.modes` is "the modes evaluated", a
   set.
8. Bad `--fail-on` refused by clap's value list: conforms. It exits 3 through the shared parser
   refusal arm (`EXIT_INVALID = Outcome::Invalid`), like every other non-change-assurance parse
   error. stdout is empty, and stderr names both modes. The message is clap prose, not a JSON
   envelope, the same as every other parse refusal in this shell.
9. Hand-written retained-help JSON: correct mechanism. `help.rs` `include_str!`s that file and
   serves it for exact help routes, so editing it is the only way to ship the page. The VER-106
   precedent did the same. FND-002 covers its text.
10. tc_136 helper refactor into `tests/common`: fine. It preserves behaviour (tc_136 passes 15/15),
    and `#![allow(dead_code, unreachable_pub, ...)]` with a reason is the established pattern in six
    other crates' `tests/common`.
11. FR-136 spec text: FR-162 Dependencies already carries the decision to widen, so no new CR is
    needed. FR-136's CR note still says the reader reads "the five record keys", which is now
    false. FND-003 asks for a one-sentence amendment in this PR.
12. Snapshots embedding fixture text: acceptable output snapshotting, not vendoring. FR-162-AC-13
    mandates snapshot-tested Markdown and JSON. The snapshots hold quoin's rendered output: titles,
    cells and rationale strings, with no frontmatter, no section structure and no document. No test
    reads them as input, and `git grep` finds no shipped fixture document in the tree. The coupling
    is that a wording change upstream reds the snapshot test. That is the same coupling as reading
    the fixtures in place, and it is accepted. Note that the snapshot currently records FND-001's
    wrong commits as expected output.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | `locate()` runs `git log <branch> --first-parent --follow --name-only -- <path>`. git's `--follow` also follows **copies**: a new anomaly file similar to an existing one (the template case, typical of real corpora) is traced into the other file's history (reproduced: `C064 AN-004.md AN-006.md`). As a result, `rationaleChange` and `fixingChange` name a commit that never touched the document, and `statusHistory` gains foreign entries, so `quoin anomaly check` emits a false `anomaly.illegal-transition` (reproduced: AN-002 "moved from closed (994d4a53)"). The committed snapshot records this as expected: `anomaly-report.json` gives AN-006's `rationaleChange` and AN-011's `fixingChange` as AN-004's commit (`authoredAt 2026-01-01T00:05`), but AN-006 and AN-011 were committed at ticks 7 and 12. The `--follow` predates this PR, but this PR builds FR-162/163's derived values on it and enshrines the wrong output. Fix: switch to `--name-status` and stop the walk at the first entry whose status is `A` or `C<n>` for the followed path, following `R<n>` only. Keep FR-136-AC-1's rename case. Add a test with two similar anomaly files committed separately, asserting each one's own commit for `fixingChange`/`rationaleChange` and no transition diagnostic, then regenerate the snapshots. | rust/crates/quoin-anomaly/src/history.rs:180-222; rust/crates/quoin-anomaly/src/derived.rs:177-202; rust/crates/quoin-cli/tests/snapshots/anomaly-report.json:263,472 |
| FND-002 | low | The shipped help text is wrong in two places. (a) `--fail-on` says "an **open** impacting anomaly has no release rationale", but `unjustified-impacting` also fails a `closed` `wont-fix` anomaly (FR-164: "a defect deliberately shipped must not pass the gate by being closed"). The same wording is in the Clap `.help` and in the retained page. Say "a carried (not closed, or closed as wont-fix) impacting anomaly". (b) In the retained `anomaly --help` page, the Subcommands row `quoin anomaly report report the...` has no column gap, and the COMMANDS row `anomaly report  Report...` is out of alignment with its wrapped continuation and the rows above it. | rust/crates/quoin-cli/tests/fixtures/retained-command-help.json:707,729; rust/crates/quoin-cli/src/anomaly.rs:82 |
| FND-003 | low | FR-136's CR note still says `quoin-anomaly` "reads the five record keys (`status`, `resolution`, `closed_by`, `safety_impact` and the assessment table)". The reader now also reads `title`, `severity` and `releaseRationale`, and `record.rs` says "The exception is recorded in FR-136". FR-162 Dependencies authorises the widening, so this is not a deviation, but FR-136 is now false. Fix: amend the CR note to name the eight keys and cite FR-162. | spec/functional/FR-136-anomaly-record-mapping.md:37-44; rust/crates/quoin-anomaly/src/record.rs:7-11 |
| FND-004 | low | Each listed document is walked twice. `row_of` calls `read_history` (one `git log` plus a `git show` per commit), and then `rationale_change` calls `locate` and `content` again for the same commits. That doubles the git subprocesses per anomaly. Fix: locate once, then derive both `History` and the rationale texts from one `Located`. | rust/crates/quoin-anomaly/src/report.rs:235-238; rust/crates/quoin-anomaly/src/derived.rs:177-194 |
| FND-005 | low | `table()` accepts a pipe block with no delimiter row as a table. The module's own review (spec-artifacts-process plat-1584 FND-001) records that such a block "is not a table", and the extractor gives it no row. The shipped fixture `rationale-no-delimiter` therefore reports `releaseRationale [{TBD, n/a}]` here, which the extractor would not produce. The gate still fails it, because the entries are placeholders. The assessment table has the same pre-existing looseness. Fix: require the delimiter row before data rows, and return `MinRows` otherwise. | rust/crates/quoin-anomaly/src/record.rs:214-243 |
| FND-006 | low | The mode spelling is one fact in two places. `GateMode::as_str` holds it, and so does a hand `match` in `run_report` with an "unreachable" error arm. Clap also holds the list as `value_parser([...])`. Fix: derive `clap::ValueEnum` on a CLI-side enum, or add a `GateMode::parse`/`all()` and build the value parser from it. | rust/crates/quoin-cli/src/anomaly.rs:81,193-203; rust/crates/quoin-anomaly/src/report.rs:39-44 |

## Review Scope

| Unit | Role | Result |
| --- | --- | --- |
| FR-162 Selection (`is_listed`, `carried`) | examined | Correct |
| FR-162 Row and Derived git values encoding (`ReportRow`, `Derived` serializer, `markdown()`) | examined | Correct encoding; values wrong under FND-001 |
| FR-162 Grouping (`groups`, ungrouped reasons) | examined | Correct (ruling 1) |
| FR-162 Unreadable and Output (`build_report`, `render.rs`, `Report::failed`) | examined | Correct |
| FR-163 `rationale_change` algorithm (present/pending/absent/not-applicable) | examined | Logic correct over its input; input wrong under FND-001 |
| FR-136 `fixing_change` and the check's fixingChange line | examined | Correct over its input |
| FR-164 gate (`gate`, `counts_as_rationale`, placeholder set) | examined | Correct; matches FR-016 value rule |
| `history.rs` `locate`/`read_history` refactor | examined | FND-001 |
| `record.rs` widened reader | examined | FND-003, FND-005 |
| quoin-cli `run_report`, exit mapping, clap value list | examined | Correct (ruling 8); FND-006 |
| retained-command-help.json pages | examined | FND-002 |
| snapshots | examined | Ruling 12; encode FND-001 |
| MP-250/MP-251 population lines, NATIVE_OWNED_ROUTES | examined | Required by tc_3003 and tc_1650 |
| `quoin anomaly refs`, `check.rs` diagnostics (unchanged) | context_only | Read for the shared reader |
