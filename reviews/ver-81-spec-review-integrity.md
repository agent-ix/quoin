---
id: SR-20261113055216
title: "Integrity review of analysis-result counting (FR-152, FR-153, NFR-033, TC-002)"
type: SpecReview
analysis: integrity
review_set: subset
scope: "agent-ix/quoin; PR #849 (git diff origin/main...origin/ver-81-spec-retained-evidence-analysis at 480ba1c2): spec/functional/FR-152, spec/functional/FR-153, spec/non-functional/NFR-033, spec/test/TC-002, spec/spec.md and index edits"
---

# Integrity review of analysis-result counting (FR-152, FR-153, NFR-033, TC-002)

## Summary

Ticket: VER-81. Checked consistency, relationships, cross-repo references and the "Not decided
here" list. `quire validate --scope .` over the five changed spec files exits 0. There are no ID
collisions: FR-152, FR-153, NFR-033 and TC-002 are on no other origin branch, and no other quoin
PR is open. The TC-002 in spec/evals.md is fixture text, not a quoin artifact. Cross-repo content is
referenced and not copied.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The "Not decided here" list includes items that the same PR's normative text already decides. Item 1 (`--link` or declared target or plan) is decided by FR-152 Behavior. Item 4 (does a satisfied report count for an obligation it does not analyse) is decided by FR-153's "SHALL NOT judge whether the analysis request is the right analysis" together with the FR-116 actor-authority premise. Item 3 (keep a disagreement distinguishable) is settled for now by FR-151 as merged, since `unrepresented` is not stored. The spec can state that and file a follow-up if a store change is wanted. Only item 2 (producer-definition content, which needs FR-116 owner approval) and item 5 (waiver representation, which quire-rs FR-078 holds for the owner's ruling) are real owner decisions. Remove items 1, 3 and 4 from the list, or restate them as decided. | spec/functional/FR-153-analysis-result-counting-by-disposition.md:204-217 |
| FND-002 | low | The cross-repo address `ix://agent-ix/quire-rs/spec/functional/FR-078` differs from the repo convention `ix://agent-ix/quire-rs/FR-078`. FR-116 uses the convention, and so does every other quire-rs reference in quoin's spec. Use the id form. | spec/functional/FR-152-analysis-run-entry-link-to-obligation.md:16, :115; spec/functional/FR-153-analysis-result-counting-by-disposition.md:16, :26, :135 |
| FND-003 | low | FR-153's Dependencies list FR-093 as upstream, but its frontmatter has no relationship to FR-093. Add a `references` or `requires` edge, or drop the dependency. | spec/functional/FR-153-analysis-result-counting-by-disposition.md:1-18, :270-273 |

## Verdict

Consistent apart from the "Not decided here" list (FND-001) and two reference nits.

Examined (role examined): the frontmatter relationships of all four new artifacts, both
Dependencies sections, the "Not decided here" list, the FR-151 Downstream edit, and the spec.md and
index entries.
