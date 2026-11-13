---
id: SR-20261113014535
title: "Code review of quoin PR 840 (support input for the SOUP report, FR-143)"
type: SpecReview
analysis: base
scope: "agent-ix/quoin@084c12bb; rust/crates/quoin-cli/Cargo.toml, rust/crates/quoin-cli/src/soup.rs, rust/crates/quoin-cli/src/soup/plan.rs, rust/crates/quoin-cli/src/soup/support.rs"
review_set: subset
---

# Code review of quoin PR 840

## Summary

Ticket: PLAT-1517. Diff `origin/main...084c12bb`: one new module `soup/support.rs` (786 lines,
pure, no file, clock or network access), `percent_decode` made `pub(super)` in `soup/plan.rs`,
`mod support` added behind `#[allow(dead_code)]`, and the `parsing` feature added to quoin-cli's
`time` dependency. Rust lane run with quoin's repo `rust-style` (which outranks the ecosystem
skill here) and ecosystem `rust-style`/`rust-review`.

Measured: `cargo test -p quoin-cli --bin quoin tc_1517` 10 passed, 0 failed; `cargo clippy -p
quoin-cli --all-targets --all-features -D warnings` clean; `cargo fmt -p quoin-cli --check`
clean (quoin 0.30.1, quire 0.44.0 / engine 0.59.1, worktree target dir, debug=0).

Checked and correct: no network, file or wall-clock access in the module; `unknown` is never
guessed (absent and `unknown` both resolve to `Resolved::Unknown`, no release-age inference);
a stated record value is authoritative and a differing entry yields `Disagree` with both values
and the source; versioned entry beats version-less, qualifiers and subpath ignored, a different
version matches nothing; `as_of > end` so the end day itself is not flagged; an `unknown`
authoritative date is never flagged; `as_of` offsets are converted to UTC before the date is
taken; every entry is validated before a `SupportFile` is returned, and the first refusal
returns nothing.

The `#[allow(dead_code)]` on `mod support` is acceptable: PLAT-1518 (`quoin soup report`) is
the caller and is blocked by this ticket. FND-005 asks only that it become `#[expect]`.

## Verdict

**CONDITIONAL (one medium, four low).** The logic meets FR-143. The medium finding is the new
`time` `parsing` feature, which adds a second RFC 3339 reader beside the workspace's single
`quoin_store::instant` reader. It is avoidable and should be fixed in this PR. Mergeable once
FND-001 is fixed and FND-002..FND-005 are fixed or explicitly disposed.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | `as_of_date` parses instants with `time`'s `Rfc3339` parser, and Cargo.toml turns on `time`'s `parsing` feature for it. The workspace has one timestamp reader, `quoin_store::instant` (PLAT-91: "nothing here may add a second reader"), and quoin-cli already depends on quoin-store. A second reader can accept a different language from the one `--as-of` gets in FR-046 and FR-144. Use `Instant::parse(text, InstantGrammar::Rfc3339)` (or `CalendarInstant`), take `epoch_millis()`, then `OffsetDateTime::from_unix_timestamp(ms.div_euclid(1000))?.date()`. That needs only default `time` features, so drop `features = ["parsing"]`. Keep the strict `YYYY-MM-DD` path for a bare date | rust/crates/quoin-cli/src/soup/support.rs:120-132, rust/crates/quoin-cli/Cargo.toml:52 |
| FND-002 | low | `parse_entry` silently ignores keys other than the four fields (author gap 3). quoin rust-style says to deny unknown fields on untrusted input: "a field the boundary silently drops is a field the caller believes it sent". Refuse an entry with any other key under a new code (for example `soup.support_unknown_field`) naming the entry and the key. Add a case to tc_1517_001 and the FR-143 sentence from the gap-analysis review | rust/crates/quoin-cli/src/soup/support.rs:264-314 |
| FND-003 | low | `identity` percent-decodes the whole `pkg:type/ns/name` base string. An encoded `/` in a name then collides with the namespace/name split: `pkg:x/a%2Fb` (name `a/b`) and `pkg:x/a/b` (namespace `a`, name `b`) decode to the same base. They match each other and are refused as duplicates. Compare the type, namespace and name segments separately, each decoded | rust/crates/quoin-cli/src/soup/support.rs:219-223 |
| FND-004 | low | `SupportError` names the entry only in message prose ("entry 2 (pkg:...)"). It has no `context` map like the `soup.*` diagnostics in soup.rs, and no `all()` with round-trip and uniqueness tests, which quoin rust-style's envelope pattern requires. PLAT-1518 cannot put the entry position and purl into diagnostic context without parsing prose. The tests assert `message.contains("entry 2")`, which `entry 20` also satisfies. Add `context` (`entry`, `purl`, and `other_entry` for a duplicate), assert on it, and add the code round-trip and uniqueness test | rust/crates/quoin-cli/src/soup/support.rs:134-198, rust/crates/quoin-cli/src/soup/support.rs:559-570 |
| FND-005 | low | `#[allow(dead_code, reason = ...)]` on `mod support` stays silently once PLAT-1518 uses the module. `#[expect(dead_code, reason = ...)]` fails the build at that point and forces its removal. It is the only dead_code suppression in the workspace | rust/crates/quoin-cli/src/soup.rs:12 |
