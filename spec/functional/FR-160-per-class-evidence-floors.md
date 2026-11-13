---
id: FR-160
title: "Per-class evidence floors: required methods, mutation floor and independence configured per software safety class"
type: FR
relationships:
  - target: "ix://agent-ix/quoin/StR-004"
    type: "traces_to"
  - target: "ix://agent-ix/quoin/FR-032"
    type: "references"
  - target: "ix://agent-ix/quoin/FR-039"
    type: "extends"
  - target: "ix://agent-ix/quoin/FR-094"
    type: "references"
  - target: "ix://agent-ix/quire-rs/FR-090"
    type: "depends_on"
  - target: "ix://agent-ix/spec-artifacts-iso/FR-009"
    type: "references"
---

# FR-160: Per-class evidence floors: required methods, mutation floor and independence configured per software safety class

Tracked by Linear PLAT-1526 (implemented by PLAT-1579).

## Description

`quoin evidence audit` and `quoin matrix` **SHALL**, when given `--class-policy <file>`, apply to each obligation the evidence floors
the policy declares for the software safety class stated on the obligation's requirement.

Class and criticality are separate axes. An obligation can have either, both or neither, and each selects its own floors. Criticality
floors read the obligation's FR-053 criticality, the per-criterion column ([quire-rs FR-053](ix://agent-ix/quire-rs/FR-053)),
exactly as they do today ([FR-039](./FR-039-mutation-score-threshold.md), [FR-032](./FR-032-evidence-auditor.md)); frontmatter
`criticality` of a requirement is not an obligation's criticality ([quire-rs FR-082](ix://agent-ix/quire-rs/FR-082)).

### Where the class comes from

The class is not on the obligation record. Quoin joins the obligation to its requirement's `requirements[]` entry in the Quire
coverage matrix and reads the entry's `carried` object ([quire-rs FR-090](ix://agent-ix/quire-rs/FR-090)) under the carried name the
policy declares as `class_field`. Quoin holds no field name. The ordered legal values are the `carried_vocabulary` entry for that name on the top-level coverage report object. Quoin never reads the class from an allocated item, from criticality or from a neighbouring obligation.

### The class policy is data, and no class has a built-in floor

The policy file (YAML or JSON) holds `class_field` and, per class value, any of `required_methods`, `mutation_floor`,
`independence`, `obligations` ([FR-161](./FR-161-per-class-obligations-report.md)) and a mandatory `rationale`. Quoin holds no class
value, no default policy and no floor. A policy is a person's recorded decision; quoin never writes it. With no `--class-policy`
the audit and the matrix add nothing: no class column, no unclassified count, no class finding.

A policy is refused with the cause named, and no output is produced, when it names a class absent from the
report's `carried_vocabulary`, a method the catalog does not declare, a mutation floor outside `[0, 1]` (as FR-039 refuses `80`), an
unknown independence dimension, or an empty `rationale`; and when the report carries no `carried_vocabulary` for `class_field`, because
nothing then says which classes exist. A floor that silently does not apply reads as a passing gate.

### The floors

This requirement reuses one existing rule, the FR-039 mutation rule. The other two floors are new and are defined here in full; they
borrow nothing else from FR-032 or FR-094, which are keyed on criticality and on exact obligation ids under a profile.

- **Required methods.** An obligation of a class with `required_methods` needs, for each listed method, at least one bound evidence
  relationship whose run `evidence_kind` equals the `evidence_kind` the catalog declares for that method. A binding records no
  method id, so the kind is the join. Methods that share an `evidence_kind` are therefore **interchangeable** for this floor:
  evidence of one satisfies a requirement for another of the same kind, and the policy author chooses methods knowing it. Each method
  whose kind has no bound evidence yields **`class-method-missing`** at `medium`, naming the class, the method, its kind and the
  obligation. No store change is made.
- **Mutation floor.** The class's ratio is evaluated by the FR-039 rule for that obligation, with its `metric: mutation-score`
  filter, worst-symbol reading and tokens (`insufficient-mutation-score`, `unmeasured-mutation-score`); the finding also names the
  class. When a criticality floor applies as well, the higher ratio is evaluated and the finding names the source that set it.
- **Independence.** The class's `independence` lists FR-094 dimensions. For each obligation of the class, two distinct
  obligation-to-suite relationships must state different non-empty values on every listed dimension, by the same pairwise rule
  FR-094 states for a profile requirement. The profile `IndependenceAssessment` cannot carry a class (its profile is an
  assurance-profile id and it names a requirement of a profile policy), so quoin builds a separate **class independence
  assessment** record for each obligation of the class: the requirement id the obligation derives from, the class value, the listed
  dimensions, the per-dimension values of the bound relationships, and `source: class`. The auditor consumes these records
  alongside the profile assessments, which carry `source: profile`, and applies the FR-094 pairwise comparison to both. An unmet
  floor yields `insufficient-independence` naming the `source`, the class when `source` is `class`, and the dimensions still shared
  or missing. When `--independence-policy` is also given and selects dimensions for the same obligation, **both** must hold, each
  is assessed on its own dimensions, and each failure is its own finding naming its `source`; the dimension sets are not united.

