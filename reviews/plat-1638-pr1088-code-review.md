---
id: SR-967175026302197768900474181941935084796
title: 'code-review review of Quoin PR #1088 authored rating coverage'
type: SpecReview
analysis: code-review
scope: agent-ix/quoin; PLAT-1638 and PLAT-1640; spec/functional/FR-235-quoin-selftest.md;
  rust/crates/quoin-cli/src/validation.rs; rust/crates/quoin-cli/tests/tc_1636_validation_ratings.rs;
  rust/crates/quoin-completeness/src/{ratings,lib}.rs
review_set: subset
relationships:
- target: ix://agent-ix/quoin/FR-235
  type: references
---

## Summary

Reviewed the changed Rust ratings command, shared coverage types and regression tests against FR-235-AC-4/11. Runtime behavior meets the amendment; one trace binding needs correction.

## Verdict

**CONDITIONAL** — The single medium finding concerns trace attribution, not a reproduced command behavior failure.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The unreadable-spec regression test is still tagged FR-235-AC-4, so the matrix attributes partial-result evidence to the readable-bundle success criterion instead of FR-235-AC-11; an AC-11-selected run omits the partial-result regression. Retag it to FR-235-AC-11. | rust/crates/quoin-cli/tests/tc_1636_validation_ratings.rs:125 |

## Tool Identity

