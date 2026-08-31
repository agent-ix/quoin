---
id: FR-067
title: "Governed graph portfolio reporting"
type: FR
verification_method: test
evidence:
  - kind: test_case
    ref: tests/graph-portfolio.test.ts
relationships:
  - target: "ix://agent-ix/quoin/StR-007"
    type: "satisfies"
  - target: "ix://agent-ix/quoin/US-019"
    type: "implements"
  - target: "ix://agent-ix/quoin/FR-045"
    type: "extends"
  - target: "ix://agent-ix/quoin/FR-062"
    type: "requires"
  - target: "ix://agent-ix/quoin/FR-066"
    type: "requires"
---
# FR-067: Governed graph portfolio reporting

## Description

When graph reporting is selected, `quoin report --portfolio` SHALL extend each
repository row with governed graph-quality history, partitions, gaps,
provenance, raw-evidence identity, and FR-062 structural views. It SHALL compare
only observations whose plan, definition, producer configuration, and complete
population identity are compatible.

## Inputs

- The repository locations and existing presentation options from FR-045.
- Stored graph-quality measurement collections produced by FR-066.
- Optional repeated `--graph-export <repository>=<path>` mappings accepted by
  the Quire adapter.
- Optional repeated `--changed <repository>=<requirement-id>` seeds for
  change-impact; seeds are inapplicable when no export is supplied.

Repository identities are resolved absolute paths. Repeating the same
repository with equivalent paths deduplicates. Two different graph-export
paths for one resolved repository are refused as `duplicate_graph_export`;
repeated changed seeds for that repository deduplicate by requirement id.

## Outputs

Each repository report SHALL retain its existing FR-045 fields and add:

- `graph_quality`: active plan, current state, complete ordered history,
  definition, producer/config/source/corpus revisions, population identity,
  partitioned observations, comparison, and raw observation/scorer digests;
- `graph`: export availability and premises plus FR-062 fan-out and churn
  reports, and change-impact reports for requested seeds; and
- `gaps`: each missing, unreadable, incompatible, unknown, or not-applicable
  graph premise with owner/action fields when the active plan declares them.

Availability SHALL be one of `available`, `missing`, `unreadable`, `unknown`,
`incompatible`, or `not_applicable`. Measurement state remains FR-044's
separate `measured` or `not_computed` fact.

## Behavior

- The portfolio SHALL show every graph-quality collection in deterministic
  timestamp/id order, including historical schema versions that remain
  structurally readable.
- One unreadable collection SHALL become a repository-local gap naming its
  path and cause while every structurally readable collection in that same
  repository remains available for history and current selection.
- The portfolio SHALL retain a collection without a matching active
  MeasurementPlan as historical evidence, mark it `incompatible` for current
  reporting and comparison, and omit its numeric values from current
  measurements.
- The report SHALL keep partitions separate by measure, dimension, and key
  without summing or averaging languages, node kinds, relation kinds, resolver
  tiers, repositories, or unlike populations.
- The portfolio SHALL reuse FR-044 compatibility checks and additionally require
  equal corpus revision and exact population identity.
- The comparison report SHALL name every blocking premise and emit no delta for
  an incompatible pair.
- Raw producer-record and scorer digests SHALL remain visible on current,
  history, and comparison rows.
- When a graph export is supplied, the portfolio SHALL embed the same FR-062
  report objects used by standalone graph commands. When it is absent or
  refused, the portfolio SHALL emit its exact availability and reason instead
  of an empty structural view.
- One unreadable repository or graph artifact SHALL NOT hide readable siblings.

## Error Conditions

Invalid command mappings fail before repository reads with
`invalid_repository_mapping` or `duplicate_graph_export`. Repository,
collection, attachment, and graph-export read failures become typed local gaps
(`missing`, `unreadable`, `unknown`, `incompatible`, or `not_applicable`) and do
not abort readable repositories or readable collections in the same store.

## Constraints

| ID | Constraint | Type | Validation |
|----|------------|------|------------|
| FR-067-CON-1 | Portfolio reporting SHALL neither run a producer, Quire command, Git command, or network request nor write evidence. | Architecture | Inspection |
| FR-067-CON-2 | The report SHALL derive no cross-repository aggregate, trust score, release verdict, or threshold classification. | Responsibility | Test |
| FR-067-CON-3 | Graph partitions and FR-062 completeness/gap states SHALL NOT be collapsed or reinterpreted by the portfolio layer. | Integrity | Test |
| FR-067-CON-4 | Historical FR-044 collections and non-graph portfolio rows SHALL remain readable without migration. | Compatibility | Test |

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-067-AC-1 | Each readable repository reports its active graph-quality plan, current collection, definition, complete producer tuple, source/corpus revisions, population identity, and raw record/scorer digests. | Test (TC-1305) |
| FR-067-AC-2 | History retains every structurally readable graph-quality collection in timestamp/id order with its availability and premises, including records no longer governed by the active plan. | Test (TC-1306) |
| FR-067-AC-3 | Every measure/dimension/key partition remains a distinct row in human and JSON output, with no sum or average across partitions or repositories. | Property (TC-1307) |
| FR-067-AC-4 | Missing, unreadable, incompatible, unknown, and not-applicable graph inputs remain distinct from measured and not-computed states and are never rendered as numeric zero. | Test (TC-1308) |
| FR-067-AC-5 | Equal plan, definition, configuration, tool, corpus revision, and population identity permit comparison; changing each premise independently blocks the delta and names the mismatch. | Property (TC-1309) |
| FR-067-AC-6 | Current, historical, and comparison rows resolve to the exact retained producer record and scorer digests rather than a transcribed summary. | Integration (TC-1310) |
| FR-067-AC-7 | A supplied accepted export embeds byte-equivalent FR-062 fan-out and churn reports plus requested change-impact reports; no export yields `missing`, and no seed yields change-impact `not_applicable`. | Integration (TC-1311) |
| FR-067-AC-8 | An unreadable repository, collection, raw attachment, or graph export is named locally while readable sibling repositories retain complete reports. | Test (TC-1312) |
| FR-067-AC-9 | Reordered repository arguments and equivalent store enumeration produce byte-identical canonical JSON, and human output renders the same report object. | Property (TC-1313) |
| FR-067-AC-10 | Existing measurement histories and non-graph portfolio goldens remain readable without migration, and graph output contains no aggregate score or verdict (CON-2, CON-4). | Integration (TC-1314) |
| FR-067-AC-11 | Static boundaries prove portfolio reporting executes and writes nothing and consumes FR-062 reports rather than recomputing graph semantics (CON-1, CON-3). | Inspection (TC-1315) |

## Dependencies

- **Upstream**: [FR-045](./FR-045-portfolio-measurement-report.md), FR-062
  (`agent-ix/quoin#152`), and
  [FR-066](./FR-066-graph-producer-adapters.md).
- **Downstream**: shared assurance campaigns use this portfolio without
  changing producer definitions or population boundaries.
