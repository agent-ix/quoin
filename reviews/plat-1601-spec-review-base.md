---
id: SR-20261113052922
title: "Base spec review of PLAT-1601 OSV-Scanner adapter and advisory join"
type: SpecReview
analysis: base
scope: "agent-ix/quoin; PR #881; spec/functional/FR-142-advisory-join-onto-soup-records.md, spec/functional/FR-144-soup-report.md, spec/functional/FR-149-osv-scanner-advisory-report-adapter.md, spec/functional/FR-150-licence-and-ban-policy-findings.md, spec/functional/FR-167-release-snapshot-format.md, spec/non-functional/NFR-030-soup-views-state-and-never-derive.md, spec/functional/index.md, spec/spec.md"
review_set: all
---

# Base spec review of PLAT-1601 OSV-Scanner adapter and advisory join

## Summary

Ticket: PLAT-1601. Reviewed the nine changed paths in PR #881, including the full FR-142, FR-144 and FR-149 acceptance criteria, the changed NFR-030 criterion, and the index summaries. AP-201 is scoped to finding-quality measurement and reporting; AP-202 is scoped to live Jev lens evaluation. Neither applies to this scanner-adapter change, so this is a direct review. The selected base checklist plus all seven analyses are covered by the sibling SR artifacts.

`quoin matrix --repo . --json` recognizes the new criteria, but all FR-142/144/149 and NFR-030 criteria currently have no bound tests, consistent with a spec-only change. Full-tree `quire validate --scope . 'spec/**/*.md' --summary` reports a pre-existing FR-113 acceptance-table width error outside this diff; the changed requirements emitted no EARS or quality warnings in the targeted read.

## Verdict

FAIL. The new prose pins an exact external tool version, and the amended requirements no longer meet the ticket's container OS-package fixture acceptance criterion.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | FR-149 identifies source documentation as release `v2.7.1` even though no binary was run and the research is not an application-version record. This is an exact tool-version pin in prose. Remove the release number; the review checklist requires exact tool and toolchain versions to live only in package manifests and lockfiles. | spec/functional/FR-149-osv-scanner-advisory-report-adapter.md:45 |
| FND-002 | high | FR-142 now marks OS-package purls unsupported and FR-142-AC-19 exercises only npm, PyPI and Go. PLAT-1601's acceptance criteria require each container OS-package fixture component to receive its matching advisory. This spec therefore leaves a stated ticket outcome unfulfilled. The owner must decide whether to change the scanner or explicitly revise that acceptance criterion before this work is accepted. | spec/functional/FR-142-advisory-join-onto-soup-records.md:35; spec/functional/FR-142-advisory-join-onto-soup-records.md:90; spec/functional/FR-142-advisory-join-onto-soup-records.md:93; spec/functional/FR-142-advisory-join-onto-soup-records.md:202 |
| FND-003 | medium | FR-149-AC-10 says adapter fixtures are checked in unedited, while FR-149-AC-3 requires a truncated-file fixture. Those instructions conflict for the malformed-input fixture. Limit “checked in unedited” to captured scanner outputs and identify malformed-input fixtures as derived cases. | spec/functional/FR-149-osv-scanner-advisory-report-adapter.md:119; spec/functional/FR-149-osv-scanner-advisory-report-adapter.md:112 |
| FND-004 | low | FR-149 describes a future “Rust scanner” specifically. That language choice is not needed to specify the adapter boundary and adds an unrequested future implementation commitment. State the boundary generically as a later scanner using a new adapter and tool name. | spec/functional/FR-149-osv-scanner-advisory-report-adapter.md:31 |

## Verdict details

Other examined changes preserve the `producer-execute` boundary, text-only adapter purity, per-person dispositions, explicit unsupported states, and unchanged severity words. No paywalled standard name or text was added. No digest or signature record was added. The database being reported as `not recorded` is consistent with the research claim that the report exposes no database identity.

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | after_excerpt: “The documented report shape below was reviewed from OSV-Scanner project documentation and source.” |
| FND-002 | fixed | after_excerpt: “Given retained osv-scanner scans over authentic captured reports from FR-044 fixtures holding npm (including a scoped package), PyPI, Go module, and container OS-package components, each fixture component with a known fixture advisory receives exactly the findings whose valid purl equals its own, and no component receives another’s advisory, including a PyPI and npm record of the same name.” |
| FND-003 | fixed | after_excerpt: “Rust adapter tests in the existing harness parse authentic, unedited OSV-Scanner reports captured through the FR-044 request over the npm, Python, Go and container OS-package fixtures defined by FR-044; malformed and truncated inputs are derived test fixtures and are not represented as captured scanner outputs.” |
| FND-004 | fixed | after_excerpt: “The join (FR-142) and the report (FR-144) read `FindingRecord` entries by tool name and know nothing of OSV-Scanner’s layout, so a later scanner using a new adapter and tool name is added, and the join is unchanged except for one row of its assessed-types table.” |
