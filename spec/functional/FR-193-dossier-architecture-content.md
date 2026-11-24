---
id: FR-193
title: "The dossier architecture document lists parts, ports, connections, allocations, ADRs and the requirements each part serves"
type: FR
relationships:
  - target: "ix://agent-ix/quoin/StR-010"
    type: "traces_to"
  - target: "ix://agent-ix/quoin/US-024"
    type: "implements"
  - target: "ix://agent-ix/quoin/FR-191"
    type: "extends"
  - target: "ix://agent-ix/quoin/FR-160"
    type: "references"
  - target: "ix://agent-ix/quire-rs/FR-090"
    type: "references"
  - target: "ix://agent-ix/spec-objects-architecture/FR-012"
    type: "references"
---

# FR-193: The dossier architecture document lists parts, ports, connections, allocations, ADRs and the requirements each part serves

## Description

The architecture document (`--doc architecture`, document id `DOS-ARCH`) SHALL
list parts, their ports, connections (with flow direction and ends),
allocations and ADRs, and for each part the requirements allocated to or
realized by it, from the quire-rs design join (quire-rs FR-086).

## Inputs

- The `design-join` source (`quire_rs::design_join`) and the `design-matrix` source (quire-rs FR-092), which `quoin-quire` exposes as `rust/crates/quoin-quire/src/design_join.rs` and `design_matrix.rs`.
- The `systems-records` source: the systems records `PartRecord`, `PortRecord`, `ConnectionRecord` and allocation of quire-rs `src/semantic/systems.rs`, and the objects `Part`, `Port`, `Connection` and `Allocation` of spec-objects-architecture.
- Documents of the `ADR` archetype.
- Source contract: the class of a part is a frontmatter field of its `part` document (spec-objects-architecture FR-012), which the quire engine extracts as the part record key `softwareSafetyClass`. The `systems-records` source (`PartRecord` of quire-rs, wrapped by `quoin-quire`) carries no frontmatter, so `quoin-quire` SHALL expose that record key, as written and omitted when absent, beside each `Part`, matched to the `PartRecord` by the part document's id.

## Outputs

- `dossier/DOS-ARCH.md`, then `Gaps`.

## Behavior

- The architecture document SHALL list elements in dossier order (FR-191) by element id.
- The architecture document SHALL show for each part the software safety class recorded in its `part` document, exactly as recorded.
- If a part records no software safety class, the document SHALL show `unassigned` and record one `safety_class_unassigned` gap naming the part.
- The document SHALL NOT infer, default or inherit a class, from a parent part, an allocated requirement or elsewhere.
- The architecture document SHALL show for each connection its flow direction and both ends.
- The architecture document SHALL list for each part the requirements allocated to or realized by it, from the join.
- If the join reports a requirement in the `uncovered` state in a design relation, the document SHALL record one `no_design_element` gap per `uncovered` row, naming the requirement and the `check` token of that row's relation.
- A `no_design_element` gap SHALL state that the relation's check found no element.
- A `no_design_element` gap SHALL NOT state that the requirement has no design element at all when it has an element under another relation.
- A requirement with `uncovered` rows in two relations has two gaps, told apart by the `check` token.
- If a design reference resolves to no element, the document SHALL show the reference.
- If a design reference resolves to no element, the document SHALL record a `dangling_design_reference` gap naming it.
- If the join lists a design element of a kind the relation does not accept (`kind_mismatch`), the document SHALL show the element.
- If the join lists a design element of a kind the relation does not accept (`kind_mismatch`), the document SHALL record a `design_kind_mismatch` gap naming the requirement and the element.
- If a part has no allocated requirement, the document SHALL record a `no_allocation` gap naming the part.
- If the loaded modules declare no `design_relations`, the document SHALL record a `design_relations_unavailable` gap that states the cause.
- A dangling design reference or a kind mismatch SHALL NOT be dropped.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-193-AC-1 | A fixture with two parts, one connection, one ADR and one requirement with no design element lists all elements and a `no_design_element` gap naming that requirement | Test |
| FR-193-AC-2 | A dangling design reference is listed and produces a `dangling_design_reference` gap | Test |
| FR-193-AC-3 | A module set declaring no `design_relations` yields status `incomplete` with a `design_relations_unavailable` gap, and `generate` does not exit 0 | Test |
| FR-193-AC-4 | A part with no allocated requirement produces a `no_allocation` gap | Test |
| FR-193-AC-5 | A connection shows its flow direction and both ends | Test |
| FR-193-AC-6 | A wrong-kind design element under an `outgoing` relation is listed and produces a `design_kind_mismatch` gap naming the requirement and the element | Test |
| FR-193-AC-7 | A part that records class `class-3` shows `class-3` as recorded. | Test |
| FR-193-AC-8 | A part with no recorded class shows `unassigned` and one `safety_class_unassigned` gap naming the part, even when its parent part and an allocated requirement record a class. | Test |
| FR-193-AC-9 | A requirement with a `covered` row in one relation and an `uncovered` row in a second records one `no_design_element` gap carrying the second relation's `check` token and no gap for the first; a requirement with `uncovered` rows in two relations records two gaps with their two `check` tokens. | Test |

## Dependencies

- **Upstream**: [FR-191](./FR-191-dossier-command.md); quire-rs [FR-086](ix://agent-ix/quire-rs/FR-086) and [FR-092](ix://agent-ix/quire-rs/FR-092); quire-cli [FR-041](ix://agent-ix/quire-cli/FR-041) (existing rendering of the join).
- **Upstream (open)**: `quoin-quire` exposes the design join and design matrix but not the systems records (`PartRecord`, `PortRecord`, `ConnectionRecord`, allocation). Exposing them is build work for PLAT-1437, which implements this document. Until it lands the generator cannot list parts, ports and connections.
- **Downstream**: none.
