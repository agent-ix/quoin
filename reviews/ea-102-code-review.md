---
id: SR-20261113111444
title: "Code review (Rust lane) — EA-102 quoin release snapshot (quoin PR 866)"
type: SpecReview
analysis: code-review
scope: "agent-ix/quoin; rust/crates/quoin-cli/src/release.rs, rust/crates/quoin-cli/src/release/history.rs, rust/crates/quoin-cli/src/release/snapshot.rs, rust/crates/quoin-cli/src/release/tests.rs, rust/crates/quoin-cli/src/soup/report.rs, rust/crates/quoin-cli/src/soup/report/snapshot.rs, rust/crates/quoin-cli/src/soup/report/render.rs, rust/crates/quoin-cli/src/matrix.rs, rust/crates/quoin-cli/src/evidence/audit.rs, rust/crates/quoin-anomaly/src/carried.rs, rust/crates/quoin-anomaly/src/record.rs, rust/crates/quoin-anomaly/src/lib.rs, rust/crates/quoin-cli/tests/tc_3604_release_snapshot.rs, spec/assurance/MP-250-nfr003-actionable-failures.md, spec/assurance/MP-251-nfr003-silent-failures.md"
review_set: subset
---

## Summary

Ticket: EA-102. PR head `63155109`, diff `git diff origin/main...HEAD`. Rust lane (rust-style)
folded in. Checked against FR-166, FR-167, FR-168, NFR-036 and the sources they read (FR-115,
FR-144, FR-162, FR-163). Gates run on this head: `cargo fmt --all --check` (clean), `cargo clippy
--workspace --all-targets --all-features --locked -D warnings` (clean, rc 0), and focused tests:
quoin-cli bin unit tests 107 passed (6 in `release::tests`), tc_3604_release_snapshot 16/16,
tc_3003_nfr_populations 13 passed / 1 ignored, tc_1518_soup_report 24/24, tc_1080_matrix_command
6/6, tc_3206 1/1, tc_retained_help_capture 6/6, tc_shell_contract 3/3, quoin-anomaly 15/15.
tc_1515_soup_promote fails 15/16 in test setup: the local spec-objects-security checkout lacks
the soup_record skeleton the test reads. That is environmental and independent of this PR (the
test file and promote code are unchanged by the diff), so it fails the same way on main.

The command does what the spec centrally asks. It runs no git and reads no clock. It computes no
digest, hash, manifest or pin. It derives no `fixingChange` or `rationaleChange`. Every instant
comes from `--as-of`. It builds every section before it writes, and it writes with `create_new`,
so an existing `--out` is refused and a partial write is removed. The seven refusal codes match
the spec's spelling exactly. The orderings are total and byte-stable: `open_findings` is keyed on
every member it has, `not_assessed` on all three of its members, `change_records` on
(`record_id`, `revision`, `commit`) after duplicate pairs are refused, `coverage` on matrix.build's
BTreeMap order, and anomalies on (id, path). Stale, suspect, undischarged and no-run statuses are
copied from the matrix and counted, never folded into `bound`. The matrix.rs and evidence/audit.rs
refactors preserve behaviour.

## Verdict

Mergeable after fixes. FND-001 needs coordination with the parallel PLAT-1585 PR before either
lands. FND-002 and FND-003 are real gaps in the SOUP section. The rest are low.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | quoin#865 (PLAT-1585, open) widens the same `record.rs` reader differently and adds its own selection (`report.rs` `is_listed`/`build_report`). Both PRs add `RationaleEntry` and its `lib.rs` export. #865 makes an absent `title` a `Missing` mapping error; this PR makes it `Option` (`null`), which is what FR-162 Row says. Whichever lands second hits a merge conflict and leaves two selection paths. FR-166 says the snapshot uses the selection and record-reading code of `quoin anomaly report`. `build_report` in #865 calls `read_history` (git), so the snapshot cannot call it as it stands. Fix: land one reader, and have `quoin anomaly report` select through `carried::is_carried`, or the reverse, with the git-free selection split out. | rust/crates/quoin-anomaly/src/carried.rs:60-65; rust/crates/quoin-anomaly/src/record.rs:51-86 |
| FND-002 | medium | The `detail` of `unmatched_advisory` and `unmatched_policy_finding` uses the finding's `path` (`package`), not the purl that FR-167 defines. For cargo-audit that is `ghost@9.9.9`, not `pkg:cargo/ghost@9.9.9`. For SARIF-sourced scans it is the `artifactLocation.uri`, which can be a host path (NFR-036). The test asserts only the `RUSTSEC-2026-0002\|` prefix, so the deviation is not pinned. | rust/crates/quoin-cli/src/soup/report/snapshot.rs:214-235; rust/crates/quoin-cli/tests/tc_3604_release_snapshot.rs:815-822 |
| FND-003 | medium | `Report.unreadable_scans` is dropped from the snapshot. These are retained scan files that could not be read and that no scan state covers. `quoin soup report` states them; FR-167 has no member for them. So an unreadable newer scan disappears without notice while an older readable scan of the same tool is stated as read. That is failed evidence silently not reported (EA-102 AC-2). Either refuse the snapshot when the list is non-empty, or add a `not_assessed` kind in FR-167. | rust/crates/quoin-cli/src/soup/report/snapshot.rs:154-159,208-248 |
| FND-004 | low | Every `Stop::Fail` from `discharge_report` becomes `release_snapshot_input_invalid` naming `--binding` with cause `bad_request`, exit 2. That includes evidence-store read failures and core failures, not only a binding of the wrong shape. Also, a `--support` file that exists but cannot be read becomes an exit-1 failure, not `release_snapshot_input_unreadable`. | rust/crates/quoin-cli/src/soup/report/snapshot.rs:131-140; rust/crates/quoin-cli/src/release.rs:283-288 |
| FND-005 | low | The `soup/report.rs` refactor moves the `--binding` read and the discharge report ahead of the records read. Before, a missing `--records` with a bad binding was refused `soup.path_missing` (exit 2). Now it is the binding failure (exit 1). It is an untested change in `quoin soup report`'s error precedence. | rust/crates/quoin-cli/src/soup/report.rs:133-135 |
| FND-006 | low | The "closed schema" is the `deny_unknown_fields` structs. `SoupRow.support_status` and `end_of_support` are `serde_json::Value`, so the schema is not closed at that depth, which FR-167 Behavior asks for ("at any depth"). The FR-167-AC-1 positive case also validates a hand-written `sample()`, not a snapshot built from a fixture. | rust/crates/quoin-cli/src/soup/report/snapshot.rs:69-72; rust/crates/quoin-cli/src/release/tests.rs:207-250 |
| FND-007 | low | `release:unknown-flag` is inserted between `report --repo:missing-path` and `report:unknown-flag`, out of the list's alphabetical order. The set check in tc_3003 passes regardless. | spec/assurance/MP-250-nfr003-actionable-failures.md:130; spec/assurance/MP-251-nfr003-silent-failures.md:130 |

