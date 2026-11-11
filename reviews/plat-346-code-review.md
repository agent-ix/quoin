---
id: SR-16321
title: "Code review of quoin PR 775 (requirement lineage as satisfies)"
type: SpecReview
analysis: base
scope: "agent-ix/quoin@b763f79; templates/semantic-module/{{cookiecutter.repo_name}}/spec/functional/FR-001..003, skills/specify/references/us-to-fr.md"
review_set: subset
---

# Code review of quoin PR 775

## Summary

Ticket: PLAT-346. Scoped to `git diff origin/main...HEAD` at b763f79: three semantic-module
template FRs change their US-001 edge from `implements` to `satisfies`, and the specify skill's
US-to-FR reference now says `type: satisfies`. Vocabulary authority: spec-artifacts-iso FR-004
(`implements` = interface/contract fulfilment; `satisfies`/`satisfied_by` = requirement lineage).
Tools: quoin 0.29.1, quire 0.36.2 (engine 0.50.2).

Checks run: the template was rendered with `cookiecutter --no-input` and every rendered
`spec/**/*.md` passed `quire validate` (exit 0); the three changed template files also pass
`quire validate` in place. No test or fixture reads `templates/semantic-module` relationship
types (the `tc_446`/`tc_452`/quoin-semantic tests read `tests/fixtures/semantic-module`, a
different tree; the template's own pytest files carry only trace marks), so no focused test
applies and none needed updating. quoin's graph consumers already accept `satisfies`
(`quoin-assurance` `DOWNWARD_FROM_CHILD`, `quoin-graph-analysis` `DEFAULT_RELATION_KINDS`).
quoin's own `spec/` FRs that still use `implements` for FR to US are the later per-repo
migration and are out of scope.

## Verdict

**PASS WITH FINDINGS (one medium, one low).** The diff itself is correct and complete for
the template and the US-to-FR reference. Two other skill files in quoin still steer agents
toward `implements` for requirement lineage.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The relationship-verb reference says authors "must use one of the following strict semantic primitives", and the list has no `satisfies`/`satisfied_by` (nor `traces_to`, `refines`, `constrains`, which the template itself uses). The only verb that reads like FR to US lineage is `implements`, so an agent following this guide picks the verb this PR retires | skills/spec-object-review/references/object-type-guide.md:89-110 |
| FND-002 | low | The specify skill's scoping examples phrase the FR to US link as "the FR that implements US-003" and "the requirement that implements it", in the same skill whose reference this PR corrects to `satisfies` | skills/specify/SKILL.md:21-22 |

## Examined scope

```yaml
clean: false
scope:
- id: templates/semantic-module/FR-001
  path: "templates/semantic-module/{{cookiecutter.repo_name}}/spec/functional/FR-001-declare-the-semantic-contract.md"
  role: examined
  excerpt: "relationships: - target: \"ix://{{ cookiecutter.org }}/{{ cookiecutter.repo_name }}/US-001\" type: \"satisfies\""
- id: templates/semantic-module/FR-002
  path: "templates/semantic-module/{{cookiecutter.repo_name}}/spec/functional/FR-002-emit-schemas-deterministically.md"
  role: examined
  excerpt: "relationships: - target: \"ix://{{ cookiecutter.org }}/{{ cookiecutter.repo_name }}/US-001\" type: \"satisfies\""
- id: templates/semantic-module/FR-003
  path: "templates/semantic-module/{{cookiecutter.repo_name}}/spec/functional/FR-003-authoring-forms-and-fixtures.md"
  role: examined
  excerpt: "relationships: - target: \"ix://{{ cookiecutter.org }}/{{ cookiecutter.repo_name }}/US-001\" type: \"satisfies\""
- id: skills/specify/references/us-to-fr.md
  path: skills/specify/references/us-to-fr.md
  role: examined
  excerpt: "link it back to the story via frontmatter relationships: (type: satisfies, target the US-XXX)."
- id: skills/spec-object-review/references/object-type-guide.md
  path: skills/spec-object-review/references/object-type-guide.md
  role: examined
  excerpt: "When linking specs, you must use one of the following strict semantic primitives: ... - implements: Interface/contract fulfillment."
- id: skills/specify/SKILL.md
  path: skills/specify/SKILL.md
  role: examined
  excerpt: "- \"Add the FR that implements US-003\" -> one FR (traced to US-003)."
- id: templates/semantic-module/StR-001
  path: "templates/semantic-module/{{cookiecutter.repo_name}}/spec/stakeholder/StR-001-typed-declarations-consumers-can-read.md"
  role: context_only
  excerpt: "type: \"satisfied_by\""
```

## Dispositions

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | 4072c8d9 points the verb list at spec-artifacts-iso FR-004 `edge_types` and adds a Traceability & Lineage group with `satisfies`/`satisfied_by`, `traces_to`, `derives_from`, `constrains`, `verifies`, `exercises`, `supersedes`; `implements` now states it is not lineage. Every listed verb, and `relates_to` (replacing the non-existent `related_to`), was checked against spec-artifacts-iso origin/main `spec_artifacts_iso/manifest.yaml`; `satisfies` is the declared inverse of `satisfied_by` |
| FND-002 | fixed | 4072c8d9 changes both scoping examples to "the FR that satisfies US-003" and "the requirement that satisfies it" |
