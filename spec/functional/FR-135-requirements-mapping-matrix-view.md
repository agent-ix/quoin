---
id: FR-135
title: "Requirements mapping matrix view over the discharge report"
type: FR
relationships:
  - target: "ix://agent-ix/quoin/FR-046"
    type: "extends"
  - target: "ix://agent-ix/quoin/FR-133"
    type: "extends"
---

# FR-135: Requirements mapping matrix view over the discharge report

## Description

Quoin SHALL render a requirements mapping matrix (RMM) from a live
`clause-binding-v1` report and the stored discharge facts: one row per clause
of the binding report, with the columns a standards-compliance matrix such as
the NPR 7150.2D Appendix C matrix asks for (Req#, Applicability, Compliance,
Tailoring Description, Tailoring Rationale) and an Evidence column carrying the
evidence links.

The view is read-only. It is derived on every read from the FR-133 discharge
report and holds no store of its own. The requirement list, its numbering and
its applicability come from the clause set and the binding report; Quoin
carries no standard's text.

## Rationale

Projects submit a compliance matrix by hand. Every cell is already decided by
the discharge accounting (FR-046, FR-133), so the matrix is a view of it. An
honest matrix shows the gaps as rows; a blank cell reads as an optimistic
answer.

## Behavior

- **One row per binding-report clause, in the report's order.** Every clause
  appears whatever its state; a clause is never omitted and rows are never
  merged.
- **Req#** is the clause id. **Applicability** is `Applicable`,
  `Not applicable` or `Unresolved`, from the binding report's outcome.
- **Compliance** is one of:
  - `Compliant` - the clause binds and a current direct fact discharges it.
    Evidence holds the fact's evidence references.
  - `Tailored` - the clause binds and a current approved disposition decides it
    (accepted risk, temporary exception or delegated). The disposition is not
    evidence that the expected output exists, so it is never `Compliant`.
    Tailoring Description names the decision and the approval reference;
    Tailoring Rationale is the disposition's rationale.
  - `Not compliant - undischarged` - the clause binds and no fact exists.
  - `Not compliant - attestation not current` - the clause binds, a fact exists
    and its attestation has expired or is not yet valid at `asOf`. The reason
    is shown in the row and the fact earns no `Compliant` or `Tailored`.
  - `Undetermined` - applicability is unresolved; the reason is shown.
  - `Not applicable` - the clause does not bind.
- Evidence and tailoring cells that have nothing to show render as `-`.
- **Facts that were not spent** (a fact for an unknown, not-binding or
  unresolved clause) are listed after the table with the reason they were not
  used.
- The Markdown render is deterministic; a `|` or line break inside a cell is
  escaped so a cell never splits a row. An empty binding report renders the
  header and a line stating that the report names no clauses.
- `quoin discharge --repo <repo> --rmm` renders the matrix and `--json`
  emits it. `--rmm` needs `--repo` and excludes `--inward`.

## Constraints

- The view SHALL NOT persist anything, SHALL NOT embed a standard's requirement
  text, and SHALL carry no score, percentage, ratio or count of compliant rows.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-135-AC-1 | The matrix has exactly one row per clause of the binding report, in the report's order, with Req#, Applicability, Compliance, Evidence, Tailoring Description and Tailoring Rationale, and an empty binding report renders the header and a line stating that no clauses are named. | Test (TC-3200) |
| FR-135-AC-2 | A binding clause with a current direct fact is `Compliant` with the fact's evidence references; one with a current disposition is `Tailored` with the decision, approval reference and rationale, and is not `Compliant`. | Test (TC-3201) |
| FR-135-AC-3 | A binding clause with no fact is `Not compliant - undischarged`; one whose fact has expired is `Not compliant - attestation not current` with the reason shown and is neither `Compliant` nor `Tailored`; an unresolved clause is `Undetermined` and a not-binding clause is `Not applicable`. | Test (TC-3202) |
| FR-135-AC-4 | Facts that were not spent are listed after the table with their reason, and the matrix carries no score, percentage, ratio or count of compliant rows. | Test (TC-3203) |
| FR-135-AC-5 | The Markdown render is byte-identical across two renders, and a cell containing a pipe character or a line break stays within its row. | Test (TC-3204) |
| FR-135-AC-6 | `evidence.requirements_matrix` answers through the production runtime from the live binding report and the stored facts, refuses a request over its ceiling, and exits as a bad request when given a doubled binding clause. | Test (TC-3205) |
| FR-135-AC-7 | `quoin discharge --repo ... --rmm` renders the matrix from recorded facts and `--json` emits it; `--rmm` without `--repo`, or with `--inward`, is refused. | Test (TC-3206) |

## Dependencies

- FR-133 supplies the stored facts and `discharge_report_from_store`.
- FR-046 supplies the discharge partition and the fact predicates, including
  the required rationale of a disposition.
