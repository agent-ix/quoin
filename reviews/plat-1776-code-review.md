---
id: SR-20261117175239
title: "Code review — Quoin Windows release smoke stubs"
type: SpecReview
analysis: code-review
scope: "agent-ix/quoin; rust/crates/quoin-workflow-check/src/smoke.rs"
review_set: subset
---

## Summary

Reviewed PLAT-1776's Windows command stubs in the release smoke. The change uses a cmd-compatible body for the `.cmd` fixtures, preserves the Unix shell bodies and the expected success/refusal statuses, and leaves the fatal smoke path intact.

## Verdict

**PASS** — no defects found in the changed smoke behavior or its assertions.

## Review Scope

- PLAT-1776 AC-1: The Windows `.cmd` stubs use Windows command syntax and emit the expected JSON.
- PLAT-1776 AC-2: The existing formalization success scenario advances to `checked` with a clean FSL outcome and matching trace.
- PLAT-1776 AC-3: The refusal fixture retains exit status 20; the retry scenario continues to require `qsl_check_failed`, retained diagnostics, and the expected retries.
- PLAT-1776 AC-4: The release workflow invokes the smoke checker against the packaged executable for each native artifact, including Windows x64.
- `rust/crates/quoin-workflow-check/src/smoke.rs`: inspected the changed fixture writer and its formalization success/refusal scenarios.
- `.github/workflows/build-test.yml`: inspected the existing Windows workflow behavior job used to exercise the same smoke on a Windows runner.
- `.github/workflows/native-release-build.yml`: inspected the packaged-binary smoke call and fatal exit handling; unchanged by this PR.

## Evidence

- `cargo fmt --all -- --check`: passed.
- `git diff --check origin/main...HEAD`: passed.
- GitHub CI run 44162283947: `Windows workflow behavior` passed; `Rust supply chain` passed; the separate `Rust` aggregate job was still running at review completion.
- `quire validate` on the scratch SpecReview exited 0. The installed module emitted 25 `UnknownEdgeType` diagnostics for unrelated archetypes. The local Quoin binary resolves to 0.35.0, older than the reviewed repository v0.36.2, so `quoin write` was not used.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |
