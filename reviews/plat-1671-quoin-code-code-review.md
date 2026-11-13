---
id: SR-20261113070100
title: "code-review and rust-review of the licence-policy adapter, policy join and report rows (quoin PR 851)"
type: SpecReview
analysis: base
scope: "agent-ix/quoin@503030c1; rust/crates/quoin-evidence/src/adapters/licence_policy.rs; rust/crates/quoin-evidence/src/adapters/sbom.rs; rust/crates/quoin-evidence/src/adapters/registry.rs; rust/crates/quoin-evidence/src/purl.rs; rust/crates/quoin-evidence/src/types/scan.rs; rust/crates/quoin-core/src/ops/evidence/{documents,records,wire}.rs; rust/crates/quoin-cli/src/evidence.rs; rust/crates/quoin-cli/src/soup/{advisory,plan,policy,report}.rs; rust/crates/quoin-cli/src/soup/report/render.rs; rust/crates/quoin-cli/tests/fixtures/retained-command-help.json; rust/crates/quoin-evidence/tests/fixtures/*"
review_set: subset
---

# Code review and Rust review of the licence-policy adapter, policy join and report rows (quoin PR 851)

## Summary

Ticket: PLAT-1671. Frozen head 503030c1, diff `origin/main...HEAD` (33 files, +2920/-116). The Rust-review lane is folded in. The repo `.claude/skills/rust-style` was loaded and outranks the ecosystem skill. The branch was not edited.

What was measured:

- Focused tests, run in the reviewer's own target dir through `locked-build.sh` with `CARGO_TARGET_DIR` unset (the full workspace run was not repeated). `tc_licence_policy`, `tc_1671_licence_policy_command`, `tc_1518_soup_report` and `tc_retained_help_capture`: 44 passed, 0 failed. The built `quoin evidence record --help` is byte-identical to the fixture page.

- The Syft capture `licence-npm.syft.cdx.json` was checked against a fresh run of Syft 1.57.1 over engineering-assurance `origin/main` `tests/fixtures/licence-npm/scan`, using the FR-038 request shape (`scan dir:scan -o cyclonedx-json=out/sbom.cdx.json -q --config out/empty-config.yaml`, the FR-038 environment under `env -i`). Byte length is the same (5154). Sorted JSON differs only in `metadata.timestamp` and `serialNumber`, which change on every run. So the capture is real and unedited. The scratch directory was deleted.
- `cargo-deny-licences-real.jsonl` was reproduced with cargo-deny 0.19.8 over a scratch `denyscratch`/`nolic`/`itoa` graph. All 12 lines are byte-identical; only the interleaving of the `bans` and `licenses` lines differs, and that ordering is not deterministic. In `cargo-deny-not-allowed-real.jsonl` both diagnostics reproduce byte-identical. Its summary's `notes: 1` comes from a config difference: notes are not emitted at the default log level. The summaries' error and warning counts agree with the diagnostics in all three captures. Scratch was deleted.
- The `retained-command-help.json` hand edit: this fixture is the help source. `help.rs` `include_str!`s it, so `quoin evidence record --help` prints exactly that page, and no oclif shell remains to recapture from. The edit was checked for consistency instead. `--policy` appears in USAGE, in the `--adapter` option list and in FLAGS in alphabetical order. Its text matches the behaviour of `FindingAdapter::run` and `refuse_policy`. The wrapping follows the page's existing widths. No finding.
- 700-line ceiling (`tc_90_module_sizes`, which exempts `tests.rs`): the largest production module touched is `soup/advisory.rs` at 692. `policy.rs` is 465 and `render.rs` is 552. Within the ceiling.
- No default policy and no embedded licence list: `licence_policy.rs` names no licence id, and a grep found no `std::process`, `std::fs`, `std::net` or `Command`. `allow` is required and must not be empty, and with no `--policy` the command refuses.
- No digest or pin ceremony, no compatibility layer, no vendoring. `evaluated` is `Option` with `skip_serializing_if`, so it is an optional field and not a compatibility path.
- `tc_1045_spec_integrity`'s dangling FR-145..FR-148 citations do not come from this diff. The diff touches no file under `spec/` and has zero `FR-14[5-8]` mentions; those documents and citations are already on `origin/main`.

