---
id: SR-182
title: Quoin owned workflow runtime and 0.29.0 release review
type: SpecReview
analysis: code-review
review_set: subset
scope: Quoin-owned workflows, native launch and invariant provider, release checker, live catalog validation, skills, requirements, packaging workflows, dependency security updates and 0.29.0 versions
---

## Summary

Implementation self-review using the repository Rust idioms and dev-tools Rust review checklist. This is not independent approval or a human acceptance receipt. Measured with the worktree native Quoin build based on 105d6392, manifest version 0.29.0; installed Quire CLI 0.36.2 / engine 0.50.2 validates the authored documents.

Reviewed all changed files, including plugin-owned review/to-plan assets, Rust provider and launcher, the separate native release checker, linked Quire frontmatter API, dependency/lock changes, skills, setup/release documentation, requirements, Make targets and GitHub workflows. No live Jev lens evaluation or measurement scoring behavior changes; AP-201 and AP-202 do not scope this change.

The runtime no longer locates or installs the archived workflow provider. Validation reads actual documents through the installed catalog, requires base plus the selected specification lenses, rejects duplicate or mismatched evidence, checks the current AssuranceProfile policy contract and revalidates before human acceptance. The runner retains lifecycle and acknowledgement ownership. Legacy receipts remain unverified rather than being backfilled.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Joining output readers after killing a timed-out child can wait indefinitely if a descendant keeps a pipe open, violating the launch deadline. | rust/crates/quoin-cli/src/flow.rs:195; FR-021 |

## Dispositions

FND-001: fixed. Output readers return bounded results through channels; receiving stdout and stderr shares the original launch deadline. A closed or overdue channel produces a refusal, and no synchronous thread join can extend the deadline.

## Verification

- Workspace tests: 2,028 passed, zero failed; six pre-existing ignored development/live cases. No new ignore or skip was added.
- Workspace formatting and all-target/all-feature Clippy with warnings denied pass after the launcher correction.
- cargo-deny advisories, bans, licenses and sources pass. Fixed vulnerable lock entries and replaced the unmaintained YAML implementation with the maintained yaml_serde package already used by linked Quire; no advisory exemptions were added.
- Native lifecycle smoke passes against a physically relocated binary, published ix-flow 0.3.0, isolated home, synthetic authored catalog contracts and unavailable network proxies. Exercises missing base/files/tasks, wrong analysis, invalid Findings, edits after validation, full lens selection, active profile policy, missing catalog/schema vocabulary, automatic gate override refusal, explicit acknowledgement persistence, incompatible runner refusal and child exit propagation.
- CI retains the existing test lanes and adds installed workflow behavior. Every supported native archive and the published npm launcher run the same behavioral acceptance checker. Shipping archives still contain one executable.
- Read-only legacy inventory does not validate or retrospectively approve historical reviews. Document presence is not reported as an affected-review count.

## Packaging correction

The owner directed normal ix-flow plugin packaging after this first review.
Definitions now ship adjacent to spec-review/spec-to-plan, their SKILL.md files
declare the relative workflows directory and native provider, and ix-flow reads
that installed skill directly. Binary embedding, extraction and immutable
snapshot code were removed. The convenience launcher requires an explicit
--path. CI supplies copied released plugin files without an agent host.
The corrected CLI test suite, workspace formatting and all-target/all-feature
Clippy pass. Direct ix-flow smoke passes with copied plugin skills and a relocated
CLI; it checks the recorded installed-skill path, absence of workflow extraction,
matching Claude/Codex plugin versions, context initialization through the optional
CLI wrapper, and the same actual evidence validation and human gate scenarios.
The release workflows now supply plugin files from the release tag. Updated CI
and all four native release checks remain required before release.

## Owner decision on the separate language check

On 2026-10-24 the owner explicitly directed removal of the blocking language
check rather than bypassing or disguising its result. The Quoin-specific
language-boundary workflow is removed, and its matching required-check ruleset
is removed from GitHub. No bypass remains configured. The previous refused
results remain historical evidence; this change does not report that language
check as passing. Rust formatting, Clippy, supply-chain checks, workspace tests
and installed workflow behavior remain in CI and the release process.

## Verdict

No unresolved findings in this self-review. Source checks do not establish cross-platform archive or public npm behavior; those remain required release workflow gates before reporting the release complete.
