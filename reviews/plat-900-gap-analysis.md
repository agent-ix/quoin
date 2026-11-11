---
id: SR-20261103118173
title: "Gap analysis of quoin PR 801 (FR-070 coverage)"
type: SpecReview
analysis: base
scope: "agent-ix/quoin@fdfb889; FR-070 criteria, spec/matrix.md, rust/crates/quoin-semantic"
review_set: subset
---

# Gap analysis of quoin PR 801

## Summary

Ticket: PLAT-900. Plan completion: not assessed. `quoin matrix --json` (quoin 0.30.0) was run
on the PR head and on an origin/main worktree, then FR-070 was compared criterion by
criterion. On main, FR-070-AC-1..13 and CON-1 are tagged and CON-2 is untagged (method
"Build inspection"). On the head, AC-1..14 and CON-1 are tagged and CON-2 no longer exists.
Repo totals: tagged 371 -> 372 and untagged 700 -> 699. `quire trace --id FR-070-AC-3`
lists `tc_452_640` beside `tc_452_612` and `tc_452_661`. No live criterion lost a binder, and
the TC-1343 row removed from the matrix was already withdrawn. An rg over FR-070 and
quoin-semantic finds no filament-core-service ownership claim. There is no `scripts/`
directory, so no script reads a filament-core-service checkout. The remaining "vendored"
comments in quoin-semantic make no private-repo sourcing claim, so they are out of scope.

## Verdict

**PASS.** No coverage gap and no stale ownership claim in scope.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Examined scope

```yaml
clean: true
scope:
- {id: FR-070-AC-3, path: spec/functional/FR-070-semantic-module-manifest-extension.md, role: examined, excerpt: "tagged on main and head; binders tc_452_612, tc_452_640, tc_452_661"}
- {id: FR-070-AC-14, path: spec/functional/FR-070-semantic-module-manifest-extension.md, role: examined, excerpt: "tagged on head: tc_452_644"}
- {id: FR-070-CON-1, path: spec/functional/FR-070-semantic-module-manifest-extension.md, role: examined, excerpt: "tagged on main and head: tc_452_641"}
```
