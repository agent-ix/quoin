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
  - target: "ix://agent-ix/spec-objects-safety/FR-103"
    type: "depends_on"
  - target: "ix://agent-ix/spec-objects-safety/FR-203"
    type: "depends_on"
  - target: "ix://agent-ix/spec-objects-safety/FR-204"
    type: "depends_on"
  - target: "ix://agent-ix/spec-objects-safety/FR-305"
    type: "references"
---

# FR-146: Risk advice policy cells: the cell each risk estimate lands in under the authored policy

## Description

Tracked by Linear PLAT-1651 and PLAT-1652. The `policy_cells` section of `quoin risk advise` (FR-145) shows,
for every risk estimate, the policy cell its entered severity and probability land in. The region is a lookup of
a cell a person wrote (`agent-ix/spec-objects-safety` FR-102); the report computes no score and combines no levels.

`quoin risk-summary` (`agent-ix/spec-objects-safety` FR-305) already shows each estimate's `policy_region`. This
section shows the same region and adds the cell itself, the policy it was read from, the region's
`outside_acceptable` flag, and the finding that says why an estimate has no region.

### One policy source

A residual risk references one post-control estimate and one policy (`agent-ix/spec-objects-safety` FR-103). The
policy of an estimate is therefore:

- for a post-control estimate, the policy of the residual risk that references it;
- for a pre-control estimate, the policy of the residual risk that references a post-control estimate of the same situation.

If no such residual risk exists, the estimate has no policy. If the residual risks found reference more than one
policy, the estimate has no single policy. The command takes no policy argument.

In this requirement "the acceptable region" and "outside the acceptable region" are as `agent-ix/spec-objects-safety`
FR-101 defines them: the regions whose `outside_acceptable` flag is `false`, and the regions whose flag is `true`. The `quality:ambiguous-term` lint on the quoted output string in the Behavior section is a known false positive: the string repeats that defined term and the tool applies no meaning of its own.

## Behavior

- The command SHALL list one entry for every `risk_estimate` in scope, pre-control and post-control, ordered by estimate id.
- If the estimate has a policy and both levels are members of that policy's level lists, then the entry SHALL carry the cell, the region name the policy lists for it, and that region's `outside_acceptable` flag as the policy wrote it.
- The Markdown rendering of a resolved entry SHALL read "the policy marks this cell as <region>".
- If the region's flag is `true`, then the Markdown rendering SHALL add "the policy flags this region as outside its acceptable region".
- If a level is not in the policy's lists, then the entry SHALL carry the finding `level-not-in-policy` naming the level and no region.
- If a level is `unknown`, `not_assessed` or `not_applicable`, then the entry SHALL carry the finding `level-not-assessed` naming the marker and no region.
- If the estimate has no policy, then the entry SHALL carry the finding `policy-not-found` and no region.
- If the estimate has no single policy, then the entry SHALL carry the finding `policy-ambiguous` naming the policies and no region.
- The command SHALL NOT substitute a level for a marker.
- The command SHALL NOT combine, rank, weight or average levels.
- The command SHALL NOT compare a post-control entry with a pre-control entry of the same situation.
- Where the same estimate has a `policy_region` in `quoin risk-summary`, the entry's region SHALL equal it.

### Entry keys

Every key is present, `null` or `[]` when empty.

| Key | Content |
|---|---|
| `estimate` | the estimate id |
| `phase` | `pre_control` or `post_control` |
| `severity` | as entered |
| `probability` | as entered |
| `policy` | the policy id, or `null` |
| `cell` | `{ severity, probability }` of the matched cell, or `null` |
| `region` | the policy's region name, or `null` |
| `outside_acceptable` | the region's flag as the policy wrote it, or `null` |
| `findings` | the finding tokens above that name this entry |

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-146-AC-1 | Over a fixture of two pre-control and two post-control estimates, the section holds four entries ordered by estimate id, each with exactly the keys of the entry key table. | Test |
| FR-146-AC-2 | An estimate whose levels are in its policy shows the region written in that policy cell, and the Markdown line reads "the policy marks this cell as" followed by that region name. | Test |
| FR-146-AC-3 | A region with `outside_acceptable: true` adds the "outside its acceptable region" wording, and a region with `false` does not. | Test |
| FR-146-AC-4 | A severity absent from the policy lists yields `level-not-in-policy` naming the level and no region. | Test |
| FR-146-AC-5 | A probability of `not_assessed`, `unknown` and `not_applicable` each yields `level-not-assessed` naming the marker, no region and no substituted level. | Test |
| FR-146-AC-6 | A post-control estimate with no referencing residual risk yields `policy-not-found`, and one referenced by two residual risks on different policies yields `policy-ambiguous`; neither shows a region. | Test |
| FR-146-AC-7 | A pre-control estimate takes the policy of the residual risk on a post-control estimate of its situation, and shows the same policy id. | Test |
| FR-146-AC-8 | The keys of every entry equal the entry key table, with no numeric, rank or verdict key. | Test |
| FR-146-AC-9 | A pre-control and a post-control estimate of one situation each show their own cell and no key relates them. | Test |
| FR-146-AC-10 | For each fixture estimate, the region equals the `policy_region` that `quoin risk-summary` shows for it. | Test |

## Dependencies

- **Upstream**: [FR-145](./FR-145-risk-advise-command.md); `agent-ix/spec-objects-safety` FR-101, FR-102, FR-103, FR-203, FR-204.
- **Context only**: `agent-ix/spec-objects-safety` FR-305.
- **Downstream**: [FR-148](./FR-148-risk-advise-decision-queue.md).
