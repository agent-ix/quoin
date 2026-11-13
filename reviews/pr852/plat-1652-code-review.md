---
id: SR-20261113073533
title: "Code review and Rust review of quoin risk advise (PR 852, FR-145 to FR-148, NFR-031)"
type: SpecReview
analysis: code-review
review_set: subset
scope: "agent-ix/quoin; PR #852 (git diff origin/main...HEAD at 03fe9207): rust/crates/quoin-risk-advice/**, rust/crates/quoin-cli/src/{risk.rs,help.rs,main.rs,tests.rs}, rust/crates/quoin-cli/tests/{tc_145_risk_advise_command.rs,tc_shell_contract.rs,fixtures/retained-command-help.json}, rust/Cargo.toml, rust/Cargo.lock, spec/assurance/MP-250, MP-251, spec/functional/FR-145..148, spec/non-functional/NFR-031"
relationships:
  - {target: "ix://agent-ix/quoin/FR-145", type: references}
  - {target: "ix://agent-ix/quoin/FR-146", type: references}
  - {target: "ix://agent-ix/quoin/FR-147", type: references}
  - {target: "ix://agent-ix/quoin/FR-148", type: references}
  - {target: "ix://agent-ix/quoin/NFR-031", type: references}
---

# Code review and Rust review of quoin risk advise (PR 852)

## Summary

Ticket: PLAT-1652 (with PLAT-1653 and PLAT-1654). One code-review file with the rust-review lane
folded in. quoin's own `rust/` rust-style skill was applied as the higher authority, then the
ecosystem rust-review and rust-style checklists.

What was checked and holds:

- JSON keys are exactly `attribution`, `policy_cells`, `evidence_gaps` and `decision_queue`
  (struct field order, `Serialize`). Every `findings` array holds `{finding, names}`. The entry
  key tables of FR-146, FR-147 and FR-148 match the structs field for field.
- The command is read-only. Nothing in the crate or `risk.rs` opens a file for writing. Output
  is deterministic: BTreeMap/BTreeSet everywhere, files sorted by path, and duplicate ids resolve
  last-by-path, which is stable. No score, rank, verdict, weight, total or priority key exists.
- The tokens are the safety module's (FR-102, FR-203, FR-204, FR-301, FR-302, FR-303), spelled
  as `spec-objects-safety` main spells them. `policy-not-found` and `policy-ambiguous` are
  FR-146's own and do not collide with any safety token.
- Verified means an incoming `verifies` edge from any document, and nothing else (FR-302).
  Implemented means an incoming `satisfies` edge from an `FR` or `NFR`. No test outcome or
  control field is read.
- The decision queue is ordered by kind and then id. Region is never used to order anything.
- Anomalies are read through `quoin_anomaly::parse_record`, the existing FR-136 stand-in. That
  reuse is right: FR-147 asks for kind and impact "as `quoin anomaly refs` reports them".
- Production code has no `unwrap`, `expect`, `panic` or indexing. The crate opts into the
  workspace lints. `cargo clippy -p quoin-risk-advice --all-targets -D warnings` and
  `cargo fmt --check` are clean. The error envelope copies the sibling `quoin-anomaly` shape.
- MP-250 and MP-251 each gain one `risk:unknown-flag` population row, the same shape as their
  neighbours. The root `--help` fixture, the unknown-command catalogue and the
  `NATIVE_OWNED_ROUTES` census edits are needed for the new command family, and are legitimate.
- No file was vendored or copied from `spec-objects-safety`. Every test fixture is authored
  inline in a tempdir.
- The three spec edits from commit 03fe9207 (ix:// citations, the skip clause and the
  `{finding, names}` shape) agree with the code.

