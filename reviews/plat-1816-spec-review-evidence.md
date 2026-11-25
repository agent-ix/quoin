---
id: SR-2026112016223475846355049
title: "PLAT-1816 evidence review"
type: SpecReview
analysis: evidence
scope: "FR-191-AC-31 and its Rust test binding; dossier schema validation evidence"
review_set: subset
relationships:
  - {target: "ix://agent-ix/quoin/FR-191", type: references}
---
## Summary
The Test verification method is appropriate: the criterion is checked by a built-executable integration test and the dossier output is also validated against the merged process schema in focused coder evidence. The repository evidence store has no recorded runs, but direct focused run logs were provided and inspected.

## Findings
| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No issues found (placeholder) | - |

## Coverage
Inspected `/tmp/plat1816-quoin-process-schema-test.log` (successful merged-process-schema validation), `/tmp/plat1816-quoin-cli-producer-test.log`, and `/tmp/plat1816-quoin-dossier-writer-tests.log`. FR-191-AC-31's test launches `CARGO_BIN_EXE_quoin --version` and checks generated frontmatter for exact equality, separate document version, and forbidden subject/provenance fields. Engine matrix reports no run evidence in its central evidence store; its status was not inferred from that store.
