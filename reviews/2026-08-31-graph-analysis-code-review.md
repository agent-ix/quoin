---
id: SR-106
title: "Code review of read-only evidence graph analysis"
type: SpecReview
analysis: code-review
scope: "PLAN-004; FR-062; quire-rs assurance-v1 at 3fe2c7e0e9de445af290603c3728857803b61183"
review_set: subset
relationships:
  - target: "ix://agent-ix/quoin/PLAN-004"
    type: reviews
  - target: "ix://agent-ix/quoin/FR-062"
    type: references
---

# Code review of read-only evidence graph analysis

## Summary

The implementation replaces the draft frontmatter/Quire-subprocess graph with a schema-validated
consumer of quire-rs #386's committed assurance-v1 contract. The review covered the three command
paths, input and identity validation, indexed graph closure, fan-out/churn projections, renderers,
tests, command registration, schema provenance, and the separate TC-1154 harness correction.

## Verdict

**PASS.** Five findings from the independent review were corrected and regression-tested. No
Golden Path, mock-boundary, completeness, code-test alignment, or reverse-traceability defect
remains in the reviewed change.

## Findings

| ID      | Severity | Summary                                                                                                                                                                                                                        | Refs                  |
| ------- | -------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | --------------------- |
| FND-001 | high     | Resolved: accepted premises previously admitted extra modules or schemas. Validation now requires exact format/version/module/schema equality after canonical ordering.                                                        | FR-062-AC-9; TC-1257  |
| FND-002 | high     | Resolved: syntactically valid but malformed `bindings.json` could expose `undefined` and crash an analyzer. The loader now validates the retained schema and classifies malformed content as unreadable/not-computed.          | FR-062-AC-9; TC-1257  |
| FND-003 | high     | Resolved: change-impact treated every artifact type as a requirement. Its seeds and edges are now restricted to StR, US, FR, and NFR; Plan, Task, SpecReview, and other artifact types are excluded.                           | FR-062-AC-4; TC-1252  |
| FND-004 | medium   | Resolved: accepted-premise and retained-auditor array permutations could survive into report bytes. Modules, schemas, findings, healthy ids, unevaluated checks/suites, and loaded bindings are now canonically ordered.       | FR-062-AC-10; TC-1258 |
| FND-005 | high     | Resolved: non-JSON commands inherited the interactive package update check. Every graph command now disables that inherited nudge, and TC-1259 exercises the human command path and statically seals all four command classes. | FR-062-CON-1; TC-1259 |

## Review evidence

- Tests execute real pure analyzers, AJV/zod validation, filesystem reads, evidence-store loading,
  and all three oclif command classes. The only mock is `console.log`, the external output boundary,
  and `afterEach` restores it.
- No `TODO`, `FIXME`, skip, `.only`, placeholder return, producer invocation, Quire subprocess,
  frontmatter reader, write path, network call, Git call, scoring path, or independent graph builder
  occurs in the feature surface.
- Change-impact uses an indexed reverse adjacency map and binary min-heap, terminates cycles, and
  deterministically prefers the lexicographically first shortest path.
- Export, acceptance-premise, and audit-envelope validation finish before rows. Audit source/export
  identity and exact accepted modules/schemas must match; the nested FR-032 result is projected
  without recomputation and in canonical order.
- The change-impact population is explicitly requirement-only. Valid non-requirement corpus edges
  are ignored rather than mislabeled dangling, while genuinely unresolved selected edges remain
  visible as gaps.
- The inherited update nudge is disabled for graph commands before a human or JSON view runs, so
  neither rendering path can invoke `npm view` or write its cache.
- The repository is not a React project. The Python-specific class/pytest conventions in the generic
  review checklist do not apply to this TypeScript/Vitest codebase.
- Reverse code-to-spec inspection found every changed production surface owned by FR-062, except
  the separately committed TC-1154 harness correction owned by NFR-014.

## Validation

- `make lint`: pass.
- `corepack pnpm run build`: pass, including all three command entries and assurance schema copy.
- Focused graph tests: 17/17 pass.
- Evidence-audit, auditor, assurance, measurement, portfolio, and graph regressions: 104/104 pass.
- Full Vitest suite: 817/819 pass. The remaining failures are pre-existing external drift: the
  installed skill set declares `architecture-evaluation` ahead of the checked-out module vocabulary,
  and the installed Quire coverage payload omits `binding_census[].tagged` required by the unchanged
  coverage-v1 schema. Neither failure intersects the #152 implementation.
