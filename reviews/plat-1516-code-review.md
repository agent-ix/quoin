---
id: SR-20261113015347
title: "Code review of quoin PR 841 (advisory join onto SOUP records, FR-142)"
type: SpecReview
analysis: base
scope: "agent-ix/quoin@3b9ddbe5; rust/crates/quoin-cli/src/soup/advisory.rs, rust/crates/quoin-cli/src/soup.rs, rust/crates/quoin-cli/src/soup/plan.rs, rust/crates/quoin-assurance/src/argument/instant.rs, rust/crates/quoin-assurance/src/argument/mod.rs, rust/crates/quoin-assurance/src/lib.rs"
review_set: subset
---

# Code review of quoin PR 841

## Summary

Ticket: PLAT-1516. Diff `origin/main...3b9ddbe5`. It adds one new module, `soup/advisory.rs`
(1218 lines). The module is pure: it takes records, retained `FindingRecord`s and a
`DischargeReport`, and does no file, clock or network access. The diff also adds
`disposition_clause` to `Existing` and fills it in `read_record`, and makes quoin-assurance's
`instant_epoch_millis` `pub` with a re-export at the crate root. The Rust lane was run with
quoin's repo `rust-style`, which outranks the ecosystem skill here, plus ecosystem
`rust-style` and `rust-review`.

Measured: `cargo test -p quoin-cli --bin quoin -- tc_1516 a_purl_path` ran 12 tests, all
passed. `cargo clippy -p quoin-cli -p quoin-assurance --all-targets --all-features -D warnings`
was clean, and so was `cargo fmt --all --check`. Versions: quoin 0.30.1, quire 0.44.0 (engine
0.59.1). The build used the worktree target dir with debug=0.

Checked and correct:

- **Not run, vacuous and failed scans are never clean.** `is_clean` holds only for
  `Read { advisories: 0 }`, and `free_of` lists only tools whose scan was `Read`. A refused
  cargo-deny log leaves no record, so the join reports `NotRun`.
- **Resolution follows the spec.** The join maps every `DischargeState` × fact pair to the
  FR-142 reason. An advisory is resolved only when the state is `Disposition`, the fact is
  `Disposition` and `attestedAt >= newest reporting scan`. Equal instants resolve, as FR-142
  requires ("at or after").
- **The fact is copied unchanged.** `decision`, `rationale`, `approvalRef` and `attestedBy`
  come across as they are.
- **Findings are counted once.** Each scan dedups on `(rule_id, path)`, and each distinct
  finding goes either to one record or to the unmatched list. Ambiguous findings are listed and
  block `free_of` on every candidate.
- **Excluded inputs stay out.** `finding_refs` and the `vulnerability` object are not inputs to
  `join`. Nothing computes a score or class, and severity words are carried unchanged.
- **AC-11 holds by inspection.** No advisory parser was added. The join selects scans with
  `select_finding_adapter`, and its purl handling reuses `plan::parse_purl`. `identity_of`
  only splits the `<name>@<version>` string the FR-034 adapters already write.
- **The `expect(dead_code)` guards are temporary by construction.** They use
  `cfg_attr(not(test), expect(...))`. Once PLAT-1518 (`quoin soup report`, FR-144, Backlog,
  blocked by this ticket) calls the module, the expectation goes unfulfilled and the build
  fails until the guard is removed.
- **The PLAT-1515 `Existing` change is safe.** Only three places build `Existing`: the one
  production constructor (`read_record`), which now fills the field, and the two test
  literals, which were updated. Promote never writes `disposition_clause`, so promote's
  behaviour is unchanged.

The author made five choices FR-142 does not specify. Verdict on each:

1. **`free_of`: change the code and amend the spec** (FND-001).
2. **Newest scan taken over both tools for one rule id: keep the code, amend the spec**
   (FND-004).
3. **A finding with no path is unmatched `NoVersion`: keep the code, amend the spec.** The text
   is in FND-001.
4. **An unparseable timestamp sorts oldest: change the code and amend the spec** (FND-002).
5. **Ties broken by suite id: keep the code, amend the spec** (FND-005).

## Verdict

