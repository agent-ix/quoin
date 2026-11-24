---
id: SR-20261124021650
title: "code review of the risk-summary authored texts and the DOS-RISK benefit-risk and overall evaluation sections"
type: SpecReview
analysis: code-review
scope: "agent-ix/quoin; PR #1041 at 0078ebcb over origin/main 61c4dfdc; rust/crates/quoin-risk-advice/src/{records,summary,lib}.rs, rust/crates/quoin-dossier/src/{risk_management,gap}.rs and their tests"
review_set: subset
relationships:
  - target: ix://agent-ix/quoin/FR-169
    type: references
  - target: ix://agent-ix/quoin/FR-199
    type: references
  - target: ix://agent-ix/quoin/FR-191
    type: references
---

# Code review of the risk-summary authored texts and the DOS-RISK record sections

## Summary

Ticket: PLAT-1810. PR agent-ix/quoin#1041, head 0078ebcb, one commit on origin/main
61c4dfdc. Code review with the repository rust-style skill (which outranks the
ecosystem rust-review) as the Rust lane. Scope is `git diff origin/main...HEAD` only.
Tool versions: quoin 0.36.8, quire 0.52.0 (engine 0.62.2).

Focused runs (locked-build, debug=0, incremental=0, SPEC_OBJECTS_SAFETY_DIR set):
`quoin-risk-advice` tc_169 (25 pass) and lib, `quoin-dossier` tc_1622 (8 pass) and lib
(117 pass). The dossier lib tests compile only with PR #1040's `rmm.rs` test hunk applied
locally, because origin/main's `rmm.rs` test calls the 3-argument
`discharge_report_from_store` that #1034 changed. That patch was reverted. `cargo fmt
--check` is clean. `cargo clippy -p quoin-risk-advice -p quoin-dossier` could not reach
these crates: it stops on the out-of-scope quoin-evidence lint errors first.

Mutation pass, five builds, several mutants per build, each attributed by the panic line
of a test only that mutant reaches:

- Killed: residual `rationale` dropped; harm_texts carries an extra key; harm_texts out
  of order; an extra key on overall_evaluations entries; overall_evaluations descending;
  the evaluation and benefit-risk Markdown tables swapped; overall evaluations read
  without the contract check (`CHECKED` minus the type); `overall_evaluation_undecided`
  dropped; DOS-RISK evaluations in descending order; `not stated` printed for a present
  Product, a present residual rationale, or a present hazard/situation/harm text.
- Survived: the tool writes its own acceptance label for an accepted overall evaluation
  (FND-001); `overall_evaluation_undecided` keyed on a missing decider instead of the
  decision (FND-002); the situation title and a harm title copied into the
  risk-summary Markdown without `|` or line-break escaping (FND-005).

## Examined scope

- FR-169 Output contract, row key table, harm-text, residual-risk, benefit_risks and
  overall_evaluations entries, Markdown cell rules (`\|`, `<br>`), exit-2 contract
  check: `summary.rs`, `records.rs`, `lib.rs`.
- FR-199 Outputs and Behavior (authored texts verbatim, `not stated`, two new sections,
  `none`, `no_overall_evaluation`, `overall_evaluation_undecided`, no own acceptance
  label): `risk_management.rs`.
- FR-191 closed gap list and sort key: `gap.rs`.
- Markdown escaping helpers used by the new sinks: `quoin-dossier/src/markdown.rs`
  (`cell`, `line`, `prose`), `summary.rs` `esc`.
- The spec-objects-safety manifest column patterns for Rationale, Benefit, Product and
  Decided by, and the Condition / Description section extraction.
- Overlap with open PRs #1040 (same `GapKind` list) and #1039.

## Verdict

CONDITIONAL. The contract is implemented as specified: the four top-level keys, the
thirteen row keys, closed entry key sets, ascending order, `[]`/`null`, `<br>` for line
breaks, the two new Markdown tables, the overall evaluation contract check, the two new
gap kinds in the FR-191 position with the document-level and record subjects, and the
two DOS-RISK sections in dossier id order with `none`. Most mutations are killed. The
findings are test gaps on stated behaviour (FND-001, FND-002, FND-005), untrusted prose
reaching the rendered dossier (FND-003), and a silent cross-PR merge break (FND-004).
None is a wrong result on a valid corpus today; FND-004 must be handled at merge time.

Pre-existing, not this PR's defects, reported to the lead: the FR-191 order of the three
release gap kinds against the `GapKind` enum (code since PLAT-1620 #952), the dossier lib
tests not compiling on main, and the red quoin-cli and risk-advice tests on main (see the
gap-analysis SR of this PR).

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | medium | FR-199-AC-14 ("no heading, column label or gap detail that the tool writes names acceptance ... no score, class or acceptability value appears") is checked only on the all-undecided document. On the decided document the test accepts any changed line that mentions `RR-1` or `ORE-1`. A mutant that writes `ORE-1: overall residual risk acceptable` after the evaluation table for an accepted record survives every test. Assert the decided document too: every changed line is the record's own table row or a `Gaps:` line, and no tool-written text contains `accept` outside copied cells | rust/crates/quoin-dossier/tests/tc_1622_risk_management.rs:378-423, rust/crates/quoin-dossier/src/risk_management.rs:294-316 | correct-requirement-no-evidence |
| FND-002 | medium | `overall_evaluation_undecided` is raised on `decision == "undecided"` as FR-199 says, but no test has an undecided evaluation that names a decider. Changing the condition to `decided_by.is_none()` survives (AC-15's fixture is undecided with an empty decider). The manifest allows `Decided by` on an undecided record, so this edge is reachable. Add an undecided-with-decider case to the AC-15 test | rust/crates/quoin-dossier/src/risk_management.rs:116-127, rust/crates/quoin-dossier/tests/tc_1622_risk_management.rs:336-346 | correct-requirement-no-evidence |
| FND-003 | medium | The new DOS-RISK sinks copy the hazard Condition and situation/harm Description text through `prose`, which escapes only a line that opens with `#`. The section text has no `<>` constraint in the safety manifest. A text whose line is followed by `---` or `===` becomes a setext heading, and a raw `<!--` hides everything after it, including `## Gaps`, in a rendered dossier. The `#` escape exists to stop forged headings (FR-192/193), so the setext form defeats it. Raw `<` reaches the risk-summary Markdown cells the same way. Escape setext underlines and `<` in `prose` (shared helper), and make `cell` handle `<` too | rust/crates/quoin-dossier/src/markdown.rs:26-37, rust/crates/quoin-dossier/src/risk_management.rs:246-258, rust/crates/quoin-risk-advice/src/summary.rs:598-603 | implementation-bug-despite-evidence |
| FND-004 | medium | PR #1040 adds `SafetyClassUnassigned` and `RunIdentityUnrecorded` to the same `GapKind` list and also bumps `ALL: [Self; 39]` to `[Self; 41]`. The two PRs merge with no textual conflict (checked with `git merge-file`), leaving `[Self; 41]` over 43 entries, so the `quoin-dossier` lib test build fails after the second merge. Whichever merges second must set `[Self; 43]`. Merge #1040 first, which also fixes the `rmm.rs` test compile, then rebase this PR with the count at 43 | rust/crates/quoin-dossier/src/gap.rs:107 | implementation-bug-despite-evidence |
| FND-005 | low | FR-169 requires every value copied from a record to escape a pipe as backslash-pipe and write line breaks as `<br>`. The tests cover only `Decided by` with a pipe (AC-27) and `hazard_text` with line breaks (AC-35). Copying `situation_title` and the harm `title` into the Markdown without `esc` survives every test. Add one fixture whose titles and texts carry a pipe and a line break, and check the cell count | rust/crates/quoin-risk-advice/src/summary.rs:658-665, rust/crates/quoin-risk-advice/src/summary.rs:723, rust/crates/quoin-risk-advice/tests/tc_169_risk_summary.rs:1174-1228 | correct-requirement-no-evidence |
| FND-006 | low | `assert_markdown_voice` now skips every line that starts with a pipe then `BR-` or `ORE-`, so tool-written `accept` wording in those rows would pass the FR-169-AC-13 check. Exclude only the copied `decision` cell of those rows, not the whole line | rust/crates/quoin-risk-advice/tests/tc_169_risk_summary.rs:165-175 | correct-requirement-no-evidence |

