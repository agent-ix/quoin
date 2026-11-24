---
id: SR-2026112016223475846355012
title: "PLAT-1855 quoin PR 1042 spec-review EARS"
type: SpecReview
analysis: ears-conformance
scope: "agent-ix/quoin; spec/functional/FR-161-per-class-obligations-report.md, spec/functional/FR-165-matrix-design-column.md"
review_set: subset
relationships:
  - target: "ix://agent-ix/quoin/FR-161"
    type: references
  - target: "ix://agent-ix/quoin/FR-165"
    type: references
---

# PLAT-1855 quoin PR 1042 spec-review EARS

## Summary

Ticket: PLAT-1855. EARS conformance and criterion testability of the requirement text PR 1042 (head d561a059) adds or amends. The FR-161 and FR-165 statements are unchanged and conform; the `Partial` (exit 1) gate wording is unchanged and still matches FR-160. No score, rank or acceptability is assigned by the tool, no paywalled standard text, and no pin, SHA or count is introduced. `quire validate --scope .` on both files exits 0 (quire 0.52.0, engine 0.62.2). FR-161-AC-12 (amended) and FR-161-AC-15 are atomic and testable. Three new criteria have a weak or unobservable oracle.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-161-AC-16 "reads ... as before for a requirement outside the scope" has no oracle: "as before" names no baseline. The `when_any` scope is invisible to `quoin class obligations` (it reads the FR-086 join rows only), so a unit test over a join fixture is AC-4 again. State the oracle, e.g. "reads the same status as over the same join without that relation", and say whether the fixture must be an engine-produced join over a module declaring `when_any` (end to end) or a join fixture. | spec/functional/FR-161-per-class-obligations-report.md:136 |
| FND-002 | low | FR-161-AC-17 asserts only "reports a row for each and is not refused"; "each satisfiable by a loaded relation" is undefined and no status is asserted, so a pinned `design-trace` row reading `not-available` (wrong relation lookup) passes. Give the fixture (a `covered` row in each named relation) and assert `met` for all three rows. | spec/functional/FR-161-per-class-obligations-report.md:137 |
| FND-003 | medium | FR-165-AC-20's precondition "a relation scoped by `when_any`" is not observable by `matrix.build`: the `coverage` payload carries no scope and FR-165-CON-1 forbids filtering, so the criterion reduces to AC-1 plus an absent block. Its outcome "markdown shows `Design gap:` only under the first" is also false for a fixture whose second requirement has another `uncovered` block (for example the realization relation). Drop the `when_any` qualifier (or move the scope claim to an engine-side criterion) and require the second requirement's other blocks to be `covered`. | spec/functional/FR-165-matrix-design-column.md:128 |

## Verdict

CHANGES REQUESTED (medium). Statements conform; FR-161-AC-16 and FR-165-AC-20 need a stated, observable oracle and FR-161-AC-17 a status assertion before the code step tags tests to them.

## Examined

- FR-161 statement (examined): "`quoin class obligations <scope> --class-policy <file> [--format markdown|json]` SHALL report, for each requirement the Quire coverage payload lists, which of the activities its software safety class demands are met, unmet or not available."
- FR-161 "Unmet activities can gate" SHALL/SHALL NOT clauses (examined, unchanged, consistent with FR-160 `Partial` exit 1)
- FR-161-AC-12, FR-161-AC-13, FR-161-AC-14, FR-161-AC-15, FR-161-AC-16, FR-161-AC-17 (examined)
- FR-161 section "Effect of design relations a module adds" (examined; descriptive, no SHALL)
- FR-165 statement and the added paragraph after "Rendering" (examined)
- FR-165-AC-1, FR-165-AC-20 (examined)

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-004 | low | FR-161-AC-12 now ends with a sentence about a test, not about behaviour: "so the existing test that refuses a `design-trace` entry with `relation` is superseded by this and AC-15". A criterion states what the command does; which test changes is code-step work. The behavioural half ("A `design-trace` entry naming a non-blank `relation` that a loaded module declares is accepted") is enough; drop the clause about the test. | spec/functional/FR-161-per-class-obligations-report.md:139 |
| FND-005 | low | The amended FR-193 bullet carries two requirements in one item (SHALL record one gap per `uncovered` row naming the check token, and SHALL NOT state no design element at all), while the neighbouring bullets split each SHALL into its own item (the dangling and kind-mismatch rules are each two bullets). Split it so each clause is one testable statement. | spec/functional/FR-193-dossier-architecture-content.md:48 |

## Dispositions

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | 17892164: FR-161-AC-16 now names X and Y over one corpus, adds an `uncovered` row of S for X only, and states the oracle: X turns `met` to `unmet` with `missing` listing S, and Y's status and `missing` equal those over the join without S. This is measurable over a join fixture and consistent with one engine row per requirement per relation. |
| FND-002 | fixed | 17892164: FR-161-AC-17 drops "satisfiable", names S and G declared by loaded modules, and asserts `met`, `unmet` and `not-available` per entry. |
| FND-003 | fixed | 17892164: FR-165-AC-20 drops the `when_any` precondition and states observable blocks: A with `uncovered` P and Q gives two `Design gap:` lines, B with no P block and `uncovered` Q gives one, C with only `covered` blocks gives none. This matches FR-165's one-line-per-`uncovered`-block rendering. |
