---
id: SR-20261113052922
title: "Base spec review of licence and ban policy findings (FR-150; FR-041, FR-142, FR-144 amendments)"
type: SpecReview
analysis: base
review_set: subset
scope: 'agent-ix/quoin; PR #847 (gh pr diff 847): spec/functional/FR-150-licence-and-ban-policy-findings.md, spec/functional/FR-041-sbom-inventory-evidence.md, spec/functional/FR-142-advisory-join-onto-soup-records.md, spec/functional/FR-144-soup-report.md, spec/functional/index.md, spec/spec.md'
---

# SR-20261113052922: Base spec review of licence and ban policy findings (FR-150; FR-041, FR-142, FR-144 amendments)

## Summary

Ticket: PLAT-1671. Checklist pass over the PR #847 diff: ID formats, AC quality, coverage, traceability, ticket criteria, consistency with merged FR-141..FR-144, FR-149, engineering-assurance FR-044 and NFR-030. FR-150 is the next free id on main (highest merged is FR-149; no open PR claims FR-150). IDs, AC numbering and index/spec.md entries are consistent; quire validate passes the four touched FRs. The ticket's six criteria (person-authored policy with no shipped default or licence list; disallowed/banned component as a finding tied to its record by purl; no licence data is unknown, never allowed; findings reach the SOUP report with a person-owned disposition; unmatched findings reported, not dropped; refused reads not clean) are each stated. No paywalled standard names or text (SPDX identifiers only), no digest or pin, no roadmap content. The FR-142-AC-13 revision is explicit and backed by the new FR-142-AC-28, but it rests on a cargo-deny code assignment the spec defers to fixtures that cannot supply it.

## Verdict

FAIL on one high and two medium findings.

## Findings

| ID | Severity | Summary | Refs |
|----|----------|---------|------|
| FND-001 | high | A cargo-deny finding carries only its `code` (FR-034-AC-21), so 'belongs to its bans or licenses check' needs a code-to-check table. The spec defers it: 'the code assignment is fixed by the FR-034 captured fixtures; a code assigned to neither check is set aside'. The captured fixtures (rust/crates/quoin-evidence/tests/fixtures/cargo-deny-*.jsonl) hold exactly one diagnostic code, `banned`; the licence checks appear only as summary counts. Under the rule as written every licence code (`rejected`, `unlicensed`, ...) is assigned to neither check and is set aside, which contradicts FR-142-AC-13 and FR-142-AC-28 (a licence finding is joined). The merged test tc_1516_009 already uses `rejected`, a code no fixture assigns. The set is also unbounded the other way: warning-level codes such as `duplicate`, `wildcard`, `license-not-encountered` or `unmatched-skip` are bans/licenses diagnostics about configuration or graph shape, and some have no crate, so read as policy findings they would put unresolved rows in the first group or fill the unmatched list. Fix: enumerate in FR-142 (or FR-034) the cargo-deny 0.19.8 codes that are policy findings and the ones set aside, capture a licence-diagnostic fixture (a `rejected` and an `unlicensed` crate) beside the existing ones, and tie AC-28 to it. | spec/functional/FR-142-advisory-join-onto-soup-records.md:53-58, 173, 188 |
| FND-002 | medium | FR-041-CON-3 is unchanged: 'quoin SHALL NOT judge the inventory's contents. Whether a component's version or licence passes is the auditor's and the consumer's gate policy.' FR-150 is quoin evaluating each component's licence and recording `licence-denied` / `licence-not-allowed`. FR-150-CON-2's distinction (quoin applies a person's policy and does not decide acceptability) is sound, but CON-3 is not amended to say so, and the new FR-041 paragraph reads 'FR-041-CON-3 stands: the reader decides whether no licence passes', which says the opposite of what is meant (the reader decides nothing). Fix: reword CON-3 to scope it to the reader and note FR-150 applies a person-authored policy, and change the sentence to 'the reader judges no licence'. | spec/functional/FR-041-sbom-inventory-evidence.md:91-96, 119 |
| FND-003 | medium | NFR-030 constrains FR-141..FR-144 and FR-149 and its Scope names 'the FR-149 adapter that feeds the advisory join'. FR-150 is a second adapter feeding the same join, emitting severity words, rulesets and messages that reach the report, yet the PR adds no `constrains` edge to FR-150, no scope line and no NFR-030 criterion over a refused policy or a refused SBOM (the analogue of NFR-030-AC-4 for grype). NFR-030's 'failed or suppressed scans read as clean = 0' and 'scores, ranks or classes emitted = 0' thus do not reach licence-policy. Fix: add FR-150 to NFR-030's relationships and scope, and an AC: over a refused policy, a refused SBOM and a vacuous licence-policy scan, no record is shown with no policy findings. | spec/non-functional/NFR-030-soup-views-state-and-never-derive.md:6-27; spec/functional/FR-150-licence-and-ban-policy-findings.md:1-22 |
