---
id: SR-20261114002432
title: "Gap analysis — Quoin filament-analyze migration"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quoin; rust/Cargo.toml, rust/crates/quoin-cli, rust/crates/quoin-evidence, rust/crates/quoin-quire, rust/crates/quoin-core, rust/deny.toml, tests/fixtures/evidence, spec/functional/FR-151-differential-report-v2-adapter.md, spec/functional/FR-153-analysis-result-counting-by-disposition.md, spec/test/TC-001-differential-report-v2-adapter.md; PLAT-1707"
review_set: subset
relationships:
  - target: "ix://agent-ix/quoin/FR-151"
    type: references
  - target: "ix://agent-ix/quoin/FR-153"
    type: references
---

## Summary

Audited the changed implementation against the computed matrix and its owning FR-151/FR-153 requirements. The relevant FR-151 acceptance criteria are tagged, but the identity requirements conflict and the changed upstream dependency's Windows-build constraint has no tagged verification.

## Verdict

**FAIL** — two unresolved specification/evidence gaps affect the migrated adapter contract and its upstream dependency.

## Coverage

Plan completion: not assessed.

Examined FR-151-AC-1 (entry symbol), FR-151-CON-2 (record schema and projection pairing), FR-151-CON-4 (all release targets), FR-153's retained entry identity statement and FR-153-AC-9 (source-symbol query), and TC-001 step 4 (new and retired tool names). The computed `quire matrix --scope . --format json` showed FR-151-AC-1 through AC-12 and AC-14/15 tagged (AC-13 is explicitly Inspection), and FR-153-AC-7/9 tagged. FR-151-CON-2 and FR-151-CON-4 are untagged. The `quoin gap-analysis` command is unavailable in the installed CLI, so this is a manual matrix and changed-surface review; the optional semantic review was skipped. Reverse-gap inspection covered the changed adapter, registry, call sites, tests, Cargo manifests/lockfile and fixture corpus; no new behavior outside these declared requirements was found.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-151 requires `<analysisKind>` as the run-entry symbol while FR-153 requires `<analysisKind>::<requestDigest>` as the retained entry identity. The implementation and changed tests follow FR-151, leaving FR-153's identity contract unsatisfied and making the spec contradictory. | spec/functional/FR-151-differential-report-v2-adapter.md:215; spec/functional/FR-153-analysis-result-counting-by-disposition.md:288 |
| FND-002 | medium | The change moves Quoin to the filament-analyze `main` dependency, but FR-151-CON-4 requires that dependency to build on every release target including Windows, and the computed matrix marks this constraint untagged. The ticket's recorded focused checks do not supply that target evidence. | spec/functional/FR-151-differential-report-v2-adapter.md:193; rust/Cargo.toml:81 |
