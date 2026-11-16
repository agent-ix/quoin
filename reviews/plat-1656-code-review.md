---
id: SR-20261116004344
title: "Code and Rust review of PLAT-1656 risk management plan dossier"
type: SpecReview
analysis: code-review
scope: "agent-ix/quoin; PLAT-1656 PR #948 patch against peter/plat-1619-anomalies: rust/crates/quoin-cli/src/dossier.rs, rust/crates/quoin-dossier/src/gap.rs, rust/crates/quoin-dossier/src/lib.rs, rust/crates/quoin-dossier/src/risk_plan.rs, rust/crates/quoin-dossier/src/sources.rs, rust/crates/quoin-dossier/tests/tc_1437_generators.rs, rust/crates/quoin-dossier/tests/tc_1617_verification_report.rs, rust/crates/quoin-dossier/tests/tc_1618_soup.rs, rust/crates/quoin-dossier/tests/tc_1619_anomalies.rs"
review_set: subset
relationships:
  - target: "ix://agent-ix/quoin/FR-200"
    type: references
  - target: "ix://agent-ix/quoin/FR-191"
    type: references
---

## Summary

Reviewed PR #948's patch against `peter/plat-1619-anomalies`, including the Rust implementation, CLI adapter, trace-tagged tests, and applicable FR-200/FR-191 behavior. The plan renderer and typed source path follow the selected policy-only design, but the patch reverses the required order of the two new gaps and its AC-4 test does not exercise the record-reader/command failure path.

## Verdict

**CONDITIONAL** — two medium findings need disposition before merge.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The new gap kinds are ordered opposite to FR-191, so documents with neither policy nor plan record emit the two gaps in reverse order. | rust/crates/quoin-dossier/src/gap.rs:60-63,90-91; FR-191 |
| FND-002 | medium | The AC-4 test checks only `render` with a synthetic document; it does not verify malformed records reach source exit 3 without writing selected output. | rust/crates/quoin-dossier/src/risk_plan.rs:262-267; FR-200-AC-4 |

## Scope Examined

| Unit | Role | Evidence |
| --- | --- | --- |
| FR-200-AC-1 | examined | Unit test checks policy levels, region flag, cell and permanent plan gap. |
| FR-200-AC-2 | examined | Unit test checks both missing-source gaps and no default policy tables; its expected gap order conflicts with FR-191. |
| FR-200-AC-3 | examined | Unit test checks policies are ordered by id and no policy is selected. |
| FR-200-AC-4 | examined | Unit test reaches `render` directly with a synthetic empty flag; it does not call the typed record source or `generate`. |
| FR-200-AC-5 | examined | Unit test checks all three plan sections state the missing record. |
| FR-200-AC-6 | examined | Unit test compares repeated render bytes and checks no acceptance verdict wording. |
| FR-191 | examined | Checked the closed gap-kind order, source-error/no-write contract, generator registration and `Sources` extension. |
| AP-201 | context_only | Its declared scope is finding measurement and report quality; this PR renders dossier policy records, so it does not apply. |
| AP-202 | context_only | Its scope is live evaluation of Jev semantic lenses; this PR performs no such evaluation. |

## Review Evidence

- The production policy reader uses `quoin_quire::records::read` with the selected acceptability-policy type and maps reader failures to `SourceFailure`.
- `generate` computes selected documents before writing; the source-error/no-write engine path is inherited from FR-191 and remains in the common engine.
- `Gap::order` sorts by `GapKind::position`, so the enum/`ALL` order directly determines emitted gap order in both Markdown and JSON.
- Six FR-200 test functions carry the matching `#[trace(...)]` IDs. The installed matrices report those criteria untagged with `tagged-without-citing-matrix-row`; this is the known PLAT-1767 matrix classification limitation, not a missing test annotation in this patch.
- No vendored files, signatures, keys, SHA pins, or build-gate changes were added.
- The PR body reports `cargo test --locked -p quoin-dossier` passing (85 unit and 14 integration tests; 3 existing ignored) and focused Clippy passing. No independent Cargo build/test was run in this review; the active lead is running the workspace gate.
