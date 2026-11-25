---
id: FR-227
title: "Report unreachable and unmapped SACM export content"
type: FR
relationships:
  - target: "ix://agent-ix/quoin/FR-234"
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
- Node or edge omissions made by the SACM mapping in
  [FR-226](./FR-226-sacm-export.md).
- An optional `--sacm-report <PATH>` output path.

## Outputs

- One diagnostic line per item on stderr.
- When requested, a UTF-8 JSON report with exactly the `unmapped` and
  `unreachable` top-level members.

## Behavior

- The SACM exporter SHALL write an unmapped item to stderr as
  `unmapped: <source>: <reason>`.
- For an unmapped edge, the item SHALL identify the origin node's Quoin
  source id in `source`. The `reason` SHALL name the edge kind, the target
  node's Quoin source id and the mapping limit.
- The SACM exporter SHALL write an unreachable requirement to stderr as
  `unreachable: <id>`.
- The JSON report SHALL preserve source ids and reasons, use deterministic
  ordering, and include every reported item exactly once.
- The SACM XMI SHALL contain no element invented to stand in for an unreachable
  or unmapped item. The exporter SHALL NOT replace an omitted edge with a
  different relationship or use it to connect otherwise mapped endpoints.
- If the report write fails, then the command SHALL return an error.
- When the report write fails, the command SHALL NOT claim that the report was
  retained.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-227-AC-1 | A fixture with one unreachable requirement and one unmapped source item writes both diagnostics to stderr and records both in the JSON report with their ids and reasons. | Test |
| FR-227-AC-2 | Unreachable and unmapped source items produce no SACM stand-in elements and are not silently dropped. | Test |
| FR-227-AC-3 | Repeating the report for unchanged input produces byte-identical JSON with deterministic item order. | Test |
| FR-227-AC-4 | An unwritable report path returns an error and does not report successful report retention. | Test |
| FR-227-AC-5 | Exporting an authored goal → strategy → strategy → goal chain via `quoin assurance --argument <ID> --sacm --sacm-report <PATH>` retains the two Claims and both ArgumentReasoning elements. Stderr and the JSON report name both ends of the omitted nested edge and explain the omission. No inference or evidence relationship crosses the omitted edge. An unrelated goal → strategy → goal path still yields its own inference. | Test |

## Dependencies

- **Upstream**: [FR-234](./FR-234-gsn-model-export.md)
- **Downstream**: [PLAT-1647](https://linear.app/agent-ix/issue/PLAT-1647), [PLAT-1649](https://linear.app/agent-ix/issue/PLAT-1649)
