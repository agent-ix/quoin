---
id: SR-2026112016223475846355004
title: "PLAT-1834 quoin PR 1029 code-review"
type: SpecReview
analysis: code-review
scope: "agent-ix/quoin; PR 1029 (rust/crates/quoin-cli/src/dossier.rs, rust/crates/quoin-dossier/src/risk_plan.rs, rust/crates/quoin-dossier/tests/tc_1834_risk_management_plan.rs)"
review_set: subset
relationships:
  - target: "ix://agent-ix/quoin/FR-200"
    type: references
  - target: "ix://agent-ix/quoin/FR-191"
    type: references
---

# PLAT-1834 quoin PR 1029 code-review

## Summary

Ticket: PLAT-1834. Code review with the Rust lane (rust-review, rust-style) of `git diff origin/main...HEAD` at 20def4917d8a64b80586a44155110acc04509303. DOS-RMP now reads `risk_management_plan` records beside the policies, renders the four plan tables per plan, drops `no_plan_record` when a plan exists, names the referenced policies without copying them, and refuses an unfilled plan table. The person-accepts-risk rule holds: no score, class, acceptability, "in force" or "approved" wording is generated. One medium ordering defect against FR-191 and one low error-label defect.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Plans (and policies) are sorted by byte order (`left.id.cmp(&right.id)`), not the FR-191 dossier order (`crate::order::dossier_cmp`) that every other dossier generator uses. `RMP-10` renders before `RMP-2`, and plan-level `no_acceptability_policy` gaps follow the same wrong order. FR-191 "Quoin SHALL compare ids in dossier order". The AC-11 test uses `RMP-A`/`RMP-Z`, where both orders agree, so it cannot fail on this. Use `dossier_cmp` and test with `RMP-2`/`RMP-10`. | rust/crates/quoin-dossier/src/risk_plan.rs:81-83 |
| FND-002 | low | A plan-table failure (missing table, no row, blank cell) is reported with `source: "acceptability policy"`, and the CLI read failure for both kinds keeps the same label. A user whose plan record is malformed is pointed at the policy. Name the source per record kind (or a neutral "risk management plan" source). | rust/crates/quoin-dossier/src/risk_plan.rs:240-245; rust/crates/quoin-cli/src/dossier.rs:204-207 |

## Verdict

CHANGES REQUESTED at 20def4917d8a64b80586a44155110acc04509303 (FND-001). Otherwise sound: no new indexing, `unwrap` or panic path in production code; all `write!` to `String` results are discarded per the existing file idiom; work is linear in plans x 4 sections; output is deterministic for distinct ids. `references` matching (`*target == policy.id`) is correct: `quoin_quire::records` normalizes an `ix://owner/repo/ID` target to its last segment, the same edge form `quoin-risk-advice` and `threat_model.rs` match on, so `ix://` targets are not a problem in code (see gap-analysis for the missing test). No overlap with DOS-RISK (`risk_management.rs` untouched, PLAT-1810 lane). No CI workflow change.

Gates run through locked-build.sh (debug=0, incremental=0, shared CARGO_HOME), SPEC_OBJECTS_SAFETY_DIR pointed at spec-objects-safety PR #50 head dbd860e4: `cargo test -p quoin-dossier --lib risk_plan` 11 passed; `--test tc_1834_risk_management_plan` 2 passed; `--test tc_1656_risk_management_plan` 1 passed; `-p quoin-cli --test tc_1614_dossier_command` 31 passed, 1 ignored; `cargo fmt --check -p quoin-dossier -p quoin-cli` clean. `cargo clippy -p quoin-dossier --all-targets` exit 0 with no warning in a changed file; with `-D warnings` it fails only on pre-existing warnings in `quoin-evidence` (timestamp.rs, store/*.rs, qualify/runs.rs) and the unused `engine::render` in `quoin-dossier/src/engine.rs:95`, none touched by this PR.

## Examined

- rust/crates/quoin-dossier/src/risk_plan.rs (examined)
- rust/crates/quoin-cli/src/dossier.rs (examined)
- rust/crates/quoin-dossier/tests/tc_1834_risk_management_plan.rs (examined)
- rust/crates/quoin-quire/src/records.rs relationships() (context_only)
- rust/crates/quoin-dossier/src/order.rs (context_only)

## Dispositions

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | 76cf9c5b: policies and plans sort with `dossier_cmp` (risk_plan.rs:106-108); plan-level gaps follow plan order. New test `plans_and_policies_sort_in_dossier_order_not_byte_order` (FR-200-AC-13) uses RMP-2/RMP-10, POL-2/POL-10 and asserts gap subjects `["RMP-2", "RMP-10"]`; under byte order `RMP-10` sorts first, so every assertion fails (checked by reasoning, not by mutating the branch). |
| FND-002 | fixed | 76cf9c5b: `invalid_record` names `risk management plan` for a plan and `acceptability policy` for a policy; `unusable_source` names the one kind whose contract failed, else the combined name; CLI read failures use the combined name. tc_1656 unchanged and still passes on `acceptability policy`. |