A class has no multiplicity floor (FR-032 `multiplicity_requires` stays criticality-keyed).

### Unclassified and unconfigured

An obligation whose requirement carries no class is **unclassified**: no class floor applies, it is counted in the audit summary,
and its class column shows `unassigned` in human output. In JSON the class key is omitted for it. The count and the class column
exist only with `--class-policy`. An obligation whose class has no policy entry gets no class floor and one **`class-unconfigured`**
finding at `medium` naming the class. Neither changes the exit status by itself.

### `quoin matrix`

`quoin matrix` accepts `--class-policy` as it accepts `--mutation-floor`, `--multiplicity-requires` and `--independence-policy`, and
reports the same class findings and class column as the audit for the same inputs.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-160-AC-1 | Without `--class-policy`, the audit and `quoin matrix` output carry no class column, no unclassified count and no class finding; with it, the added output is exactly the class column or key, the unclassified count and the class findings. | Test |
| FR-160-AC-2 | A policy giving `class-2` the method `mutation-testing` leaves a `class-2` obligation with no bound evidence of that method's catalog `evidence_kind` with one `class-method-missing` finding naming the class, method, kind and obligation; binding evidence of that kind, even from another method of the same kind, clears it. | Test |
| FR-160-AC-3 | A policy giving `class-2` a `mutation_floor` of `0.8` yields `insufficient-mutation-score` for a `class-2` obligation scoring `0.6` and `unmeasured-mutation-score` for one with no mutation-score entry; each names the class and the ratio; a `class-1` obligation is unaffected. | Test |
| FR-160-AC-4 | A policy giving `class-3` the dimensions `actor` and `technique` yields `insufficient-independence` for a `class-3` obligation whose two suites share an actor, naming the class and the shared dimension; two suites that differ on both clear it; one suite alone yields it. | Test |
| FR-160-AC-5 | An obligation whose FR-053 criticality column is `high` and whose requirement carries no class gets only criticality floors; one whose requirement carries a class and whose column is empty gets only class floors; with both, the higher mutation ratio is evaluated and the finding names its source. | Test |
| FR-160-AC-6 | An obligation whose requirement carries no class gets no class floor, appears in the summary count of unclassified obligations, shows `unassigned` in the human class column, has no class key in JSON, and does not change the exit status. | Test |
| FR-160-AC-7 | An obligation whose class has no policy entry yields one `class-unconfigured` finding naming the class and no other class finding. | Test |
| FR-160-AC-8 | A policy naming a class absent from the report's `carried_vocabulary`, an undeclared method, a ratio of `80`, an unknown dimension or an empty rationale is refused naming the cause, and no audit output is produced; so is a policy over a report with no `carried_vocabulary` for `class_field`. | Test |
| FR-160-AC-9 | An obligation whose requirement states `class-1` and which is allocated to a `class-3` item is audited against the `class-1` floors only. | Test |
| FR-160-AC-10 | With `--independence-policy` selecting `actor` and a class selecting `technique` for the same obligation, an obligation whose suites differ on `technique` and share an actor yields one `insufficient-independence` finding with `source: profile`; one sharing `technique` and differing on actor yields one with `source: class` naming the class; one failing both yields two. | Test |
| FR-160-AC-11 | The class column or key appears in the human and JSON output of both `quoin matrix` and the audit under `--class-policy`, with the carried value as written, `unassigned` in human output and no key in JSON for an absent class. | Test |
| FR-160-AC-12 | With the same store, payload and policy, two audit runs print identical findings in identical order, and the run writes no file and no store record. | Test |
| FR-160-AC-13 | No quoin source or default file contains a class value, a default policy or a floor ratio for any class, or a carried field name. | Inspection |

## Constraints

| ID | Constraint | Type | Validation |
|----|-----------|------|------------|
| FR-160-CON-1 | Quoin SHALL NOT derive, default, suggest or write a class, from criticality, relationships, item classes or evidence. | Design | Test (FR-160-AC-9) |
| FR-160-CON-2 | Quoin SHALL NOT parse requirement or item Markdown; it reads the class from the Quire payload. | Design | Inspection |

## Dependencies

- **Upstream**: [FR-039](./FR-039-mutation-score-threshold.md) (reused rule), [FR-032](./FR-032-evidence-auditor.md), [FR-094](./FR-094-profile-selected-evidence-independence.md) (pairwise rule and dimensions), quire-rs FR-090 (`carried` on requirement entries, `carried_vocabulary` on the top-level report)
- **Downstream**: [FR-161](./FR-161-per-class-obligations-report.md) (shares the policy file), [NFR-034](../non-functional/NFR-034-class-reports-state-and-never-assign.md)
