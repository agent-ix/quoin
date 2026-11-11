---
id: SR-16322
title: "Dependency analysis of the semantic-module template lineage edges in quoin PR 775"
type: SpecReview
analysis: dependency
scope: "agent-ix/quoin@b763f79; templates/semantic-module/{{cookiecutter.repo_name}}/spec/**"
review_set: subset
---

# Dependency analysis of the semantic-module template lineage edges

## Summary

Ticket: PLAT-346. Scoped to the `relationships:` edges in the semantic-module template spec
after quoin PR 775 at b763f79. Tools: quoin 0.29.1, quire 0.36.2 (engine 0.50.2).

The rendered template's edges now read: US-001 `traces_to` StR-001; StR-001 `satisfied_by`
its FRs; FR-001, FR-002 and FR-003 `satisfies` US-001; NFR-001 `constrains`. Every lineage
edge uses the spec-artifacts-iso FR-004 lineage verbs, no edge uses `implements`, and no
edge points at an artifact that is not in the template. The rendered spec validates with
`quire validate` (exit 0). No cycle, no dangling target, no enablement work mixed into the
feature FRs.

## Verdict

**PASS.** No dependency defects in the edited edges.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Examined scope

```yaml
clean: true
scope:
- id: FR-001
  path: "templates/semantic-module/{{cookiecutter.repo_name}}/spec/functional/FR-001-declare-the-semantic-contract.md"
  role: examined
  excerpt: "target: ix://{{ cookiecutter.org }}/{{ cookiecutter.repo_name }}/US-001, type: satisfies"
- id: FR-002
  path: "templates/semantic-module/{{cookiecutter.repo_name}}/spec/functional/FR-002-emit-schemas-deterministically.md"
  role: examined
  excerpt: "target: ix://{{ cookiecutter.org }}/{{ cookiecutter.repo_name }}/US-001, type: satisfies"
- id: FR-003
  path: "templates/semantic-module/{{cookiecutter.repo_name}}/spec/functional/FR-003-authoring-forms-and-fixtures.md"
  role: examined
  excerpt: "target: ix://{{ cookiecutter.org }}/{{ cookiecutter.repo_name }}/US-001, type: satisfies"
- id: StR-001
  path: "templates/semantic-module/{{cookiecutter.repo_name}}/spec/stakeholder/StR-001-typed-declarations-consumers-can-read.md"
  role: context_only
  excerpt: "type: satisfied_by"
- id: US-001
  path: "templates/semantic-module/{{cookiecutter.repo_name}}/spec/usecase/US-001-declare-a-type-once.md"
  role: context_only
  excerpt: "type: traces_to"
```