## Dispositions

Round 1 was checked against head 8d58b5a2, rebased onto #1040.

Focused runs: `quoin-risk-advice` and `quoin-dossier` crate tests all pass, except
`tc_1834_risk_management_plan` and `tc_145`. `tc_1834_risk_management_plan` fails because
the local spec-objects-safety checkout lacks the plan skeleton. `tc_145` is fixed on main
by #1044, which this branch does not yet contain. Both failures are outside this PR.

I then ran one mutation batch. All mutants were killed, and a probe confirmed the
setext bypass in FND-003.

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | The AC-14 test now asserts the decided document changes in exactly three lines (the RR-1 row, the ORE-1 row and `Gaps: none`), and that no `accept` text remains outside the copied decision cells. The label mutant is killed |
| FND-002 | rejected | The decider-keyed mutant cannot reach the report under the declared module. `EvaluationDetermination.json` forbids `decided_by` when the decision is `undecided` and requires it when the decision is `accepted` or `not_accepted`. The new test shows both mismatches exit 3. On every input the module admits, the decider test and the decision test agree, and the code keys the gap on the decision as FR-199 states |
| FND-003 | still-open | `prose` now escapes `#`, fences, every `<` and a setext underline with no trailing spaces. CommonMark allows trailing spaces or tabs after a setext underline, and `Title` followed by a `---` line with two trailing spaces is not escaped. The probe confirmed this: `prose` returns the input unchanged, so the line still forges a level-2 heading. Test the underline after `trim_end()` and add the case to `setext_underlines_fences_and_raw_html_are_escaped` |
| FND-004 | fixed | Rebased onto #1040. `GapKind::ALL` is `[Self; 42]` with 42 entries, and the dossier lib tests compile and pass (129) |
| FND-005 | fixed | The new test puts a pipe, a line break and `<` in every copied title and text and checks 15 cells. The unescaped `situation_title` and harm `title` mutants are killed |
| FND-006 | fixed | `assert_markdown_voice` now blanks only cell 3, the copied decision, of benefit-risk and evaluation rows, and checks the rest of the row |
