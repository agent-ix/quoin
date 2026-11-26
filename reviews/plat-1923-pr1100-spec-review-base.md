---
id: SR-967175026302197768900474181941935084902
title: spec-review/base review of PLAT-1923 PR 1100
type: SpecReview
analysis: base
scope: 'agent-ix/quoin; PLAT-1923; PR #1100; changed Rust, FR-167, MP-250/251/252
  and repository traceability'
review_set: subset
---

## Summary

Reviewed PR #1100 at 92e5d613dbfa5c3c3187e4302637225ddac5f7ba. The changed implementation, tests and applicable criteria were inspected.

## Verdict

**FAIL** — The findings below require a fix or explicit disposition before merge.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | The new criterion restates the typeless SKILL.md acceptance rule already owned by FR-166-AC-14. It creates a second owner for one rule, contrary to the duplication rule; remove AC24 and bind the regression and implementation to FR-166-AC-14. | spec/functional/FR-167-release-snapshot-format.md:167 |

## Assurance Context

AP-201 (`spec/assurance/AP-201-finding-quality.md`) applies to the MP-250/251/252 population changes and snapshot report rendering; policy is recommend. The review checked population membership against the probes, preserved unmatched-failure assertions, and examined the snapshot omission boundary. AP-202 does not apply: no Jev lens evaluation changes. Selected architecture context: the changed module-loading/error boundary and FR-166/FR-167 contracts were available; a broader architecture evaluation was not requested. Change-impact evidence: callers, record reader, risk-summary path and focused integration results were available. Measurement execution records for the revised plans were unavailable beyond the passing focused test runs. Independent runtime evidence was not produced by this reviewer; the coder supplied the recorded focused results. No producer-reliance decision or separate structural-gap disposition was available for the changed population plans. Active exceptions: none found.

## Verification

The supplied exact-head focused results passed: TC-455 boundary (1), TC-1322 (4), TC-997 (3), NFR-003 (1), NFR-007 (1), adapter purity (1), TC-3604 (34), and claimed approval (13). Formatting, metadata, diff checks and changed FR-167 validation passed. Clippy failed on unchanged baseline quoin-evidence files; Minder assigned those diagnostics to codex-verify. No workflow or dependency pin changed. No aggregate suite was run for this review. Source inspection traced the malformed-corpus decisions through records::read, invalid_identity_paths, risk_findings, and render_vulnerabilities.

Resolved installed tools: /home/peter/.npm-global/bin/quoin reports 0.36.8; quire reports 0.52.0 (engine 0.62.2). The version discrepancy was reported to the parent with actual paths and results. Quire validation emits an existing UnknownEdgeType warning for ADR realizes_requirement.

## Examined Scope

