---
id: SR-20261113071103
title: "Base spec review of quoin risk discrepancies (FR-154, FR-155)"
type: SpecReview
analysis: base
review_set: subset
scope: "agent-ix/quoin; PR #853 (git diff origin/main...HEAD at 3b00d92c): spec/functional/FR-154, spec/functional/FR-155, spec/functional/index.md, spec/spec.md"
---

# Base spec review of quoin risk discrepancies (FR-154, FR-155)

## Summary

Ticket: PLAT-1570. Checklist review of the read-only `quoin risk discrepancies` report against
spec-objects-safety FR-101..106 and FR-303 on `origin/main`, and quoin FR-032, FR-145..148 and
NFR-031 on `origin/main`.

The read-only, no-verdict contract matches safety FR-104 and quoin NFR-031's intent: the
command writes nothing, stores no score, region, finding or verdict, defaults no decision, lists
no agreeing risk, and carries FR-104's attribution. The six FR-105 tokens and their rules match
FR-105 on `origin/main` exactly (decision-without-region, accepted-outside-acceptable-region,
benefit-risk-undecided, benefit-risk-not-accepted, not-accepted-inside-acceptable-region,
residual-risk-undecided), and FR-155-AC-1..8 mirror FR-105-AC-1..11. `--strict` matches
quoin's existing gate flag (FR-032 `evidence audit`: non-zero when a finding remains) and is
FR-105's promote flag. No paywalled standard text or name appears in the diff. The FR-154/FR-155
ids are unused on `origin/main` and by open PRs #850, #851 and #852 (none touches an FR file).
No hash, digest or pin antipattern is introduced.

Applicable AssuranceProfiles: spec/assurance/AP-201-finding-quality.md and
AP-202-jev-lens-evaluation.md were inspected; neither scope covers a safety risk report, so
neither applies and no enforced run is required.

Examined (role examined): FR-154 statement, Behavior and FR-154-AC-1..9; FR-155 statement,
Behavior, names table and FR-155-AC-1..13. Context only: spec-objects-safety FR-101..106, FR-303;
quoin FR-032, FR-145..148, NFR-031.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-154-AC-7 forbids any wording that states a risk accepted, but FR-155 requires the token accepted-outside-acceptable-region and names.decision "accepted" copied from the record, so AC-7 fails on FR-155-AC-1's fixture or cannot be written. Add NFR-031-AC-2's carve-out (finding tokens, the attribution, values copied from a record). NFR-031's Scope covers risk advise only, so citing it as "wording rules this report follows" binds nothing; either extend NFR-031's scope or keep the rule in AC-7. | FR-154-AC-7, FR-154:75 |
| FND-002 | medium | For post-control-estimate-without-decision, residual_risk, decision, region and policy are all null, and the Markdown line names only those plus the token, so it identifies no record. Add the estimate to the Markdown names and test it. | FR-155:63, FR-155-AC-12 |
| FND-003 | low | A --strict finding exit and a bad-scope or parse error are both "non-zero", so a gate cannot tell findings from a broken run. quoin already separates reject (exit 1) from refused (exit 2) in FR-108-AC-7 and FR-115. Consider stating the codes. | FR-154:53, FR-154:55 |

## Verdict

Not mergeable as written: two medium findings, both spec-text fixes inside this PR. The
dependency review has a high finding (tc_118_001 dangling citations) that blocks the merge too.

## Dispositions

Round 1, against commit 678a797b.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | - |
| FND-002 | fixed | - |
| FND-003 | fixed | - |
