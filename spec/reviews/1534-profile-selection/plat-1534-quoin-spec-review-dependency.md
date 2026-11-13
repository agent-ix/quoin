---
id: SR-20261113140012
title: "Dependency review of source profile selection relationships"
type: SpecReview
analysis: dependency
review_set: subset
scope: "agent-ix/quoin; PR #870 at 9e8997c3: relationships and Dependencies sections of spec/functional/FR-180, FR-181, FR-182, spec/non-functional/NFR-037"
---

# Dependency review of source profile selection relationships

## Summary

Ticket: PLAT-1534. Checked every new `relationships:` edge and every Dependencies line. Each in-repo target exists
(FR-004, FR-006, FR-029, FR-046, FR-180, FR-181, FR-182). The cross-repo target
`ix://agent-ix/spec-objects-security/FR-020` exists only in the unmerged spec-objects-security PR #48, and it is the
id that collides there with PR #49.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-180 `depends_on` `ix://agent-ix/spec-objects-security/FR-020`, an id the open PR #49 (PLAT-1426) also takes for "control mappings are clause references". If PR #49 merges first, this edge resolves to the wrong artifact. Merge order with the renumbering must keep this edge pointing at the profile format. | spec/functional/FR-180-source-profile-selection.md:10-11 |
| FND-002 | low | FR-181 `extends` FR-046, but it adds nothing to discharge. It produces discharge's binding inputs, and its own Dependencies section lists FR-046 as Downstream. Use `references`, or say what FR-181 extends. | spec/functional/FR-181-source-profile-evaluation.md:10-11, :72 |
| FND-003 | low | FR-182 specifies how a licensed pack joins a profile but has no edge to FR-137, which defines a pack (derived from `check_publishable`) and where it is installed. Add a `depends_on` FR-137 edge so a pack has one definition. | spec/functional/FR-182-local-pack-joins-a-profile.md:1-10 |

## Verdict

The edges are sound apart from the cross-repo id collision (FND-001) and two edge-type nits.

Examined (role examined): the frontmatter relationships and Dependencies of FR-180, FR-181, FR-182 and NFR-037.

## Dispositions

Round 1, against fix commit 6f68e3ed (quoin#870), with quoin#871 20e50a4a, spo#48 a69f437e and spo#49 d3658e0e as companion heads.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | spo#49 renumbered to FR-014..FR-018, so spo FR-020 is only the source profile format. |
| FND-002 | fixed | - |
| FND-003 | fixed | - |
