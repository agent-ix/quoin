---
id: SR-20261113104326
title: "Code review and Rust review of quoin risk discrepancies (PR 864, FR-154, FR-155)"
type: SpecReview
analysis: code-review
review_set: subset
scope: "agent-ix/quoin; PR #864 (git diff origin/main...HEAD at 708e3b8b): rust/crates/quoin-risk-advice/src/{discrepancies.rs,lib.rs}, rust/crates/quoin-risk-advice/tests/{common/mod.rs,tc_145_risk_advice.rs,tc_154_risk_discrepancies.rs}, rust/crates/quoin-cli/src/{risk.rs,tests.rs}, rust/crates/quoin-cli/tests/tc_154_risk_discrepancies_command.rs"
relationships:
  - {target: "ix://agent-ix/quoin/FR-154", type: references}
  - {target: "ix://agent-ix/quoin/FR-155", type: references}
---

# Code review and Rust review of quoin risk discrepancies (PR 864)

## Summary

Ticket: PLAT-1570. One code-review file with the rust-review lane folded in. quoin's own
`rust/` rust-style skill was applied first, then the ecosystem rust-review and rust-style
checklists.

What was checked and holds:

- JSON keys are exactly `attribution` and `findings`. Each finding is `{finding, refs}`. `refs`
  has the seven FR-155 keys, each always present (`null` or `[]`). The seven tokens are spelled
  as FR-105 and FR-303 spell them.
- The token rules follow the FR-155 Behavior lines: undecided wins whatever the region;
  `accepted` outside the region checks cover (none, any accepted, any undecided, else not
  accepted); `not_accepted` inside the region; no region gives `decision-without-region`.
- Ordering is by `refs.estimate`, then residual-risk id, then token string. Output is
  deterministic (BTreeMap reads, stable sort).
- The command is read-only. Nothing opens a file for writing. No score, rank, verdict, weight,
  total, proposal or priority key exists. The stem "accept" appears only in the attribution,
  the tokens and copied decision values.
- Reuse is real. `load` is shared with `advise`: one `quoin_quire::records::read`, one
  `records::Records::read`, one `policy::lookup`, one `AdviceError`. So FR-154-AC-8 holds by
  construction, and the CLI refusal codes are the `risk.<code>` ones `advise` uses.
- Exit codes: `--strict` with findings maps to `Outcome::Partial` (1) and keeps the payload;
  a bad scope or a contract-breaking record gives a refusal (2) with or without `--strict`.
- `risk.rs` refactors argument building into `scope_arg`/`format_arg`/`module_arg` and
  `inputs`. Behaviour is unchanged for `advise`.
- No retained-help fixture entry was added. That is right: PR 852's review found Clap pages in
  that fixture to be a self-copy test, and `quoin risk discrepancies --help` renders from
  Clap. MP-250/251 needed no edit for their tests to pass (see FND-004 for why).
- Tests are behaviour tests tagged to ACs. Moving the shared helpers to `tests/common/mod.rs`
  is fine. No hash, pin, file-count or self-copy test. The before/after file snapshot asserts
  the no-write behaviour. Nothing is vendored: the CLI test reads spec-objects-safety in place
  from `SPEC_OBJECTS_SAFETY_DIR`.
- Rust: no `unwrap`, `expect`, `panic` or indexing in production code. Public items are
  documented. `let _ = write!(String, ..)` is the normal infallible-write idiom.

