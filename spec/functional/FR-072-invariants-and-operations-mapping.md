---
id: FR-072
title: "Invariants and Operations mapping to ClauseRef and OperationDecl"
type: FR
relationships:
  - target: "ix://agent-ix/quoin/US-020"
    type: "implements"
  - target: "ix://agent-ix/quoin/FR-071"
    type: "depends_on"
---

# FR-072: Invariants and Operations mapping to ClauseRef and OperationDecl

## Description

Quoin SHALL publish the mapping from `## Invariants` and `## Operations`
sections to semantic-core `ClauseRef[]` and `OperationDecl[]` as golden
fixtures that Quire implements, so that one clause has one language and one
editable authority.

## Rationale

Ticket #293 mapping (c). Formal clauses are opaque to Quire; the fence language
selects the checker downstream. Duplicating a clause inline and externally
creates two authorities.

## Behavior

- Each language-tagged fence under `## Invariants` SHALL map to one `ClauseRef` with `language` = the fence info string, `clauseId` = the text of the nearest preceding `### <clauseId>` heading, and `sourceSpan` = a semantic-core `SourceLocus` (`sourceIdentity`, `path`, `startLine`, `startColumn`, `endLine`, `endColumn`) covering the fence.
- A `clauseId` SHALL be an `Identifier` (`^[A-Za-z_][A-Za-z0-9_]*$`).
- If a clause heading's text is not an `Identifier`, then validation SHALL fail at the heading.
- Quire SHALL accept the `language` value `ocl` without a finding, and `sysml`, `fretish`, or a namespaced `<ns>:<name>` with the advisory finding `semantic.clause-language-unchecked` until a frontend exists for it.
- If a fence under `## Invariants` carries no language, or a language outside the `ClauseLanguage` pattern, then validation SHALL fail at the fence.
- If two clauses in one artifact share a `clauseId`, including across `## Invariants` and `## Operations` subsections, then validation SHALL fail at the second.
- Each `### <name>` subsection under `## Operations` SHALL map to one `OperationDecl` whose `name` is the heading (an `Identifier`), whose `params` come from a typed table with header `Param | Type | Multiplicity | Constraints` using the FR-071 cell grammars, whose `returns` comes from a `Returns:` line (`<Type>[<mult>]`), and whose `pre`/`post` come from `Pre:`/`Post:` lines listing clause ids.
- If two `### <name>` subsections under `## Operations` share a name, then validation SHALL fail at the second.
- If a `Pre:`/`Post:` line names a clause id declared nowhere in the artifact, then validation SHALL fail at that line.
- Quire SHALL extract fence text verbatim into the clause-text map without parsing, normalizing, or evaluating it.
- If the same `clauseId` is declared both by a fence and by an external reference line `Clause: <relative path>#<clauseId>`, then validation SHALL fail at the second occurrence.

## Constraints

| ID | Constraint | Type | Validation |
|---|---|---|---|
| FR-072-CON-1 | Clause semantics (typechecking, evaluation) SHALL stay outside Quire and Quoin; only extraction and the mapping are specified here. | Boundary | Static scan for clause parsers |

## Acceptance Criteria

| ID | Criteria | Verification |
|---|---|---|
| FR-072-AC-1 | An `## Invariants` section with one `ocl` fence under `### immutable` has an expected `ClauseRef { language: ocl, clauseId: immutable, sourceSpan }` fixture and the fence text verbatim. | Test |
| FR-072-AC-2 | A fence without a language, or tagged `tla`, has an expected failure at the fence; `sysml`, `fretish`, and `acme:tla` have an expected `semantic.clause-language-unchecked` advisory. | Test |
| FR-072-AC-3 | Two `### immutable` clauses, and a `### not-archived` heading, have expected failures at the second clause and at the heading. | Test |
| FR-072-AC-4 | An `## Operations` subsection `### archive` with a two-row param table, `Returns: ConfigVersion[1]`, `Pre: notArchived`, `Post: archived` has the expected `OperationDecl` fixture. | Test |
| FR-072-AC-5 | `Post: missing` has an expected failure at that line. | Test |
| FR-072-AC-6 | A clause declared by a fence and by `Clause: ./clauses.md#immutable` has an expected failure at the second occurrence. | Test |

## Dependencies

- **Upstream**: [FR-071](./FR-071-typed-properties-mapping.md), semantic-core `ClauseRef`/`OperationDecl`/`SourceLocus`
- **Downstream**: `agent-ix/quire-rs#388`, `agent-ix/quire-contract-ir#52`
