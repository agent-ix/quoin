---
id: FR-103
title: "Consolidate the Quoin side of corpus tooling"
type: FR
relationships:
  - target: "ix://agent-ix/quoin/FR-096"
    type: "requires"
---

# FR-103: Consolidate the Quoin side of corpus tooling

## Description

The corpus-accounting and source-selection consolidation is withdrawn: Quoin
no longer owns that subsystem. The one-shot corpus-measurement subsystem was
disposed of under [#388](https://github.com/agent-ix/quoin/issues/388), and the
accepted corpus is owned by `agent-ix/qa-corpus`; this repository has no
`corpus/` submodule or gitlink. The retained read-only boundary is specified by
FR-100-CON-3.

## Inputs

- The retained read-only corpus boundary in FR-100-CON-3.

## Outputs

- A recorded disposition that this repository owns no corpus-measurement
  capability.

## Behavior

- No active Quoin measurement crate consumes corpus accounting or resolves a
  corpus source. Reintroducing either capability requires a new requirement and
  must follow FR-100's shared-ownership and read-only boundaries.

## Error Conditions

None: no corpus measurement or consolidation runs in this repository.

## Constraints

| ID | Constraint | Type | Validation |
|----|-----------|------|------------|
| FR-103-CON-1 | Quoin SHALL NOT implement corpus accounting or select a corpus source; any future capability requires a new requirement and ownership ruling. | Responsibility | Inspection |

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-103-AC-2 | Withdrawn: the consolidation was disposed of under #388 before any accepted-corpus tree was changed; the read-only guarantee remains FR-100-CON-3. | Withdrawn |
| FR-103-AC-3 | Withdrawn: Quoin no longer runs corpus measurement or selects a corpus source; a future such capability requires a new requirement. | Withdrawn |
| FR-103-AC-4 | Withdrawn: no current Quoin capability consumes corpus accounting from `engineering-assurance`; FR-100 governs any future shared capability. | Withdrawn |
| FR-103-AC-7 | Withdrawn: the corpus is not a submodule, so no revision is recorded or reported. | Withdrawn |

## Dependencies

- **Upstream**: the recorded disposition in [#388](https://github.com/agent-ix/quoin/issues/388) and the retained read-only boundary in [FR-100-CON-3](./FR-100-rust-evidence-measurement-change-assurance.md).
- **Downstream**: none. FR-100's measurement crates retain measurement-record behavior and SHALL NOT restate corpus accounting.
