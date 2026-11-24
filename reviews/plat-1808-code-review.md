---
id: SR-20261123202514
title: "Code and Rust review of release snapshot risk and anomaly-gate open findings"
type: SpecReview
analysis: code-review
scope: "agent-ix/quoin; PR #1035 (PLAT-1808) diff origin/main...HEAD: rust/crates/quoin-anomaly/src/lib.rs, rust/crates/quoin-anomaly/src/report.rs, rust/crates/quoin-cli/src/release.rs, rust/crates/quoin-cli/src/release/snapshot.rs, rust/crates/quoin-cli/src/risk.rs, rust/crates/quoin-cli/tests/tc_3604_release_snapshot.rs, rust/crates/quoin-quire/src/records.rs, rust/crates/quoin-risk-advice/src/lib.rs, rust/crates/quoin-risk-advice/src/summary.rs"
review_set: subset
relationships:
  - target: "ix://agent-ix/quoin/FR-166"
    type: references
  - target: "ix://agent-ix/quoin/FR-167"
    type: references
---

## Summary

Ticket: PLAT-1808 (PR agent-ix/quoin#1035, head 74dc902d). Code review with the Rust review lane folded in, over the
PR diff only. The release snapshot now adds FR-170 risk-completeness tokens (built by `quoin_risk_advice::summarize`,
deduplicated through `Summary::distinct_findings`) and FR-164 anomaly-gate failures (through the new
`quoin_anomaly::gate_carried`, which shares the gate predicate with `build_report` via `GateFacts`) to
`open_findings`, takes repeatable `--module DIR`, and refuses an unreadable or unparseable markdown file below
`--repo` with `release_snapshot_input_unreadable`.

Evidence: `tc_3604_release_snapshot` 30/30 passed; `quoin-anomaly` `tc_162_anomaly_report` 33/33 and
`quoin-risk-advice` `tc_169_risk_summary` 21/21 passed (the gate refactor regresses nothing). Six mutations (drop the
dedup, evaluate one gate mode, drop the sort, skip the unusable refusal, keep the absolute anomaly path, drop the
rationale condition) were each killed by one new test, then reverted. `cargo fmt --check` clean; clippy reports
nothing in the touched files (the `-D warnings` run stops on the pre-existing `quoin-evidence` errors, out of scope).

The files touched outside the original list are justified: `gate_carried`/`GateFacts` are what lets the snapshot use
the gate code of `quoin anomaly report` without git (FR-166); `Summary.unusable`, `distinct_findings`,
`RowToken::as_str` and `module_arg` are the smallest public surface the CLI needs. The `frontmatter_unparseable`
helper is needed but implements the unreadable rule wrongly (FND-001).

## Verdict

**FAIL** for merge until FND-001 is fixed. FND-002 should be fixed in the same round; FND-003 is low.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | `frontmatter_unparseable` tests `status != Present`, so an `Absent` block also refuses: an empty `---`/`---` block (valid YAML, null) and a page whose first line is a `---` thematic break with no closing fence both yield `release_snapshot_input_unreadable` "frontmatter is not valid YAML" (reproduced). FR-166 says only an unreadable file or a block that is not valid YAML refuses, and a valid-YAML block with no `type`/`id` SHALL NOT refuse. Use the engine's own `Malformed` status (the walker's `DocumentWithoutFrontmatter { malformed }` flag) instead of re-reading each file twice. | rust/crates/quoin-quire/src/records.rs:230-243; FR-166; FR-166-AC-14 |
| FND-002 | medium | The unusable path is shown as `path.strip_prefix(repo)`, but the quire reader canonicalizes the scope, so with `--repo .` (or any relative or symlinked `--repo`) the refusal's `context.path` and message carry the absolute host path (reproduced: `/tmp/.tmpXXXX/docs.md`). Strip the canonical scope, or return scope-relative paths from the summary. | rust/crates/quoin-cli/src/release/snapshot.rs:119-131; FR-166-AC-14 |
| FND-003 | low | `advice_refusal` picks the failing `--module DIR` by `error.message.contains(dir)`, although `AdviceError.message` is documented "never branched on", and falls back to the first module, which can name a module that loaded (a `type_undeclared` failure over several modules); an ambient-module failure is reported as `--repo`. The branch has no test. Carry the failing root as structured data, or name no single module for `type_undeclared`. | rust/crates/quoin-cli/src/release/snapshot.rs:157-176; FR-166 |

## Scope Examined

- FR-166 Behavior on `--module`, `release_snapshot_input_invalid` and `release_snapshot_input_unreadable` (examined).
- FR-167 `open_findings` mapping table, ordering and closed-shape rules (examined).
- FR-164 gate modes (context only), FR-170 tokens (context only).

## Dispositions

Round 1 (fix commit 4b3fc242). I checked each fix against the code. tc_3604 passed 33/33, tc_169 risk-summary command 5/5,
quoin-risk-advice tc_169 21/21 and quoin-quire modules 2/2. Three mutations, each reverted, were caught by the new tests:
going back to `!= Present`, dropping the canonical scope, and naming the first module root.
fmt is clean and clippy reports nothing in the touched files. The two extra files cause no regression. `AdviceError.module` is set only
for `ModuleLoad`. `ModuleSelection::resolve` loads each root again only on the failure path, to name the root
that fails alone. A root that loads only together with a sibling could still be named for another root's failure.
This is not a regression: the earlier code always named the first root.

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | - |
| FND-002 | fixed | - |
| FND-003 | fixed | - |