What is right (no finding):

- Unknown is never allowed. No licence, `NOASSERTION` or `NONE` gives `licence-unknown` with the message `no licence declared`.
- Precedence is denied, then not-allowed, then unknown, with at most one licence finding per component. A ban gives one `package-banned` per matching entry, and a version-less entry matches every version.
- A refused policy or SBOM returns `Err` before anything is written. `--policy` is refused on every run adapter (`record_entries`, `parsed_results`) and on every finding adapter (`FindingAdapter::run`), and the error names the adapter.
- `evaluated` semantics hold: a record not in `evaluated`, including a scan with no `evaluated` field, is `not evaluated` and never `no policy findings`.
- Policy findings are kept apart from advisories. The `set_aside` count excludes the four policy codes.
- Each policy finding is resolved against the timestamp of the scan that reported it, which is one scope rule shared with advisories.
- An unmatched licence-policy finding is listed, not dropped, and a no-purl finding blocks a "no policy findings" statement on every record.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-142's one purl match rule now has two implementations that disagree. The advisory join, and the cargo-deny policy match in `read_deny` through `RecordIndex`, use `plan::parse_purl`. It keys on the raw `pkg:<type>/<namespace>/<name>` string, so the type is not case-folded and the namespace is not percent-decoded. The new `quoin_evidence::purl::PurlIdentity` lower-cases the type and decodes the namespace. A record `pkg:NPM/%40s/x@1` therefore matches a licence-policy finding but not an advisory with the same purl. The PR moved `percent_decode` but left the second parser in place. Use one parser for both joins. | rust/crates/quoin-cli/src/soup/plan.rs:31-49; rust/crates/quoin-evidence/src/purl.rs:26-44; rust/crates/quoin-cli/src/soup/policy.rs:381 |
| FND-002 | medium | The CycloneDX licence reader silently drops a malformed entry: a `license` object with neither `id` nor `name`, a non-string `expression`, or an empty text. A component that declares `[{"license":{"id":"MIT"}}, {"license":{"url":"…"}}]` then reads as only `MIT`, so it is all-allowed and gets no finding. A declared licence the reader could not read becomes an absence. FR-150 says unknown is never allowed. Keep an unreadable entry as `licence-unknown`, or refuse it, instead of dropping it. | rust/crates/quoin-evidence/src/adapters/sbom.rs:140-163; rust/crates/quoin-evidence/src/adapters/licence_policy.rs:177-218 |
| FND-003 | low | The SPDX single-id test checks only for whitespace and parentheses. `GPL-2.0+` (the SPDX `+` operator) is classed `id`, so it becomes `licence-not-allowed` rather than `licence-unknown`. FR-150 says an SPDX value is an id only with "no whitespace, parenthesis or operator". | rust/crates/quoin-evidence/src/adapters/sbom.rs:198-212 |
| FND-004 | low | The JSON row's `policy_status.state` holds the full prose sentence, including the timestamp and the canonical ruleset JSON. Every other `state` field in the report is a short token (`read`, `not run`, `vacuous`, `resolved`). A consumer has to string-match prose to tell the six statuses apart. Emit a token plus separate `timestamp` and `ruleset` fields. | rust/crates/quoin-cli/src/soup/report/render.rs:433-438 |

## Verdict

CONDITIONAL. The adapter, reader extension, policy join and report rows do what FR-150, FR-142 and FR-144 require on the main paths. The fixtures are real tool output, verified by fresh runs. There is no shipped policy or licence list, and nothing is read as clean from a refused, vacuous, unordered or unevaluated input. Two medium findings: two divergent purl equality rules (FND-001), and malformed licence entries dropped where they should become unknown (FND-002). Neither produces a wrong result for the Syft capture used here. Both should be fixed in this PR.
