---
id: SR-20261113140425
title: "Dependency review of typed control and clause-set output across quoin#871 and spec-objects-security#49"
type: SpecReview
analysis: dependency
review_set: subset
scope: "agent-ix/quoin; PR #871 (20e50a4a) FR-171 to FR-174 and NFR-038 relationships and Dependencies, checked against agent-ix/spec-objects-security PR #49 (d3658e0) FR-014 to FR-018 and NFR-003, PR #48 FR-020 and FR-021, quire-rs FR-073, FR-083 and FR-091 on main, and tickets PLAT-1521 and PLAT-1522"
---

# Dependency review of typed control and clause-set output across quoin#871 and spec-objects-security#49

## Summary

Ticket: PLAT-1426. Every `relationships` edge and Dependencies line in the quoin diff was resolved to its target.
The upstream quoin targets FR-046, FR-133, FR-135, FR-137 and FR-138 exist and say what they are cited for. The
targets in spec-objects-security#49 also exist and agree: FR-014 defines the `{ set, clause }` mapping in the
`authority/id@version` form, which quoin already renders (quoin-modules pack.rs:29); FR-017 defines the
`scope_exclusion` with `set`, `clause`, `owner` and `rationale`; FR-018 defines the mapped-and-excluded error
that FR-173 also refuses. The `ControlEffectiveness` enum (`not_assessed`, `ineffective`, `partially_effective`,
`effective`) matches FR-173. PLAT-1521 and PLAT-1522 put the sets in "spec-objects-security (or the module the
PLAT-1426 spec names)", so the home FR-172 names agrees with them. The tickets fix no key, so the keys in
FR-172 contradict nothing. Neither PR reuses an id of the PLAT-1534 PRs: #49 uses FR-014 to FR-018 and NFR-003,
and #48 uses FR-020, FR-021 and NFR-002.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Companion #49, FR-015: quire-rs FR-083 does not supply the resolution FR-015 relies on. FR-083 specifies only the manifest `clause_sets` key and `check_publishable`. The registry does expose loaded sets (`Registry::clause_sets`, quire-rs src/registry.rs:436), but no quire-rs FR and no code lets a module declare a rule that resolves a record field against a loaded clause set. The same is true of the duplicate-row rule (FR-015) and the cross-artifact pair rule (FR-018). FR-091 covers only ordered-field, required-field and vocabulary findings. FR-015's and FR-018's "Quire SHALL report" therefore needs a quire-rs requirement that no ticket allocates. | spec-objects-security spec/functional/FR-015-mapping-resolution-and-duplicates.md:24-28; spec/functional/FR-018-mapped-and-excluded-contradiction.md:23-24 |
| FND-002 | medium | Only quoin FR-172 decides the module home of the NIST and ASVS sets. Companion #49 adds no spec-objects-security requirement that the module declares `clause_sets` (quire-rs FR-083) or ships those sets with rights metadata. FR-172-AC-1 and AC-2 verify content of another repository from quoin's spec. Add the owning FR in spec-objects-security, or point FR-172 at PLAT-1521 and PLAT-1522 as the place that requirement will live. | spec/functional/FR-172-clause-set-sources-and-licence-boundary.md:20-25,47-48 |
| FND-003 | medium | The rights posture conflicts with #48. FR-172 lets the NIST and ASVS sets carry text "as that licence allows" and tests publishability with `check_publishable`, which also accepts `explicitly_cleared` text. #48 FR-021-AC-3 admits only `text: none` or `original` to a shipped profile. Public-domain NIST text or CC BY-SA ASVS text would be `explicitly_cleared`, not `original` (authored here). Such a set would pass FR-172 but could not be listed in `web-app-baseline`. #48 also calls these sets "pack work", while under FR-137 a module with no restricted set is not a pack. Fix the `rights.text` value in FR-172, or align #48. | spec/functional/FR-172-clause-set-sources-and-licence-boundary.md:22-25 |
| FND-004 | low | FR-173's Dependencies name spec-objects-security FR-017 and FR-018, but its `relationships` reference only FR-014. Add `references` edges so the trace graph shows the exclusion and contradiction rules it consumes. | spec/functional/FR-173-facts-from-mappings.md:5-11,63 |

## Verdict

The quoin side's own edges are sound. The author's claim that "quire-rs FR-083 clause-set resolution exists as
FR-015 requires" did not hold up: FR-083 exists, but it specifies publishability, not resolution against loaded
sets (FND-001). That finding blocks #49 rather than #871. FND-002 and FND-003 are cross-PR consistency gaps to
settle before either PR lands.

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | spo#49 at f61e87f drops the Quire-side resolution, duplicate and contradiction rules; FR-014 assigns them to quoin FR-173, which now refuses each case (FR-173-AC-5). No quire-rs capability is needed. |
| FND-002 | fixed | spo#49 adds FR-017, under which the module declares `nist/sp800-53@5.2.0` and `owasp/asvs@5.0.0` in `clause_sets` with rights metadata. FR-172 references it. |
| FND-003 | fixed | FR-172 and spo FR-017 both admit `none`, `original` or `explicitly_cleared`, and merged spo FR-021-AC-3 now defers to `quire publish-check`, which accepts all three. |
| FND-004 | fixed | FR-173 relationships now reference spo FR-014 and FR-016 (the renumbered scope_exclusion). The contradiction rule FR-018 is gone from #49. |
