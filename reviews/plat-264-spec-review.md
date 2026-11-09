---
id: SR-17072
title: "Spec review of quoin PR 782 (FR-076, FR-078, FR-083 edits)"
type: SpecReview
analysis: base
scope: "agent-ix/quoin@c73b0123; spec/functional/FR-076-semantic-module-template-variants.md, spec/functional/FR-078-generated-manifest-semantic-block.md, spec/functional/FR-083-template-render-self-tests.md and the documents that reference them"
review_set: subset
---

# Spec review of quoin PR 782

## Summary

Ticket: PLAT-264. Reviewed the changed FR-076, FR-078 and FR-083 text for consistency,
completeness, statement-to-AC agreement and dangling references after the deletion of
FR-076-CON-3 and the two FR-083 drift bullets. Integrity and EARS-statement checks were run
inline. No other sub-analysis applies to a deletion and a one-word narrowing. Tools: quoin
0.29.1, quire 0.36.2 (engine 0.50.2).

Dangling-reference sweep (`FR-076-CON-3`, `TC-1456`, `maintained_modules`, `drift_exemptions`,
`publication_keys`, "maintained ... repositories"): `spec/matrix.md` TC-1456 is the only
dangling row in the computed-matrix table. It is owned by PR #774 and left alone here.
Historical SpecReviews under `reviews/` and `spec/reviews/` record their own point in time and
were not counted. The other live references are in the findings below.

## Verdict

**CHANGES REQUESTED (one medium, two low).** The FR-078 change to "declared exact version"
and the FR-083 Rationale rewrite are consistent and correct. Deleting FR-076-CON-3 is right:
the rendered driver is templated, so a byte-identity check against the template could never
pass. The deletion of the drift requirement is incomplete upward: StR-008 still demands it.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | StR-008-VC-3 still requires "A required surface that a maintained module repository carries and the template omits is reported by a conformance gate". The two FR-083 bullets that realised it are deleted, so the stakeholder criterion now has no FR behind it and demands the ceremony this PR removes. Rewrite it to the contract-named surfaces (what FR-083-AC-4 checks) or withdraw it | spec/stakeholder/StR-008-conforming-module-repositories-by-construction.md:45 |
| FND-002 | low | FR-076's Inputs and Behavior still say "`<org>/<repo>@<version>`" while FR-076-AC-7 now requires the exact `<x.y.z>` form and refuses `^1.2.0`. The AC goes beyond its statement, and two implementers would read `<version>` differently. The AC also embeds its rationale ("because the module-manifest schema admits exact versions only"), which belongs in the description | spec/functional/FR-076-semantic-module-template-variants.md:35,55,88 |
| FND-003 | low | The active plan still cites the deleted constraint and the deleted drift check. TASK-045 Notes say "Keeping every rendered copy byte-identical to the template's is FR-076-CON-3". plan.md checks FR-083 off as "fails on drift from the maintained repositories as checked out" and lists drift in G5 and TASK-050 | plan/PLAN-008-semantic-module-cookiecutter/tasks/TASK-045-template-core-and-input-contract.md:67; plan/PLAN-008-semantic-module-cookiecutter/plan.md:59,78,97 |

## Examined scope

```yaml
clean: false
scope:
- {id: FR-076-AC-7, path: spec/functional/FR-076-semantic-module-template-variants.md, role: examined, excerpt: "`imported_modules: [\"agent-ix/spec-objects-business\"]` and `[\"agent-ix/spec-objects-business@^1.2.0\"]` abort naming the entry; the exact `<org>/<repo>@<x.y.z>` form renders, because the module-manifest schema admits exact versions only."}
- {id: FR-076, path: spec/functional/FR-076-semantic-module-template-variants.md, role: examined, excerpt: "If an entry of `imported_modules` is not of the `<org>/<repo>@<version>` form, then the template SHALL abort rendering naming the entry."}
- {id: FR-076-CON-1, path: spec/functional/FR-076-semantic-module-template-variants.md, role: examined, excerpt: "The template SHALL contain no copy of the schema emitter, the Quire runtime, or the semantic-core grammar; each is reached as a versioned dependency."}
- {id: FR-078, path: spec/functional/FR-078-generated-manifest-semantic-block.md, role: examined, excerpt: "When `imported_modules` is non-empty, the rendered `semantic.imports` SHALL map each imported package identity to its declared exact version."}
- {id: FR-078-AC-5, path: spec/functional/FR-078-generated-manifest-semantic-block.md, role: examined, excerpt: "A mixed-variant rendering with two imported modules yields two `semantic.imports` entries carrying the declared exact versions; an object-variant rendering with none yields `imports: {}`."}
- {id: FR-083, path: spec/functional/FR-083-template-render-self-tests.md, role: examined, excerpt: "A conformance contract that names the required surfaces turns a dropped surface into a failing test in this repository rather than a discovery in the next module."}
- {id: FR-083-AC-4, path: spec/functional/FR-083-template-render-self-tests.md, role: examined, excerpt: "Removing a surface the conformance contract names fails the gate naming that surface."}
- {id: StR-008-VC-3, path: spec/stakeholder/StR-008-conforming-module-repositories-by-construction.md, role: examined, excerpt: "A required surface that a maintained module repository carries and the template omits is reported by a conformance gate rather than discovered during the next migration."}
- {id: TC-1456, path: spec/matrix.md, role: context_only, excerpt: "| TC-1456 | The rendered emit driver is byte-identical to the template's copy in every variant | Property | P1 | FR-076-CON-3 |"}
- {id: TASK-045, path: plan/PLAN-008-semantic-module-cookiecutter/tasks/TASK-045-template-core-and-input-contract.md, role: examined, excerpt: "Keeping every rendered copy byte-identical to the template's is FR-076-CON-3; extracting it into a versioned package is separate work."}
```
