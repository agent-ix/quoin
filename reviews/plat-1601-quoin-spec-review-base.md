---
id: SR-20261113052917
title: Base spec review of the Grype advisory adapter and join (FR-149, FR-142, FR-144)
type: SpecReview
analysis: base
review_set: subset
scope: 'agent-ix/quoin; PR #846 (gh pr diff 846 at 888cbecb): spec/functional/FR-149-grype-advisory-report-adapter.md, spec/functional/FR-142-advisory-join-onto-soup-records.md, spec/functional/FR-144-soup-report.md, spec/functional/index.md, spec/spec.md'
---

# SR-20261113052917: Base spec review of the Grype advisory adapter and join (FR-149, FR-142, FR-144)

## Summary

Ticket: PLAT-1601. Checklist review of the quoin PR #846 diff only (`gh pr diff 846`, head 888cbecb): new FR-149, amended FR-142 (coverage table, purl match rule, AC-19..AC-25) and FR-144 (AC-16, AC-17), plus the two index lines. Checked against the merged FR-033, FR-034, FR-041, FR-133, FR-141..FR-144 and NFR-030 on main, and cross-references only against engineering-assurance PR #221 (FR-044) and engineering-assurance FR-039 on main.

`quire validate --scope . <3 changed FRs>` exits 0 (quire 0.44.0, quoin 0.30.1). FR-149 is the next free id: FR-145..FR-148 are held by open PR #845 and no other open PR touches FR-149 or above, so there is no collision. Applicable AssuranceProfiles AP-201 and AP-202 (`review_policy: require`) govern finding-quality measurement and Jev lens evaluation, not the SOUP join, so they do not apply; this is a direct review.

What holds: join by purl, one newest scan per tool across suites, resolution only via `disposition_clause` under FR-133, no derived score or severity mapping (FR-149 Behavior, FR-142-AC-9), an empty `matches` in a valid envelope is a clean scan (FR-149-AC-2), a missing envelope and a non-empty `ignoredMatches` are refused (FR-149-AC-3/4), a refused or failed scan is not run (FR-142-AC-22), unmatched grype findings are listed (FR-142-AC-23), and `lake`/`generic` records are `unsupported` and never clean (FR-142-AC-20). No paywalled standard name or text appears. No digest, pin or tracking record is introduced; the database build time recorded as the ruleset is evidence content, not a pin.

## Verdict

FAIL. FND-001 and FND-002 are high. Mergeable once the findings in this and the sibling PLAT-1601 quoin SRs are fixed in this PR.

## Findings

| ID | Severity | Summary | Refs |
|----|----------|---------|------|
| FND-001 | high | FR-144-AC-16 and FR-144-AC-17 sit after a blank line that ends the Acceptance Criteria table, so they are a headerless second table that no tool reads as criteria. `quoin matrix --repo . --json` on the PR head lists FR-144-AC-1..AC-15 and no AC-16 or AC-17, so no test can ever be tagged to them and the matrix will never show them untested. Fix: delete the blank line at FR-144 line 72 so both rows join the table. | spec/functional/FR-144-soup-report.md:72-74 |
| FND-002 | high | A grype match with no purl is recorded with path `<name>@<version>` (FR-149 line 50, AC-7). FR-142's primary match rule maps a `<name>@<version>` path to `pkg:cargo/<name>@<version>` (FR-142 lines 65-66), so an npm, PyPI, Go or deb match without a purl attaches to a cargo record of the same name and version. That contradicts FR-142 line 70-71 ('A finding never attaches to a record of another purl type'), and an implementer must pick one. It also falsifies 'as every grype finding's is' (line 67). Fix: scope the `<name>@<version>` rule to cargo-audit and cargo-deny findings, and list a grype finding with no purl as unmatched (reason e.g. `no purl`), with an AC. | spec/functional/FR-149-grype-advisory-report-adapter.md:50, 80 |
| FND-003 | medium | FR-149-AC-10 cites bare `FR-039`, which in quoin is the mutation-score threshold, not an SBOM fixture set; the intended source is `ix://agent-ix/engineering-assurance/FR-039`. That FR's fixtures are a Rust crate, an npm package, a Python project and a container image with no `os-release` (engineering-assurance FR-039 Inputs); there is no Go module fixture, and without a distro Grype cannot match deb/apk/rpm packages, so a capture over them yields no OS-package match and FR-142-AC-19's container case has no advisory to receive. 'checked in unedited' is also not checkable by Inspection, and the truncated-file fixture of FR-149-AC-3 is an edit by construction. Fix: cite the EA FR by ix:// id, name fixtures that do yield npm, PyPI, Go and OS-package matches (or drop the Go and OS cases from AC-19 and AC-10), and drop 'unedited' or restrict it to the captured success reports. | spec/functional/FR-149-grype-advisory-report-adapter.md:83 |
| FND-004 | low | Two-tool wording survives the three-tool amendment: 'A finding carries no purl. Both adapters record the crate ...' (line 65, now false for grype), and 'the newest retained scan, of either tool' (lines 77 and 108). Fix: say 'cargo-audit and cargo-deny findings carry no purl' and 'of any tool'. | spec/functional/FR-142-advisory-join-onto-soup-records.md:65, 77, 108 |
