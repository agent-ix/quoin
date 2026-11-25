---
id: FR-234
title: "Export assurance cases as a GSN model"
type: FR
relationships:
  - target: "ix://agent-ix/quoin/FR-040"
    type: "extends"
  - target: "ix://agent-ix/quoin/FR-047"
    type: "extends"
  - target: "ix://agent-ix/quoin/FR-223"
    type: "requires"
  - target: "ix://agent-ix/quoin/FR-224"
    type: "requires"
---

# FR-234: Export assurance cases as a GSN model

## Description

When the caller supplies `--gsn`, the `quoin assurance` command SHALL emit a
JSON model in the `quoin.gsn-case/v1` format for either the derived assurance
case or the authored argument selected with `--argument`. The model SHALL
preserve source identifiers and statements, represent every mapped node and
relationship, and report any source content it cannot map. The model uses the
GSN Community Standard v3 vocabulary and the dialectic counter-claim mapping
specified here. It does not claim support for GSN modular extensions.

The GSN model is an explicit export. Without `--gsn`, the existing Markdown,
Mermaid, and `--json` outputs remain unchanged. When a caller supplies both
`--gsn` and `--json`, `quoin assurance` SHALL reject the invocation as a usage
error.

The model contains `format`, `nodes`, `edges`, `roots`, `unreachable`, and
`unmapped`. Each node carries a generated `id`, `kind`, source `source`,
`statement`, and its applicable status, open reason, and evidence fields. The
model kind set is `goal`, `strategy`, `solution`, `context`, `assumption`,
`justification`, and the Quoin dialectic role `counter_claim`. `counter_claim`
is an explicit Quoin extension and does not claim to be a GSN v3 core element.
Edge types are `supported_by`, `in_context_of`, and the Quoin dialectic edge
`challenges`. These dialectic fields preserve an open authored challenge and
are not presented as a claim of full GSN interchange conformance. A generated
id is unique and deterministic;
its prefix identifies the kind (`G`, `S`, `Sn`, `C`, `A`, `J`, or `CC`) and
its counter follows depth-first traversal in case order. The `source` field
preserves the corresponding Quoin identifier.

### Derived case mapping

- A case goal maps to a GSN goal; a case strategy maps to a GSN strategy.
- A supported obligation leaf maps to a solution. An open obligation leaf
  maps to an undeveloped goal with open status and the auditor's reason
  verbatim. A goal with no sub-claim and no obligation maps to an undeveloped
  open goal.
- When an obligation leaf has `LeafEvidence`, the shared GSN model SHALL
  preserve it in the mapped node's `evidence` field. This applies to the
  supported solution and to the undeveloped open goal. Preserve each bound
  suite, its observation time when available, and the recorded verdict.
  An open obligation goal has no outgoing `supported_by` edge.
- Unreachable requirements appear in the top-level `unreachable` list and
  do not become nodes.
- Producer-trust and evidence-independence entries map to context nodes
  connected to the root goal with `in_context_of`. They never support a goal.
- A tool-confidence strategy SHALL show the source facts from its
  `IntendedUse`, retained self-test run, validation report, and anomaly records
  as specified in [FR-224](./FR-224-tool-confidence-argument.md). It SHALL NOT
  assign a confidence score or imply that the evidence decides trustworthiness.
- The context, assumption, justification, and counter-claim nodes defined by
  [FR-223](./FR-223-assurance-case-context-and-counter-claims.md) map to their
  corresponding Quoin model roles and retain their named target and source
  records.

### Authored argument mapping

- The top claim maps to the root goal. Each reasoning entry maps to a strategy
  connected by `supported_by` to the element named by its `supports` field.
- Each sufficiency criterion maps to a sub-goal of its strategy. A criterion
  with a current sufficiency decision maps its evidence references to solution
  nodes; a criterion without a current decision maps to an undeveloped open
  goal carrying the view's reason.
- Top-claim evidence references map to solution children of the root goal. A
  missing, unresolved, stale, or vacuous reference keeps the root open and
  carries the reason required by FR-047.
- For an authored solution, preserve the citation in `evidence.references`.
  Preserve the authored view's citation resolution state and reason when it
  supplies them. For a sufficiency citation, retain the associated decision
  and criterion status. A citation string does not select a retained run.
- An assumption maps to an assumption node connected to the root goal with
  `in_context_of`, retaining its owner, status, and review instant.
- A challenge whose current view status is resolved maps to a justification
  node connected to its target with `in_context_of`, retaining the source id,
  owner, resolution references, and expiry when present.
