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

- Each clause set SHALL be one table. Pack clause sets keep the pack's index order; separate bindings keep the command-line order.
- Each row SHALL carry every member of the source row, and the document SHALL show `As of` and `unusedFacts`.
- With no `--clause-binding` and no `--pack`, the document SHALL carry a `no_clause_binding` gap and be `incomplete`; this is not an error.
- A row with no compliance or evidence SHALL produce a `clause_open` gap.
- A clause whose rights are restricted (the rights metadata of `quire clauses`) SHALL render its id and status only, and its text SHALL NOT be written to any output, gap detail included.
- `As of` is the `--as-of` value; Quoin SHALL NOT read the clock.

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
