---
id: SR-20261114002403
title: "Code review of Quoin PR 891 (VER-97 read-time analysis evidence)"
type: SpecReview
analysis: base
scope: "agent-ix/quoin@028b3273ac8a6091f4bbf177cbe7270744253abd; rust/crates/quoin-assurance/**, rust/crates/quoin-cli/src/{analysis_evidence.rs,assurance.rs,matrix.rs,run_observation.rs}, rust/crates/quoin-core/src/ops/evidence/records.rs, rust/crates/quoin-evidence/**, rust/crates/quoin-quire-types/src/lib.rs, rust/crates/quoin-quire/src/evidence.rs"
review_set: subset
---

# Code review of Quoin PR 891

## Summary

Ticket: VER-97. Reviewed only `origin/main...HEAD` at `028b3273ac8a6091f4bbf177cbe7270744253abd`. Rust lane (`rust-review`) folded in. The code keeps checkout SHA, CI association, signer, signature result and selected HEAD in CLI-only observation types. The only persisted addition is the FR-151 allowlisted projection beside the run; no persisted observation fields, added digest, or commit pin appeared. The current FR-093 trust assessment and release workflows are unchanged.

Read-time actor admission compares the full Git-resolved pair-introducing commit with `signature_result.verified_commit_id`, checks the current run and projection bytes, and maps absent, malformed, mismatched, tag-kind and unresolved identities to no admission. Matrix and assurance attach items by full `TargetLocus`, suppressing ambiguous identities. Tests for FR-153-AC-16/18 are tagged and `quire matrix` binds them.

`cargo fmt --all --check` passed via the required locked wrapper. Workspace clippy could not complete: the first run stopped because the generated semantic schema under `node_modules` was absent; the install attempt failed on `npm.ix` HTTP 401 (and an initial default-store attempt failed with SQLite unable to open its database). The targeted workspace `cargo test -p quoin-cli -p quoin-assurance -p quoin-evidence -p quoin-quire --locked` also stopped at that missing generated schema; `cargo deny check` exited 1 because its advisory database lock under `/home/peter/.cargo` is read-only. Installed quoin resolves as 0.30.1 while the supplied release notice names 0.30.0; `spec-artifacts-process` resolves at v0.31.0.

## Verdict

**FAIL — one medium code finding.** Build/test evidence is incomplete because dependency setup is blocked by registry authentication.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | An empty CI association member is refused before qualification, contrary to FR-153-AC-15 requiring an unevaluated execution-association result | rust/crates/quoin-cli/src/run_observation.rs:104-118 |

## Examined scope

```yaml
clean: false
scope:
  - {id: FR-153-AC-12, path: spec/functional/FR-153-analysis-result-counting-by-disposition.md, role: examined, excerpt: "Both views expose the selected analysis result, FR-078 category, record locators and each unmet premise."}
  - {id: FR-153-AC-13, path: spec/functional/FR-153-analysis-result-counting-by-disposition.md, role: examined, excerpt: "An observation for another record cannot qualify the selected analysis result; the selected entry, obligation and declared method come from the canonical record and catalog."}
  - {id: FR-153-AC-14, path: spec/functional/FR-153-analysis-result-counting-by-disposition.md, role: examined, excerpt: "Missing or malformed selected_head or checkout_sha remains unevaluated for performed revision; differing valid values yield wrong-revision; conflicting supplied selected_head values yield record-conflict."}
  - {id: FR-153-AC-15, path: spec/functional/FR-153-analysis-result-counting-by-disposition.md, role: examined, excerpt: "An absent CI association or one with an empty member remains `unevaluated` for execution association; a manual association yields `qualification-rejected`; none produces `qualified-record`."}
  - {id: FR-153-AC-17, path: spec/functional/FR-153-analysis-result-counting-by-disposition.md, role: examined, excerpt: "Invalid observation-file JSON/protocol, empty required `protocol` or `record`, wrongly typed or incomplete members, unknown members, empty/duplicate obligation ids, duplicate record selectors or selectors outside the selected canonical store are refused."}
  - {id: FR-153-AC-16, path: spec/functional/FR-153-analysis-result-counting-by-disposition.md, role: examined, excerpt: "The host resolves C's full Git object id and requires signature_result.verified_commit_id to equal it. C adds both selected files with bytes matching the read pair."}
  - {id: FR-153-AC-17, path: spec/functional/FR-153-analysis-result-counting-by-disposition.md, role: examined, excerpt: "Invalid observation-file JSON/protocol, wrongly typed or incomplete members, unknown members, duplicate record selectors or selectors outside the selected canonical store are refused."}
  - {id: FR-153-AC-18, path: spec/functional/FR-153-analysis-result-counting-by-disposition.md, role: examined, excerpt: "Actor authority remains unevaluated when C cannot be resolved or verified_commit_id is absent, null, malformed or differs from C's full Git object id, or when the supplied result is for a tag."}
  - {id: FR-153-CON-1, path: spec/functional/FR-153-analysis-result-counting-by-disposition.md, role: examined, excerpt: "The bridge SHALL add no analysis-specific field, status or report member to quire-rs."}
  - {id: FR-153-CON-2, path: spec/functional/FR-153-analysis-result-counting-by-disposition.md, role: examined, excerpt: "Counting SHALL be a read of the store and supplied host observations: no solver is run ... and nothing is written."}
  - {id: FR-153-CON-3, path: spec/functional/FR-153-analysis-result-counting-by-disposition.md, role: examined, excerpt: "Binding, per-suite supersession and the audit's failed-run rule SHALL stay as FR-030 and FR-032 define them."}
  - {id: FR-093-CON-1, path: spec/functional/FR-093-evidence-producer-trust.md, role: examined, excerpt: "Quoin SHALL NOT infer trust from producer name, vendor, version, or absence of a decision."}
  - {id: FR-093-CON-2, path: spec/functional/FR-093-evidence-producer-trust.md, role: examined, excerpt: "Quoin SHALL NOT run, certify, approve, or globally qualify a producer."}
  - {id: FR-093-CON-3, path: spec/functional/FR-093-evidence-producer-trust.md, role: examined, excerpt: "Quoin SHALL NOT silently convert invalidated or unobserved reliance to accepted reliance."}
  - {id: FR-040-AC-20, path: spec/functional/FR-040-assurance-case-view.md, role: examined, excerpt: "A derived Analysis solution node carries the FR-153/FR-078 result items for its exact target."}
  - {id: FR-115-AC-16, path: spec/functional/FR-115-evidence-backed-test-matrix.md, role: examined, excerpt: "With FR-153 analysis evidence supplied, each Analysis criterion receives only result items whose exact FR-078 target matches its identity; qualified claims appear as `qualified-record`, and every non-qualified cause retains its exact category, locator and unmet-premise detail."}
  - {id: FR-151-AC-15, path: spec/functional/FR-151-differential-report-v2-adapter.md, role: context_only, excerpt: "For each accepted disposition, the retained projection has exactly the FR-151 allowlist and has no symbol, request digest or other analysis identity."}
```

## Finding detail

**FND-001 (medium): empty CI members are rejected before qualification.** `read_run_observations` returns an input error at lines 113-116 for an empty `ci_association` member. FR-153-AC-15 explicitly requires a present association with an empty member to remain `unevaluated` for the execution-association premise, so the read produces no qualification report for that case. FR-153-AC-17 also says incomplete members are refused; the implementation exposes that AC-15/AC-17 tension as a refusal rather than the AC-15 result.
