---
id: SR-208
title: "Integrity review of PLAT-1436 dossier spec (quoin PR #924)"
type: SpecReview
analysis: integrity
scope: "agent-ix/quoin; PR #924 @ ab633975: FR-188..FR-192, NFR-039, US-024; context spec/stakeholder/StR-007, spec/stakeholder/*"
review_set: subset
relationships:
  - target: "ix://agent-ix/quoin/NFR-039"
    type: references
  - target: "ix://agent-ix/quoin/FR-190"
    type: references
  - target: "ix://agent-ix/quoin/FR-189"
    type: references
---

## Summary

Ticket: PLAT-1436. Checked the set for consistency, completeness, atomicity and
upward trace. Two requirements contradict each other, one rule has no
vocabulary to express its result, and the upward trace targets an unrelated
stakeholder requirement.

## Verdict

FAIL. FND-001 and FND-002 are contradictions between requirements and must be
fixed before PLAT-1614, PLAT-1437 and PLAT-1438 implement against them.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | NFR-039 requires all output ordered by requirement, criterion, element, then clause id, and AC-4 requires the same bytes from records supplied in a different order. FR-192 orders RMM tables by pack index order, or by command-line order of `--clause-binding`. So reordering two `--clause-binding` flags must both change the RMM (FR-192) and not change it (NFR-039-AC-4). | spec/non-functional/NFR-039-dossier-is-deterministic-and-records-only.md:25; spec/non-functional/NFR-039-dossier-is-deterministic-and-records-only.md:55; spec/functional/FR-192-dossier-rmm-content.md:35; spec/functional/FR-192-dossier-rmm-content.md:36 |
| FND-002 | high | FR-190 requires a module set with no `design_relations` to make the document `incomplete` "with that cause stated in `Gaps`". FR-188 makes `gap_kind` a closed list in which no kind names that cause. The generator must either break the closed list or misreport the cause under an unrelated kind. | spec/functional/FR-190-dossier-architecture-content.md:40; spec/functional/FR-188-dossier-command.md:59; spec/functional/FR-190-dossier-architecture-content.md:49 |
| FND-003 | medium | `unbacked_criterion` has two meanings. FR-189 uses it for a requirement with zero acceptance criteria (the subject is a requirement). FR-191 uses it for a criterion with no test tag (the subject is a criterion). A `--json` consumer cannot tell "no criteria written" from "criterion untested" by kind. | spec/functional/FR-189-dossier-srs-content.md:35; spec/functional/FR-191-dossier-trace-matrix-content.md:38 |
| FND-004 | medium | FR-189 lists StRs and raises `unbacked_criterion` for any requirement with zero acceptance criteria. quoin's StRs carry Validation Criteria (`StR-007-VC-1`), not ACs, and none of the eight files under `spec/stakeholder/` has an `AC-` id. Every StR therefore raises a gap, the SRS of this repository can never be complete, and the StR validation criteria are not rendered. | spec/functional/FR-189-dossier-srs-content.md:18; spec/functional/FR-189-dossier-srs-content.md:35; spec/stakeholder/StR-007-governed-assurance-portfolio.md:18 |
| FND-005 | medium | FR-188..FR-192 and US-024 `traces_to` StR-007, "Assurance owners inspect governed evidence without rerunning producers", whose need is lossless graph-evidence intake and portfolio views (satisfied_by FR-066/FR-067). Dossier generation is a different need and runs source commands. The upward trace is wrong, and no StR states the dossier need. | spec/functional/FR-188-dossier-command.md:6; spec/usecase/US-024-hand-a-reviewer-dossier-documents-generated-from-records.md:6; spec/stakeholder/StR-007-governed-assurance-portfolio.md:1 |
| FND-006 | medium | Gap order is "requirement id, then criterion id, then element id, then clause id", but `no_requirements` and `no_clause_binding` have no id, and `no_allocation` keys on a part. Their position is undefined. "id order" also never says whether it is ASCII byte order (which MatrixOutput uses) or numeric, so FR-99 against FR-100 is ambiguous. | spec/functional/FR-188-dossier-command.md:61; spec/non-functional/NFR-039-dossier-is-deterministic-and-records-only.md:25; spec/functional/FR-188-dossier-command.md:88 |
| FND-007 | medium | Exit 3 and FR-188-AC-9 depend on "a source command" failing, but the term is never defined. FR-190's sources are `quoin-quire` library calls, while FR-191 and FR-192 name CLI invocations. Whether the dossier runs subprocesses or in-process operations, and what "the report names that command" means for a library call, is unstated, so AC-9 has no defined trigger. | spec/functional/FR-188-dossier-command.md:71; spec/functional/FR-188-dossier-command.md:86; spec/functional/FR-190-dossier-architecture-content.md:25 |
| FND-008 | low | NFR-039 Scope says it applies to `generate` and `check`, but all four statements name only `quoin dossier generate`. `check`'s clock-free and ordering obligations are implied rather than stated. | spec/non-functional/NFR-039-dossier-is-deterministic-and-records-only.md:23; spec/non-functional/NFR-039-dossier-is-deterministic-and-records-only.md:33 |

