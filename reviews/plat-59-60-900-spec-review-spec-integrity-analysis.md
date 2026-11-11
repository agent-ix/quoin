---
id: SR-16331
title: "Spec review of FR-070, FR-072, FR-079, FR-003 and matrix edits in quoin PR 774"
type: SpecReview
analysis: integrity
scope: "agent-ix/quoin@93b3504; spec/functional/FR-070-semantic-module-manifest-extension.md; spec/functional/FR-072-invariants-and-operations-mapping.md; spec/functional/FR-079-generated-skeletons-and-fixtures.md; spec/matrix.md; templates/semantic-module/{{cookiecutter.repo_name}}/spec/functional/FR-003-authoring-forms-and-fixtures.md; templates/semantic-module/{{cookiecutter.repo_name}}/spec/spec.md; tests/fixtures/semantic-module/mapping/README.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quoin/FR-070
    type: reviews
  - target: ix://agent-ix/quoin/FR-072
    type: reviews
---

# Spec review of quoin PR 774

## Summary

Ticket: PLAT-59, PLAT-60, PLAT-900. Integrity review of the spec hunks: each edited statement
checked against its body, its AC table and its matrix rows. Tools: quoin 0.29.1, quire 0.36.2
(engine 0.50.2). `quire validate --scope .` on FR-070, FR-072, FR-079 and matrix.md exits 0.
FR-003 (rendered-template spec) validates with `--scope` set to the template root. The template's
`spec/spec.md` fails structural validation unrendered (the Jinja frontmatter does not parse), which
also fails on origin/main, so the failure is not from this diff.

FR-079 Behavior, Rationale and AC-3, FR-003 Behavior and AC-3, the template spec.md scope line,
TC-1423 and the FR-079 matrix row all say `quire` consistently. FR-072-AC-7 and TC-1715 now cite
only `fence-ocl-carried`. FR-072 Behavior already covers the `quire` path (no finding, not lossy).
The FR-070 deletion removes the filament-core-service delegation bullet, and no other
delegation or vendoring sentence remains in FR-070.

## Verdict

**PASS WITH LOW FINDINGS.**

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | After the deletion, FR-070's next bullet "Quoin SHALL take that schema from the published `@agent-ix/semantic-schema` package" has no antecedent. The preceding bullet names no schema. Name the module-manifest JSON Schema (it also restates FR-070-CON-2) | spec/functional/FR-070-semantic-module-manifest-extension.md:37 |
| FND-002 | low | FR-070 Dependencies still list `agent-ix/filament-core-service#21` as upstream, though the bullet that made filament-core-service the schema owner is now deleted | spec/functional/FR-070-semantic-module-manifest-extension.md:75 |
| FND-003 | low | The README row the PR edits still traces the config-version files to FR-072-AC-7, but AC-7 now covers only the `ocl` `fence-ocl-carried` case. The `quire` clause in config-version matches the FR-072 `quire` Behavior bullet / AC-1 shape, not AC-7 | tests/fixtures/semantic-module/mapping/README.md:10 |

## Examined scope

```yaml
clean: false
scope:
- {id: FR-070, path: spec/functional/FR-070-semantic-module-manifest-extension.md, role: examined, excerpt: "Quoin SHALL take that schema from the published `@agent-ix/semantic-schema` package at the version its lockfile records, and the Quire output schemas from the linked `quire-rs` engine (FR-029)."}
- {id: FR-072-AC-7, path: spec/functional/FR-072-invariants-and-operations-mapping.md, role: examined, excerpt: "An `ocl` fence under `### immutable` has an expected `ClauseRef { language: ocl, clauseId: immutable }` ... under semantic-core `0.2.0` (`operations-cases.json` `fence-ocl-carried`)."}
- {id: FR-079-AC-3, path: spec/functional/FR-079-generated-skeletons-and-fixtures.md, role: examined, excerpt: "Every rendered skeleton carries at least one `quire` fence under its own `### <clauseId>` heading."}
- {id: FR-003-AC-3, path: "templates/semantic-module/{{cookiecutter.repo_name}}/spec/functional/FR-003-authoring-forms-and-fixtures.md", role: examined, excerpt: "Every skeleton carries at least one `quire` clause under its own heading, with distinct clause ids."}
- {id: TC-1715, path: spec/matrix.md, role: examined, excerpt: "The `fence-ocl-carried` case (semantic-core `0.2.0`) carries the `ocl` body verbatim with one `semantic.clause-language-unchecked` advisory at the fence and a lossy clauses kind"}
- {id: TC-1423, path: spec/matrix.md, role: examined, excerpt: "Every rendered skeleton carries at least one `quire` fence under its own `### <clauseId>` heading"}
```

## Dispositions

Round 1, checked against fix commit d53395c6 (head 0cd7173). `quire validate --scope .` on FR-070 and matrix.md exits 0.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | FR-070 bullet names the module-manifest JSON Schema |
| FND-002 | fixed | `agent-ix/filament-core-service#21` removed from FR-070 Upstream |
| FND-003 | fixed | README row traces config-version to FR-072's `quire` clause Behavior |
