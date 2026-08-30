---
id: FR-046
title: "Record the semantic data planes"
type: FR
relationships:
  - target: "ix://agent-ix/quoin/US-013"
    type: "traces_to"
---

# FR-046: Record the semantic data planes

## Description

When the semantic-module architecture is consulted, Quoin SHALL maintain an indexed architecture
record that distinguishes the meta, definition, execution/observation, and presentation planes.

## Rationale

The same domain concept can appear as a definition, an occurrence, and a report. Treating those as
interchangeable encodings creates competing authorities and encourages operational records to be
edited as prose. The four-plane model preserves their relationship without imposing one universal
runtime envelope.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-046-AC-1 | The record defines all four planes and names representative semantic-module concepts, authorities, and projections for each. | Test (TC-1125) |
| FR-046-AC-2 | The record distinguishes a definition from an occurrence and a presentation of that occurrence, including `TestCase`, `TestExecution`, and a run report. | Test (TC-1126) |
| FR-046-AC-3 | The record states that structural kind and semantic role are independent and rejects a universal `SemanticObject` envelope. | Test (TC-1127) |
| FR-046-AC-4 | The architecture index links every normative record and labels provisional decisions and external resolution gates explicitly. | Test (TC-1128) |

## Constraints

- This requirement records architecture only. It introduces no compiler, schema migration,
  persistence migration, emitter, publication, or runtime enforcement.

## Dependencies

- **Upstream**: [US-013](../usecase/US-013-reason-about-semantic-module-boundaries.md)
- **External basis**: `agent-ix/filament-core-data#8`, especially ARCH-005
