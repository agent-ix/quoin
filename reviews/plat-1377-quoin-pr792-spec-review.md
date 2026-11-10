---
id: SR-20261103118156
title: "Spec review of quoin PR 792 (FR-070, FR-071, FR-078 and template FR-001 edits)"
type: SpecReview
analysis: base
scope: "agent-ix/quoin@3b70d3a1; spec/functional/FR-070-semantic-module-manifest-extension.md, spec/functional/FR-071-typed-properties-mapping.md, spec/functional/FR-078-generated-manifest-semantic-block.md, spec/functional/FR-076-semantic-module-template-variants.md, spec/non-functional/NFR-017-non-disruptive-manifest-evolution.md, templates/semantic-module/{{cookiecutter.repo_name}}/spec/functional/FR-001-declare-the-semantic-contract.md"
review_set: subset
---

# Spec review of quoin PR 792

## Summary

Ticket: PLAT-1377 (also PLAT-278 and PLAT-1221). The review covers the four spec files in
`git diff origin/main...HEAD` at 3b70d3a1. EARS conformance of the edited statements was
checked inline; no other sub-analysis applies to edits of this size. `quire validate` (quire
0.39.0, engine 0.55.0) passes on the three quoin FRs.

The edits were checked against quire-rs `origin/main` FR-069 ("A module declares no
semantic-core version; the loader ... SHALL NOT refuse a module over a `semantic_core` value")
and FR-031. FR-031 rejects retired archetype fields with no tolerate path, but FR-069 covers
`semantic_core` explicitly, and quoin follows FR-069.

## Verdict

**PASS with one low finding.** All of the following are consistent with the code and with
quire-rs FR-069:

- the FR-070 key list (nine admitted keys)
- FR-070-AC-2 (the retired key loads)
- the FR-078 behaviour line and AC-1 (eight rendered keys, no `semantic_core`)
- the template FR-001 (eight keys)
- the deletion of the FR-071 PLAT-278 verification note, whose expiry condition (pin bumped,
  `FieldDecl`/`TypeRef`/`ConstraintDecl` checks added) is met in this PR

The `BLOCKED-BY PLAT-1221` notes are gone everywhere in the diff, which is correct now that the
blocker is closed.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | The FR-070 bullet "SHALL admit exactly these keys" now carries a second normative behaviour in a parenthetical: the retired `semantic_core` loads, and Quoin removes it before validation and never requires, reads or checks it. The statement is compound. Read next to "When `quoin module install` meets an unknown key inside `semantic`, Quoin SHALL reject the manifest", it leaves `semantic_core` both outside the admitted set and not rejected. Split it into its own EARS bullet, for example "If the `semantic` block carries `semantic_core`, then Quoin SHALL ignore it and validate the block without it". | spec/functional/FR-070-semantic-module-manifest-extension.md:35 |

## Examined scope

- FR-070 Behavior, key-list bullet (FND-001).
- FR-070-AC-2. Clean.
- FR-071 note deletion. Clean.
- FR-078 Behavior, `semantic_core` bullet. Clean.
- FR-078-AC-1. Clean.
- Template FR-001 Behavior bullet and FR-001-AC-1. Clean.

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-002 | low | The amended NFR-017-AC-4 says "No `required` array that the pre-CR-003 manifest schema carried gains or loses a key ... so a manifest that validated before still validates". semantic-schema 0.4.2 breaks that for `lint_rules`: the pre-CR-003 `/$defs/LintRuleEntry/properties/pattern` `required: [kind]` array is gone (`pattern` is now a string), and `LintRuleEntry` gains a root `required: [type, id, section]`. `tc_452_641` records this honestly as accepted upstream evolution, but the AC that test backs claims it cannot happen. Name the `LintRuleEntry` exception (semantic-schema 0.4.2, accepted upstream evolution) in the AC so the spec matches its test. | spec/non-functional/NFR-017-non-disruptive-manifest-evolution.md:63 |

## Dispositions

Disposition pass 1 checked `git diff 3b70d3a1..5a33c2ff`. In that diff:

- FR-070 now refuses the retired key, which matches the owner's no-shim decision.
- FR-070-AC-2 and AC-3 match the code and `tc_452_612`.
- The FR-076 target bullet and AC-8 match `tc_264_008`.
- `quire validate` passes on FR-070, FR-076 and NFR-017.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | The retired-key rule is now its own bullet: "The `semantic` block SHALL NOT admit `semantic_core`: the key is retired (PLAT-1221), a module declares no semantic-core version, and a manifest that still carries it is refused like any other unknown key." The admitted-keys bullet no longer carries a parenthetical. |