- A challenge whose current view status is open maps to a `counter_claim`
  node with an outgoing `challenges` edge to its target. Its node retains the
  challenge statement, owner, current open reason, declared status, resolution
  references, and expiry when present. It SHALL NOT map to a context node or
  use `in_context_of` or `supported_by` to represent the challenge. The target
  is the goal, strategy, or assumption named by the authored challenge. The
  authored top claim remains open as required by FR-047.
- Participants and authority map to context nodes connected to the root goal
  with `in_context_of`.

Content that cannot be represented by this mapping is listed in `unmapped`
with its source identifier and a reason. The exporter SHALL NOT silently omit
it or approximate an unsupported extension. GSN shapes and diagram rendering
belong to [PLAT-1645](https://linear.app/agent-ix/issue/PLAT-1645). SACM export
belongs to [PLAT-1646](https://linear.app/agent-ix/issue/PLAT-1646). Tool
confidence and signed acceptance are described by [FR-224](./FR-224-tool-confidence-argument.md)
and [FR-225](./FR-225-signed-acceptance-of-case-content.md), respectively.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-234-AC-1 | `quoin assurance --gsn` emits a model with format `quoin.gsn-case/v1`, and the model validates against the schema shipped at `rust/crates/quoin-assurance/schemas/gsn-case-v1.schema.json`. | Test |
| FR-234-AC-2 | A derived fixture exports goal, strategy, and solution nodes; `supported_by` edges preserve case order; every node's `source` equals its Quoin source id; and solution evidence matches the fixture's `LeafEvidence`. | Test |
| FR-234-AC-3 | `quoin assurance --gsn` on a derived repository with a bound run and a bound suite with no run retains the bound solution evidence and maps the open obligation as an undeveloped open goal. The open goal carries the auditor reason verbatim and its `LeafEvidence`, including the suite with no timestamp, and has no `supported_by` child. A childless ordinary goal is undeveloped and an unreachable requirement appears only in `unreachable`. | Test |
| FR-234-AC-4 | Producer-trust and evidence-independence entries export as context nodes connected only by `in_context_of`; no such node supports a goal. | Test |
| FR-234-AC-5 | `quoin assurance --argument <ID> --gsn` on an authored fixture exports its top claim, reasoning, criteria, evidence, assumptions, participants, and authority. Its citations retain their resolution outcomes or associated sufficiency decisions without a run selection. An undecided criterion is an undeveloped goal, and a missing or unresolved top-claim evidence reference keeps the root open with its reason. | Test |
| FR-234-AC-6 | A currently open authored challenge exports as a `counter_claim` node and one `challenges` edge to its goal, strategy, or assumption target; it is absent from `context` nodes and has no `in_context_of` or `supported_by` challenge edge; its source id, statement, owner, current reason, declared status, resolution references, and expiry are preserved. | Test |
| FR-234-AC-7 | A currently resolved authored challenge exports as a justification connected to its target by `in_context_of`, retaining its source id, owner, resolution references, and expiry when present. | Test |
| FR-234-AC-8 | Content outside the declared mapping is listed in `unmapped` with a reason and is never silently dropped or approximated. | Test |
| FR-234-AC-9 | Invoking `--gsn` with `--json` is a usage error and writes no model to stdout; invoking the command without `--gsn` preserves the existing output. | Test |
| FR-234-AC-10 | Exporting unchanged input twice produces byte-identical JSON, including generated node ids, node ordering, edge ordering, and top-level lists. | Test |
| FR-234-AC-11 | A tool-confidence strategy reports the intended-use, retained-run, validation-report, and anomaly source facts without a score or inferred trust verdict. | Test |

## Dependencies

- **Upstream**: [FR-040](./FR-040-assurance-case-view.md), [FR-047](./FR-047-authored-assurance-arguments.md), [FR-223](./FR-223-assurance-case-context-and-counter-claims.md), [FR-224](./FR-224-tool-confidence-argument.md)
- **Downstream**: [PLAT-1447](https://linear.app/agent-ix/issue/PLAT-1447), [PLAT-1645](https://linear.app/agent-ix/issue/PLAT-1645), [PLAT-1646](https://linear.app/agent-ix/issue/PLAT-1646)

## Reference

Goal Structuring Notation Community Standard, Version 3, published by the
Assurance Cases Working Group: https://mail.goalstructuringnotation.info/gsn-standard.
This requirement names the standard and its element vocabulary; it contains no
copied standard text.
