---
id: SR-2026112016223475846355010
title: "PLAT-1836 quoin PR 1032 spec-review integrity"
type: SpecReview
analysis: integrity
scope: "agent-ix/quoin; spec/functional/FR-160-per-class-evidence-floors.md, spec/functional/FR-161-per-class-obligations-report.md"
review_set: subset
relationships:
  - target: "ix://agent-ix/quoin/FR-160"
    type: references
  - target: "ix://agent-ix/quoin/FR-161"
    type: references
---

# PLAT-1836 quoin PR 1032 spec-review integrity

## Summary

Ticket: PLAT-1836. Integrity review of the FR-160 and FR-161 edits (passing-run rule, `class` member, `gate` key, three relation-backed kinds, AC-14..16 and AC-10..13) against each other and against the sibling requirements they touch (FR-032, FR-115). New AC ids are sequential, collide with no open PR (open PRs: #1031, #998, #999 touch no spec under FR-160/FR-161), and the class policy stays user data: the spec names no class value, archetype or relation.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-160 now says "`quoin matrix` reports class findings and gates nothing; the matrix has `--strict` of its own." FR-115-AC-11 states `quoin matrix` has no `--strict` flag, and the CLI has none (`quoin matrix --help`; no `strict` arg in quoin-cli matrix). The added clause is false; drop it or point at `quoin evidence audit --strict`. | spec/functional/FR-160-per-class-evidence-floors.md:116-117; spec/functional/FR-115-evidence-backed-test-matrix.md:336 |
| FND-002 | medium | The gate adds a second non-zero exit to `quoin evidence audit`, but FR-032 (Outputs: "Under `--strict`, a non-zero exit"; Downstream: "this command reports and, under `--strict`, exits non-zero") and FR-115-AC-11 ("`audit.rs`'s `Outcome::Partial` arm fires only when `arguments.get_flag(\"strict\")`") still state `--strict` is the only trigger. They now contradict FR-160 and the code; update them in this PR. | spec/functional/FR-032-evidence-auditor.md:71; spec/functional/FR-032-evidence-auditor.md:130; spec/functional/FR-115-evidence-backed-test-matrix.md:336 |
| FND-003 | medium | A relation-backed entry whose `relation` names no relation of the loaded modules (a typo, or a module not loaded) reads `not-available` for every requirement, and FR-161 says `not-available` never gates. Under `gate: true` that is a gate that silently passes over an activity the policy demands. FR-160 refuses the analogous case for methods precisely because "A floor that silently does not apply reads as a passing gate". Distinguish "the join declares no relation named R" (a policy/configuration error: refuse, or gate under `gate: true`) from "the requirement has no row in R" (not obliged: `not-available`). Today, with no module declaring SOUP/segregation relations, those two policy kinds can never gate. | spec/functional/FR-161-per-class-obligations-report.md:66-68; spec/functional/FR-161-per-class-obligations-report.md:85-87; spec/functional/FR-160-per-class-evidence-floors.md:66 |
| FND-004 | low | "quoin holds no default for it" contradicts the stated and implemented behaviour that an absent `gate` means off (`#[serde(default)] gate: bool`). Say that an absent `gate` is off and that quoin writes none. | spec/functional/FR-160-per-class-evidence-floors.md:115-116 |

## Verdict

CHANGES REQUESTED (FND-001, FND-002, FND-003). The passing-run definition is internally consistent but rests on whole-run outcomes; see code-review SR-2026112016223475846355008 FND-001 for the binding-symbol inconsistency with FR-032.

## Examined

- FR-160 statement and sections "The class policy is data", "Required methods", "Class findings can gate" (examined)
- FR-160-AC-2, FR-160-AC-6, FR-160-AC-13, FR-160-AC-14, FR-160-AC-15, FR-160-AC-16 (examined)
- FR-161 sections "Declared as data", "Unmet activities can gate" (examined)
- FR-161-AC-8, FR-161-AC-10, FR-161-AC-11, FR-161-AC-12, FR-161-AC-13 (examined)
- FR-032 Outputs and Downstream, FR-115-AC-11 (context_only)

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-005 | low | The relation-backed status sentence still says `not-available` "if the join is absent or empty", but the next sentence (and `check_relations_declared`) refuses any named `relation` the join does not declare, which an empty join never does. The empty-join clause now applies only to a `detailed-design` entry without `relation`; say so, so the two sentences do not read as conflicting outcomes for the same input. | spec/functional/FR-161-per-class-obligations-report.md:66-70 |

## Dispositions

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | a102c716: FR-160 now reads "`quoin matrix` reports class findings and gates nothing; it has no `--strict` flag (FR-115)". |
| FND-002 | fixed | a102c716: FR-032 Outputs and Downstream name `gate: true` as a second non-zero exit beside `--strict`; FR-115-AC-11 says the `Partial` arm fires on `--strict` "and also when a class policy given by `--class-policy` sets `gate: true` and a finding it caused remains". `quire validate` passes on FR-032, FR-115, FR-160, FR-161. |
| FND-003 | fixed | a102c716: `check_relations_declared` refuses a `relation` absent from the design join's declared relations with an `Err` naming class, entry and relation, the same path and exit status as an undeclared method (tc_plat1836_007 compares the two exit codes; empty stdout). quire-rs v0.62.2 `design_join` emits a section for every declared design relation even with no rows, so a declared relation is never refused. A requirement with no row in a declared relation stays `not-available` (a_named_relation_reads_only_its_own_rows, tc_plat1836_003). New FR-161-AC-14, FR-161-AC-11 updated. Passed. |
| FND-004 | fixed | a102c716: FR-160 now reads "Quoin SHALL NOT write a `gate` value: the policy author sets it, and an absent `gate` means off." |
| FND-005 | still-open | New in this pass (low); no fix yet. Does not block merge. |
