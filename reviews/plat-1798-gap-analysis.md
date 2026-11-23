---
id: SR-20261117175267
title: "Gap analysis — PLAT-1798 PR #1006 regulated risk report tests"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quoin; FR-169 risk-summary implementation/tests, FR-144 VEX report checks, and PLAT-1798 installed-pack evidence boundary; PR #1006 head 827897cae1f16e4660c43137dac80f87e2740c1d"
review_set: subset
relationships:
  - target: "ix://agent-ix/quoin/FR-169"
    type: "references"
  - target: "ix://agent-ix/quoin/FR-144"
    type: "references"
  - target: "ix://agent-ix/quoin/FR-172"
    type: "references"
---

## Summary

Ticket: PLAT-1798. This planless gap analysis is scoped to the changed FR-169 risk-summary behavior and the ticket's installed-pack evidence boundary; it is not a repository-wide audit. The computed matrix reports all 28 FR-169 criteria tagged, including AC-28. The new implementation has an owning requirement and no stub was found in the scoped changed code. However, the existing AC-2 exact-output test fails against the new serialized shape, so the tagged criterion's evidence is red rather than passing.

## Verdict

**FAIL.** A focused criterion test fails. Separately, the required FR-172 live synthetic-pack demonstration and the coder-reported static restriction checks were not independently verified in this review; missing evidence is not a pass.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | FR-169-AC-2's existing exact-output test fails because the new mandatory `retired_requirements` key is absent from its expected JSON; the checked-in report snapshots also omit the field. | rust/crates/quoin-risk-advice/tests/tc_169_risk_summary.rs:232-235 |
| FND-002 | medium | The ticket's installed-pack evidence boundary is incomplete in this review: FR-172-AC-3/4's live synthetic-pack demonstration was reported blocked and could not be independently run, and the static scan/source-selection results were not independently reproduced. This evidence gap cannot be counted as a pass. | spec/functional/FR-172-clause-set-sources-and-licence-boundary.md:55-56 |

## Coverage

- Target selection: `spec/spec.md` identifies `agent-ix/quoin`; no plan was supplied.
- Matrix verification: `quoin matrix --repo . --json` was filtered to FR-169; 28 criteria were tagged. AC-2 is bound to `a_complete_chain_is_one_row_with_exact_keys_and_no_findings`; AC-28 is bound to `retired_requirements_remain_linked_and_are_reported_by_exact_status`. The AC-2 test fails as described above.
- Reverse-gap/stub check: inspected the changed FR-169 implementation and tests against the owning FR-169 requirements; no unowned behavior or stub was found in this scoped feature. The output field change is directly owned by AC-2/AC-28.
- FR-172-AC-3/4 are demonstration criteria and are relevant to the ticket's licensing/source-selection acceptance, but are not in the changed paths. Their live run and reported static checks were not independently confirmed; the CLI build was blocked by absent generated `quoin-semantic` build inputs. The coder's temp-root and dependency-restore blockers were not reproduced here.
- Excluded PLAT-1790 and PLAT-1788 from analysis.
- Gates: `git diff --check` passed. One exact-name Rust test failed; one exact-name new CLI test could not execute due missing generated build inputs. No broader Cargo invocation was run.
- Plan completion: not assessed
- Semantic review: skipped

## Dispositions

### Disposition pass 1 — PR #1006 head `70291aa0b04ec34cde514ad2a8f2f9f8b04a9b58`

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | The exact-output assertion and both snapshots now include the `retired_requirements` field. The focused exact-output and snapshot tests passed. |
| FND-002 | deferred | The static restriction and source-selection evidence is now independently confirmed (`tc_172_001_no_shipped_skill_or_fixture_file_carries_a_restricted_id` and `a_restricted_pack_uses_index_order_and_never_renders_clause_text`), and the synthetic installed-pack `tc_172_004` test passed. These close the unverified code-level boundary evidence. The separate FR-172-AC-3/4 live skill demonstration is still not established by `tc_172_004`; it is outside this code PR's changed behavior and must be recorded separately before claiming those demonstration criteria passed. |

Verification evidence: `/tmp/plat-1798-review-risk-ac2-final.log`, `/tmp/plat-1798-review-risk-snapshots.log`, `/tmp/plat-1798-review-ac28-final.log`, `/tmp/plat-1798-licensed-boundary-escalated.log`, `/tmp/plat-1798-licensed-static-final.log`, and `/tmp/plat-1798-source-selection-final.log`. The final test-helper `clean_base` uses marker-based `.git` detection over ancestors and the installed fixture uses that shared `scratch()` helper. No production module/source-selection boundary files changed in this fix round.

## Final head coverage check

At PR head `827897cae1f16e4660c43137dac80f87e2740c1d`, the computed matrix reports FR-144-AC-21/22/23 and FR-169-AC-2/28 as tagged. The changed VEX report logic has focused tagged cases for stale statements, unavailable/ambiguous scan evidence, unresolved dispositions, and Markdown association. No new gap was found in those changed paths. The separately required FR-172-AC-3/4 live demonstration remains deferred as recorded above; this VEX fix round did not change that evidence boundary.
