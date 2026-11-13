---
id: SR-20261113080829
title: "Base spec review of per-class evidence floors and class obligations (FR-160, FR-161, NFR-034)"
type: SpecReview
analysis: base
review_set: subset
scope: "agent-ix/quoin; PR #855 (git diff origin/main...015a0665): spec/functional/FR-160-per-class-evidence-floors.md, spec/functional/FR-161-per-class-obligations-report.md, spec/non-functional/NFR-034-class-reports-state-and-never-assign.md, spec/functional/index.md, spec/non-functional/index.md"
---

# Base spec review of per-class evidence floors and class obligations (FR-160, FR-161, NFR-034)

## Summary

Ticket: PLAT-1526. I checked the diff against quoin main code and spec. On the quoin side I read FR-032, FR-039,
FR-094, `quoin-auditor/src/audit/{input,scores}.rs`, `quoin-evidence/src/types/{binding,run,independence}.rs`,
`quoin-quire-types/src/lib.rs` and `quoin matrix --help`. On the quire-rs side I read FR-053 and FR-082 on `origin/main`,
and FR-090 and FR-091 on the sibling PR quire-rs#669. I also read spec-objects-architecture FR-012
(spec-objects-architecture#45), spec-artifacts-iso FR-009 (spec-artifacts-iso#73), quire-rs#668 (FR-086, open) and the
spec-artifacts-process verification catalog. `quire validate --scope .` over the five changed files exits 0. It raises one
`ears:non-singular` warning, on NFR-034. No assurance profile applies: AP-201 covers finding scoring and measurement
persistence, and AP-202 covers Jev lenses.

These claims are confirmed:

- The FR-039 mutation floor and the FR-032 two-suite multiplicity are keyed on obligation criticality
  (`scores.rs` `mutation_finding`, `multiplicity_finding`).
- Class is never derived, defaulted or assigned: FR-160-CON-1, FR-160-AC-9, FR-161-AC-7, FR-161-CON-2 and NFR-034.
- Quoin holds no built-in class, policy or floor (FR-160-AC-11).
- Class and criticality are separate axes.
- There is no compatibility layer, fallback or shim, and no standard is named or quoted.
- There is no hash, digest or pin record.

FR-161 cites the design join only as the ticket "PLAT-1417". It does not cite the unmerged quire-rs FR-086.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | "Bound evidence for each listed method" has no defined join on main. Neither a binding nor a run records a verification method id. A binding carries obligation, suite, symbols and lineage. A run carries only `evidence_kind`, and catalog methods share kinds: `mutation-testing` is `Static`, the same kind as static analysis. So `class-method-missing` (FR-160-AC-2) and FR-161 `item-evidence` cannot be decided the same way by two implementers. State how a bound suite is judged to be evidence of a method, or record the method on the binding (a store change with its own AC). | FR-160-AC-2; FR-161-AC-1; FR-161-AC-2; rust/crates/quoin-evidence/src/types/binding.rs:57-66; rust/crates/quoin-evidence/src/types/run.rs:174-191; spec-artifacts-process manifest.yaml:1507-1513 |
| FND-002 | medium | FR-160-AC-1 says "byte-identical except for the added class column", and the Description says "with no `--class-policy` the audit behaves exactly as before". FR-160-AC-6 adds an unclassified count to the summary with no policy condition, and FR-160-AC-10 adds the column for every obligation. The three disagree on what changes without a policy. Name every delta (column or key, and unclassified count), or gate them on `--class-policy`. | FR-160-AC-1; FR-160-AC-6; FR-160-AC-10 |
| FND-003 | medium | FR-160-AC-5 writes "an obligation carrying `criticality: high`" but never says where obligation criticality comes from. On quire-rs main, obligation criticality is only the FR-053 per-criterion column. Frontmatter `criticality` reaches only the `requirements[]` entry (FR-082-AC-7). The AC follows the quire-rs#669 FR-090-AC-4 premise that frontmatter criticality reaches obligations. Under that premise a fixture using frontmatter `criticality: high` gets no criticality floor. State that criticality floors read the FR-053 obligation criticality, and build the fixture from the criterion's column. | FR-160-AC-5; quire-rs spec/functional/FR-082-coverage-matrix-criticality.md (AC-7); rust/crates/quoin-quire-types/src/lib.rs:112-117 |
| FND-004 | low | FR-160-AC-10 puts the string `unassigned` in JSON for an absent class. FR-090 says absent is absent, and that `unassigned` is a display of absence. FR-090-AC-6 carries any written value, so a written `unassigned` cannot be told apart from absence. Omit the key (or use null) in JSON, and show `unassigned` only in human output. | FR-160-AC-10 |
| FND-005 | medium | FR-161 does not name its item and allocation inputs. The coverage payload quoin reads carries obligations and no part records. FR-012-AC-11 puts `safetyClass.class` in `quire trace` output, and FR-161 does not name that output. "Allocation edges" names no verb. FR-091 keeps `allocated_to` as module data, so quoin would have to hard-code it. "Software item" does not say whether a system `part` (FR-012) also gets rows. Name the Quire output, the declaration that supplies the allocation verb, and the item selector. | FR-161; spec-objects-architecture FR-012-AC-11; quire-rs FR-091 (field_consistency edges) |
| FND-006 | medium | The `design-trace` kind reads "the design trace join (PLAT-1417)". That is a ticket, not a payload key. FR-086 (quire-rs#668) and `quire relations --design` (quire-cli#187) are both unmerged, so "the payload carries no design trace join" (FR-161-AC-4) has no defined test input. FR-161 also does not say which FR-086 entry state counts as "a design element linked". Name the output and key once FR-086 lands, with a `depends_on` edge, or keep `design-trace` out of FR-161 until then. | FR-161-AC-4; FR-161-AC-1; quire-rs#668 FR-086 |
| FND-007 | low | FR-161-AC-1 and FR-161-AC-2 use the method `unit-test`. The shipped catalog declares `unit-testing`. With the default catalog, FR-161-AC-8 would refuse the AC-1 policy as naming an undeclared method. Use `unit-testing`, or state that the fixture uses its own catalog. | FR-161-AC-1; FR-161-AC-8; spec-artifacts-process manifest.yaml:1435 |

## Verdict

Changes requested. The class-not-assigned, no-built-in-floor and separate-axis rules are sound. FND-001 is the blocker:
the required-method floor and `item-evidence` depend on a method-to-evidence join that the store does not have. FND-003
carries the FR-090-AC-4 / FR-082-AC-7 contradiction from quire-rs#669 into FR-160-AC-5.

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-008 | medium | FR-161 sets the boundary between `not-available` and `unmet` for `design-trace` at "no FR-086 join in the payload". FR-086 on main never leaves the join absent. A corpus with no `design_relations` yields `{"relations": []}` (FR-086-AC-9), and a requirement that no design relation obliges, such as an NFR when only an FR relation is declared, simply has no row. FR-161 does not say whether an empty join, or a requirement with no join row, is `not-available` or `unmet`. Two implementers would split there, and FR-161-AC-4's precondition cannot be built from FR-086 output. State that an empty `relations` list, and a requirement with no row in any design relation, are `not-available` (or `unmet`), and pin both cases in AC-4. | FR-161-AC-4; quire-rs spec/functional/FR-086-design-element-join.md (No design relations, AC-9) |

## Dispositions

Round 1, checked against head fa1fa4ce.

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | Required methods now join on the run `evidence_kind` against the catalog `evidence_kind` of the method. Methods of one kind are stated as interchangeable, and no store change is made (FR-160 Required methods, AC-2). Every shipped catalog method declares a kind. |
| FND-002 | fixed | Without `--class-policy` the audit and matrix add nothing, and with it the deltas are named (FR-160-AC-1, Unclassified and unconfigured). |
| FND-003 | fixed | Criticality floors read the FR-053 per-criterion column, and frontmatter `criticality` is stated not to be obligation criticality (FR-160 Description, AC-5). |
| FND-004 | fixed | JSON omits the class key for an absent class, and `unassigned` is human output only (FR-160-AC-6, AC-11; FR-161-AC-6). |
| FND-005 | fixed | Items, allocation and item class come from the policy's `item_relation` (FR-084 rows) and `carried` on the row `document` (quire-rs FR-090-AC-9). The system part case is pinned (FR-161-AC-10). |
| FND-006 | fixed | `design-trace` reads the FR-086 join row `state: covered`, which matches FR-086 as merged. The `realized_criteria` description matches FR-086 (per resolved `linked` entry). The remaining boundary case is FND-008. |
| FND-007 | fixed | The examples use `unit-testing` (FR-161-AC-1). |
