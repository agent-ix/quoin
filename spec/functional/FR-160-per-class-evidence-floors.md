---
id: FR-160
title: "Per-class evidence floors: required methods, mutation floor and independence configured per software safety class"
type: FR
relationships:
  - target: "ix://agent-ix/quoin/StR-004"
    type: "traces_to"
  - target: "ix://agent-ix/quoin/FR-032"
    type: "extends"
  - target: "ix://agent-ix/quoin/FR-039"
    type: "extends"
  - target: "ix://agent-ix/quoin/FR-094"
    type: "extends"
  - target: "ix://agent-ix/quire-rs/spec/functional/FR-090"
    type: "depends_on"
  - target: "ix://agent-ix/spec-artifacts-iso/FR-009"
    type: "references"
---

# FR-160: Per-class evidence floors: required methods, mutation floor and independence configured per software safety class

Tracked by Linear PLAT-1526 (implemented by PLAT-1579).

## Description

Quoin already applies a floor per criticality: `--mutation-floor <criticality>=<ratio>`
([FR-039](./FR-039-mutation-score-threshold.md)), two independent suites
([FR-032](./FR-032-evidence-auditor.md)) and the profile-selected independence dimensions
([FR-094](./FR-094-profile-selected-evidence-independence.md)). This requirement keys the same three floors on the
**software safety class** a person stated on the requirement an obligation derives from
([spec-artifacts-iso FR-009](ix://agent-ix/spec-artifacts-iso/FR-009), carried as
`software_safety_class` by [quire-rs FR-090](ix://agent-ix/quire-rs/spec/functional/FR-090)). Class and criticality
are separate axes; an obligation can have either, both or neither, and each selects its own floors.

### The floor is data, and no class has a built-in one

`quoin evidence audit --class-policy <file>` reads a normalized class policy (YAML or JSON) holding, per class value:
`required_methods` (verification method ids the catalog declares), `mutation_floor` (a ratio in `[0, 1]`),
`independence` (the FR-094 dimensions the obligation's evidence must differ on) and a `rationale`; any of the three
floors may be omitted for a class. The class values are the keys of the policy and must be values the Quire payload's
`software_safety_class` enumeration declares. Quoin holds no class value, no default policy and no floor. With no
`--class-policy` the audit behaves exactly as before. The policy is the project's recorded decision, written by a person,
and quoin never writes it.

A policy that names a class the payload does not declare, a method the catalog does not declare, a ratio outside
`[0, 1]` (refused as FR-039 refuses `80`), an unknown independence dimension or an empty rationale is refused with the
cause named, and no audit runs: a floor that silently does not apply reads as a passing gate.

### How each floor is applied

The class of an obligation is the class carried on it, and only that. Quoin does not take the class from an allocated
item, from criticality or from a neighbouring obligation.

- **Required methods.** An obligation of a class with `required_methods` needs bound evidence for each listed method; each
  missing method yields **`class-method-missing`** at `medium`, naming the class, the method and the obligation.
- **Mutation floor.** The class's ratio is evaluated by the FR-039 rule for that obligation, with the same
  `metric: mutation-score` filter, worst-symbol reading and tokens (`insufficient-mutation-score`,
  `unmeasured-mutation-score`); the finding additionally names the class. If both a criticality floor and a class floor apply,
  the higher ratio is evaluated and the finding names the one that set it.
- **Independence.** The class's dimensions are evaluated by the FR-094 rule for that obligation; an unmet floor yields
  `insufficient-independence` naming the class and the dimensions still shared or missing.

Every finding names the class, the floor and the shortfall (the missing method, the score against the ratio, or the
dimensions not separated).

### Unclassified and unconfigured

An obligation with no class is reported as **unclassified**: no class floor applies to it, it is counted in the audit
summary, and `quoin matrix` and the audit output show its class column as `unassigned`. This is a statement of
absence, not a verdict, and does not change the exit status. An obligation whose class has no entry in the policy gets
no class floor and a `class-unconfigured` finding at `medium` naming the class, so a missing policy entry is visible
and not silently a pass.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-160-AC-1 | With no `--class-policy`, the audit output over a fixture corpus is byte-identical to the output before this requirement, except for the added class column. | Test |
| FR-160-AC-2 | A policy giving class `class-2` the method `mutation-testing` leaves a `class-2` obligation without evidence of that method with one `class-method-missing` finding naming the class, the method and the obligation; adding the evidence clears it. | Test |
| FR-160-AC-3 | A policy giving `class-2` a `mutation_floor` of `0.8` yields `insufficient-mutation-score` for a `class-2` obligation scoring `0.6` and `unmeasured-mutation-score` for one with no mutation-score entry; each names the class and the ratio; a `class-1` obligation is unaffected. | Test |
| FR-160-AC-4 | A policy giving `class-3` the dimensions `actor` and `technique` yields `insufficient-independence` for a `class-3` obligation whose two suites share an actor, naming the class and the shared dimension; two suites that differ on both clear it. | Test |
| FR-160-AC-5 | An obligation carrying `criticality: high` and no class gets only criticality floors, and an obligation carrying a class and no criticality gets only class floors; with both, the higher mutation ratio is evaluated and the finding names its source. | Test |
| FR-160-AC-6 | An obligation with no class gets no class floor, appears in the summary count of unclassified obligations, shows `unassigned` in the class column of `quoin matrix` and the audit, and does not change the exit status. | Test |
| FR-160-AC-7 | An obligation whose class has no policy entry yields one `class-unconfigured` finding naming the class and no other class finding. | Test |
| FR-160-AC-8 | A policy naming an undeclared class, an undeclared method, a ratio of `80`, an unknown dimension or an empty rationale is refused naming the cause, and no audit output is produced. | Test |
| FR-160-AC-9 | An obligation whose requirement states `class-1` and is allocated to a `class-3` item is audited against the `class-1` floors only. | Test |
| FR-160-AC-10 | The class column appears in the human and JSON output of `quoin matrix` and the audit for every obligation, with the carried value as written or `unassigned`. | Test |
| FR-160-AC-11 | No quoin source or default file contains a class value, a default policy or a floor ratio for any class. | Inspection |

## Constraints

| ID | Constraint | Type | Validation |
|----|-----------|------|------------|
| FR-160-CON-1 | Quoin SHALL NOT derive, default, suggest or write a class, from criticality, relationships, item classes or evidence. | Design | Test (FR-160-AC-9) |
| FR-160-CON-2 | Quoin SHALL NOT parse requirement or item Markdown; it reads the class from the Quire payload. | Design | Inspection |

## Dependencies

- **Upstream**: [FR-032](./FR-032-evidence-auditor.md), [FR-039](./FR-039-mutation-score-threshold.md), [FR-094](./FR-094-profile-selected-evidence-independence.md), quire-rs FR-090
- **Downstream**: [FR-161](./FR-161-per-class-obligations-report.md) (shares the policy file)
