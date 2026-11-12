---
id: SR-20261112191019
title: "Integrity analysis of FR-140 pack requirements mapping report in quoin PR 832"
type: SpecReview
analysis: integrity
scope: "agent-ix/quoin@f0a0122c; spec/functional/FR-140-pack-requirements-mapping-report.md, spec/functional/FR-135-requirements-mapping-matrix-view.md, spec/functional/index.md, spec/spec.md"
review_set: subset
---

# Integrity analysis of FR-140 pack requirements mapping report

## Summary

Ticket: PLAT-1487. Scoped to the PR 832 diff at f0a0122c. Tools: quoin 0.30.1, quire 0.43.0
(engine 0.58.0). `quire validate` over the four changed files is clean (one unrelated
module-import notice). Feasibility was checked against origin/main: `quoin_quire::evaluate`
(`rust/crates/quoin-quire/src/clauses.rs:92`) takes one `ClauseSetRef` and fails only on module
load or an absent version; quire-rs `ClauseSet::evaluate` is infallible; `Registry::clause_sets()`
exists for enumeration; `discharge_report_from_store` reads facts per `ClauseSetKey`
(`quoin-assurance/src/discharge/store.rs:372`). The discharge CLI today makes `--binding`
`required(true)` and reports `--rmm` usage conflicts as exit 1 (`tc_3206`), while clap
`conflicts_with` errors exit 3. Criterion-strength analysis (Jev) was unavailable on this machine
and was skipped; AC strength was judged manually here.

## Verdict

**CHANGES REQUESTED.** The design is feasible and the licensing posture is sound (ids, set keys
and counts are citation-level under `StructureRights`; stdout is the only default egress), but
one AC clause is unreachable, the usage status is not pinned to a number, one oracle false-fires,
and the FR-135 edit is a semantic carve-out without a rationale.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | AC-8 "a pack whose set fails to evaluate" is unreachable: set evaluation is infallible once the module loads, so no fixture can produce it | spec/functional/FR-140-pack-requirements-mapping-report.md:86-89,121 |
| FND-002 | medium | "Usage status of FR-135-AC-7" names no number; existing test asserts 1, clap conflicts exit 3, and `--binding` is `required(true)` today | spec/functional/FR-140-pack-requirements-mapping-report.md:92-94,122 |
| FND-003 | medium | AC-4 "no `%`" over the whole output false-fires on percent-encoded evidence references, which the FR-135 escape leaves intact | spec/functional/FR-140-pack-requirements-mapping-report.md:117 |
| FND-004 | medium | FR-135 edit carves an exception to its own "no count of compliant rows" ban; FR-140 Rationale does not justify per-Compliance counts | spec/functional/FR-135-requirements-mapping-matrix-view.md:84-85 |
| FND-005 | low | `--context` without `--pack` is not specified (accepted, ignored, or refused) | spec/functional/FR-140-pack-requirements-mapping-report.md:40-42,92-94 |
| FND-006 | low | AC-2 "in no other set's" depends on the fixture; it must state that the other sets do not reference the changed dimension | spec/functional/FR-140-pack-requirements-mapping-report.md:115 |

## Examined scope

```yaml
clean: false
scope:
- {id: FR-140, path: spec/functional/FR-140-pack-requirements-mapping-report.md, role: examined, excerpt: "quoin discharge --rmm --pack <name> SHALL report on an installed pack (FR-137): one FR-135 requirements mapping matrix for every clause set of the pack, preceded by a pack index."}
- {id: FR-140-AC-1, path: spec/functional/FR-140-pack-requirements-mapping-report.md, role: examined, excerpt: "Each matrix is byte-equal to the single-set FR-135 render for the same binding and facts."}
- {id: FR-140-AC-3, path: spec/functional/FR-140-pack-requirements-mapping-report.md, role: examined, excerpt: "With facts recorded in the repo's store for one set only, that set shows Compliant rows ..."}
- {id: FR-140-AC-7, path: spec/functional/FR-140-pack-requirements-mapping-report.md, role: examined, excerpt: "An unknown name and an installed non-pack module ... each exit with status 2"}
- {id: FR-140-AC-11, path: spec/functional/FR-140-pack-requirements-mapping-report.md, role: examined, excerpt: "Two runs from the same inputs with facts recorded in a different order produce byte-identical Markdown and JSON"}
- {id: FR-140-AC-12, path: spec/functional/FR-140-pack-requirements-mapping-report.md, role: examined, excerpt: "The --json output of AC-1 validates against the JSON Schema generated from the pack report type"}
- {id: FR-140-AC-13, path: spec/functional/FR-140-pack-requirements-mapping-report.md, role: examined, excerpt: "The retained quoin discharge command help lists --pack, --context and --out, and the help fixture matches the command."}
- {id: FR-135, path: spec/functional/FR-135-requirements-mapping-matrix-view.md, role: context_only, excerpt: "The view SHALL NOT persist anything, SHALL NOT embed a standard's requirement text, and SHALL carry no score, percentage, ratio or count of compliant rows."}
- {id: spec/functional/index.md, path: spec/functional/index.md, role: examined, excerpt: "- [FR-140: Pack-level requirements mapping report: one matrix per clause set and a pack index](./FR-140-pack-requirements-mapping-report.md)"}
```
