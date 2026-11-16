---
id: SR-20261116004328
title: "Base review of VER-103 formalization coverage spec"
type: SpecReview
analysis: base
scope: "agent-ix/quoin; PR #936; spec/functional/FR-202-formalization-coverage-state.md, spec/functional/FR-203-formalization-as-evidence-kind.md"
review_set: all
---

## Summary

Reviewed both new FRs, their acceptance criteria, traceability and ID use. AP-201 applies because the change adds coverage counts and an assurance report; its `review_policy.mode` is `recommend` for spec-review, code-review and gap-analysis. AP-202 does not apply because no live semantic lens is being evaluated. No human-enforced review gate applies.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | high | The new formalization-coverage FR reuses FR-202, already assigned to the dossier threat-model requirement on this branch's mainline. The duplicate makes the requirement and its AC identifiers ambiguous; allocate unused IDs and update references. | spec/functional/FR-202-formalization-coverage-state.md:2; spec/functional/FR-202-dossier-threat-model-content.md:2 | wrong-requirement |
| FND-002 | low | The new FR-203 and formalization FR are absent from `spec/functional/index.md`; add their links after allocating unique IDs so readers can discover them. | spec/functional/index.md:185 | missing-requirement |

The full-scope validation failure is baseline: the PR and detached clean `origin/main` both exit 3 with the same missing `functional_coverage` row in `spec/matrix.md` and identical 37 warning-bearing documents. No PR-only validation diagnostic appeared. The new requirements produce no EARS warnings. The requirements contain no hash, digest, SHA, checksum, pin or file-tracking instruction.
