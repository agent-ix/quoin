---
id: SR-20261117175266
title: "Code review — PLAT-1798 PR #1006 regulated risk report tests"
type: SpecReview
analysis: code-review
scope: "agent-ix/quoin; rust/crates/quoin-risk-advice/src/evidence.rs, rust/crates/quoin-risk-advice/src/records.rs, rust/crates/quoin-risk-advice/src/summary.rs, rust/crates/quoin-risk-advice/tests/tc_169_risk_summary.rs, rust/crates/quoin-cli/tests/tc_169_risk_summary_command.rs, rust/crates/quoin-soup/src/soup/report.rs, rust/crates/quoin-soup/src/soup/report/render.rs, rust/crates/quoin-cli/tests/tc_1518_soup_report.rs; PLAT-1798; PR #1006 head 827897cae1f16e4660c43137dac80f87e2740c1d"
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

Ticket: PLAT-1798. This review is limited to the FR-169 risk-summary changes and the ticket's installed-pack evidence boundary. PLAT-1790 and PLAT-1788 changes are excluded. Rust review is folded into this artifact. The new implementation uses exact `status == "retired"` matching and keeps retired IDs in the full `requirements` list. The synthetic FR-169 fixture contains no protected source text.

The name-filtered existing FR-169 contract test was run through `locked-build.sh` with a unique target and failed: its exact JSON assertion omits the newly mandatory `retired_requirements: []` key. The checked-in JSON and Markdown snapshots also lack the new output field. The new CLI test could not compile because `quoin-semantic` build inputs under `node_modules/@agent-ix/semantic-core/generated/json-schema` are absent. No Cargo invocation was broadened.

The coder's `/var/tmp`/writable-root/temp-root and dependency-restore blockers are consistent with the reported unavailability, but I did not reproduce those setup failures. The FR-172 static source-selection/license test results are coder-reported only; the FR-172 live synthetic-pack demonstration was not independently run. Those FR-172 checks are outside the changed paths but remain relevant to the ticket's acceptance, and the missing live evidence is not a pass.

## Verdict

**FAIL.** A focused existing test fails because the visible JSON shape changed without updating its exact assertion and snapshots. The new AC-28 test also does not exercise the specified case-sensitive status boundary (`Retired`). The current production implementation handles that boundary correctly, but the regression test would not catch a case-insensitive mutation.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | The existing exact-output test fails because each control now serializes `retired_requirements: []`, while its expected JSON omits the key; the checked-in report snapshots also retain the old shape. | rust/crates/quoin-risk-advice/tests/tc_169_risk_summary.rs:232-235 |
| FND-002 | medium | The AC-28 fixture covers `retired`, `draft`, and absent status, but not a differently cased value such as `Retired`. A case-insensitive regression would pass this test despite violating the exact-lowercase boundary. | rust/crates/quoin-cli/tests/tc_169_risk_summary_command.rs:118-121 |

## Assurance Context

AP-201 (`spec/assurance/AP-201-finding-quality.md`) applies because this change alters a deterministic report view. I evaluated whether the retired population is preserved in the serialized report, whether status is interpreted exactly, and whether report shape regressions have healthy assertions. The implementation preserves the complete linked requirement list and computes the retired subset by exact status equality. The existing exact-output test exposes a report-shape regression. AP-201's architecture description, linked measurement plans/records, evidence-independence records, producer-reliance decisions, and exception records were unavailable in this scoped review; no exception was identified in the profile. The PR does not modify finding scoring, comparison, or measurement persistence, so those selected practices were not applicable to the changed behavior.

## Coverage

- Examined FR-169-AC-2 exact JSON key contract, FR-169-AC-5 undecided-risk handling, FR-169-AC-7 linked control mapping, and FR-169-AC-28 retired-requirement population and exact-status boundary.
- Examined `evidence.rs`, `records.rs`, `summary.rs`, the existing exact-output test/snapshots, and the new tagged CLI test.
- Read FR-172-AC-3/4 and the unchanged synthetic installed-pack test comments to assess the ticket's source-selection/licensing evidence boundary. Coder-reported static scans were not reproduced; live demonstration remained blocked/unverified.
- Excluded PLAT-1790 and PLAT-1788 changes as directed.
- Verification: `git diff --check` passed. The exact-name FR-169 advice test failed on the missing output key. The exact-name new CLI test did not execute because required generated build inputs were absent. No aggregate Cargo test or broad test suite was run.

## Dispositions

### Disposition pass 1 — PR #1006 head `70291aa0b04ec34cde514ad2a8f2f9f8b04a9b58`

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | The exact-output assertion now includes `retired_requirements: []`, and the JSON/Markdown snapshots include the field. The focused exact-output and snapshot tests passed. |
| FND-002 | fixed | The AC-28 fixture now includes `status: Retired` and still expects only exact lowercase `retired`; the focused AC-28 test passed. |

Verification evidence: `/tmp/plat-1798-review-risk-ac2-final.log`, `/tmp/plat-1798-review-risk-snapshots.log`, and `/tmp/plat-1798-review-ac28-final.log`. The final test-helper `clean_base` uses the marker-based ancestor `.git` check and `scratch()` places the fixture outside git work trees. The installed-pack test passed at `/tmp/plat-1798-licensed-boundary-escalated.log`; no production module/source-selection boundary files are changed by this fix round.