**CONDITIONAL: two medium findings, three low.** FR-142 is met on the paths it specifies, and
`not run`, `vacuous` and stale-attestation scans never read as clean. The two medium findings
are both edges where a record is wrongly reported free of a tool's advisories. Fix both in this
PR, with the FR-142 amendment text given in each. PR 841 is mergeable once FND-001 and FND-002
are fixed and FND-003..FND-005 are fixed or explicitly disposed.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | `free_of` can wrongly report a record free of a tool's advisories. It excludes only ambiguous candidates. An unmatched `NoVersion` finding whose path names the record's crate without a version (`unversioned` vs a record `pkg:cargo/unversioned@1.0.0`) still leaves that record free of the tool. So does a finding with no path at all, which could belong to any record. Fix: have `RecordIndex::find` return candidates for `NoVersion` too: every record whose base is `pkg:cargo/<name>` when the path names a crate, and every record when there is no path. `ambiguous_for` then excludes them. Add an assertion to tc_1516_003 with a record `pkg:cargo/unversioned@1.0.0`. FR-142 amendment, replacing the free-of sentences: "A record SHALL be reported free of a tool's advisories only when that tool's newest retained scan was read and is not vacuous, no advisory of that tool is attached to the record, and no unmatched finding of that tool could belong to it: an ambiguous finding whose candidates include it, a finding whose path names its crate without a version, or a finding with no path." Also replace "If a finding's path carries no version" with: "If a finding has no path, or its path carries no version, then the join SHALL list it as unmatched with the reason `no version`." | rust/crates/quoin-cli/src/soup/advisory.rs:334-341, rust/crates/quoin-cli/src/soup/advisory.rs:499-508 |
| FND-002 | medium | `newest` sorts a timestamp that is not an instant before every instant (`None < Some`). `quoin evidence record --timestamp` is user-supplied and only `check_scalar`ed (quoin-core ops/evidence/records.rs:66-75). So a scan recorded as `2026-11-12 10:00` is never read whenever an older well-formed scan exists. The older scan is read instead, and it can mark records `free_of` advisories the newer scan reports. `UnorderedInstants` arises only when every scan of the tool is malformed. `attestedAt` always parses, because the discharge parse gate rejects non-instants, so that variant is otherwise unreachable. Fix: if any retained scan of a tool has a non-instant timestamp, give that tool a new non-`Read` outcome (e.g. `ScanOutcome::Unordered { suites }`). Then no scan of that tool is read, no record is free of it, and FR-144 lists it after the rows. Remove `Unresolved::UnorderedInstants`. FR-142 amendment: "If a retained scan of a tool has a `timestamp` that is not an instant, then the join SHALL report that tool's scan as unordered, SHALL NOT read any scan of that tool, and SHALL NOT report any record free of that tool's advisories." | rust/crates/quoin-cli/src/soup/advisory.rs:415-434, rust/crates/quoin-cli/src/soup/advisory.rs:193-196, rust/crates/quoin-cli/src/soup/advisory.rs:556-558 |
| FND-003 | low | `instant_epoch_millis` becomes `pub` in quoin-assurance and is re-exported at the crate root, just for quoin-cli. Its whole body is `Instant::parse(value, InstantGrammar::CalendarInstant)?.epoch_millis()`. That is quoin-store's public reader, and quoin-cli already depends on quoin-store. Reusing the one reader is right. Widening a second crate's API to reach it is not. Revert the three visibility edits and add a private three-line helper in advisory.rs over `quoin_store::instant`. It is the same grammar the discharge gate uses for `attestedAt` (discharge/parse.rs:106), so the ordering stays consistent. This matches the PLAT-1517 FND-001 resolution | rust/crates/quoin-assurance/src/argument/instant.rs:97-98, rust/crates/quoin-assurance/src/argument/mod.rs:54, rust/crates/quoin-assurance/src/lib.rs:47 |
| FND-004 | low | The same advisory reported by both tools on one record is handled by an untested choice. It is attached twice, once per tool, and both copies are held to the newest reporting scan over both tools (`newest_report` keyed by `(record, rule_id)`). That is a sound reading, but FR-142 says only that "a repeated finding with the same rule id and path counts once". Add a test: cargo-audit at BEFORE and cargo-deny at AFTER both report RUSTSEC-X on one record, with an attestation in between, and both copies are unresolved `scan newer than attestation`. FR-142 amendment: "A repeated finding with the same rule id and path in one tool's scan counts once; the same advisory reported by both tools is attached once per tool." In Resolution, after "the newest retained scan that reports that advisory", add "on that record, of either tool". | rust/crates/quoin-cli/src/soup/advisory.rs:370-378, rust/crates/quoin-cli/src/soup/advisory.rs:453-473 |
| FND-005 | low | When two scans of one tool have the same instant, the tie is broken by raw timestamp text and then suite id, and the last one wins. That is deterministic but unstated and untested. Add a test with two equal-instant scans and swapped input order. FR-142 amendment: "Where two retained scans of a tool have the same instant, the join SHALL read the one whose suite id sorts last." | rust/crates/quoin-cli/src/soup/advisory.rs:415-434 |
