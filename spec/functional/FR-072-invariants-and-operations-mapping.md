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

The mapping contract SHALL define how `## Invariants` and `## Operations`
sections map to semantic-core `ClauseRef[]` and `OperationDecl[]`, so that one
clause has one language and one editable authority.

## Rationale

Ticket #293 mapping (c). Formal clauses are opaque to Quire; the fence language
selects the checker downstream. Duplicating a clause inline and externally
creates two authorities.

## Behavior

- Each language-tagged fence under `## Invariants` SHALL map to one `ClauseRef` with `language` = the fence info string (`ocl` in this version; `sysml` and `fretish` admitted by the grammar but marked advisory until their frontends exist), `clauseId` from the fence's preceding `### <clauseId>` heading or an `id:` comment on the first line, and `sourceSpan` = the fence's byte span.
- If a fence under `## Invariants` carries no language, or a language outside the `ClauseLanguage` pattern, then validation SHALL fail at the fence.
- If two clauses in one artifact share a `clauseId`, then validation SHALL fail at the second.
- Each `### <name>` subsection under `## Operations` SHALL map to one `OperationDecl` whose `params` come from a typed table with header `Param | Type | Multiplicity | Constraints`, whose `returns` comes from a `Returns:` line (`Type[mult]`), and whose `pre`/`post` come from `Pre:`/`Post:` lines listing clause ids declared under `## Invariants` or in a fence inside the subsection.
- If a `Pre:`/`Post:` line names a clause id declared nowhere in the artifact, then validation SHALL fail at that line.
- Quire SHALL extract fence text verbatim into the clause-text map without parsing, normalizing, or evaluating it.
- If the same clause text appears both inline (fence) and by external reference (`clause: <path>`), then validation SHALL fail at the second occurrence.

## Constraints

| ID | Constraint | Type | Validation |
|---|---|---|---|
| FR-072-CON-1 | Clause semantics (typechecking, evaluation) SHALL stay outside Quire and Quoin; only extraction and the mapping are specified here. | Boundary | Inspection |

## Acceptance Criteria

| ID | Criteria | Verification |
|---|---|---|
| FR-072-AC-1 | An `## Invariants` section with one `ocl` fence under `### immutable` extracts to one `ClauseRef { language: ocl, clauseId: immutable, sourceSpan }` and the fence text verbatim. | Test |
| FR-072-AC-2 | A fence without a language, or tagged `tla`, fails at the fence. | Test |
| FR-072-AC-3 | Two `### immutable` clauses fail at the second. | Test |
| FR-072-AC-4 | An `## Operations` subsection `### archive` with a two-row param table, `Returns: ConfigVersion[1]`, `Pre: not-archived`, `Post: archived` extracts to the expected `OperationDecl`. | Test |
| FR-072-AC-5 | `Post: missing` fails at that line. | Test |
| FR-072-AC-6 | A clause present inline and by `clause:` reference fails at the second occurrence. | Test |

## Dependencies

- **Upstream**: [FR-071](./FR-071-typed-properties-mapping.md), semantic-core `ClauseRef`/`OperationDecl`
- **Downstream**: `agent-ix/quire-rs#388`, `agent-ix/quire-contract-ir#52`