## Dispositions

Disposition round 1, checked against PR #924 head 5bba8af4 (fix commits 196404ac
and 5bba8af4).

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | - |
| FND-002 | fixed | - |
| FND-003 | fixed | - |
| FND-004 | fixed | - |
| FND-005 | fixed | - |
| FND-006 | fixed | - |
| FND-007 | fixed | - |
| FND-008 | fixed | - |

- FND-001 after_excerpt (`spec/non-functional/NFR-039-dossier-is-deterministic-and-records-only.md:23,25,55`): `... SHALL give byte-identical Markdown for the same records and options, where the options include the order of `--clause-binding` and the index order of a pack.` / `... SHALL order output by the dossier order and gap order of FR-188, except where FR-192 keeps a supplied order.` / `| NFR-039-AC-4 | Fixture documents created in a different order on disk give the same bytes, and swapping two `--clause-binding` options changes only the order of the two RMM tables | Test |`
- FND-002 after_excerpt (`spec/functional/FR-190-dossier-architecture-content.md:44`): `- If the loaded modules declare no `design_relations`, the document SHALL record a `design_relations_unavailable` gap that states the cause.` (`design_relations_unavailable` is in FR-188's closed list at FR-188:91.)
- FND-003 after_excerpt (`spec/functional/FR-189-dossier-srs-content.md:38,42`): `- If a requirement has zero criteria in its criteria table, the SRS SHALL record a `no_acceptance_criteria` gap naming it.` / `- The SRS SHALL NOT record an `unbacked_criterion` gap, which is reserved for a criterion without a test tag (FR-191).`
- FND-004 after_excerpt (`spec/functional/FR-189-dossier-srs-content.md:24,39`): `- For an StR, the criteria are the rows of its `Validation Criteria` table.` / `- The SRS SHALL list the Validation Criteria of an StR as its criteria.`
- FND-005 after_excerpt (`spec/functional/FR-188-dossier-command.md:6-7`; the same edge in FR-189..FR-192 and US-024): `- target: "ix://agent-ix/quoin/StR-010"` / `type: "traces_to"`. New `spec/stakeholder/StR-010-reviewers-receive-documents-generated-from-records.md` states the dossier need.
- FND-006 after_excerpt (`spec/functional/FR-188-dossier-command.md:77,80`): `- Quoin SHALL compare ids in dossier order: an id is split at `-` into segments; two segments of ASCII digits compare by numeric value, ... `FR-99` sorts before `FR-100`, and `FR-100-AC-2` before `FR-100-AC-10`.` / `- A gap with no id (`no_requirements`, `design_relations_unavailable`, `no_clause_binding`) sorts before gaps that name an id.`
- FND-007 after_excerpt (`spec/functional/FR-188-dossier-command.md:69,71`): `- A source is one of the record readers named in FR-189 to FR-192: `requirements`, `design-join`, `design-matrix`, `systems-records`, `matrix` and `discharge`. A source is either a library operation of the Quoin workspace or the equivalent Quoin command, and gives the same result as that command.` / `- If a source fails, the command SHALL exit 3, SHALL name the failed source and its error in `source_failure`, and SHALL write nothing.`
- FND-008 after_excerpt (`spec/non-functional/NFR-039-dossier-is-deterministic-and-records-only.md:27`): `` `quoin dossier generate` and `quoin dossier check` SHALL NOT read the wall clock or the network. `` (all four statements now name both subcommands).
