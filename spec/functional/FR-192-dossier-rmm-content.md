---
id: FR-192
title: "The dossier requirements mapping matrix renders one table per clause set"
type: FR
relationships:
  - target: "ix://agent-ix/quoin/StR-010"
    type: "traces_to"
  - target: "ix://agent-ix/quoin/US-024"
    type: "implements"
  - target: "ix://agent-ix/quoin/FR-188"
    type: "extends"
---

# FR-192: The dossier requirements mapping matrix renders one table per clause set

## Description

The requirements mapping matrix (`--doc rmm`, document id `DOS-RMM`) SHALL
render one table per clause set from `requirements-mapping-v1`, or the pack
form `requirements-mapping-pack-v1`, with every row member, `As of`, and
`unusedFacts`.

## Inputs

- The `discharge` source: `quoin discharge --binding <clause-binding.json> --repo <dir> --as-of <ISO-8601> --rmm --json` for each `--clause-binding`, or `--pack <pack> --rmm --json`. The code is `rust/crates/quoin-assurance/src/discharge/rmm.rs`, `pack_rmm.rs` and `rust/crates/quoin-cli/src/discharge.rs`. A clause binding comes from `quire clauses`.
- `--as-of`, required whenever `rmm` is selected and a binding or pack is given.

## Outputs

- `dossier/DOS-RMM.md`, then `Gaps`.

## Behavior

- The RMM SHALL render each clause set as one table.
- Where a pack supplies the clause sets, the RMM SHALL order the tables by the pack's index order.
- Where separate bindings supply the clause sets, the RMM SHALL order the tables by the command-line order of `--clause-binding`; this supplied order is an input and is not re-sorted.
- The RMM SHALL order the rows of each table as the discharge source gives them.
- The RMM SHALL carry every member of each source row (`clauseId`, `force`, `applicability`, `compliance`, `reason`, `evidenceRefs`, `tailoringDescription`, `tailoringRationale`).
- The RMM SHALL show `As of` and `unusedFacts`.
- The RMM SHALL take `As of` from the `--as-of` value.
- Quoin SHALL NOT read the clock for the RMM.
- If `rmm` is selected, a binding or pack is given, and `--as-of` is absent, the command SHALL exit 2.
- If `rmm` is selected, a binding or pack is given, and `--as-of` is absent, the command SHALL write nothing.
- If `rmm` is selected and neither `--clause-binding` nor `--pack` is given, the RMM SHALL record a `no_clause_binding` gap.
- If `rmm` is selected and neither `--clause-binding` nor `--pack` is given, the RMM SHALL have status `incomplete`.
- If `rmm` is selected and neither `--clause-binding` nor `--pack` is given, the command SHALL NOT treat the absence as an error.
- If `rmm` is selected and neither `--clause-binding` nor `--pack` is given, the command SHALL NOT require `--as-of`.
- If a row's `compliance` is `undischarged`, `attestation_not_current`, `temporary_exception_expired` or `undetermined`, the RMM SHALL record a `clause_open` gap naming the clause.
- The detail of a `clause_open` gap SHALL be the compliance value and the row's `reason`.
- If a row's `compliance` is `compliant`, `tailored` or `not_applicable`, the RMM SHALL NOT record a `clause_open` gap for it, whether or not it has `evidenceRefs`.
- The RMM SHALL NOT write the text of any clause, whatever the binding, the pack or `quire clauses` carries.
- The RMM SHALL NOT read the rights metadata of `quire clauses`; the clause id is the only clause member it writes.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-192-AC-1 | The rows equal the `quoin discharge --rmm --json` rows for the fixture binding, in the source's order | Test |
| FR-192-AC-2 | A pack with two clause sets gives two tables in index order | Test |
| FR-192-AC-3 | With `--doc rmm` and no `--clause-binding` and no `--pack` the document has a `no_clause_binding` gap, status `incomplete`, the command exits 1 and not 2, and no `--as-of` is needed | Test |
| FR-192-AC-4 | A fixture whose binding and pack carry clause text and whose clauses are marked restricted in `quire clauses` renders only clause ids and the source row members; no clause text appears anywhere in the file or in a gap detail | Test |
| FR-192-AC-5 | Rows with `compliance` `undischarged`, `attestation_not_current`, `temporary_exception_expired` and `undetermined` each produce one `clause_open` gap | Test |
| FR-192-AC-6 | `As of` and `unusedFacts` appear in the document as the source reports them | Test |
| FR-192-AC-7 | `--doc rmm --clause-binding <file>` without `--as-of` exits 2 and writes nothing | Test |
| FR-192-AC-8 | Rows with `compliance` `compliant`, `tailored` and `not_applicable`, including `tailored` and `not_applicable` rows with empty `evidenceRefs`, produce no `clause_open` gap | Test |
| FR-192-AC-9 | Swapping two `--clause-binding` options swaps the two tables and changes nothing else in the file | Test |

## Dependencies

- **Upstream**: [FR-188](./FR-188-dossier-command.md); the discharge mapping output.
- **Downstream**: none.