Focused tests run at 03fe9207: `quoin-risk-advice` tc_145 (16 passed),
`quoin-cli` tc_145_risk_advise_command (2), tc_retained_help_capture (6), tc_shell_contract (3),
all passed. In `quoin-core` tc_1045_spec_integrity, 11 passed and 1 failed. The one failure is
`tc_118_001`, reporting only `spec/functional/FR-152...md:16: ADR-100 does not resolve`. FR-152
comes from #849 on main. FR-145..148 and NFR-031 now resolve. The branch is one commit behind
origin/main (#853, spec-only, no overlapping file).

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | `corpus.rs` and `records.rs` are a second hand-rolled stand-in for the quire-rs record extractor. quire-rs 0.59, which quoin already links through quoin-quire, exports `extract` (the manifest `body_extraction` DSL) and `load_repo`. spec-objects-safety's manifest declares `body_extraction` for every type read here. No exception is recorded for this crate; FR-136's covers only quoin-anomaly. The reader also skips the manifest's asserts. A `residual_risk` whose Determination header or decision word does not match yields `decision: ""`. That record then appears in neither `queue` nor `no_record`, because it still counts as referencing its estimate, so an undecided decision disappears without a word. FR-145's skip clause covers only estimates. Read through the quire-rs extractor, or record the exception and its removal condition the way FR-136 does, and test the malformed-record case. | rust/crates/quoin-risk-advice/src/records.rs:167-190, rust/crates/quoin-risk-advice/src/corpus.rs:136-161, rust/crates/quoin-risk-advice/src/queue.rs:106-134 |
| FND-002 | medium | `retained-command-help.json` gains `risk --help` and `risk advise --help` pages in Clap format. `help::retained_help` serves any fixture argv verbatim, so `tc_1650_native_help_preserves_retained_routes...` compares the fixture with itself for these two argv. The test is a self-copy and cannot fail. The `about`/`help` strings in risk.rs are also dead for those argv, and a later edit to them never reaches `--help`. The capture is the frozen Node/oclif contract, and these pages are not part of it. Drop the two entries and let Clap render. The root `--help` edit stays. | rust/crates/quoin-cli/tests/fixtures/retained-command-help.json:764-784, rust/crates/quoin-cli/src/help.rs:144-186 |
| FND-003 | low | `corpus.rs` copies `cells`, `is_separator` and `markdown_files` from quoin-anomaly (`record.rs`, `refs.rs`). `records.rs::plain` copies the AC-id scan from `refs.rs::resolvable_ids`. The copies have already drifted: refs.rs treats a document as a hazard only when `type` and `object` are both `hazard`, but evidence.rs checks `type` only. Export the helpers from quoin-anomaly or a shared module and call them. | rust/crates/quoin-risk-advice/src/corpus.rs:86-134, rust/crates/quoin-risk-advice/src/records.rs:193-220, rust/crates/quoin-risk-advice/src/evidence.rs:60-62 |
| FND-004 | low | When both levels are policy members but the policy has no cell for the pair, or the cell names a region missing from `## Regions`, `resolve` emits `level-not-in-policy` naming `"<severity>/<probability>"`. FR-102 defines that token as naming a level, and neither level is out of the policy here. The pair string is a value no record holds. State the case in FR-146 (an incomplete policy) or name the missing cell's region. | rust/crates/quoin-risk-advice/src/policy.rs:83-95 |
| FND-005 | low | The new root-help row `risk  Advise on risk decisions from the safety records, without deciding.` is 88 columns. Every other row is hand-wrapped at 80 with the 20-space continuation indent. Shorten `ABOUT` or wrap it. | rust/crates/quoin-cli/src/risk.rs:17-18, rust/crates/quoin-cli/src/help.rs:101 |

## Verdict

Not mergeable as is: two medium findings and three low ones, with no high finding. The report
logic meets FR-145..148 and NFR-031 as written. FND-001 is the substantive one, because a
malformed undecided record is invisible in a report whose purpose is to show undecided
decisions.

## New findings (disposition pass 1)

Against head e7389f42, which was rebased on main and includes #853.

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-006 | high | `quoin_quire::records::read` takes edges from `harvest_edges`, and `harvest_edges` adds every body `ix://` link as a `references` edge. The two sections then pick a residual risk's policy differently. `links::policies_of` (policy_cells) still takes `references.first()`, while `queue.rs` takes `Records::policy_of`. Reproduced at e7389f42 with an undecided RR-1 whose Rationale cell links `ix://agent-ix/demo/HAZ-1`. policy_cells shows `policy: HAZ-1` with `policy-not-found`, and decision_queue shows `policy: POL-1`, region Low, for the same record. A benefit_risk whose Benefit cell links an FR also lists that FR in `references` as a covered residual risk. Read only frontmatter `relationships` for these verbs, use one policy rule in both sections, and test a body link. | rust/crates/quoin-quire/src/records.rs:124, rust/crates/quoin-risk-advice/src/links.rs:88-98, rust/crates/quoin-risk-advice/src/records.rs:142-148 |
| FND-007 | medium | A residual risk whose frontmatter has two `references` edges to policies in scope resolves silently to the alphabetically first one. Both sections then print that policy's region and flag. Reproduced: RR-1 references POL-0 (Low flagged outside) and POL-1 (Low not flagged), and the report printed only POL-0's "outside its acceptable region" wording. Quire does not catch this, because the manifest's `allowed_links` has no cardinality. FR-146 defines `policy-ambiguous` for more than one policy, and that should apply to one record naming two. | rust/crates/quoin-risk-advice/src/records.rs:142-148, rust/crates/quoin-risk-advice/src/links.rs:88-98 |
| FND-008 | medium | The new runtime dependency is not met by the default modules. `default-modules.yaml` pins spec-objects-safety `v0.8.0`, which predates the decision-record types (#39, d4fcf97, untagged). With installed modules, a repository holding a `benefit_risk` exits 2 with `risk.unreadable` and the message `...BR-1.md: unknown type 'benefit_risk' (no archetype registered for it)`, so the command is unusable without `--module` pointed at a main checkout. The refusal mentions neither `--module` nor the module and version needed. Release spec-objects-safety and bump the pin, or at least name the module and `--module` in the refusal. | default-modules.yaml:84-90, rust/crates/quoin-cli/src/risk.rs:81-84 |
| FND-009 | low | `AdviceError` distinguishes `Read` from `Record`, but the CLI maps every error to the one diagnostic code `risk.unreadable`. A broken record, an unreadable scope and a missing record type therefore look the same to a caller. quoin's rust-style says each condition a caller must distinguish gets its own code. | rust/crates/quoin-cli/src/risk.rs:81-84, rust/crates/quoin-risk-advice/src/error.rs:9-17 |
| FND-010 | low | `advise` collects the reader's corpus notices into a local `notices` and drops them. A document the engine walker cannot load (`DocumentUnreadable`, for example a non-UTF-8 residual_risk file) is therefore absent from the report without a word. That is the same silent-drop class FND-001 closed for contract violations. | rust/crates/quoin-risk-advice/src/lib.rs:66-68 |
| FND-011 | low | FR-146's new `cell-not-in-policy` clause says "`quire validate` reports the incomplete policy (FR-101)". The safety manifest says the per-pairing cell rule "is a cross-table rule the assertions cannot state ... waits on an engine assertion", so today nothing reports it, and the token is the only signal. The token choice itself is sound: FR-102 defines only `level-not-in-policy` and `level-not-assessed`, both naming a level, and FR-105 folds every "FR-102 resolves no region" case into `decision-without-region`. Correct the clause. | spec/functional/FR-146-risk-advise-policy-cells.md:55 |

## Dispositions

Round 1, against commit e7389f42.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | - |
| FND-002 | fixed | - |
| FND-003 | fixed | - |
| FND-004 | fixed | - |
| FND-005 | fixed | - |
