---
id: SR-20261113095721
title: "Integrity review of quoin risk-summary (FR-169, FR-170)"
type: SpecReview
analysis: integrity
review_set: subset
scope: "agent-ix/quoin; PR #863 (git diff origin/main...HEAD at a1ece4e9): spec/functional/FR-169, spec/functional/FR-170"
---

# Integrity review of quoin risk-summary (FR-169, FR-170)

## Summary

Ticket: PLAT-1576. Checks FR-169 and FR-170 against each other, against quoin FR-145..148, FR-154 and
FR-155, and against spec-objects-safety FR-305 and FR-101..107, FR-201..205, FR-301..304 on `origin/main`.

No duplication: the policy cell, the `outside_acceptable` flag and the missing-region reason stay in
FR-146 and are reused by reference (one shared lookup, FR-169-AC-9); the undecided queue stays in
FR-148; the FR-105 tokens are computed by the FR-155 code (FR-170-AC-10) and the control tokens by the
FR-147 rule. FR-170 coins no token. The row key set, the one-row-per-pairing rule, the null placement,
the ordering, the epistemic markers and the "no in-force estimate" rule match FR-305. The read-only and
no-verdict rules sit inside FR-169, as required, since NFR-031 does not bind this command.

Examined (role examined): FR-169 statement, Behavior and FR-169-AC-1..19; FR-170 statement, Behavior,
token table and FR-170-AC-1..13. Context only: spec-objects-safety FR-101..107, FR-201..205,
FR-301..305; quoin FR-145..148, FR-154, FR-155.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | "A post_control estimate that assumes no control SHALL show controls: [] and no further finding" contradicts FR-169, where controls is a row key holding every control any post-control estimate of the situation assumes. With two post-control estimates on one situation, one assuming C and one assuming nothing, the row shows controls [C] and nothing tells the reader the second estimate assumes no control. Replace the bullet by the reused FR-204 token post-control-without-control naming the estimate, on every row of its situation. | FR-170:64, FR-169:99 |
| FND-002 | medium | A risk_estimate with no estimates edge belongs to no row, so its estimate entry, the residual risk that decides it, and that record's FR-105 findings (and post-control-estimate-without-decision) appear nowhere, with no word. FR-305 says the report shows the residual-risk decision. State that records reachable from no hazard or situation are outside this report and are listed by quoin risk advise (FR-147, estimate-without-situation), and add a criterion. | FR-169:90, FR-170:53 |
| FND-003 | medium | The row mixes two policy sources. policy_region uses the FR-146 estimate policy, which is null (policy-ambiguous) when two residual risks on one estimate name different policies; the residual-risk findings use each record's own policy (FR-155, FR-155-AC-13). The row can show policy_region null next to not-accepted-inside-acceptable-region or accepted-outside-acceptable-region, which assert a region, and no decision-without-region. This is the divergence of SR-20261113071104 FND-001 again. State the case and test it, or say which source a reader should trust. | FR-169:63-65, FR-170:62, FR-170-AC-9 |
| FND-004 | medium | FR-169 implements FR-305 but diverges from its contract without saying so: FR-305 lists each entry's identity as "id" and FR-305-AC-5 requires estimate entry keys to equal that set exactly, while FR-169 names them estimate, control and residual_risk; FR-305 says findings holds "finding tokens", FR-170 holds {finding, names} objects. Record both divergences in FR-169 and file the FR-305 amendment in agent-ix/spec-objects-safety. | FR-169:104-106, FR-170:43 |

## Verdict

Not mergeable as written: four medium findings. FND-001..003 are spec-text fixes inside this PR.
FND-004 needs one sentence here; the contract change itself belongs in the safety module's FR-305.

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-005 | low | The new "Divergence from the safety module's contract" section says FR-169 differs from FR-305, but agent-ix/spec-objects-safety PR #42 aligns FR-305, so the sentence is false as soon as both land; it also cites "amends FR-305" as a bare other-repo id. Drop the section when PR #42 merges (merge #42 first, or both together). | FR-169:122-124 |
| FND-006 | low | The rule that an estimate with no situation, and its residual risk, appear in no row is stated three times. Keep the FR-169:78 statement and the FR-170:71 no-token rule; drop FR-169:115. | FR-169:78, FR-169:115, FR-170:71 |

## Dispositions

Round 1, against commit 68717d15.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | - |
| FND-002 | fixed | - |
| FND-003 | fixed | - |
| FND-004 | fixed | - |
