# Semantic-module mapping fixtures (quoin FR-071, FR-072, FR-074)

These fixtures are the normative surface of the Markdown → semantic-core mapping
that `agent-ix/quire-rs#388` implements. Quoin does not execute the extraction;
its tests assert that every expected output here validates against the vendored
semantic-core schemas (`src/semantic/schemas/semantic-core/`, version recorded
in each file), that the table and fence forms of the same content share one
expected `FieldDecl[]`, and that every expected diagnostic carries a locus.

| File | Requirement | Hand-off |
|---|---|---|
| `config-version.table.md`, `config-version.fence.md`, `config-version.expected.json` | FR-071-AC-1, AC-2 | #388 extracts both to the expected array (normalized) |
| `both-forms.md` | FR-071-AC-3 | #388 fails at the second form's locus |
| `cell-cases.json` | FR-071-AC-4..AC-8 | #388 executes each cell/fence-line case |
| `operations.md`, `operations.expected.json`, `operations-cases.json` | FR-072-AC-1..AC-6 | #388 extracts clauses/operations and emits the listed diagnostics |
| `legacy-bullets.md`, `legacy-mixed.md`, `legacy.expected.json`, `../corpus/config-service/` | FR-074-AC-1, AC-2 | quoin's `classifyArtifact` and #388 agree on form, line, and warning |

`../corpus/config-service/FR-006-config-version-entity.md` is a verbatim copy
pinned by `PROVENANCE.json`; it is never edited.