## Review Scope

| Unit | Role | Result |
| --- | --- | --- |
| No git, network or clock (release.rs, snapshot.rs, carried.rs, soup snapshot) | examined | Clean. No `Command`, gix or `now`. The shared matrix/soup paths reach no git (`quire.coverage` reads installed modules only). The PATH-empty tests pass |
| No digest/hash/manifest/pin | examined | Clean (FR-166-CON-2, FR-167) |
| Refusal codes | examined | All seven match FR-166/FR-168 spelling |
| Orderings: open_findings, not_assessed, change_records, coverage, anomalies | examined | Total and byte-stable |
| Failed/stale/suspect never counted as passing | examined | Copied from matrix.build, and freshness counts each status. FND-003 for SOUP scans |
| history.rs | examined | Members, protocol, duplicate pair and baseline mismatch per FR-168 |
| carried.rs selection vs FR-162 Selection | examined | Faithful: not closed, closed wont-fix, or unassessed. FND-001 |
| record.rs Release Rationale reader vs spec-artifacts-process FR-016 | examined | `Author \| Rationale`, at least one row, trimmed cells. Same mapping errors as the assessment table |
| matrix.rs / evidence/audit.rs refactor | examined | Behaviour preserved |
| soup/report.rs refactor | examined | FND-005 |
| scan_refused never emitted | examined | Consistent with FR-144, which renders a refused log as not run (FR-144-AC-5). Spec point recorded in the gap analysis |
| no --module/policy flags | examined | FR-166 Inputs names none, so the snapshot equals `quoin matrix` without flags |
| matrix decline maps to `release_snapshot_input_invalid` naming `--repo` | examined | Acceptable under FR-166 ("fails its own format") |
| MP-250/MP-251 population edit | examined | Justified: the new top-level command adds a probed `release:unknown-flag`. FND-007 |

Rust lane: no `unwrap`/`expect`/indexing in production code, test modules carry the scoped
`#[allow]` with a reason, no unsafe, no new public API beyond `quoin-anomaly::carried`, errors are
typed (`Refusal`/`Stop`), and `create_new` closes the exists-then-write race.

## Dispositions

Round 1, head `c95917ed`. Gates re-run on this head: `cargo fmt --all --check` clean, `cargo clippy --workspace --all-targets --all-features --locked -D warnings` clean. Focused tests pass: tc_3604 22/22, quoin-cli bin unit 108, tc_3003 13 passed + 1 ignored, tc_1518 24, tc_1080 6, retained help 6, shell contract 3, quoin-anomaly tc_136 15.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | deferred | `record.rs` now equals #865's reader (944f52ee) except `title: Option<String>`, which FR-162 Row requires. A trial merge conflicts in record.rs (3 hunks) and lib.rs (1), all mechanical. It is not purely textual: #865's `report.rs` assigns `record.title` to `ReportRow.title: String` and will not compile until that field becomes `Option` with `not stated`/`null` rendering. The selection is still duplicated (`carried::is_carried` vs #865's `is_listed`/`carried`, identical logic). Neither can be resolved here, because #865's code is not on main. The PR that lands second must widen `ReportRow.title` and make `report.rs` select through `carried::is_carried` |
| FND-002 | fixed | - |
| FND-003 | fixed | - |
| FND-004 | fixed | - |
| FND-005 | fixed | - |
| FND-006 | fixed | - |
| FND-007 | fixed | - |
