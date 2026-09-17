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

Ticket #293 mapping (c). Clauses are Quire (QSpec AD-006; filament-core-data
FR-028, semantic-core `0.2.0`): `quire` is the one checked clause language, and
every other admitted language is carried verbatim and never evaluated.
Duplicating a clause inline and externally creates two authorities.

## Behavior

- Each language-tagged fence under `## Invariants` SHALL map to one `ClauseRef` with `language` = the fence info string, `clauseId` = the text of the nearest preceding `### <clauseId>` heading, and `sourceSpan` = a semantic-core `SourceLocus` (`sourceIdentity`, `path`, `startLine`, `startColumn`, `endLine`, `endColumn`) covering the fence.
- A `clauseId` SHALL be an `Identifier` (`^[A-Za-z_][A-Za-z0-9_]*$`).
- If a clause heading's text is not an `Identifier`, then validation SHALL fail at the heading.
- The golden fixtures SHALL pin semantic-core `0.2.0` and author every checked invariant in a `quire` fence.
- When a fence under `## Invariants` is tagged `quire`, Quire SHALL extract the clause with no finding and report the clauses kind available and not lossy.
- When a fence under `## Invariants` is tagged `ocl`, `sysml`, `fretish`, or a namespaced `<ns>:<name>`, Quire SHALL carry the clause text verbatim, emit the advisory finding `semantic.clause-language-unchecked` at the fence, and report the clauses kind available and lossy.
- If a fence under `## Invariants` carries no language, or a language outside the `ClauseLanguage` pattern, then validation SHALL fail at the fence.
- If two clauses in one artifact share a `clauseId`, including across `## Invariants` and `## Operations` subsections, then validation SHALL fail at the second.
- Each `### <name>` subsection under `## Operations` SHALL map to one `OperationDecl` whose `name` is the heading (an `Identifier`), whose `params` come from a typed table with header `Param | Type | Multiplicity | Constraints` using the FR-071 cell grammars, whose `returns` comes from a `Returns:` line (`<Type>[<mult>]`), and whose `pre`/`post` come from `Requires:`/`Ensures:` lines listing clause ids.
- If two `### <name>` subsections under `## Operations` share a name, then validation SHALL fail at the second.
- If a `Requires:`/`Ensures:` line names a clause id declared nowhere in the artifact, then validation SHALL fail at that line.
- Quire SHALL extract fence text verbatim into the clause-text map without parsing, normalizing, or evaluating it.
- If the same `clauseId` is declared both by a fence and by an external reference line `Clause: <relative path>#<clauseId>`, then validation SHALL fail at the second occurrence.

## Constraints

| ID | Constraint | Type | Validation |
|---|---|---|---|
| FR-072-CON-1 | Quoin and the clause extraction SHALL NOT typecheck or evaluate clause text; checking a `quire` clause belongs to the Quire checker, and only extraction and the mapping are specified here. | Boundary | Static scan for clause parsers |

## Acceptance Criteria

| ID | Criteria | Verification |
|---|---|---|
| FR-072-AC-1 | Under semantic-core `0.2.0`, an `## Invariants` section with `quire` fences under `### notArchived` and `### archived` has an expected `ClauseRef { language: quire, clauseId, sourceSpan }` per fence, the fence text verbatim, no diagnostic, and the clauses kind available and not lossy. | Test |
| FR-072-AC-2 | A fence without a language, or tagged `tla`, has an expected failure at the fence; `sysml`, `fretish`, and `acme:tla` have an expected `semantic.clause-language-unchecked` advisory. | Test |
| FR-072-AC-3 | Two `### immutable` clauses, and a `### not-archived` heading, have expected failures at the second clause and at the heading. | Test |
| FR-072-AC-4 | An `## Operations` subsection `### archive` with a two-row param table, `Returns: ConfigVersion[1]`, `Requires: notArchived`, `Ensures: archived` has the expected `OperationDecl` fixture. | Test |
| FR-072-AC-5 | `Ensures: missing` has an expected failure at that line. | Test |
| FR-072-AC-6 | A clause declared by a fence and by `Clause: ./clauses.md#immutable` has an expected failure at the second occurrence. | Test |
| FR-072-AC-7 | Under semantic-core `0.2.0`, an `ocl` fence under `### immutable` has an expected `ClauseRef { language: ocl, clauseId: immutable }`, the body carried verbatim, exactly one `semantic.clause-language-unchecked` advisory at the fence, and the clauses kind available and lossy. | Test |

## Dependencies

- **Upstream**: [FR-071](./FR-071-typed-properties-mapping.md), semantic-core `ClauseRef`/`OperationDecl`/`SourceLocus`
- **Downstream**: `agent-ix/quire-rs#388`, `agent-ix/quire-rs#432`, `agent-ix/quire-contract-ir#52`