Focused tests run at 708e3b8b against a temporary spec-objects-safety origin/main worktree
(bf44b49): quoin-risk-advice tc_145 (30) and tc_154 (11), quoin-cli tc_145 command (4) and
tc_154 command (3), all passed. Probes run with the built binary are cited in the findings.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | FR-154 says a record that cannot be parsed exits 2, with or without `--strict`. The shared reader instead drops two kinds of unparseable file. A residual-risk file with invalid UTF-8 becomes a `risk.document_unreadable` stderr diagnostic, and the command exits 0. A residual-risk file whose YAML frontmatter does not parse is skipped with no diagnostic at all, and the command exits 0. Both probes ran with `--strict --format json`, printed `findings: []`, and exited 0. So the gate passes over a decision record it never read. FR-145 allows this leniency for `advise`. FR-154 does not. `discrepancies` should refuse with exit 2 on any unreadable file, and on a document whose frontmatter fails to parse, naming the path. Add a test for each. | rust/crates/quoin-risk-advice/src/lib.rs:85-117, rust/crates/quoin-cli/src/risk.rs:143-171, rust/crates/quoin-quire/src/records.rs:102-118 |
| FND-002 | medium | Spec gap in FR-155. For any `accepted`/`not_accepted` risk with no region, the code emits `decision-without-region`. When the reason is not a level (the policy is not loaded, the policy is ambiguous, the cell is not in the policy, or the estimate is missing or was skipped), it emits `region_missing: null`. That is within FR-155's letter, because its table says "else `null`". But FR-155's Description says the finding "carries ... the reason" and lists only the two level reasons. The probe with a residual risk on a missing policy `POL-X` gives a Markdown line with no reason: `decision-without-region: estimate EST-1, residual risk RR-1, recorded decision accepted, policy POL-X.` The person cannot tell what to fix. Edit FR-155 in this PR so `region_missing` also takes `policy-not-found`, `policy-ambiguous`, `cell-not-in-policy` and `estimate-not-found`. The lookup already computes the first three tokens. Then emit them and test each. | spec/functional/FR-155-risk-discrepancy-findings.md:33-36,68, rust/crates/quoin-risk-advice/src/discrepancies.rs:135-141, rust/crates/quoin-risk-advice/src/discrepancies.rs:143-192 |
| FND-003 | medium | FR-154 says the command reads only `residual_risk`, `benefit_risk` and `risk_estimate` records and the policies they reference. `load` reuses `advise`'s reader whole. It contract-checks `overall_residual_risk_evaluation` and parses Anomalies. A malformed `overall_residual_risk_evaluation` therefore makes `risk discrepancies` exit 2 over a record the command does not use. A module set that does not declare that type also refuses. Pass a checked-type list that holds only the four FR-154 types, or skip evaluations and anomalies in `discrepancies`. | rust/crates/quoin-risk-advice/src/lib.rs:85-117, rust/crates/quoin-risk-advice/src/records.rs:14-20, rust/crates/quoin-risk-advice/src/records.rs:98-137 |
| FND-004 | low | Pre-existing, and now extended to a second command. The NFR-003 census (MP-250/251, `nfr_populations::help_tree::walk`) reads subcommands only under an uppercase `COMMANDS` heading. Clap renders `quoin risk --help` with `Commands:`, so the walk never reaches `risk advise` or `risk discrepancies`. Their unknown-flag, missing-path (`scope`, `--module`) and missing-argument classes are outside the population, although help_tree.rs claims that "a command ... added to the binary joins the population without an edit". The claim that MP-250/251 are untouched is accurate only because of this blind spot. Make the walker read Clap's `Commands:`/`Options:` sections too, which adds the risk rows to MP-250/251. Or record the gap as a follow-up ticket. | rust/crates/quoin-cli/tests/nfr_populations/help_tree.rs:146-190, spec/assurance/MP-250-nfr003-actionable-failures.md:132 |

## Verdict

Not mergeable as is: one high finding, two medium and one low. The token logic, the key shape,
the ordering, the reuse of the reader and the lookup, the `--strict` exit mapping and the
no-write and no-verdict rules all meet FR-154 and FR-155. FND-001 is the blocker, because a
`--strict` gate that exits 0 over an unparseable residual-risk file defeats the gate. FND-002 is
the coder's open question, and the answer is a small FR-155 edit in this PR.

## New findings (disposition pass 1)

Against head e8f758d8.

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-005 | medium | A residual-risk file whose frontmatter fence is never closed still passes the gate. Probe: `---` / `id: RR-1` / `type: residual_risk` / ... with no closing `---`. `risk discrepancies` prints empty findings and exits 0 with and without `--strict`, and writes nothing to stderr. quire-rs reports such a file as `DocumentWithoutFrontmatter { malformed: false }`, the same flavor it uses for a plain README. The new `Corpus::malformed` collects only `malformed: true`, so this case falls through. That is the same FR-154 rule as FND-001 ("a record in it cannot be parsed" exits 2), on a narrower input. Fix: also refuse a `malformed: false` file whose text opens with a `---` fence. A README without a fence stays quiet, as the probe over a plain README confirms. Add a test. | rust/crates/quoin-quire/src/records.rs:117-128, rust/crates/quoin-risk-advice/src/lib.rs:88-112 |

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | - |
| FND-002 | fixed | - |
| FND-003 | fixed | - |
| FND-004 | deferred: follow-up ticket filed | PLAT-1683 (the NFR-003 help_tree walker reads only the uppercase COMMANDS layout). The coordinator filed it because the fix is larger than this PR. |
