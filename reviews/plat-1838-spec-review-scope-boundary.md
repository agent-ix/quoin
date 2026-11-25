---
id: SR-967175026302197768900474181941935084765
title: "Scope-boundary review of Quoin PLAT-1838 dossier requirements"
type: SpecReview
analysis: scope-boundary
scope: "agent-ix/quoin; spec/functional/FR-191-dossier-command.md, FR-229 through FR-232, spec/usecase/US-024-hand-a-reviewer-dossier-documents-generated-from-records.md"
review_set: subset
relationships:
  - target: "ix://agent-ix/quoin/FR-191"
    type: "reviews"
  - target: "ix://agent-ix/quoin/FR-229"
    type: "reviews"
  - target: "ix://agent-ix/quoin/FR-230"
    type: "reviews"
  - target: "ix://agent-ix/quoin/FR-231"
    type: "reviews"
  - target: "ix://agent-ix/quoin/FR-232"
    type: "reviews"
---

## Summary

Quoin owns deterministic document generation and preserves human-authored decisions; the FRs correctly prohibit Quoin from assigning class, risk, or acceptability values. One source-selection boundary remains undefined: the specification does not say how a selected architecture `part` is identified as a software item.

## In-Scope Responsibilities

- Quoin writes deterministic Markdown from selected records, preserves source identity, emits explicit gaps, and carries its running producer identity.
- People author product, device, design, and documentation-level decision facts; Quoin does not infer intended use or decide software class, risk, or acceptability.

## External Dependencies

| Dependency | Type | Assumed or Guaranteed | Contract |
| --- | --- | --- | --- |
| Caller-selected software/device/decision source records | Module records | Assumed | No installed catalog schema or owning artifact defines three of the named record types. |
| Architecture parts, ports, connections, allocations, and decisions | Architecture object records | Guaranteed by installed module schemas | FR-231 does not define which selected `part` records are software items. |
| DossierDocument/producer identity | Process artifact contract | Guaranteed | spec-artifacts-process FR-018, referenced by FR-191. |
| Quire export | External CLI | Guaranteed by existing FR-191 and quire-cli contracts | FR-191 delegates final export; this PR adds no new export dependency. |

## Responsibility Allocation

| Requirement | Owning Component | Class |
| --- | --- | --- |
| FR-191 | Quoin dossier command | cross-cutting |
| FR-229–FR-232 | Quoin dossier generators | core |
| Authorship of source records and all classification/acceptability decisions | Adopter/person-authored module records | core |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | FR-231 requires one section per software item but gives Quoin no explicit marker or relationship for distinguishing software `part` records from other parts. | FR-231 |

## Evidence

FR-231 Inputs calls for software-item `part` records and says not to infer associations from names or paths. The installed `part` schema contains `owner`, `declaredType`, and `multiplicity`, plus optional person-assigned software safety-class fields; it has no software-item discriminator. Since safety class is optional and must remain person-authored, it cannot define the selection rule. Two conforming implementations can select different parts or emit different sections.
