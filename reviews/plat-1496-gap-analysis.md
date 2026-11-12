---
id: SR-20261112201004
title: "Gap analysis of quoin PR 835 (computed matrix replaces spec/matrix.md)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quoin@843068e4; computed matrix over spec/ and rust/, before (origin/main f608a588) and after"
review_set: subset
---

# Gap analysis of quoin PR 835

## Summary

Ticket: PLAT-1496. Plan completion: not assessed. I computed the matrix twice, once on a
detached origin/main worktree and once on the PR head, using two tools: `quire matrix --format
tsv` (quire 0.43.0) and `quoin matrix --json` built from the branch (0.31.0-2-g843068e4). The
two tools gave the same counts.

| status | before | after |
| --- | --- | --- |
| tagged | 476 | 648 |
| untagged | 724 | 552 |
| method-without-symbol | 88 | 88 |
| retired | 2 | 2 |

No criterion went from tagged to untagged. 172 went from untagged to tagged: the hand table had
been suppressing them as `criterion-tagged-outside-row`, and all 479 of those warnings are gone.
In FR-136..FR-140, 45 criteria are now tagged (all 45 were untagged before) and one,
FR-136-AC-9, is method-without-symbol. The 552 untagged criteria are real gaps that existed
before this PR and are not in its scope. The largest are FR-116 (18), NFR-027 (17), FR-029,
FR-038, FR-090 and FR-099 (13 each), FR-089, FR-096 and FR-069 (12 each), and the FR-084..FR-123
band. 76 more tests carry a bare `FR-NNN` tag that binds nothing (`untracked-id-has-minted-children`).

## Verdict

**PASS WITH FINDINGS (one low).** The computed matrix fully replaces the hand table, with no
lost coverage. FR-136..FR-140 now show as tagged.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | The spec-to-plan skill still tells authors to mint TC ids and to wire them as `verifies` relationships (`ix://<org>/<component>/TC-004`). In a repository with no TestMatrix, those ids resolve to nothing, which is the source of the 236 dangling plan targets. Rewrite the skill to make tasks verify criterion ids | skills/spec-to-plan/references/step-2-test-plan.md:22, skills/spec-to-plan/references/step-3-execution-plan.md:109 |

## Examined scope

```yaml
clean: false
scope:
- {id: FR-136..FR-140, path: spec/functional, role: examined, excerpt: "45 tagged, 1 method-without-symbol after; 45 untagged before"}
- {id: computed-matrix, path: spec/, role: examined, excerpt: "1288 criteria; 0 tagged->untagged regressions; 172 untagged->tagged"}
```
