---
id: SR-20261113203424
title: "Gap analysis - PLAT-1580 per-class obligations report"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quoin; PLAT-1580; full spec-to-test-to-source matrix plus changed Rust files"
review_set: subset
relationships:
  - {target: "ix://agent-ix/quoin/FR-161", type: references}
  - {target: "ix://agent-ix/quoin/NFR-034", type: references}
---

# Gap analysis - PLAT-1580 per-class obligations report

## Summary

Planless repository gap analysis for PLAT-1580. The current branch's computed Quoin matrix tags every FR-161 acceptance criterion, but the new happy path is only proven by a unit test with supplied join results, and the three NFR-034 measurement criteria have no trace bindings. The broader Quire matrix also reports 591 untagged criteria repository-wide; the findings below are the gaps in this change's assurance scope.

## Verdict

**CONDITIONAL** — two medium findings remain. Plan completion: not assessed.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-161-AC-1's successful design-and-evidence path is not exercised through command assembly and the FR-160 auditor join. The only CLI test has an empty design relation and missing evidence; the unit test supplies `build_rows` with a covered relation and an empty missing-method map, bypassing the production joins. | rust/crates/quoin-cli/tests/tc_160_class_policy_command.rs:358-378; rust/crates/quoin-cli/src/class_obligations.rs:492-523 |
| FND-002 | medium | The computed matrix leaves NFR-034-M-1, M-2, and M-3 untagged. `tc_161_001_class_obligations_reuses_the_audit_policy_without_writing` asserts class preservation, unchanged files, and repeat output but tags only FR-161 criteria and CON-1. Bind the test to the three measurements it checks. | rust/crates/quoin-cli/tests/tc_160_class_policy_command.rs:342-350 |

## Coverage

Plan completion: not assessed.

### Matrix

Ran `quoin matrix --repo /home/peter/dev/quoin/.worktrees/plat-1580-code --json` using the branch-built `quoin 0.31.0-57-g385ac280`: all FR-161-AC-1 through FR-161-AC-9 were `tagged`; NFR-034-M-1 through M-3 were `untagged`. The installed `quoin matrix` does not accept `--strict` (exit 2, unexpected argument); the matching `quire matrix --scope <repo> --format json --strict` path reported 591 untagged criteria across the repository and no ignored-only criteria. Existing corpus-wide warnings make that global count broader than this ticket; the focused Quoin matrix is used for this implementation's AC bindings.

### Reverse gaps and stubs

Read all changed production files and tests. The new CLI behavior is governed by FR-161; `quire.design_join` extends the existing quoin boundary operation set and is covered by the FR-086 boundary/adapter path. No TODO/FIXME/XXX, `todo!`, `unimplemented!`, placeholder implementation, or new duplicated parser was found in the diff. FR-161-CON-2 has no direct trace marker, but the repository Rust trace convention accepts requirement/AC IDs, not CON IDs; AC-7 is tagged and tests the carried-class-only behavior.

### Test evidence

The branch-built matrix binds all nine FR-161 ACs. `class_obligations::tests` passed (4), and `tc_161_001_class_obligations_reuses_the_audit_policy_without_writing` passed as a CLI integration test. Parent-reported checks also passed formatting, Clippy, and the core boundary and operation-census tests; the reviewer independently reran the focused unit and integration tests through the required locked-build wrapper.

## Review Scope

| Unit | Role | Excerpt / result |
| --- | --- | --- |
| `FR-161-AC-1` | examined | With a policy giving `class-3` the obligations `design-trace` and `evidence` (method `unit-testing`), a `class-3` requirement with a `covered` join row and unit-testing-kind evidence on all its obligations reports both rows `met`. |
| `FR-161-AC-2` | examined | The same requirement with a join row `uncovered` reports `design-trace` `unmet`; with one obligation lacking evidence of the method's kind it reports `evidence` `unmet` listing that obligation. |
| `FR-161-AC-3` | examined | A `class-1` requirement with a policy giving `class-1` no obligations has no rows; a requirement of a class the policy does not name has no rows and is listed under `unconfigured`. |
| `FR-161-AC-4` | examined | With an absent FR-086 join or `{"relations": []}`, every `design-trace` row is `not-available` naming the missing join; a requirement with no row in any design relation while the join is non-empty is `not-available`; a requirement whose row is `uncovered` is `unmet`; `evidence` rows are unaffected. A requirement with one `covered` row and one `uncovered` row in two design relations is `unmet`. |
| `FR-161-AC-5` | examined | A requirement whose `requirements[]` entry carries no class is listed as unassigned with no rows. |
| `FR-161-AC-6` | examined | `--format json` emits one object whose facts equal the Markdown rows, with no class key for an unassigned requirement, and the two formats agree row for row. |
| `FR-161-AC-7` | examined | A requirement's rows come from its own carried class only: the same requirement allocated to parts of other classes reports the same rows. |
| `FR-161-AC-8` | examined | A policy `obligations` entry with a kind other than `design-trace` or `evidence`, an `evidence` entry with no method or an undeclared method, or a duplicate `id` within a class is refused naming the cause. |
| `FR-161-AC-9` | examined | The command writes no file, record or class value, and two runs over the same inputs print identical output. |
| `NFR-034-M-1` | examined | Class values appearing in output that no input states: 0. Finding FND-002. |
| `NFR-034-M-2` | examined | Files or store records written by the three commands: 0. Finding FND-002. |
| `NFR-034-M-3` | examined | Differing outputs across repeated runs over identical inputs: 0. Finding FND-002. |
| Changed source and tests | examined | All 17 changed files were scanned for source-to-spec ownership, tags, stubs, duplicates, and changed behavior; see Findings and Coverage above. |
