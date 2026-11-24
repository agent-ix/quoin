---
id: FR-194
title: "The dossier trace matrix renders the evidence-backed matrix per requirement"
type: FR
relationships:
  - target: "ix://agent-ix/quoin/StR-010"
    type: "traces_to"
  - target: "ix://agent-ix/quoin/US-024"
    type: "implements"
  - target: "ix://agent-ix/quoin/FR-191"
    type: "extends"
  - target: "ix://agent-ix/quoin/FR-115"
    type: "references"
  - target: "ix://agent-ix/quoin/FR-165"
    type: "references"
  - target: "ix://agent-ix/quoin/FR-160"
    type: "references"
  - target: "ix://agent-ix/quire-rs/FR-090"
    type: "references"
  - target: "ix://agent-ix/spec-artifacts-iso/FR-009"
    type: "references"
---

# FR-194: The dossier trace matrix renders the evidence-backed matrix per requirement

## Description

The trace matrix (`--doc trace-matrix`, document id `DOS-TRACE`) SHALL render
one row per criterion of the `quoin matrix` `MatrixOutput` (FR-115, with the
design column of FR-165), grouped by requirement, with the criterion text and the
requirement criticality read from the requirement documents and the design cell,
method, static status and evidence status read from `MatrixOutput`.

## Inputs

- The `matrix` source: the output of `quoin matrix --json` over the scope. It supplies, per criterion, `id`, the design cell (`design_elements`), `method`, `static_status` and `evidence_status`, and, when the matrix is empty, `reason`. It does not supply the criterion text or the criticality.
- The `requirements` source of FR-192: the criterion `statement`, the requirement `criticality` and the requirement's recorded software safety class (FR-192), joined to the matrix rows by criterion id and requirement id.

## Outputs

- `dossier/DOS-TRACE.md`, then `Gaps`.

## Behavior

- The trace matrix SHALL have one row for each criterion of `quoin matrix --json`, and no other row.
- The trace matrix SHALL group rows by requirement id and order them in dossier order (FR-191), which can differ from the byte order of `MatrixOutput`.
- The trace matrix SHALL show for each row the criterion id, its statement, the design cell, the method, the static status, the evidence status, the criticality and the software safety class of its requirement exactly as recorded.
- If a requirement records no software safety class, the trace matrix SHALL show `unassigned` on each of its rows and record one `safety_class_unassigned` gap naming the requirement.
- The trace matrix SHALL NOT infer, default or inherit a class.
- The trace matrix SHALL take the design cell, method, static status and evidence status from `MatrixOutput` as it reports them.
- The trace matrix SHALL take the statement and the criticality from the requirement documents.
- If a requirement has no `criticality`, the trace matrix SHALL show `unassigned`.
- If a requirement has no `criticality`, the trace matrix SHALL record a `criticality_unassigned` gap naming the requirement.
- If a criterion has no test tag (`static_status` is not `tagged`), the trace matrix SHALL record an `unbacked_criterion` gap naming the criterion.
- If the `evidence_status` of a criterion is not `bound`, the trace matrix SHALL record a `no_evidence` gap naming the criterion and the status.
- If the matrix has no criteria, the trace matrix SHALL render the matrix's `reason` text.
- If the matrix has no criteria, the trace matrix SHALL record a `no_requirements` gap whose detail is that text.
- The trace matrix SHALL NOT use the static-only `quire matrix` (quire-cli FR-043) as its source.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-194-AC-1 | The set of rows equals the set of `quoin matrix --json` criteria for the fixture, one to one, in dossier order | Test |
| FR-194-AC-2 | A criterion with evidence status `undischarged` appears with a `no_evidence` gap | Test |
| FR-194-AC-3 | An empty population renders the `reason` text, a `no_requirements` gap and status `incomplete` | Test |
| FR-194-AC-4 | A criterion with no test tag appears with an `unbacked_criterion` gap | Test |
| FR-194-AC-5 | Each row shows design cell, method, static status and evidence status as `MatrixOutput` reports them, and criterion statement and criticality as the requirement documents state them | Test |
| FR-194-AC-6 | A requirement with no `criticality` shows `unassigned` on each of its rows and one `criticality_unassigned` gap | Test |
| FR-194-AC-7 | Each row shows its requirement's recorded software safety class as written, in a column separate from criticality. | Test |
| FR-194-AC-8 | A requirement with no recorded class shows `unassigned` on each of its rows and one `safety_class_unassigned` gap; no class value appears on those rows. | Test |

## Dependencies

- **Upstream**: [FR-191](./FR-191-dossier-command.md), [FR-192](./FR-192-dossier-srs-content.md), [FR-115](./FR-115-evidence-backed-test-matrix.md), [FR-165](./FR-165-matrix-design-column.md).
- **Downstream**: none.
