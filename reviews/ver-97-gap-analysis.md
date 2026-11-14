---
id: SR-20261114002404
title: "Scoped gap analysis of Quoin PR 891 (VER-97)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quoin@028b3273ac8a6091f4bbf177cbe7270744253abd; FR-153-AC-12..18 and changed Rust implementation/tests"
review_set: subset
---

# Scoped gap analysis of Quoin PR 891

## Summary

Ticket: VER-97. Plan completion: not assessed. This review is limited to the requested PR diff. The installed Quoin exposes no `gap-analysis` command (`quoin gap-analysis --help` exits 2), so I manually checked the changed requirements, tests and implementation. `quire matrix --scope . --format json` (quire 0.45.0, engine 0.60.0) reports FR-153-AC-16 tagged by `tests::exact_pair_introduction_commit_admits_mapped_actor` and AC-18 tagged by the fail-closed and unresolved-introduction tests. AC-12, AC-14, AC-16, AC-17 and AC-18 are tagged; AC-13 and AC-15 are untagged. The added test function id `tc_001_projection_contains_only_the_qualified_allowlist` is unique in the Rust workspace.

`quoin matrix --repo . --json` exited 0 but reports AC-16 and AC-18 as `untagged` with no binders. The direct `quire matrix` result confirms both mappings, so this is a diagnostic/output discrepancy in the installed Quoin matrix tool, not a coverage defect in this PR. The Quoin/Quire command output also carries existing module and cross-platform symbol diagnostics outside the changed files.

The reverse-gap check over changed behavior found no unowned behavior or hollow tests. The record projection writer persists only FR-151 allowlisted projection members; read-time observations remain in the CLI layer. FR-093 source/spec and the release workflow files are unchanged.

## Verdict

**FAIL — two high traceability gaps in the scoped criteria.** The full repository gap-analysis workflow was unavailable as a CLI; the findings below come from the computed Quire matrix, limited to the requested diff scope.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | FR-153-AC-13 has no tagged test; the computed matrix reports it untagged | spec/functional/FR-153-analysis-result-counting-by-disposition.md, rust/crates/quoin-cli/src/analysis_evidence.rs::retained_runs |
| FND-002 | high | FR-153-AC-15 has no tagged test; the computed matrix reports it untagged | spec/functional/FR-153-analysis-result-counting-by-disposition.md, rust/crates/quoin-cli/src/run_observation.rs::validate |

## Coverage

Reconciliation: manual changed-scope acceptance-criteria-to-tests review with `quire matrix` (quire 0.45.0, engine 0.60.0). `quoin matrix` output omitted valid binders for the two specifically checked criteria; direct Quire output resolved the discrepancy. Changed criteria checked: 7 (AC-12..18); tagged: 5; untagged: 2 (AC-13, AC-15). The new `tc_001_projection_contains_only_the_qualified_allowlist` test function id is unique in the Rust workspace. Source behaviors inventoried in the changed feature: repeatable observation input, selected-record loading, host-fact qualification, exact Git introduction/current-byte check, and exact-target matrix/assurance rendering; untraced: 0. Source stubs: 0. Test stubs: 0. Plan completion: not assessed.

## Examined scope

- FR-153-AC-12..18 and FR-153-CON-1..3
- FR-153-AC-13 is untagged; FR-153-AC-15 is untagged and its empty-CI behavior is refused by the input validator rather than reported as unevaluated
- FR-093-CON-1..3
- FR-040-AC-20 and FR-115-AC-16
- `rust/crates/quoin-cli/src/{run_observation.rs,analysis_evidence.rs,matrix.rs,assurance.rs}`
- `rust/crates/quoin-assurance/src/{matrix.rs,case.rs,case/leaf.rs}` and their changed tests
- `rust/crates/quoin-evidence/src/{adapters/differential_report.rs,record.rs,store/records.rs,qualify/**}`
- `rust/crates/quoin-core/src/ops/evidence/records.rs`
