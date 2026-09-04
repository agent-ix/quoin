---
id: FR-087
title: "Evaluate declared Markdown mappings and validate the extracted record"
type: FR
relationships:
  - target: "ix://agent-ix/quoin/US-022"
    type: "traces_to"
  - target: "ix://agent-ix/quoin/FR-086"
    type: "depends_on"
---

# FR-087: Evaluate declared Markdown mappings and validate the extracted record

## Description

The corpus measurement SHALL report, for every `measured` document whose resolving module publishes a
mappings declaration, exactly one outcome from `pass`, `fail` and `could-not-run`, obtained by
building the record that declaration describes and validating it against the module's JSON Schema for
the document's type.

## Rationale

The check the campaign owes the promotion gate is the one the modules actually declare: does an
authored document become the record its module says it becomes. Collapsing `could-not-run` into
either `pass` or `fail` is the specific mistake that produced a "no status lies" claim for a check
that had never executed in any repository.

## Inputs

- A `measured` document's bytes and its resolving module.
- The module's mappings declaration and the JSON Schema declared for the document's type.

## Outputs

- One evaluation record per document: outcome, the mapping entries evaluated, and — for `fail` — each
  violated schema keyword with its instance path and the document line of the mapping entry that
  produced it.
- For `could-not-run`, the mapping kind or parse rule that the evaluator does not implement.

## Behavior

- The measurement SHALL evaluate the mapping kinds `frontmatter`, `section`, `table`, `typed-table`,
  `ocl-clause`, `sysml-fence`, `list`, `token` and `provenance` named by a module's declaration.
- The measurement SHALL derive every heading, column list and row-id pattern it applies from the
  module's own declaration rather than from a vocabulary compiled into the measurement.
- If a mapping entry names a kind or a parse rule the evaluator does not implement, then the
  measurement SHALL report that document `could-not-run` naming the unimplemented rule.
- The measurement SHALL NOT report a partially built record as a `pass`.
- The measurement SHALL validate the built record against the module's declared JSON Schema for the
  document's type and SHALL report `pass` only when validation reports no error.
- When a document's type carries a declared schema and its resolving module publishes no mapping for
  that type, the measurement SHALL report that document `could-not-run` with reason
  `no-mapping-for-declared-type`.
- The measurement SHALL report every schema violation of a failing document in that document's single
  evaluation record. Reporting only the first would make a document with three defects need three
  measurement runs to describe.

## Constraints

| ID | Constraint | Type | Validation |
| --- | --- | --- | --- |
| FR-087-CON-1 | A `could-not-run` outcome SHALL NOT be counted in the numerator or the denominator of a pass rate. | Interface | Test |
| FR-087-CON-2 | Mapping evaluation SHALL NOT rewrite the bytes of a corpus document. | Safety | Test |

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-087-AC-1 | A document conforming to its module's declared mapping and schema reports `pass`. | Test (TC-1518) |
| FR-087-AC-2 | A document missing a required section declared by a `section` mapping reports `fail`, naming the schema keyword, the instance path and the mapping's heading. | Test (TC-1519) |
| FR-087-AC-3 | A typed-table whose header row differs from the declared `columns` reports `fail` at that table's line, and a row whose id cell does not match the declared `row_id.pattern` reports `fail` at that row's line. | Test (TC-1520) |
| FR-087-AC-4 | A mapping entry naming an unimplemented kind or parse rule reports `could-not-run` naming that rule, and the document is absent from both the pass numerator and the denominator. | Test (TC-1521) |
| FR-087-AC-5 | A declared type with a schema and no mapping reports `could-not-run` with reason `no-mapping-for-declared-type`. | Test (TC-1522) |
| FR-087-AC-6 | A document carrying three independent schema violations reports all three in one evaluation record. | Test (TC-1523) |
| FR-087-AC-7 | Heading names, column lists and row-id patterns used by an evaluation are those of the module's declaration at the measured revision, demonstrated by an evaluation changing when the declaration changes and the document does not. | Test (TC-1524) |

## Dependencies

- **Upstream**: [FR-085](./FR-085-resolve-the-completed-module-set.md), [FR-086](./FR-086-assign-one-measurement-state-per-document.md)
- **Downstream**: [FR-089](./FR-089-partition-every-failure.md)