Reviewed head: `7ee1f6b550c1dec3d0e3df6f32cf6afa97504bf7` (Quoin PR #1088, base main). Tickets: PLAT-1638 and PLAT-1640. Reviewer model: `gpt-6.1-sol`.

Review tooling on terra: released Quoin `0.37.0`, executable `/tmp/plat1923-review-quoin037/node_modules/.bin/quoin`, installed from `https://registry.npmjs.org/` as confirmed by its lockfile; Quire `0.52.0` (engine `0.62.2`), executable `/home/peter/.npm-global/bin/quire`; schema module `spec-artifacts-process@v0.36.0`, ISO module `spec-artifacts-iso@v0.25.0`. Global Quoin `/home/peter/.npm-global/bin/quoin` remains `0.36.8`. The tested PR executable is `target/debug/quoin`, version `0.37.0-11-g7ee1f6b5`. Cargo `1.98.1 (797e8a9bc 2026-08-05)` and rustc `1.98.1 (48a229cea 2026-09-01)`.

## Assurance Context

Inspected `spec/assurance/AP-201-finding-quality.md` and `spec/assurance/AP-202-jev-lens-evaluation.md`. Neither applies: this change reports authored supplier ratings, not benchmark finding-quality measurements or Jev live lens evaluation. No enforced review run or human acceptance gate applies. Selected architecture/impact/measurement/independence/producer-reliance evidence: not applicable to these two profiles for this change. Active exceptions: none.

## Scope

- `FR-235-AC-4`; `spec/functional/FR-235-quoin-selftest.md`; role: `examined`; excerpt: Given a high-process-risk FR with no self-test asset or trace marker, `quoin validation ratings --coverage` reports its authored rating state, identifier, and value without a self-test coverage field, and exits 0 when the spec bundle is readable.
- `FR-235-AC-11`; `spec/functional/FR-235-quoin-selftest.md`; role: `examined`; excerpt: Given a not-high-process-risk or unrated FR and any contents of a self-test asset tree, `quoin validation ratings --coverage` reports rating state and authored values, leaves the unrated FR as `unrated`, and does not inspect or report test trace markers; unreadable spec documents remain partial results.
- `FR-235`; `spec/functional/FR-235-quoin-selftest.md`; role: `context_only`; excerpt: When a user requests `quoin selftest --list` or `quoin selftest --run`, the Quoin CLI shall list or run command-level self-test runners associated with the installed release, so the user can exercise the exact installed binary without checking out the source repository.
- `FR-235`; `spec/functional/FR-235-quoin-selftest.md`; role: `examined`; excerpt: The read-only `quoin validation ratings --coverage` command SHALL report supplier `FeatureRiskRating` status, identifiers, and authored values for functional requirements.
- `FR-235`; `spec/functional/FR-235-quoin-selftest.md`; role: `examined`; excerpt: It SHALL NOT inspect self-test assets, associate a feature rating with a self-test case, or require a dedicated test for any feature.
- `FR-235`; `spec/functional/FR-235-quoin-selftest.md`; role: `examined`; excerpt: Test selection and criterion coverage remain the responsibility of the normal-suite test matrix.
- `FR-235`; `spec/functional/FR-235-quoin-selftest.md`; role: `examined`; excerpt: The command SHALL report an unrated feature as `unrated`; absence of a self-test asset or trace marker SHALL NOT change the rating report or its exit status.

## Coverage

Plan completion: not assessed

Reconciliation: `quoin matrix --repo . --json` (Quoin 0.37.0, Quire 0.52.0 / engine 0.62.2), exit 0. The whole-repository matrix reports 250 requirements and 2630 criteria: tagged 1760; untagged 644; tagged-by-ignored-test 11; method-without-symbol 179; retired 36. Every criterion has `no run evidence` (2630); this is reported once, not promoted to per-criterion findings. `quire coverage --scope . --json` also exited 0: 296 untracked symbols and 1722 diagnostics, including pre-existing issue-id tags on untouched rating tests and cfg-dependent symbol collision warnings. These are baseline context; the lead explicitly scoped actionable findings to this PR. Unchanged FR-235-AC-13 remains untagged while runner asset implementation is held; it is not a new finding or blocker for #1088.

Edited FR-235-AC-4 and FR-235-AC-11 are both tagged by the real command integration test. The unreadable-input test is tagged AC-4; its incorrect binding is reported once in the code-review artifact. Engine-backed whole-repository assurance is not asserted by this PR-scoped verdict.

The repository source inventory contains 2002 public-function/CLI-command/flag candidate lines and 29 stub-keyword candidates. Keyword candidates are comments, fixture placeholders, or substantive implementations; no reachable `todo!` or `unimplemented!` implementation was found. Detailed reverse tracing covered the ratings command, its coverage branch, value extraction, shared output types, the ordinary-ratings sibling route, the retained report's rating path, and the changed and sibling integration tests. No new unowned behavior or source/test stub was found in this diff. Inventory counts are candidates, not a claim that 2002 independent behaviors were manually reviewed. Optional whole-repository semantic review was skipped because it was not requested; the code-review lane independently assessed the edited criteria and assertions.

## Validation

`git diff --check` and `cargo fmt --all --check` exited 0. Targeted FR-235 validation exited 0 with 1/1 documents grammar-clean and zero grammar findings. Full `quire validate --scope . 'spec/**/*.md' --summary` exited 0, with the existing 97 grammar findings elsewhere (446/485 grammar-clean); FR-235 has no warning.

Executed the existing test binary built at the reviewed head, `target/debug/deps/tc_1636_validation_ratings-ca7e8b24cf14dde1`, with four separate exact filters: `tc_235_ratings_coverage_reports_ratings_without_reading_selftest_assets`, `tc_235_ratings_coverage_reports_unreadable_spec_and_returns_partial`, `tc_1636_ratings_reports_each_status_and_is_read_only`, and `tc_1636_unreadable_input_is_reported_in_json_and_human_output`. Each exited 0 with 1 passed, 0 failed, 3 filtered out. No broad test suite ran. A read-only temporary fixture probe also verified human/JSON authored values, duplicate supplier records, exclusion of adopter re-ratings and exclusion of foreign repository targets.

`quire trace --scope . --file rust/crates/quoin-cli/src/validation.rs` exited 0 and lists all ten new/existing bare `implements` links for command, run, run_coverage, rating_values_for_feature and string_field as implementations of AC-4/AC-11. Related code examined: `quoin_completeness::assess_feature_ratings`, normal ratings render/status, bundle parsing/unreadable diagnostics, and `validation/report.rs::{FeatureRating, RatingEvidence, feature_ratings, ratings_for_feature}`. No dependency pin, lockfile, CI workflow, or release packaging change is in the diff; dependency-move review is not applicable.

## Trace Bindings

```yaml
bindings:
- test_id: rust/crates/quoin-cli/tests/tc_1636_validation_ratings.rs::tc_235_ratings_coverage_reports_ratings_without_reading_selftest_assets
  ac_id: FR-235-AC-4
  trace: correct
- test_id: rust/crates/quoin-cli/tests/tc_1636_validation_ratings.rs::tc_235_ratings_coverage_reports_ratings_without_reading_selftest_assets
  ac_id: FR-235-AC-11
  trace: correct
- test_id: rust/crates/quoin-cli/tests/tc_1636_validation_ratings.rs::tc_235_ratings_coverage_reports_unreadable_spec_and_returns_partial
  ac_id: FR-235-AC-4
  trace: wrong
```

## Method Selection

Ran code-review with its Rust lane, gap-analysis, and spec-review base, EARS, integrity, failure-domain and scope-boundary lenses. Dependency analysis was not selected because no relationship or dependency revision changes; object review because no domain-object spec changes; evidence analysis because verification-method and evidence-policy declarations do not change; risk-complexity, security and architecture evaluation because no new external dependency, trust boundary, architecture or measurement policy is introduced; criterion-strength because no Jev analysis was selected. The optional whole-repository gap semantic review was not requested. No applicable selected method was unavailable.

## Gate Side Result (out of diff, non-blocking)

`make rust-deny` initially exited 2 because cargo-deny could not acquire the advisory database lock under read-only `/home/peter/.cargo`. Retried as `CARGO_HOME=/home/peter/.cache/cargo-home-shared make rust-deny`: exit 2; cargo-deny 0.19.8 reports `advisories FAILED, bans ok, licenses ok, sources ok`. It reports rustix 1.1.4 under RUSTSEC-2026-0486 (fixed in >=1.1.6) and url 2.5.8 under RUSTSEC-2026-0592 (fixed in >=2.5.9), plus a yanked yoke-derive warning. These locked dependencies are unchanged by #1088. At the lead's explicit direction this is a non-blocking out-of-diff side result, not a PR finding, no new ticket or dependency-update scope. Logs: `/tmp/quoin-pr1088-reviews/deny.log`, `/tmp/quoin-pr1088-reviews/deny-writable-home.log`. cargo-deny's metadata reordered one existing lockfile dependency; the exact HEAD file was restored and the worktree was verified clean. No aggregate test suite ran.

## Focused Clippy Limitation

The locked-build invocation `cargo clippy --locked -p quoin-completeness -p quoin-cli --all-targets --all-features -- -D warnings`, with target directory `/home/peter/dev/worktrees/plat1638-1640-coverage/target`, exited 101. It stops in unchanged dependency crate quoin-evidence before checking the changed crates: 14 warnings-as-errors (too_many_lines in qualify/premises.rs:562 and qualify/runs.rs:128, match_same_arms in store/gc.rs:199 and store/records.rs:555, missing_errors_doc in store/records.rs:50, must_use_candidate and indexing_slicing in timestamp.rs:14-25). No such file is in this PR. The complete log is `/tmp/quoin-pr1088-reviews/clippy.log`. These are recorded as out-of-diff gate context, not attributed to this PR; warnings were not suppressed. The code lead was informed of the exact limitation.

The narrower `cargo clippy --locked -p quoin-completeness --all-targets --all-features -- -D warnings` completed with exit 0 through the same locked-build/private-target configuration. It verifies the touched shared-types crate, including test targets; the CLI lint check remains incomplete because of the unchanged dependency failures above. Log: `/tmp/quoin-pr1088-reviews/clippy-completeness.log`.

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | Round 1: commit 308f16c448737dd597f848dc8b8c893c26bd785c retags the unreadable-spec regression to FR-235-AC-11; the engine matrix includes it under AC-11 and excludes it from AC-4. |

## Disposition Round 1

**PASS; mergeable: yes** for the reviewed PR scope at `308f16c448737dd597f848dc8b8c893c26bd785c`. FND-001 is fixed; no new findings. The original findings text and original verdict remain unchanged as the historical review record.

Reviewed the entire delta from `7ee1f6b550c1dec3d0e3df6f32cf6afa97504bf7`: one trace literal changes in the unreadable-spec test, and this exact reviewer-authored artifact is added under reviews/. The committed artifact matches the review-pass scratch artifact byte for byte before this disposition appendix. Test bodies and production code are unchanged. Re-read the six other method artifacts; their scopes are unchanged and their PR-scoped PASS verdicts stand.

The fixed source excerpt at `rust/crates/quoin-cli/tests/tc_1636_validation_ratings.rs:125` is:

```rust
#[trace("FR-235-AC-11")]
#[test]
fn tc_235_ratings_coverage_reports_unreadable_spec_and_returns_partial() {
```

Quire `0.52.0` (engine `0.62.2`), executable `/home/peter/.npm-global/bin/quire`, returned exit 0 for both `quire trace --scope . --file rust/crates/quoin-cli/tests/tc_1636_validation_ratings.rs` and `quire matrix --scope . --format json`. The matrix shows AC-4 tagged only by the readable-bundle regression, while AC-11 is tagged by both that regression and the unreadable-spec partial-result regression. This directly verifies the repaired binding. `git diff --check` passes; the head/tree were verified exact and clean. Runtime assertions are unchanged and were already exercised in the first review; no broad test suite or aggregate gate was rerun.

Released Quoin remains `0.37.0` at `/tmp/plat1923-review-quoin037/node_modules/.bin/quoin`; schema module remains `spec-artifacts-process@v0.36.0`. Reviewer model: `gpt-6.1-sol`. Previously recorded out-of-diff cargo-deny advisories, dependency clippy errors and repository matrix baseline remain non-blocking context under the lead's explicit scope direction. GitHub reports MERGEABLE with mergeStateStatus BLOCKED; this disposition's mergeable=yes means no open review findings, not that branch protection was bypassed by the reviewer.
