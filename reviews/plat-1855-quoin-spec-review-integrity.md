---
id: SR-2026112016223475846355013
title: "PLAT-1855 quoin PR 1042 spec-review integrity"
type: SpecReview
analysis: integrity
scope: "agent-ix/quoin; spec/functional/FR-161-per-class-obligations-report.md, spec/functional/FR-165-matrix-design-column.md"
review_set: subset
relationships:
  - target: "ix://agent-ix/quoin/FR-161"
    type: references
  - target: "ix://agent-ix/quoin/FR-165"
    type: references
  - target: "ix://agent-ix/quoin/FR-193"
    type: references
---

# PLAT-1855 quoin PR 1042 spec-review integrity

## Summary

Ticket: PLAT-1855. Consistency of PR 1042 (head d561a059) with origin/main quoin code, the quire-rs engine, quire-cli FR-043 and quoin's dossier specs.

Measured and consistent: the pinned `design-trace` reading equals today's `relation_status` with no archetypes (`rust/crates/quoin-cli/src/class_obligations.rs:379-407`, shared `fold_rows` at 344); `check_relations_declared` (412-437) already checks the `relation` of every kind, so AC-15's refusal clause needs no new check; pinned rows already emit a `relation` key (212). Engine: the finding, the FR-084 relation report and the FR-086 join all select their population through one `obliged()` (quire-rs `src/corpus/required_relations.rs:400`, used by `relation_report.rs:546` and `required_relations.rs:284`), which applies `from`, `where`, `when_any`, `exclude`; so "a missing SOUP/segregation edge is a gap only for an in-scope requirement" is what the engine does today, not a new rule. `design_relations` merge across modules in module order without duplicates (quire-rs `src/loader/mod.rs:1662`, FR-086-CON-1). FR-086 states "A relation's `from` names one archetype", so the two-entry FR/NFR advice is correct. Both relations are in open sibling PRs (spec-objects-security #79 FR-030, spec-objects-architecture #59 FR-016); both modules are quoin defaults, so the unpinned `design-trace` met-to-unmet flip, the `quire matrix --strict` failure and the matrix gap reach every default corpus whose requirements satisfy a `control` or are referenced by a `soup_record`. Stating that as the declared consequence is right (the activities are real IEC 62304 class demands and the PR gives policies a pin), but two downstream statements are inaccurate (FND-002, FND-003). No overlap with other open quoin PRs (1039, 1040, 1041, 1043 touch none of FR-161, FR-165, FR-193, FR-194 or `class_obligations.rs`).

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The unchanged Status section contradicts the new pinned reading: "For `design-trace`: a join row with `state: covered` is `met`; a join row with `state: uncovered` is `unmet`; ... a requirement with no row in any design relation, is `not-available`". For an entry pinned to `R`, an `uncovered` row in another relation is not `unmet` and a requirement with rows elsewhere but none in `R` is `not-available` (AC-15). Qualify the Status rule by "in the relations the entry reads". | spec/functional/FR-161-per-class-obligations-report.md:102-109 |
| FND-002 | medium | The new section claims the dossier (FR-193, FR-194) reads the join verbatim "so each added relation's block appears there with its `check` token and no quoin code changes". FR-193 maps every `uncovered` row to a `no_design_element` gap naming only the requirement: no `check` token, a label that is false for a requirement that has a realizing element but lacks the SOUP or segregation edge, and two uncovered relations on one requirement give indistinguishable gaps (same FR-191 sort key). Either amend FR-193 (gap detail carries the relation / `check`) in this PR, or narrow the claim to FR-165/FR-194 and state the FR-193 consequence. | spec/functional/FR-161-per-class-obligations-report.md:94-96; spec/functional/FR-193-dossier-architecture-content.md:48 |
| FND-003 | low | The population is stated as the relation's `when_any` scope only ("the engine gives a row only to a requirement in the relation's `when_any` scope"), but the engine's `obliged()` selects `from`, then `where`, then `when_any`, then `exclude` (FR-058). The `--strict` bullet then cites quire-cli FR-043 for a `when_any` conclusion, while FR-043 says the gate applies per relation's "`from`, `where` and `exclude`" and never names `when_any` (true in the engine, absent from FR-043's text; an upstream quire-cli wording gap). Say "the relation's obliged population (`from`, `where`, `when_any`, `exclude`, quire-rs FR-058)" in FR-161 and FR-165. | spec/functional/FR-161-per-class-obligations-report.md:85-92; spec/functional/FR-165-matrix-design-column.md:98-100 |
| FND-004 | low | The Description never says a `design-trace` entry naming `element_archetypes` (or a method) is refused; only AC-12 does. The `relation` line now names `design-trace`, while the `element_archetypes` line says "optional for the other two" inside the same bullet, so a reader cannot tell whether `design-trace` is one of "the other two". State the refusal in the `design-trace` paragraph. | spec/functional/FR-161-per-class-obligations-report.md:53-58,66-68 |
| FND-005 | medium | An unpinned `design-trace` row that turns `unmet` names no relation. The PR knowingly widens what that entry reads, so after a module upgrade a gated run exits `Partial` with a row that does not say which relation is uncovered, while an `evidence` row lists its `missing` obligations. Pinned rows already carry `relation`. Answering the author's open question: yes, state that an `unmet` row of `design-trace` (and of the relation-backed kinds when unpinned) lists the uncovered relation names in `missing`, and add it to AC-15/AC-16. | spec/functional/FR-161-per-class-obligations-report.md:104-106 |

## Verdict

CHANGES REQUESTED (medium). The spec is consistent with the engine and with today's quoin code paths, and the intended consequence is correctly identified; fix the Status contradiction, the FR-193 claim and the unmet-row diagnosis before the code step.

## Examined

- FR-161 `design-trace` bullet and pinned-relation paragraph (examined)
- FR-161 `relation` / `element_archetypes` sub-bullets and the undeclared-relation refusal (examined)
- FR-161 "Effect of design relations a module adds" section, all four bullets (examined)
- FR-161 Status section (examined)
- FR-165 added paragraph (examined)
- FR-193 Behavior `no_design_element` rule (context_only)
- FR-194 design cell rule (context_only)
- quire-rs FR-086 (context_only), FR-058 `when_any` (context_only)
- quire-cli FR-043 Design gate (context_only)
- spec-objects-security PR 79 FR-030 and spec-objects-architecture PR 59 FR-016 relation declarations (context_only)

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-006 | medium | The dossier trace matrix generator (DOS-TM, FR-194) also emits a `no_design_element` gap with the detail "has no design element", once per requirement with any `uncovered` block. FR-194 specifies no design gap at all, and this PR leaves FR-194 untouched, so after FR-030 and FR-016 load, DOS-TM states "has no design element" for a requirement that has a realizing element but lacks the SOUP or segregation edge. That is the defect FR-193 now forbids, left in FR-194's generator. The new FR-161 bullet names FR-193 only. State FR-194's rule here as FR-193's (one gap per `uncovered` block carrying its `check` token, never "no design element" when one exists under another relation) or have the code step drop the gap from DOS-TM. | rust/crates/quoin-dossier/src/trace_matrix.rs:100-112; spec/functional/FR-194-dossier-trace-matrix-content.md:48-57 |

## Dispositions

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | still-open | 17892164 qualifies only the `not-available` half of the Status rule ("for an entry pinning `relation`, no row in that relation"). The sentence "a join row with `state: uncovered` is `unmet`" is still unqualified, so for an entry pinned to R, an `uncovered` row in another relation still reads as `unmet`, which contradicts AC-15. Fix: "a join row it reads". |
| FND-002 | fixed | 17892164: FR-193 now records one `no_design_element` gap per `uncovered` row naming the relation's `check` token, and SHALL NOT state the requirement has no design element when it has one under another relation. FR-193-AC-9 covers both cases. FR-161 now names this as a dossier generator change for the code step. quoin-dossier `architecture.rs:144-156` (a set of requirements, one gap, detail "has no realizing design element") must change. No open quoin PR touches `architecture.rs` or FR-193; PR 1040 and PR 1041 touch `gap.rs` only. |
| FND-003 | fixed | 17892164: FR-161 now says the engine decides the population from "the `from`, `where`, `when_any` and `exclude` of the relation (quire-rs FR-058)" and that quoin cannot see the scope. It states the quire-cli FR-043 `when_any` wording gap as upstream and does not edit it. FR-165 says `matrix.build` sees only emitted blocks. |
| FND-004 | fixed | 17892164: the `element_archetypes` sub-bullet reads "optional for `soup-requirements` and `segregation`; not accepted on `design-trace`", and the `design-trace` paragraph states the refusal. |
| FND-005 | fixed | 17892164: an `unmet` `design-trace` row lists in `missing` each relation with an `uncovered` row, in join order (a pinned entry lists only its own relation). The change is stated as affecting only `unmet` rows, and AC-15 and AC-16 assert it. The code emits `missing` from `Status.missing` (`class_obligations.rs:214`), so the code step only has to keep relation names when collecting rows. |
