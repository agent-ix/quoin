---
id: SR-20261113071104
title: "Integrity review of quoin risk discrepancies (FR-154, FR-155)"
type: SpecReview
analysis: integrity
review_set: subset
scope: "agent-ix/quoin; PR #853 (git diff origin/main...HEAD at 3b00d92c): spec/functional/FR-154, spec/functional/FR-155, spec/functional/index.md, spec/spec.md"
---

# Integrity review of quoin risk discrepancies (FR-154, FR-155)

## Summary

Ticket: PLAT-1570. Checks consistency of FR-154/FR-155 with each other, with quoin FR-145..148
and with spec-objects-safety FR-101..106 on `origin/main`.

FR-154 and FR-155 do not duplicate FR-146 or FR-148 requirement text: region lookup is cited,
not restated, and the overlap with FR-148's queue (undecided records) is FR-105's own design.
A separate command is justified, because FR-145 fixes exactly four keys and an
exit-zero-on-render rule. The extra token post-control-estimate-without-decision fits FR-148's
no_record rule (a post-control estimate no residual risk references) and FR-303's relation. Its
attribution is covered in the dependency review.

Examined (role examined): FR-154 Description, Behavior and FR-154-AC-1..9; FR-155 Description,
Behavior, names table and FR-155-AC-1..13. Context only: quoin FR-145..148, NFR-031;
spec-objects-safety FR-101..106, FR-303.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-154 says each residual risk's region comes from "the same lookup as FR-146", and AC-8 requires it to equal risk advise's region for the estimate. FR-155 takes the policy from the residual risk's own policy reference (AC-13). When one post-control estimate is referenced by two residual risks on different policies, FR-146 shows policy-ambiguous and no region, while FR-155 shows a per-record region, so AC-8 and AC-13 cannot both pass. State that the region is the per-record FR-102 lookup (as FR-148 does) and limit AC-8 to estimates with a single policy, or report decision-without-region in the ambiguous case. | FR-154:34-36, FR-154-AC-8, FR-155:60, FR-155-AC-13 |
| FND-002 | medium | FR-155 says findings use "the { finding, names } shape that FR-146 to FR-148 use", but those sections carry bare token arrays under findings, and FR-147's names is a single id. FR-155's names is a fixed-key object, so the same key has two types across sibling risk commands. Drop the claim, and either rename the object (for example subject) or put its keys on the finding itself. | FR-155:24-26, FR-155:51-61 |
| FND-003 | low | "with the missing-region findings of FR-146" suggests a decision-without-region finding carries level-not-in-policy, level-not-assessed, policy-not-found or policy-ambiguous, but the finding has exactly the keys finding and names, and names has no key for them. Say whether the reason is carried (and where) or that it is not. | FR-155:30-31, FR-155-AC-8 |
| FND-004 | low | Restates FR-154's --strict exit rule, so two requirements own one obligation. Keep it in FR-154 only and leave FR-155:66 (strict changes no finding). | FR-155:65, FR-154:53 |

## Verdict

Not mergeable as written: FND-001 is a real contradiction on a reachable edge, and FND-002 is
a false consistency claim. Both are spec-text fixes inside this PR.

## Dispositions

Round 1, against commit 678a797b.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | - |
| FND-002 | fixed | - |
| FND-003 | fixed | - |
| FND-004 | fixed | - |
