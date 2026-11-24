---
id: FR-222
title: "Validate GSN model well-formedness before export"
type: FR
relationships:
  - target: "ix://agent-ix/quoin/FR-221"
    type: "requires"
---

# FR-222: Validate GSN model well-formedness before export

## Description

When `quoin assurance` is about to write a GSN model, it SHALL validate the
model against the well-formedness rules below. If validation finds one or more
violations, then the exporter SHALL report every violation with its rule id
and affected node ids. When a model violates a well-formedness rule, the
quoin assurance command SHALL write no model to stdout. When a model violates
a well-formedness rule, the quoin assurance command SHALL return a failure
exit code from the declared exit grammar.
The validator applies to both the derived case and authored argument exports.

## Well-formedness rules

- **WF-1 — SupportedBy endpoints.** A `supported_by` edge SHALL start at a
  goal or strategy and end at a goal, strategy, or solution.
- **WF-2 — InContextOf endpoints.** An `in_context_of` edge SHALL start at a
  goal or strategy and end at a context, assumption, or justification.
- **WF-3 — Terminal nodes.** A solution, context, assumption, justification,
  or counter-claim SHALL NOT have an outgoing `supported_by` or
  `in_context_of` edge. A counter-claim may have its one `challenges` edge as
  defined by WF-8.
- **WF-4 — Acyclic support.** The graph formed by `supported_by` edges SHALL be
  acyclic. A goal may have multiple parents; sharing a child is not a cycle.
- **WF-5 — Developed argument nodes.** Each goal and strategy SHALL have at
  least one outgoing `supported_by` edge or have `undeveloped: true`.
- **WF-6 — Identifiers and references.** Node ids SHALL be unique and every
  edge endpoint SHALL identify an existing node.
- **WF-7 — Undeveloped nodes.** A node marked `undeveloped: true` SHALL NOT
  have an outgoing `supported_by` edge.
- **WF-8 — Counter-claim target.** Each counter-claim SHALL have exactly one
  outgoing `challenges` edge, and that edge SHALL target a goal, strategy, or
  assumption. No other node kind may be the source of a `challenges` edge.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-222-AC-1 | A model with a `supported_by` edge from a solution to a goal is refused under WF-1, names both endpoint ids, writes no model to stdout, and exits with failure. | Test |
| FR-222-AC-2 | A model with an `in_context_of` edge from an assumption to a goal is refused under WF-2, names both endpoint ids, writes no model to stdout, and exits with failure. | Test |
| FR-222-AC-3 | A model with an outgoing `supported_by` or `in_context_of` edge from a solution, context, assumption, justification, or counter-claim is refused under WF-3 and identifies every invalid source node. | Test |
| FR-222-AC-4 | A model containing a two-goal `supported_by` cycle is refused under WF-4 and names both goal ids; a valid shared child with two parents is accepted. | Test |
| FR-222-AC-5 | A goal or strategy with no `supported_by` child and no `undeveloped: true` is refused under WF-5 and names that node id. | Test |
| FR-222-AC-6 | A model with duplicate node ids or an edge endpoint absent from `nodes` is refused under WF-6, names the duplicate or dangling ids, and writes no model to stdout. | Test |
| FR-222-AC-7 | An `undeveloped: true` goal with a `supported_by` child is refused under WF-7 and names the goal id. | Test |
| FR-222-AC-8 | A counter-claim with no `challenges` edge, more than one such edge, or an edge to a solution is refused under WF-8 and names the counter-claim and invalid target ids. | Test |
| FR-222-AC-9 | A model with multiple independent violations reports each applicable rule id and all affected node ids in one failed export, with no partial model on stdout. | Test |

## Dependencies

- **Upstream**: [FR-221](./FR-221-gsn-model-export.md)
- **Downstream**: [PLAT-1447](https://linear.app/agent-ix/issue/PLAT-1447)
