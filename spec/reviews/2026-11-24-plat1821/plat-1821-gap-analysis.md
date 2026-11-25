---
id: SR-2026112016223475846355059
title: "gap analysis review of PLAT-1821 snapshot anomaly batch"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quoin; repository spec, rust/crates source and test trees"
review_set: subset
relationships:
  - target: ix://agent-ix/quoin
    type: references
---

## Summary

The engine-backed repository matrix contains 704 untagged criteria and 5 criteria tagged only by ignored tests; the three new criteria in this PR are tagged. The required repository-wide matrix/reverse-gap/stub audit was performed without semantic review or plan assessment.

## Verdict

FAIL

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | The computed repository matrix reports 704 live criteria with no tagged test, so repository traceability does not pass. | spec/; rust/crates/ |
| FND-002 | high | The computed repository matrix reports 5 criteria whose only tagged tests are ignored. | spec/; rust/crates/ |

## Coverage

- Matrix source: `quoin matrix --repo . --json` (engine-backed); 2,540 criteria total.
- Static status: 1,626 tagged, 704 untagged, 5 tagged only by ignored tests, 169 method-without-symbol, 36 retired.
- Evidence status: 2,540 without stored run evidence. The PR reports one focused locked build passed (snapshot 1/1; dossier 1/1); no additional build/test was run for this review.
- Changed criteria: FR-167-AC-18 and FR-167-AC-23 are tagged; FR-201-AC-11 is tagged. No changed criteria are untagged or ignored-only.
- Reverse-gap inventory: 1,072 Rust crate files; 4,261 public declaration lines inventoried (functions, types, constants and related declarations; this is a declaration count, not a count of distinct behaviors); 3,533 test attributes inventoried. Changed snapshot/dossier behaviors map to FR-167/FR-201; no unowned behavior was found in the changed surface.
- Stub scan: zero TODO/FIXME/XXX/todo!/unimplemented! markers across `rust/crates`; the changed implementation and tests showed no placeholder implementation, ignored-only coverage, or coverage inflation. No test/build was run for this scan. The static inventory does not establish semantic ownership for every unrelated declaration in the full repository.
- The full matrix and static scans were performed, but reverse-gap semantic adjudication was limited to the changed area; the FAIL verdict is independently established by repository-wide traceability findings.
- Semantic review: skipped (not opted in).
- Plan completion: not assessed