```yaml
- id: FR-166-AC-13
  path: spec/functional/FR-166-release-snapshot-command.md
  role: examined
  excerpt: A fixture that `quoin risk-summary` refuses with exit 2 (a `residual_risk`
    with an unknown decision word) yields `release_snapshot_input_invalid` naming
    `--repo` and the record, exit status 2 and no file; a `--module DIR` that does
    not load yields `release_snapshot_input_invalid` naming that `--module DIR`; a
    repository with no risk record yields a snapshot with no risk-completeness item.
- id: FR-166-AC-14
  path: spec/functional/FR-166-release-snapshot-command.md
  role: examined
  excerpt: Each of a risk record file, a non-risk markdown file that is not valid
    UTF-8, and a markdown file whose frontmatter block is not a YAML mapping, placed
    below `--repo`, yields `release_snapshot_input_unreadable` naming `--repo` and
    the file, exit status 2 and no file; a markdown file with a YAML mapping and no
    `type` or `id` (a `SKILL.md` with only `name` and `description`), and one with
    an empty `---` block, are read and the snapshot is written.
- id: FR-167-AC-17
  path: spec/functional/FR-167-release-snapshot-format.md
  role: examined
  excerpt: A control with no incoming `verifies` edge that appears on two rows of
    `quoin risk-summary` yields exactly one `control-without-verification` item; a
    fixture with a `benefit-risk-not-accepted` finding naming a residual risk and
    its covering benefit-risk ids yields one item whose `obligation` is the residual-risk
    id and whose `summary` lists every name in `names` order.
- id: FR-167-AC-20
  path: spec/functional/FR-167-release-snapshot-format.md
  role: examined
  excerpt: A fixture with `open`, `in_remediation`, `remediated`, and `closed` vulnerability
    records carries only the first two into `vulnerabilities`, copies each listed
    source field unchanged, orders included records by id, and emits an empty list
    when neither open status is present.
- id: FR-167-AC-24
  path: spec/functional/FR-167-release-snapshot-format.md
  role: examined
  excerpt: A repository containing a valid non-vulnerability Markdown document outside
    `spec/vulnerabilities` produces a snapshot with an empty `vulnerabilities` list
    instead of refusing that document as a malformed vulnerability record.
- id: FR-065-AC-18
  path: spec/functional/FR-065-change-assurance-verification.md
  role: examined
  excerpt: An approved event with no `verified_approval` under a record whose `signed_approval.required`
    is false yields `claimed_approval` null and the same review check as before; a
    version 1 payload or receipt is not accepted; static boundaries prove the human-approval
    check starts no process and performs no Git, network or identity-provider operation.
- id: FR-063-AC-11
  path: spec/functional/FR-063-change-assurance-record-integrity.md
  role: examined
  excerpt: Static and golden-text checks prove writing a record runs nothing and describes
    actor labels only as recorded attribution, never identity, authority, signature,
    authenticity, or non-repudiation, and that a signer appears only as a retained
    `verified_approval` described as a claim (CON-1, CON-3).
- id: FR-131-AC-9
  path: spec/functional/FR-131-percentage-metrics-over-enumerated-population.md
  role: examined
  excerpt: NFR-003-M-1 and NFR-003-M-2 are measured by plans MP-250 and MP-251 over
    command x error-class pairs (unknown flag, unknown subcommand, missing argument,
    missing path, unknown type, malformed input, catalog conflict), each class enumerated
    from the binary's own help text; each plan's declared set equals that enumeration,
    and the producer's observation, probing every pair against the real binary, is
    accepted at intake and read by `quoin measurement verify`.
- id: FR-131-AC-10
  path: spec/functional/FR-131-percentage-metrics-over-enumerated-population.md
  role: examined
  excerpt: NFR-007-M-1 is measured by MP-252 over every workflow command crossed with
    twelve ways `ix-flow` fails (absent, old or unreadable version, hang in the handshake
    or the run, non-zero exit, garbage, empty or oversized output, error payload,
    missing instance id, failing context step); the declared set equals the enumeration,
    and the producer's observation, running each pair against the real binary, is
    accepted at intake and read by `quoin measurement verify`.
- id: rust/crates/quoin-change-assurance/tests/tc_455_boundary.rs
  path: rust/crates/quoin-change-assurance/tests/tc_455_boundary.rs
  role: examined
  excerpt: //! What this crate is not allowed to contain, asserted over its own sources.
- id: rust/crates/quoin-cli/src/release/snapshot.rs
  path: rust/crates/quoin-cli/src/release/snapshot.rs
  role: examined
  excerpt: '//! The release snapshot document (FR-167): its closed types, and the
    sections'
- id: rust/crates/quoin-cli/tests/nfr_populations/help_tree.rs
  path: rust/crates/quoin-cli/tests/nfr_populations/help_tree.rs
  role: examined
  excerpt: //! The command tree the built binary describes about itself, read from
    its
- id: rust/crates/quoin-cli/tests/nfr_populations/ix_flow.rs
  path: rust/crates/quoin-cli/tests/nfr_populations/ix_flow.rs
  role: examined
  excerpt: '//! NFR-007-M-1: every workflow command, launched against each way `ix-flow`'
- id: rust/crates/quoin-cli/tests/nfr_populations/missing_path.rs
  path: rust/crates/quoin-cli/tests/nfr_populations/missing_path.rs
  role: examined
  excerpt: '//! NFR-003, class `missing-path`: every flag or positional whose help'
- id: rust/crates/quoin-cli/tests/tc_3003_nfr_populations.rs
  path: rust/crates/quoin-cli/tests/tc_3003_nfr_populations.rs
  role: examined
  excerpt: '//! PLAT-1401: the NFR percentage metrics are measured over populations
    the'
- id: rust/crates/quoin-cli/tests/tc_3604_release_snapshot.rs
  path: rust/crates/quoin-cli/tests/tc_3604_release_snapshot.rs
  role: examined
  excerpt: '#![allow('
- id: rust/crates/quoin-evidence/tests/adapter_purity.rs
  path: rust/crates/quoin-evidence/tests/adapter_purity.rs
  role: examined
  excerpt: //! An adapter transcribes text. It does not execute, read or fetch anything.
- id: rust/crates/quoin-evidence/tests/module_sizes.rs
  path: rust/crates/quoin-evidence/tests/module_sizes.rs
  role: examined
  excerpt: Deleted file-size census test; removal authorized under the standing no-ceremony
    ruling.
- id: rust/crates/quoin-quire/tests/tc_474_module_sizes.rs
  path: rust/crates/quoin-quire/tests/tc_474_module_sizes.rs
  role: examined
  excerpt: Deleted file-size census test; removal authorized under the standing no-ceremony
    ruling.
- id: spec/assurance/MP-250-nfr003-actionable-failures.md
  path: spec/assurance/MP-250-nfr003-actionable-failures.md
  role: examined
  excerpt: '---'
- id: spec/assurance/MP-251-nfr003-silent-failures.md
  path: spec/assurance/MP-251-nfr003-silent-failures.md
  role: examined
  excerpt: '---'
- id: spec/assurance/MP-252-nfr007-external-tool-failures.md
  path: spec/assurance/MP-252-nfr007-external-tool-failures.md
  role: examined
  excerpt: '---'
- id: spec/functional/FR-167-release-snapshot-format.md
  path: spec/functional/FR-167-release-snapshot-format.md
  role: examined
  excerpt: '---'
- id: tests/fixtures/change-assurance-cli/attestation-body.json
  path: tests/fixtures/change-assurance-cli/attestation-body.json
  role: examined
  excerpt: '{'
```

