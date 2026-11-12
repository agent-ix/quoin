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

Quoin SHALL render a requirements-mapping-matrix view of the FR-133 discharge
report from a live `clause-binding-v1` report and the stored discharge facts:
one row per clause of the binding report. The column headings follow the shape
projects use for a compliance matrix, for example center templates for the
NPR 7150.2D Appendix C requirements mapping matrix: Req#, Applicability,
Compliance, Tailoring Description and Tailoring Rationale, plus an Evidence
column carrying the evidence links.

The Applicability and Compliance values are Quoin's own vocabulary, defined
below; they are not quoted from any standard. Quoin supplies no standard's
requirement list, numbering or applicability: those come from the clause set
and the binding report. The view is read-only, derived on every read from the
discharge report, and holds no store of its own.

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
    and its attestation has expired, is not yet valid at `asOf`, or has an
    expiry that is not after its attestation. The reason is shown in the row
    and the fact earns no `Compliant` or `Tailored`.
  - `Undetermined` - applicability is unresolved; the reason is shown.
  - `Not applicable` - the clause does not bind. Tailoring Description reads
    `Not applicable per binding derivation` and Tailoring Rationale is the
    clause's binding reasons joined with `; `, or `no reason given by the
    binding report` when it has none. A disposition recorded on a not-binding
    clause is not tailoring: it stays under Unused facts.
- Evidence and tailoring cells that have nothing to show render as `-`.
- **Facts that were not spent** (a fact for an unknown, not-binding or
  unresolved clause) are listed after the table with the reason they were not
  used.
- **Every interpolated string renders literally.** Clause ids, clause set key
  parts, reasons, evidence references, approval references and rationales come
  from stored facts and the binding report and are untrusted. Each goes through
  one inline escape wherever it appears (title, cells, unused-facts list):
  control characters including line breaks become spaces and CommonMark/GFM
  punctuation (backslash, pipe, backtick, angle brackets, brackets, parentheses,
  `!`, `*`, `_`, `~`, `#`, `&`) is backslash-escaped, so text can neither start
  an image, link, HTML or heading nor split a row.
- The Markdown render is deterministic. An empty binding report renders the
  header and a line stating that the report names no clauses.
- `quoin discharge --repo <repo> --rmm` renders the matrix and `--json`
  emits it. `--rmm` needs `--repo` and excludes `--inward`.

## Constraints

- The view SHALL NOT persist anything, SHALL NOT embed a standard's requirement
  text, and SHALL carry no score, percentage, ratio or count of compliant rows.
  The pack index of [FR-140](./FR-140-pack-requirements-mapping-report.md) is a separate
  document and states counts per Compliance value; each embedded matrix still carries none.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-135-AC-1 | The matrix has exactly one row per clause of the binding report, in the report's order, with Req#, Applicability, Compliance, Evidence, Tailoring Description and Tailoring Rationale, and an empty binding report renders the header and a line stating that no clauses are named. | Test (TC-3200) |
| FR-135-AC-2 | A binding clause with a current direct fact is `Compliant` with the fact's evidence references; one with a current disposition is `Tailored` with the decision, approval reference and rationale, and is not `Compliant`. | Test (TC-3201) |
| FR-135-AC-3 | A binding clause with no fact is `Not compliant - undischarged`; one whose fact has expired, is not yet valid, or has an expiry not after its attestation is `Not compliant - attestation not current` with the reason shown and is neither `Compliant` nor `Tailored`; an unresolved clause is `Undetermined`; a not-binding clause is `Not applicable` with the tailoring description and rationale stated above, whether or not a disposition was recorded on it. | Test (TC-3202) |
| FR-135-AC-4 | Facts that were not spent are listed after the table with their reason, and the matrix carries no score, percentage, ratio or count of compliant rows. | Test (TC-3203) |
| FR-135-AC-5 | Two independent builds from the same inputs, with facts supplied in a different order, render byte-identical Markdown; and a rationale, approval reference or clause id containing an image, a link, raw HTML, a line break, a backslash, a backtick or a pipe, and a clause set key containing a line break, render as the literal text in the title, the table and the unused-facts list. | Test (TC-3204) |
| FR-135-AC-6 | `evidence.requirements_matrix` answers through the production runtime from the live binding report and the stored facts, refuses a request over its ceiling, and exits as a bad request when given a doubled binding clause; a stored disposition with an empty or whitespace-only rationale refuses the operation (FR-133-AC-4: a refusal naming the file) and renders no matrix. | Test (TC-3205) |
| FR-135-AC-7 | `quoin discharge --repo ... --rmm` renders the matrix from recorded facts and `--json` emits it; `--rmm` without `--repo`, or with `--inward`, is refused with a usage status and a message naming the conflict. | Test (TC-3206) |

## Dependencies

- FR-133 supplies the stored facts and `discharge_report_from_store`.
- FR-046 supplies the discharge partition and the fact predicates, including
  the required rationale of a disposition.