### Round 2 — new findings remain open at head `70291aa0b04ec34cde514ad2a8f2f9f8b04a9b58`

| FND | outcome | reason |
| --- | --- | --- |
| FND-003 | still-open | The stale-state code still requires every applicable tool to appear in `free_of`, so an unrelated attached advisory can prevent the statement from being marked stale. |
| FND-004 | still-open | The AC-23-tagged fixture still uses a resolved advisory and `not_affected` VEX status; the affected-plus-unresolved case is absent. |
| FND-005 | still-open | The report still selects only the first statement matching an advisory, and later matching statements do not appear in the Markdown unmatched list. |
| FND-006 | still-open | The stale-state test still uses an npm record with one applicable tool; the `.all` versus `.any` countercase is absent. |
| FND-007 | still-open | The AC-21 Markdown checks still use unassociated `contains` assertions rather than asserting the VEX fields on the matching advisory line. |

### Round 3 — PR #1006 head `70291aa0b04ec34cde514ad2a8f2f9f8b04a9b58`

| FND | outcome | reason |
| --- | --- | --- |
| FND-005 | rejected | The upstream SOUP record contract, [spec-objects-security FR-022-AC-6](https://github.com/agent-ix/spec-objects-security/blob/main/spec/functional/FR-022-vex-statements-on-soup-records.md#L78), requires exact duplicate `advisory` identifiers within one record to fail located validation while distinct identifiers validate. The duplicate scenario in FND-005 is therefore invalid input, so first-match rendering does not establish an FR-144 population gap. |

### Round 4 — final disposition at PR #1006 head `827897cae1f16e4660c43137dac80f87e2740c1d`

| FND | outcome | reason |
| --- | --- | --- |
| FND-003 | fixed | Staleness now requires every applicable tool's scan to be read and no applicable tool to have an unmatched finding that could belong to the record. A new regression case confirms an unrelated live advisory does not mask a missing VEX advisory, while an ambiguous possible match prevents staleness. |
| FND-004 | fixed | A new AC-23 fixture pairs an `affected` VEX statement with an unresolved advisory and asserts the report keeps both facts separate in JSON and Markdown. |
| FND-006 | fixed | A new cargo countercase has one applicable scan read and another not run; it asserts the statement remains non-stale, distinguishing all-applicable scans from any-applicable scan. |
| FND-007 | fixed | The AC-21 test extracts the matched record's advisory line and checks its status, justification, impact, action, evidence, and advisory identity on that line. |

Verification evidence: the exact-head worktree is `827897cae1f16e4660c43137dac80f87e2740c1d`; the focused pass logs are `/tmp/plat-1798-vex-stale-edge-final.log`, `/tmp/plat-1798-vex-ac23-final.log`, `/tmp/plat-1798-vex-stale-controls-final.log`, and `/tmp/plat-1798-vex-ac21-markdown-final.log`. The computed matrix reports FR-144-AC-21/22/23 and FR-169-AC-2/28 as tagged. No new finding was identified in the changed VEX stale-state implementation or its focused tests. FND-005 remains rejected under round 3; the FR-172-AC-3/4 demonstration gap recorded in the gap-analysis SR remains deferred and outside these changes.

## New findings (disposition pass 2)

These five candidate findings were raised in the single Opus review recorded at `/tmp/plat-1798-opus-review.log` (model `claude-opus-5-5`). I independently checked each against this PR's current diff, the FR-144 requirements, the report implementation, and its tagged tests; I did not run a second Opus review. All five are valid test or report findings in the PR diff. The earlier PLAT-1790/PLAT-1788 exclusions remain in effect for unrelated changes; these findings are limited to the changed SOUP report/VEX code and tests relevant to PLAT-1798's regulated report coverage.

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-003 | medium | A VEX statement is marked stale only when every applicable tool is in `free_of`. That set excludes a tool that reported any advisory for the record, so an unrelated live advisory can mask a statement whose own advisory is absent from every applicable scan. | rust/crates/quoin-soup/src/soup/report.rs:394-396 |
| FND-004 | medium | The test tagged FR-144-AC-23 uses a `not_affected` VEX statement with a resolved advisory; it never exercises the required `affected` statement alongside an unresolved advisory disposition. | rust/crates/quoin-cli/tests/tc_1518_soup_report.rs:1214-1293 |
| FND-005 | medium | When two VEX statements name the same advisory, report rendering uses the first match for the advisory and omits the later statement from Markdown because it is not classified as unmatched; conflicting authored data is therefore hidden from the human report. | rust/crates/quoin-soup/src/soup/report/render.rs:78-83 |
| FND-006 | low | The AC-22 stale-state test uses an npm record with only one applicable tool, so replacing the required all-applicable-scans condition with an any-scan condition would still pass. | rust/crates/quoin-cli/tests/tc_1518_soup_report.rs:1296-1325 |
| FND-007 | low | The AC-21 Markdown assertions only search for `not_affected` and a generic resolution phrase; they do not tie the VEX status or other required statement fields to the matching advisory line. | rust/crates/quoin-cli/tests/tc_1518_soup_report.rs:1287-1293 |