## Method Selection

The delegated subset is base plus integrity, with code-review/Rust and gap-analysis. EARS was not run because no Description/Behavior/Constraints requirement statement changed; the new text is an acceptance criterion. No new relationship edge, domain object, verification method, security obligation or architecture topology was introduced, so dependency, object, evidence, security and architecture lenses were not separately run. AP-201 recommends reviews; it does not require an enforced human-acceptance workflow. The optional Jev semantic gap pass was not requested.

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | Removed the duplicate AC24; the regression and implementation now use existing FR-166-AC-14. |

## Disposition Verdict

**PASS for PR #1100 at 27007d2ca852ca8e085fcef8ce38a961dd474b63; mergeable: yes.** Every original finding has an explicit fixed/deferred outcome and no new finding was identified in the full PR diff or the fix-round diff. The repository baseline gap-analysis verdict remains FAIL as originally recorded; its finding is explicitly deferred to PLAT-1914/PLAT-1915 and does not block this bounded repair.

The fix was inspected through the shared frontmatter parser, corpus malformed-identity handling, vulnerability renderer and caller. The new regression asserts both typeless directory spellings and a typed missing-ID document outside the conventional directory. The coder reports a red run with exactly those two regressions failing before the fix and 35/35 TC-3604 tests passing afterward through locked-build. Diff checks and FR-166/FR-167 validation passed in this disposition pass; validation retains the known ADR UnknownEdgeType warning. No workflow or dependency change was introduced.
