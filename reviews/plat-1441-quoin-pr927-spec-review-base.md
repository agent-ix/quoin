---
id: SR-20261115183513
title: "Spec review (base) — PLAT-1441 FR-121 frontmatter-owned record keys"
type: SpecReview
analysis: spec-review
scope: "agent-ix/quoin; spec/functional/FR-121-record-key-sections-mapping.md; PLAT-1441"
review_set: subset
---

## Summary

Ticket: PLAT-1441. Base spec review of the FR-121 amendment in quoin PR #927
(`0799418e`). It checks the requirement for quality, consistency and completeness
against the ticket's intent and against quire-rs FR-069/FR-081 as amended in
quire-rs#688. The structural contradictions with the existing FR-121 rules are
recorded in the code-review artifact for this PR. They are not repeated here.

## Verdict

**CONDITIONAL.** The ownership rule is well-formed and key-agnostic. It names no
regulated-data vocabulary, which is right for a Quoin requirement. Before
fixtures are authored against it, the interaction with FR-069's duplicate-key
rule and the scope of the locator declaration need to be stated.

## Coverage

Examined: FR-121 Rationale (lines 45-52), Behavior lines 80 and 85-88,
FR-121-AC-10, AC-11, AC-12. Context: quire-rs FR-069 "Frontmatter-owned record
keys" and FR-069-AC-38 (`semantic.duplicate-record-key`), quire-rs FR-081-AC-18
in quire-rs#688.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The both-form rule (line 87, AC-11) does not say whether the ignored section is a source for quire-rs FR-069's `semantic.duplicate-record-key` error, which FR-069 applies when a section and frontmatter supply the same key and which then omits the key. An implementer following both specs can emit that error and drop the frontmatter value, the opposite of "frontmatter wins". AC-11 asserts neither the absence of that error nor that validation passes, though quire-rs FR-081-AC-18 asserts that it passes. | spec/functional/FR-121-record-key-sections-mapping.md:87 |
| FND-002 | low | Line 85, "A `frontmatter_field` locator declared for a record key", does not say where the locator is declared: the object type's `yield_pattern.match`, per quire-rs FR-069. It also does not say that ownership, like every rule here, applies only under the `record-sections` token (line 78). Without the token, a section-only key yields no diagnostic. | spec/functional/FR-121-record-key-sections-mapping.md:85 |
| FND-003 | low | The Rationale still says "Lifecycle keys are authored in body sections, not in frontmatter. Frontmatter is the artifact identity and graph surface". It does not explain why a module may now make a record key frontmatter-owned, or why such a key loses the body section's source locus. | spec/functional/FR-121-record-key-sections-mapping.md:45-47 |
