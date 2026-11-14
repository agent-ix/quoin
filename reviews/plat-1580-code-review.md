---
id: SR-20261113203423
title: "Code review - PLAT-1580 per-class obligations report"
type: SpecReview
analysis: code-review
scope: "agent-ix/quoin; PLAT-1580; origin/main...HEAD; Rust implementation, tests, FR-161 and NFR-034"
review_set: subset
relationships:
  - {target: "ix://agent-ix/quoin/FR-161", type: references}
  - {target: "ix://agent-ix/quoin/NFR-034", type: references}
---

# Code review - PLAT-1580 per-class obligations report

## Summary

Reviewed PLAT-1580's Rust diff, its FR-161/NFR-034 requirements, tests, and Rust boundary behavior. The class is read from the coverage matrix and the FR-086/FR-160 joins are reused, but the default Markdown report drops required status facts.

## Verdict

**CONDITIONAL** — one medium-severity output defect remains in the Markdown renderer.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Markdown omits activity kind, method, and reason fields. A `design-trace` row with `not-available` therefore fails to name the missing join, and the Markdown facts do not match the JSON facts. | rust/crates/quoin-cli/src/class_obligations.rs:420-444 |

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
| `FR-161-CON-1` | examined | The report SHALL emit no key named or typed as a score, rank, verdict, approval or recommendation. |
| `FR-161-CON-2` | examined | Quoin SHALL NOT infer a requirement's class, an allocation or a design link. |
| `NFR-034` | examined | Quoin SHALL read every software safety class from the Quire payload as a person wrote it. Quoin SHALL NOT assign, default, infer, raise or write a class in any output, file or store. Quoin SHALL return the same findings and rows, in the same order, for the same payload, policy and store. |
| `NFR-034-M-1` | examined | Class values appearing in output that no input states: 0. |
| `NFR-034-M-2` | examined | Files or store records written by the three commands: 0. |
| `NFR-034-M-3` | examined | Differing outputs across repeated runs over identical inputs: 0. |
| `rust/crates/quoin-cli/src/class_obligations.rs` | examined | Command assembly, class source, activity validation, evidence and design status joins, JSON and Markdown renderers, and unit tests. Finding FND-001. |
| `rust/crates/quoin-cli/tests/tc_160_class_policy_command.rs` | examined | End-to-end command test, policy reuse, no-write and repeat-run checks. |
| Other 15 changed Rust files | examined | Read-only `quire.design_join` operation and FR-086 adapter, error taxonomy, host wiring, policy reuse, tests, lockfile/dependency scope; no other defects found. |

The Rust unit suite `class_obligations::tests` passed (4 tests), and the `tc_161_001` CLI integration test passed. The parent supplied passing format and Clippy checks through `/home/peter/dev/worktrees/locked-build.sh`; my separate format check remained queued behind occupied build lanes and I stopped it after confirming the parent check. The reviewer build initially required generated schema inputs; `pnpm install --frozen-lockfile` succeeded through the wrapper with its store under `/home/peter/.cache/pnpm-store`.

## Finding detail

`render_markdown` writes only each activity's `id`, status, and missing-obligation list (lines 420-444). It never emits `kind`, `method`, or `reason`, though JSON contains these fields. For the default format, `not-available` therefore does not name the absent design join as FR-161-AC-4 requires. The same dropped fields make the JSON and Markdown facts differ under AC-6. Include the fields in Markdown and assert their presence and parity in the CLI test.
