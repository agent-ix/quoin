---
id: SR-967175026302197768900474181941935084875
title: "gap-analysis of quoin PR 1091"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quoin; PLAT-1915; rust/crates/quoin-cli/tests/tc_1915_completeness_advisory.rs; rust/crates/quoin-core/src/ops/completeness.rs; FR-037"
review_set: subset
---

## Summary

Reviewed PR #1091 at 7bd500ed2d320d22d28c2ffe024d86abea6e816d. The new CLI tests and existing core assertion correctly bind FR-037-AC-9 through AC-13 and CON-4.

## Verdict

FAIL for repository assurance — 493 Test criteria remain untagged and 11 have ignored-only bindings. No PR defect was found in the FR-037 additions; the full reverse ownership audit remains incomplete.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Coverage

Plan completion: not assessed

Computed repository matrix: 2,205 Test criteria, 1,701 tagged, 493 untagged, 11 tagged only by ignored tests. All FR-037 Test criteria are tagged; CON-2 and CON-3 are Inspection criteria. These repository gaps are pre-existing campaign work, not defects introduced by this PR. No repository-wide PASS is claimed.

Reverse and stub inspection: inventoried the completeness command (five flags), its three boundary operations, and public completeness-domain items against FR-037 and FR-096. The PR introduces no production behavior. Repository-wide scans covered 952 Rust files and found zero todo! or unimplemented! macros. The matrix contains 250 requirement records. The changed tests contain real assertions and no mocks, ignored paths, source inspection, or stub bodies. Detailed reverse ownership was limited to the changed area's production path; the complete repository surface was not manually adjudicated. Optional semantic audit not requested and not run; ordinary code-review trace-binding checks are recorded.

## Assurance Context

Inspected `spec/assurance/AP-201-finding-quality.md` (AP-201) and `spec/assurance/AP-202-jev-lens-evaluation.md` (AP-202). Neither applies to the test-only completeness change. Unavailable selected context: none. Active exceptions: none.

## Examined units and bindings

```yaml
scope:
- id: FR-037-AC-9
  path: spec/functional/FR-037-declared-vocabulary-completeness.md
  role: examined
  excerpt: '`quoin completeness` reports the gaps over a real bundle and exits 0 while
    advisory.'
- id: FR-037-AC-10
  path: spec/functional/FR-037-declared-vocabulary-completeness.md
  role: examined
  excerpt: '`quoin completeness` exits non-zero on an unjustified exclusion without
    `--strict`.'
- id: FR-037-AC-11
  path: spec/functional/FR-037-declared-vocabulary-completeness.md
  role: examined
  excerpt: '`quoin completeness --strict` exits non-zero on gaps alone.'
- id: FR-037-AC-12
  path: spec/functional/FR-037-declared-vocabulary-completeness.md
  role: examined
  excerpt: A bundle with no declared vocabulary reports `UNCHECKED`, never `PASS`.
- id: FR-037-AC-13
  path: spec/functional/FR-037-declared-vocabulary-completeness.md
  role: examined
  excerpt: The unowned set quoin reports equals the one the bundle read yields for
    the same declaration.
- id: FR-037-CON-4
  path: spec/functional/FR-037-declared-vocabulary-completeness.md
  role: examined
  excerpt: An absent vocabulary SHALL NOT report `PASS`. Nothing checked is not nothing
    wrong.
bindings:
- test_id: tc_1915_completeness_reports_a_real_gap_as_advisory
  ac_id: FR-037-AC-9
  trace: correct
- test_id: tc_1915_unjustified_exclusion_fails_without_strict_mode
  ac_id: FR-037-AC-10
  trace: correct
- test_id: tc_1915_strict_mode_fails_on_an_unowned_gap_alone
  ac_id: FR-037-AC-11
  trace: correct
- test_id: tc_1915_reported_unowned_values_match_the_bundle_projection
  ac_id: FR-037-AC-13
  trace: correct
- test_id: an_assessment_crosses_the_boundary_with_no_filesystem_at_all
  ac_id: FR-037-AC-12
  trace: correct
- test_id: an_assessment_crosses_the_boundary_with_no_filesystem_at_all
  ac_id: FR-037-CON-4
  trace: correct
```

## Checks

`cargo fmt --all --check` and `git diff --check` passed. `quire trace --scope . --file ...` resolved the new CLI and core claims (exit 0); it emitted existing cfg-platform symbol-id-collision diagnostics in campaign, selftest, snapshot-equivalence, config lock/service, and store bounded-read files. These unrelated paths are unchanged. The initial independent test attempt exited 101 because generated node_modules schema inputs had been removed; standard npmjs-only `make build-inputs` restored them. Independent `cargo test --locked -p quoin-cli --test tc_1915_completeness_advisory`: 4 passed, 0 failed, 0 ignored, exit 0. Focused `cargo clippy --locked -p quoin-cli --test tc_1915_completeness_advisory -- -D warnings` exited 101 with 14 diagnostics in unchanged `quoin-evidence` paths: `qualify/premises.rs:562`, `qualify/runs.rs:128` (too_many_lines); `store/gc.rs:199`, `store/records.rs:555` (match_same_arms); `store/records.rs:50` (missing_errors_doc); `timestamp.rs:14` (must_use_candidate) and `timestamp.rs:19-25` (indexing_slicing). These files are byte-identical to the PR base. Clippy did not reach the new test target; no clean Clippy result is claimed. Per the owner’s no-new-scope rule, unrelated side findings are recorded and left; they are not introduced by this PR. Coder-reported focused core test: 1 passed; reviewed its concrete assertions independently.
