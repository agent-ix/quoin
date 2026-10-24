---
id: SR-183
title: CRLF-safe plugin workflow release follow-up
type: SpecReview
analysis: code-review
scope: ix-flow frontmatter fix and Quoin runner minimum, LF/CRLF acceptance checks, Windows pre-tag CI, current workflow help, release versions and documentation
review_set: subset
---

## Summary

Implementation self-review of the release follow-up. Quoin's unpublished 0.29.0
candidate passed three native platforms but failed the Windows workflow gate.
The failure was reproduced with CRLF skill frontmatter and fixed in ix-flow PR
35 (commit b1a4026dd5481c6b645bfafb91468c5ace8e3b74). The original tag and failed
run remain intact. Quoin 0.29.1 requires ix-flow >=0.3.1.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | ix-flow sliced frontmatter before the closing delimiter's LF, leaving an orphan CR in CRLF files. The final plain scalar gained that CR, so a declared provider name failed exact lookup and an unquoted workflow directory could resolve incorrectly. | ix-flow/src/workflow-core/plugin.ts; native run 40125266890 |
| FND-002 | medium | Quoin's historical workflow help still described bundled workflows and omitted the new explicit plugin --path argument. | rust/crates/quoin-cli/src/help.rs; FR-020 |

## Dispositions

FND-001: fixed in ix-flow 0.3.1. Preserve the existing terminating line break in
the YAML slice. New LF/CRLF cases exercise actual skill loading, final block-list
provider names and a real provider process. Both CRLF cases failed before the
fix; all 72 tests and the CI production security audit pass afterward. The audit
required updating Zod to patched 4.6.7; no exemption was added.

FND-002: fixed. The two changed workflow routes render their actual Clap grammar.
The historical capture remains unchanged; the replay test asserts the new path
contract for these routes and retains byte comparisons for the other routes.

## Verification

- Local Quoin CLI tests, workspace formatting and all-target/all-feature Clippy
  pass after the follow-up changes.
- The workflow checker runs the complete lifecycle with both LF and CRLF copied
  plugin skills. It retains missing/invalid/modified evidence refusals, profile
  and catalog checks, explicit human gates and runner exit/version checks.
- CI adds a Windows candidate workflow job before tagging. All four native
  release targets and the npm launcher retain the same acceptance checks.
- The complete local smoke passes against published ix-flow 0.3.1 for both LF and CRLF skill files.
- New minimum ix-flow 0.3.1 prevents using the known broken runner. Matching
  Quoin CLI and plugin manifests are 0.29.1.
- The language check was removed at the owner's explicit direction in PR 696.
  No bypass is configured and no failed result is reported as passing.

## Verdict

The parser and help findings are fixed. Corrected
Windows CI and the complete native/npm release gates must pass before the
release is reported complete. This self-review is not independent approval.
