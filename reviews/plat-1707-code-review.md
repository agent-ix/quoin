---
id: SR-20261114002431
title: "Code review — Quoin filament-analyze migration"
type: SpecReview
analysis: code-review
scope: "agent-ix/quoin; rust/Cargo.toml, rust/crates/quoin-cli, rust/crates/quoin-evidence, rust/crates/quoin-quire, rust/crates/quoin-core, rust/deny.toml, tests/fixtures/evidence; PLAT-1707"
review_set: subset
---

## Summary

Reviewed PR #911 at `4cb8ef3e` against `origin/main`, including the Rust adapter migration, changed tests and fixtures, and the affected FR-151/FR-153 contracts. The implementation follows FR-151's new `<analysisKind>` identity but conflicts with FR-153's retained entry identity requirement.

## Verdict

**CONDITIONAL** — reconcile the entry identity contract before merge; the adapter behavior and fixture migration otherwise align with the updated FR-151 behavior.

## Coverage

Examined these spec units: FR-151-AC-1 (agreement creates a pass entry with symbol `<analysisKind>`); FR-151-AC-12 (only the selected tool names choose this adapter); FR-153's requirement that the symbol `<analysisKind>::<requestDigest>` remain the retained entry identity; and FR-153-AC-9 (source-symbol query for that identity returns no claim). Also inspected the changed Rust adapter, registry, callers, tests, Cargo dependency/lockfile, and fixture set. Focused checks are recorded on the PR; builds and aggregate gates were not rerun per review brief.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The adapter collapses report identity to `analysisKind`, conflicting with FR-153's required `<analysisKind>::<requestDigest>` retained entry identity. FR-151 now specifies `<analysisKind>`, so the specifications cannot both be met; reconcile FR-153 before treating this behavior as final. | rust/crates/quoin-evidence/src/adapters/differential_report.rs:86 |
