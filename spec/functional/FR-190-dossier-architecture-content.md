---
id: FR-190
title: "The dossier architecture document lists parts, ports, connections, allocations, ADRs and the requirements each part serves"
type: FR
relationships:
  - target: "ix://agent-ix/quoin/StR-007"
    type: "traces_to"
  - target: "ix://agent-ix/quoin/US-024"
    type: "implements"
  - target: "ix://agent-ix/quoin/FR-188"
    type: "extends"
---

# FR-190: The dossier architecture document lists parts, ports, connections, allocations, ADRs and the requirements each part serves

## Description

The architecture document (`--doc architecture`, document id `DOS-ARCH`) SHALL
list parts, their ports, connections (with flow direction and ends),
allocations and ADRs, and for each part the requirements allocated to or
realized by it, from the quire-rs design join (quire-rs FR-086).

## Inputs

- The design join (`quire_rs::design_join`) and design matrix (quire-rs FR-092), which `quoin-quire` already exposes as `rust/crates/quoin-quire/src/design_join.rs` and `design_matrix.rs`.
- The systems records `PartRecord`, `PortRecord`, `ConnectionRecord` and allocation of quire-rs `src/semantic/systems.rs`, and the objects `Part`, `Port`, `Connection` and `Allocation` of spec-objects-architecture.
- Documents of the `ADR` archetype.

## Outputs

- `dossier/DOS-ARCH.md`, then `Gaps`.

## Behavior

- Elements SHALL be listed by element id; connections show direction and both ends.
- Each part SHALL list the requirements allocated to or realized by it, from the join.
- A requirement in the join's `uncovered` state SHALL produce a `no_design_element` gap naming it.
- A design reference that resolves to no element SHALL be shown and produce a `dangling_design_reference` gap. It SHALL NOT be dropped.
- A part with no allocated requirement SHALL produce a `no_allocation` gap.
- A module set that declares no `design_relations` SHALL make the document `incomplete` with that cause stated in `Gaps`.
- Dependency gap: `quoin-quire` exposes the design join and design matrix but not the systems records (`PartRecord`, `PortRecord`, `ConnectionRecord`, allocation). Exposing them is build work for PLAT-1437, which implements this document. Until it lands the generator cannot list parts, ports and connections.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-190-AC-1 | A fixture with two parts, one connection, one ADR and one requirement with no design element lists all elements and a `no_design_element` gap naming that requirement | Test |
| FR-190-AC-2 | A dangling design reference is listed and produces a `dangling_design_reference` gap | Test |
| FR-190-AC-3 | A module set declaring no `design_relations` yields status `incomplete` with that cause, and does not exit 0 | Test |
| FR-190-AC-4 | A part with no allocated requirement produces a `no_allocation` gap | Test |
| FR-190-AC-5 | A connection shows its flow direction and both ends | Test |

## Dependencies

- **Upstream**: [FR-188](./FR-188-dossier-command.md); quire-rs FR-086 and FR-092; quire-cli FR-041 (existing rendering of the join); PLAT-1437 for the systems-record exposure.
- **Downstream**: none.
