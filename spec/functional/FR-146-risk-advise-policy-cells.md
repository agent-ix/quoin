---
id: FR-146
title: "Risk advice policy cells: the cell each risk estimate lands in under the authored policy"
type: FR
relationships:
  - target: "ix://agent-ix/quoin/FR-145"
    type: "depends_on"
  - target: "ix://agent-ix/spec-objects-safety/FR-101"
    type: "depends_on"
  - target: "ix://agent-ix/spec-objects-safety/FR-102"
    type: "depends_on"
  - target: "ix://agent-ix/spec-objects-safety/FR-203"
    type: "depends_on"
  - target: "ix://agent-ix/spec-objects-safety/FR-204"
    type: "depends_on"
---

# FR-146: Risk advice policy cells: the cell each risk estimate lands in under the authored policy

## Description

Tracked by Linear PLAT-1651 and PLAT-1652. The `policy_cells` section of `quoin risk advise` (FR-145) shows,
for every risk estimate, the policy cell its entered severity and probability land in. The region is a lookup of
a cell a person wrote (`agent-ix/spec-objects-safety` FR-102); the report computes no score and combines no levels.

## Behavior

- The command SHALL list one entry for every `risk_estimate` in scope, pre-control and post-control, ordered by estimate id.
- Each entry SHALL carry the estimate id, its `phase`, its entered `severity` and `probability`, and the `policy` id it was read against.
- If `--policy` is supplied and both levels are members of that policy's level lists, then the entry SHALL carry the cell (the severity and probability pair), the region name the policy lists for it, and that region's `outside_acceptable` flag as the policy wrote it.
- The Markdown rendering of a resolved entry SHALL read "the policy marks this cell as <region>".
- If the region's flag is `true`, then the Markdown rendering SHALL add "the policy flags this region as outside its acceptable region".
- If a level is not in the policy's lists, then the entry SHALL carry the finding `level-not-in-policy` naming the level and no region.
- If a level is `unknown`, `not_assessed` or `not_applicable`, then the entry SHALL carry the finding `level-not-assessed` naming the marker and no region.
- If `--policy` is not supplied, then every entry SHALL carry the reason `policy-not-supplied` and no region.
- The command SHALL NOT substitute a level for a marker.
- The command SHALL NOT combine, rank, weight or average levels.
- The command SHALL NOT compare a post-control entry with a pre-control entry of the same situation.
- The entry SHALL carry no numeric score, rank or verdict key.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-146-AC-1 | Over a fixture of two pre-control and two post-control estimates, the section holds four entries ordered by estimate id, each with phase, entered levels and policy id. | Test |
| FR-146-AC-2 | An estimate whose levels are in the policy shows the region written in that policy cell, and the Markdown line reads "the policy marks this cell as" followed by that region name. | Test |
| FR-146-AC-3 | A region with `outside_acceptable: true` adds the "outside its acceptable region" wording, and a region with `false` does not. | Test |
| FR-146-AC-4 | A severity absent from the policy lists yields `level-not-in-policy` naming the level and no region. | Test |
| FR-146-AC-5 | A probability of `not_assessed`, `unknown` and `not_applicable` each yields `level-not-assessed` naming the marker, no region and no substituted level. | Test |
| FR-146-AC-6 | Omitting `--policy` yields `policy-not-supplied` for every entry and no region, and exits zero. | Test |
| FR-146-AC-7 | The JSON keys of every entry equal the set in this requirement exactly, with no numeric, rank or verdict key. | Test |
| FR-146-AC-8 | A pre-control and a post-control estimate of one situation each show their own cell and no key relates them. | Test |

## Dependencies

- **Upstream**: [FR-145](./FR-145-risk-advise-command.md); `agent-ix/spec-objects-safety` FR-101, FR-102, FR-203, FR-204.
- **Downstream**: none.
