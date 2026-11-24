---
id: FR-223
title: "Represent assurance-case context, assumptions, justifications, and counter-claims"
type: FR
relationships:
  - target: "ix://agent-ix/quoin/FR-040"
    type: "extends"
  - target: "ix://agent-ix/quoin/FR-047"
    type: "extends"
---

# FR-223: Represent assurance-case context, assumptions, justifications, and counter-claims

## Description

The assurance-case model SHALL represent context, assumption, justification,
and counter-claim nodes separately from goals, strategies, and solutions.

When building the case model, `quoin assurance` SHALL derive these nodes only
from the source records and relationships below.

If an authored source record is absent, then `quoin assurance` SHALL NOT
invent an assumption or justification to make a case appear complete.

## Inputs

- The trace graph and source corpus already loaded by `quoin assurance`.
- The authored `AssuranceArgument` data supplied by its owning module.

## Outputs

- A case model with typed nodes and explicit parent attachment.
- The derived and authored views that render those nodes without changing the
  source argument or auditor verdicts.

## Behavior

- A goal or strategy MAY carry context nodes. A context, assumption,
  justification, or counter-claim node SHALL have no child nodes, evidence,
  or nested context.
- In derived mode, each `mitigates` relation from a goal's source requirement
  to a corpus object of type hazard, threat, risk, vulnerability, or
  failure-mode SHALL produce one context node. `quoin assurance` SHALL order
  those nodes by target id. If a target is absent from the loaded corpus, then
  `quoin assurance` SHALL produce an open context node naming that target.
- In authored mode, each source assumption SHALL produce an assumption node
  attached to the top claim, preserving its id, statement, owner, status, and
  review instant.
- A resolved authored challenge SHALL produce a justification node attached
  to its declared target, preserving the challenge id, owner, resolution
  references, and expiry when present.
- An open authored challenge SHALL produce a counter-claim node attached to
  the challenge target. It SHALL NOT be represented as a context,
  justification, or supporting node. The counter-claim SHALL preserve the
  challenge id, statement, owner, open reason, declared status, resolution
  references, and expiry when present. The top claim remains open.
- The case builder SHALL produce context nodes attached to the top claim for
  authored participants and authority.
- The case builder SHALL retain participant and authority context as
  descriptive facts.
- The case builder SHALL NOT create a `supported_by` edge from a participant
  or authority context node.
- The case builder SHALL NOT change the top-claim status based on participant
  or authority context.
- If no authored source exists for an assumption or justification, then
  `quoin assurance` SHALL omit nodes of that kind from the derived case.
- The existing JSON representation SHALL omit an empty context collection so
  cases without context retain their existing bytes.
- Rendering SHALL show context, assumption, justification, and counter-claim
  nodes beneath the goal or strategy to which they are attached.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-223-AC-1 | A requirement mitigating two loaded hazards produces two context nodes ordered by target id; a missing target produces one open context node with its id and reason. | Test |
| FR-223-AC-2 | A requirement with no `mitigates` relation emits no `context` field in JSON, and existing no-context JSON fixtures remain byte-identical. | Test |
| FR-223-AC-3 | An authored argument with two assumptions, one resolved challenge, one open challenge, and participants produces two assumptions, one justification, one counter-claim, and context for the participants with all declared source fields preserved. | Test |
| FR-223-AC-4 | An open challenge is represented only as a counter-claim; it is never a Context node or a supporting node, and its top claim remains open. | Test |
| FR-223-AC-5 | A case-model constructor rejects a context, assumption, justification, or counter-claim node that contains children, evidence, or nested context. | Test |
| FR-223-AC-6 | Markdown and Mermaid render each context node under its owning goal or strategy. | Test |
| FR-223-AC-7 | The derived case contains no assumption or justification node without an authored source record. | Test |

## Dependencies

- **Upstream**: [FR-040](./FR-040-assurance-case-view.md), [FR-047](./FR-047-authored-assurance-arguments.md)
- **Downstream**: [PLAT-1447](https://linear.app/agent-ix/issue/PLAT-1447), [PLAT-1647](https://linear.app/agent-ix/issue/PLAT-1647)
