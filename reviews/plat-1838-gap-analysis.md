---
id: SR-20261117175249
title: "Gap analysis — quoin M3 dossier generators"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quoin; spec/, rust/crates/quoin-dossier/src/{product.rs,gap.rs,lib.rs}, rust/crates/quoin-cli/tests/tc_1614_dossier_command.rs, rust/crates/quoin-dossier/schemas/"
review_set: subset
relationships:
  - { target: "ix://agent-ix/quoin", type: references }
---

## Summary

Audited Quoin's computed requirement-to-test matrix and reverse code-to-spec coverage, focusing on the M3 dossier implementation in frozen PR #1068. Twelve implementation criteria in FR-191 and FR-229–233 have no correctly trace-tagged test; repository plan completion was not assessed.

## Verdict

**FAIL** — twelve criteria needed by the changed M3 behavior are untagged in the computed matrix.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | The M3 implementation lacks trace-tagged tests for FR-191-AC-32, FR-229-AC-5, FR-230-AC-4/5, FR-231-AC-1/3/4, FR-232-AC-2/3/4/5, and FR-233-AC-5. | FR-191-AC-32, FR-229-AC-5, FR-230-AC-4, FR-230-AC-5, FR-231-AC-1, FR-231-AC-3, FR-231-AC-4, FR-232-AC-2, FR-232-AC-3, FR-232-AC-4, FR-232-AC-5, FR-233-AC-5 |

## Coverage

- Reconciliation: `quoin matrix --repo . --json` (Quoin 0.36.8); the evidence-backed matrix ran successfully. `quire coverage --scope . --json` (Quire 0.52.0, engine 0.62.2) also exited 0.
- Plan completion: not assessed.
- Full-repository criteria: tagged 1,707; untagged 653; tagged-by-ignored-test 11; method-without-symbol 169; retired 36. Evidence status: no run evidence for 2,576 criteria.
- The 12 untagged criteria in FND-001 are the M3 behavior this PR implements. The rest of the repository's untagged matrix rows are outside this change's requirement set and are not attributed to this PR.
- Coverage scan: 296 untracked symbols and 1,717 diagnostics repository-wide; none of the untracked test symbols names FR-229 through FR-233. The one relevant diagnostic is the non-binding FR-229–233 module comment in `product.rs`, which is not a test trace tag. `quoin matrix` reported the FR-232 device association test as tagged to AC-1; code review found its test body actually exercises AC-3, recorded in SR-20261117175248.
- Reverse-gap inventory: changed product generators, source reader, schemas, and gap kinds map to FR-191 and FR-229–233; no unowned user-visible behavior or source/test stubs were found.
- Semantic review: skipped; not requested.
