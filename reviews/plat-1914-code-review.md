---
id: SR-967175026302197768900474181941935084796
title: "Code review — FR-208 clause approval reporting"
type: SpecReview
analysis: code-review
scope: "agent-ix/quoin; PLAT-1914; docs/how-to-verify-clause-discharge-approvals.md; rust/crates/quoin-cli/tests/tc_1488_pack_requirements_report.rs"
review_set: subset
---

## Summary

Reviewed PR #1089 at e0c775b43a294c2bd72193c46da432a0bb27aa5b for PLAT-1914. Code-review and its Rust lane found one trace-binding regression; the new FR-208-AC-5 assertions correctly exercise production report output.

## Verdict

**CONDITIONAL** — restore the successful generic direct-fact case without approvalRef, or correct the now-inaccurate FR-208-AC-3 test binding.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Adding approvalRef to the generic fact removes the only successful no-reference case in this test while retaining its FR-208-AC-3 tag. Earlier no-reference facts are submitted in atomically refused batches, so a regression rejecting direct facts without approvalRef on generic clauses now passes. Retain a successful no-reference case and assert its direct/compliant classification before recording the supplied-reference fact. | rust/crates/quoin-cli/tests/tc_1488_pack_requirements_report.rs:582 |

## Reviewed units and bindings

```yaml
scope:
- id: FR-208-AC-1
  path: spec/functional/FR-208-module-specific-discharge-approval.md
  role: examined
  excerpt: A binding clause with non-empty `approvalRoles` remains open when its direct
    fact has no `approvalRef`, or has an empty or whitespace-only reference.
- id: FR-208-AC-2
  path: spec/functional/FR-208-module-specific-discharge-approval.md
  role: examined
  excerpt: A binding clause with non-empty `approvalRoles` is direct only when its
    direct fact has a non-blank `approvalRef` that FR-211 verifies; the report retains
    the reference and shows the verified approver.
- id: FR-208-AC-3
  path: spec/functional/FR-208-module-specific-discharge-approval.md
  role: examined
  excerpt: A binding clause with empty `approvalRoles` continues to accept a direct
    fact without `approvalRef` as specified by FR-046.
- id: FR-208-AC-5
  path: spec/functional/FR-208-module-specific-discharge-approval.md
  role: examined
  excerpt: For an approval-class clause the report shows the approver verified under
    FR-211; for a clause with empty `approvalRoles` it preserves the supplied reference
    and does not claim to authenticate the approver, role, or authorization it represents.
- id: docs/how-to-verify-clause-discharge-approvals.md
  path: docs/how-to-verify-clause-discharge-approvals.md
  role: examined
  excerpt: For a clause with no approval roles, Quoin preserves a supplied reference
    without adding `verifiedApprover`. A missing or invalid approval leaves the clause
    open with a reason.
bindings:
- test_id: tc_208_003_clause_approval_roles_gate_pack_discharge_without_changing_generic_sets
  ac_id: FR-208-AC-1
  trace: correct
- test_id: tc_208_003_clause_approval_roles_gate_pack_discharge_without_changing_generic_sets
  ac_id: FR-208-AC-2
  trace: correct
- test_id: tc_208_003_clause_approval_roles_gate_pack_discharge_without_changing_generic_sets
  ac_id: FR-208-AC-3
  trace: wrong
- test_id: tc_208_003_clause_approval_roles_gate_pack_discharge_without_changing_generic_sets
  ac_id: FR-208-AC-5
  trace: correct
```

## Evidence and limits

- Focused test: `cargo test --locked --target-dir /home/peter/.cargo-target -p quoin-cli --test tc_1488_pack_requirements_report tc_208_003_clause_approval_roles_gate_pack_discharge_without_changing_generic_sets`: 1 passed, 0 failed.
- `cargo fmt --check -p quoin-cli`: passed. `git diff e0c775b^ e0c775b --check`: passed.
- Source inspection: `record_discharge_facts` in `rust/crates/quoin-assurance/src/discharge/store.rs:292-300` rejects the complete staged batch before writing any fact. Thus the initial refused batches do not establish generic no-reference acceptance. The later successful generic fact always includes approvalRef.
- Related production inspected: discharge verification (`check_direct`, `settle`), report serialization, CLI binding/report flow, and recording admission. No production edits or dependency moves occur in this PR.
- `quire trace --file rust/crates/quoin-cli/tests/tc_1488_pack_requirements_report.rs`: resolved with symbol-id-collision warnings elsewhere.
- `quire matrix --strict`: exit 1 with 632 untagged criteria, 11 criteria tagged only by ignored tests, and 250 design gaps across the repository. FR-208-AC-5 is tagged to this test; static tagging alone does not establish the FR-208-AC-3 behavior.
- Released `/tmp/regfinish-quoin-v0.37.0/quoin matrix --repo . --json`: exit 0. Authoring contract fetched with that Quoin 0.37.0 binary; module spec-artifacts-process@v0.36.0; validator quire 0.52.0 (engine 0.62.2).
- PATH Quoin resolves to `/home/peter/.npm-global/bin/quoin` 0.36.8 on terra; the dispatching lead was informed and explicitly selected released 0.37.0 instead.
- Focused clippy invocation exited 101 in unchanged `quoin-cli/build.rs` with 28 existing lint errors (including expect_used and format_push_string), before the changed test. Not classified as a PR defect. This initial invocation did not name target-dir, so its result is discounted under the repository idiom document; retry recorded below.
- `cargo deny check`: exit 1 because sandbox cannot exclusively lock read-only `/home/peter/.cargo/advisory-dbs/db.lock`; dependency advisory gate unavailable in this execution environment.
- Full workspace tests were not run: user instructions require focused tests only. Gap-analysis and spec-review are not applicable because there are no production, spec, or plan changes.
- Exact model identifier is not exposed by this harness; comment records model=unknown rather than inventing one.

- Explicit target retry: `cargo clippy --locked --target-dir /home/peter/.cargo-target -p quoin-cli --test tc_1488_pack_requirements_report -- -D warnings` exited 101 before linting because `/home/peter/.cargo-target/debug/.cargo-build-lock` is on a read-only filesystem in this invocation. This gate remains blocked; the earlier invocation without an explicit target-dir remains discounted.

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | At 6ef32379e4e6c82c32123db9f36489ddd42d08ea, the test separately records a generic direct fact without approvalRef, asserts recording succeeds, checks its compliant row, finds it in binding.direct, and asserts approvalRef remains absent before exercising a supplied reference. |

## Disposition pass 1

**PASS; mergeable=yes.** Inspected the complete diff from e0c775b43a294c2bd72193c46da432a0bb27aa5b to 6ef32379e4e6c82c32123db9f36489ddd42d08ea. Only the existing traced test changed. Its new unconditional success and classification assertions fail if generic no-reference recording or discharge is refused. FR-208-AC-3 now has a correct binding; the existing FR-208-AC-5 assertions remain intact. No new findings.

The dispatching lead reports the focused test passed 1/1 through locked-build at this head; that result is supplied evidence, not an independently rerun test. Reviewer formatting and diff checks passed. Used released Quoin 0.37.0 explicitly for authoring/version checks; validation uses quire 0.52.0 (engine 0.62.2). Initial gate limitations remain recorded above.

```rust
    let no_approval_ref = record_output(&env, &env.repo, "gdpr", "1.0.0", &[direct("no-role")]);
    assert!(
        no_approval_ref.status.success(),
        "a generic clause must accept a direct fact without approvalRef: {}",
        text(&no_approval_ref)
    );
```
