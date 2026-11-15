---
id: FR-192
title: "The dossier requirements mapping matrix renders one table per clause set"
type: FR
relationships:
  - target: "ix://agent-ix/quoin/StR-007"
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

- `quoin discharge --binding <clause-binding.json> --repo <dir> --as-of <ISO-8601> --rmm --json` for each `--clause-binding`, or `--pack <pack> --rmm --json`. The code is `rust/crates/quoin-assurance/src/discharge/rmm.rs`, `pack_rmm.rs` and `rust/crates/quoin-cli/src/discharge.rs`. A clause binding comes from `quire clauses`.
- `--as-of`, required whenever a binding or pack is given.

## Outputs

- `dossier/DOS-RMM.md`, then `Gaps`.

## Behavior

- The RMM SHALL render each clause set as one table.
- Where a pack supplies the clause sets, the RMM SHALL order the tables by the pack's index order.
- Where separate bindings supply the clause sets, the RMM SHALL order the tables by command-line order.
- The RMM SHALL carry every member of each source row.
- The RMM SHALL show `As of` and `unusedFacts`.
- If neither `--clause-binding` nor `--pack` is given, the RMM SHALL record a `no_clause_binding` gap.
- If neither `--clause-binding` nor `--pack` is given, the RMM SHALL have status `incomplete`.
- If neither `--clause-binding` nor `--pack` is given, the command SHALL NOT treat the absence as an error.
- If a row has no compliance or evidence, the RMM SHALL record a `clause_open` gap.
- If the rights metadata of `quire clauses` marks a clause as restricted, the RMM SHALL render only that clause's id and status.
- If the rights metadata of `quire clauses` marks a clause as restricted, Quoin SHALL NOT write the clause text to any output, gap detail included.
- The RMM SHALL take `As of` from the `--as-of` value.
- Quoin SHALL NOT read the clock for the RMM.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-192-AC-1 | The rows equal the `quoin discharge --rmm --json` rows for the fixture binding | Test |
| FR-192-AC-2 | A pack with two clause sets gives two tables in index order | Test |
| FR-192-AC-3 | With no `--clause-binding` and no `--pack` the document has a `no_clause_binding` gap, status `incomplete`, and the command does not exit 2 | Test |
| FR-192-AC-4 | A clause with restricted rights renders id and status only; its text appears nowhere in the file | Test |
| FR-192-AC-5 | A row with no compliance or evidence produces a `clause_open` gap | Test |
| FR-192-AC-6 | `As of` and `unusedFacts` appear in the document as the source reports them | Test |

## Dependencies

- **Upstream**: [FR-188](./FR-188-dossier-command.md); the discharge mapping output.
- **Downstream**: none.
