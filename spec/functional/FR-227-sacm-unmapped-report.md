---
id: FR-227
title: "Report unreachable and unmapped SACM export content"
type: FR
relationships:
  - target: "ix://agent-ix/quoin/FR-221"
    type: "requires"
---

# FR-227: Report unreachable and unmapped SACM export content

## Description

The SACM exporter SHALL report each unreachable requirement and each source
element it cannot map into SACM. It SHALL NOT silently drop or approximate
those elements. A report does not make the SACM package complete and does not
decide acceptance, risk, or compliance.

## Inputs

- The `unreachable` and `unmapped` lists in the validated GSN model.
- An optional `--sacm-report <PATH>` output path.

## Outputs

- One diagnostic line per item on stderr.
- When requested, a UTF-8 JSON report with exactly the `unmapped` and
  `unreachable` top-level members.

## Behavior

- An unmapped item SHALL be reported as `unmapped: <source>: <reason>`.
- An unreachable requirement SHALL be reported as `unreachable: <id>`.
- The JSON report SHALL preserve source ids and reasons, use deterministic
  ordering, and include every reported item exactly once.
- The SACM XMI SHALL contain no element invented to stand in for an unreachable
  or unmapped item.
- A report-write failure SHALL be an error; the command SHALL NOT claim that
  the report was retained.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-227-AC-1 | A fixture with one unreachable requirement and one unmapped source item writes both diagnostics to stderr and records both in the JSON report with their ids and reasons. | Test |
| FR-227-AC-2 | Unreachable and unmapped source items produce no SACM stand-in elements and are not silently dropped. | Test |
| FR-227-AC-3 | Repeating the report for unchanged input produces byte-identical JSON with deterministic item order. | Test |
| FR-227-AC-4 | An unwritable report path returns an error and does not report successful report retention. | Test |

## Dependencies

- **Upstream**: [FR-221](./FR-221-gsn-model-export.md)
- **Downstream**: [PLAT-1647](https://linear.app/agent-ix/issue/PLAT-1647), [PLAT-1649](https://linear.app/agent-ix/issue/PLAT-1649)
