---
id: SR-20261123063609
title: "Code review of Quoin PR 1021 (PLAT-1790 swap-and-refuse coverage)"
type: SpecReview
analysis: base
scope: "agent-ix/quoin@8a4fc24f2987ea291d89d05f48a0b005cc0dd353; rust/crates/quoin-quire/tests/analysis_counting.rs, rust/crates/quoin-cli/tests/tc_ver80_differential_report_command.rs"
review_set: subset
---

# Code review of Quoin PR 1021

## Summary

Ticket: PLAT-1790. Reviewed only `origin/main...8a4fc24f2987ea291d89d05f48a0b005cc0dd353` (two test files, +623 lines). The Rust lane (`rust-review`, with the repo's `.claude/skills/rust-style`) is folded in. Expected outcomes were checked against the FR-116 and FR-184 text and against `quoin-evidence/src/qualify/premises.rs`.

What holds: every bridge case in `analysis_counting.rs` is bracketed by `assert_healthy_control`, which asserts `QualifiedRecord` and exactly one binder before and after the swap. `assert_refused_exactly` compares the full `(cause, premise)` set, so an extra or missing refusal fails. The revision cases (wrong-revision for a mismatch, unevaluated for None or empty on either side), the actor cases (unevaluated when missing, qualification-rejected when the scope excludes the target) and the method-pairing case match FR-116-AC-6, AC-7 and AC-11 word for word. The swapped admitted actor that qualifies under `signer-2` is a good FR-116-AC-18 check.

What does not hold: FR-116's `selection-mismatch` cause cannot be produced at all, but five tests trace FR-116-AC-8. The CLI test never starts from a qualifying record. Its store-unchanged check restores the tampered file before it compares. Some expected pairs come from the implementation, not the contract. Several sub-cases assert only "not qualified".

PR body claims, checked: FR-065-AC-25 `policy_weakened` is on main and true. #1012 merged as 697a583c. `tc_1511_policy_history_refuses_weakened_approval` covers required->off, signature-kind and approver widening, and required->optional->required. The optional->required and unchanged cases live in the sibling test `tc_1511_policy_history_uses_normal_claim_rules_after_an_allowed_transition`, not in the named test. The "FR-116 has no draft status text" claim is literally true, since the word "draft" does not appear. But the stale "implementation remains planned" status text remains. See the gap-analysis artifact.

## Verdict

**CONDITIONAL.** Five medium and three low findings. No defects in production code are introduced (the PR is tests only).

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-116-AC-8 "a different selection is selection-mismatch" is traced by five tests, yet the qualifier's `Cause` enum has no SelectionMismatch, so the cause can never be produced. The "selection" swap test changes the FR-184 claim selection and expects record-conflict. The ticket's selection swap and exact-cause requirement are unmet, and the confirmed gap has no regression test. | rust/crates/quoin-quire/tests/analysis_counting.rs:1557-1599; rust/crates/quoin-evidence/src/qualify/mod.rs:40-59 |
| FND-002 | medium | The CLI test's healthy control is already non-qualified (no reliance decision or admitted actor). So every `!qualified-record` assertion is vacuous, and the test does not start from a qualifying record as the PR body claims. The ticket's real admitted report qualifying through the CLI, and its forward/inverse lookup, are absent. | rust/crates/quoin-cli/tests/tc_ver80_differential_report_command.rs:485-509 |
| FND-003 | medium | The store-unchanged assertion runs after the test restores the tampered projection file. A matrix run that rewrote or repaired that file (FR-116: qualification SHALL NOT write or repair the store) would be masked. Snapshot after tampering, run, compare, then restore. | rust/crates/quoin-cli/tests/tc_ver80_differential_report_command.rs:521-546 |
| FND-004 | medium | FR-184-AC-17 (a positive formal-rule qualification) is traced by a refusal-only test. For FR-184-AC-18 the required reason `draft_rule_in_judgement` is never asserted, because `premise_of` strips the detail. | rust/crates/quoin-quire/tests/analysis_counting.rs:1557, 1582-1596 |
| FND-005 | low | Several expected pairs come from the implementation's early-return cascade, not from FR-116 or FR-184 text: `(Unevaluated, bounded reliance)` added to every record-conflict and to the formal-rule refusal. This contradicts the PR's "oracle from the contract text" claim, and it locks incidental behaviour. | rust/crates/quoin-quire/tests/analysis_counting.rs:1331-1334, 1400-1403, 1563-1566, 1588-1595 |
| FND-006 | medium | Sub-cases assert only "not qualified", never the exact cause: a retained entry flipped to Fail (FR-116 names `failed`), the suite swap, the criterion-scoped actor, and the component-only cases. `assert_all_absent` also passes vacuously when there are no causes. | rust/crates/quoin-quire/tests/analysis_counting.rs:1365-1369, 1408-1413, 1449-1455, 1650-1693 |
| FND-007 | low | `swapping_the_configuration_or_reliance...` traces FR-116-AC-10 but never exercises "a reused configuration name with different required arguments/features is rejected". The configuration case asserts `stale` (FR-093 invalidation, FR-116-AC-12), not the AC-10 rejection. | rust/crates/quoin-quire/tests/analysis_counting.rs:1511-1525 |
| FND-008 | low | `refusal_set` dedups, so a duplicated refusal for one premise passes as exact. The PR body cites tc_1511 cases that are in a sibling test, not in the named test. | rust/crates/quoin-quire/tests/analysis_counting.rs:1212-1221 |

## Examined scope

```yaml
clean: false
scope:
  - {id: FR-116-AC-6, path: spec/functional/FR-116-owner-qualified-quire-evidence.md, role: examined, excerpt: "Missing authority is unevaluated; evaluated authority excluding the target/method/use is qualification-rejected. An accepted advisory use or limitation excluding the required act grants no positive verification decision."}
  - {id: FR-116-AC-7, path: spec/functional/FR-116-owner-qualified-quire-evidence.md, role: examined, excerpt: "A mismatched performed revision is wrong-revision, an unavailable performed revision is unevaluated, and a current HEAD paired with an unrelated historical record cannot make it current. No persisted run commit is introduced."}
  - {id: FR-116-AC-8, path: spec/functional/FR-116-owner-qualified-quire-evidence.md, role: examined, excerpt: "A record, source/tool observation or execution witness belonging to a different selection, record/entry, obligation or method cannot qualify the requested act; missing correlation is unevaluated, a different selection is selection-mismatch, and contradictory correlations are record-conflict."}
  - {id: FR-116-AC-10, path: spec/functional/FR-116-owner-qualified-quire-evidence.md, role: examined, excerpt: "The actual tool/version, adapter when required and effective configuration must match the bounded producer definition; a reused configuration name with different required arguments/features is rejected, and missing observations remain unevaluated."}
  - {id: FR-116-AC-11, path: spec/functional/FR-116-owner-qualified-quire-evidence.md, role: examined, excerpt: "The declared method/act-kind pair is checked through its original catalog and owner definition; absent catalog/premises do not become acceptance, unknown or incompatible evaluated methods are rejected, and evidenceKind alone grants nothing."}
  - {id: FR-116-AC-12, path: spec/functional/FR-116-owner-qualified-quire-evidence.md, role: examined, excerpt: "Invalidated/expired owner context is stale; absent observation is unevaluated; a newer failed/error canonical observation prevents an older passing act from qualifying while retaining both original outcome and owner freshness detail."}
  - {id: FR-116-AC-14, path: spec/functional/FR-116-owner-qualified-quire-evidence.md, role: examined, excerpt: "Two owner targets with a shared rendered ID are not collapsed; unknown or ambiguous targets cannot qualify, and a shared suite/test-case reference does not grant a sibling target a decision without its own observed act."}
  - {id: FR-116-AC-15, path: spec/functional/FR-116-owner-qualified-quire-evidence.md, role: examined, excerpt: "A genuine healthy audit with no positive owner premises, or a matching FR-093 reliance assessment alone, does not qualify an act; the bridge consumes the owner result rather than a precomputed healthy flag or a second checker."}
  - {id: FR-116-AC-18, path: spec/functional/FR-116-owner-qualified-quire-evidence.md, role: examined, excerpt: "The result exposes the host-admission and bounded authority basis for every positive decision; an unadmitted self-asserted context cannot obtain a positive decision merely by copying the expected field values."}
  - {id: FR-184-AC-17, path: spec/functional/FR-184-analysis-act-producer-definition.md, role: examined, excerpt: "A report qualified with expected run_kind: formal_rule whose projection has the same run kind and one matching formal_rule_selection entry with state approved for each selected claim may qualify; the qualifier checks the selected set."}
  - {id: FR-184-AC-18, path: spec/functional/FR-184-analysis-act-producer-definition.md, role: examined, excerpt: "A formal-rule qualification whose projection lacks run_kind: formal_rule, whose report has no selection, whose mapping is missing, incomplete, extra, duplicate, or source-mismatched, or whose selected claim maps to state draft yields qualification-rejected with reason draft_rule_in_judgement and no positive decision."}
  - {id: FR-153-AC-8, path: spec/functional/FR-153-analysis-result-counting-by-disposition.md, role: examined, excerpt: "A bound pass entry with a healthy quoin evidence audit and no FR-116 decision is not counted: no qualified-record status and no record_binders claim."}
  - {id: FR-065-AC-25, path: spec/functional/FR-065-change-assurance-verification.md, role: context_only, excerpt: "Given a verified same-record predecessor chain, each adjacent required: true to false transition ... makes review invalid with policy_weakened and null claimed_approval ... A three-revision chain of required, optional, required remains invalid with policy_weakened"}
```
