---
id: SR-20261117175248
title: "Code review of PLAT-1640 Quoin ratings coverage"
type: SpecReview
analysis: code-review
scope: "agent-ix/quoin; PLAT-1640; rust/crates/quoin-cli/src/validation.rs, rust/crates/quoin-cli/tests/tc_1636_validation_ratings.rs, spec/functional/FR-235-quoin-selftest.md"
review_set: subset
relationships:
  - {target: "ix://agent-ix/quoin/FR-235", type: references}
---

## Summary

Ticket: PLAT-1640. Reviewed the Rust implementation, focused fixture, and changed FR-235 criteria at head ec3e567f8f56def140a796b6b3dfb6f3e363fb35. The three fixture paths cover high-risk covered/uncovered, not-high release evidence, unrated, malformed and unknown markers. `cargo test -p quoin-cli --test tc_1636_validation_ratings --locked` passed 3/3 in 12m06s through locked-build. The user systemd bus was unavailable, so the wrapper ran with one job and no cgroup CPU/memory limits. The coder reports cargo check and rustfmt passed; narrow clippy is blocked by existing build.rs diagnostics. The implementation also returns success when its input bundle has unreadable documents, and a new helper is missing the required implements trace.

## Scope

- FR-235-AC-4 and FR-235-AC-11 (examined).
- Existing supplier-rating parsing and response exit behavior in `validation.rs` (examined).
- PLAT-1640 acceptance criteria 2, 3 and 4, as exercised by the new fixture (examined).
- PLAT-1640 acceptance criterion 5 and the Quoin result of 201 uncovered high-risk FRs (pending owner decision; not treated as a code defect).

## Assurance Context

No applicable AssuranceProfile. AP-201 covers Quoin finding scoring, measurement persistence, comparison, gates, and report rendering; this command reports supplier process-risk ratings and their self-test coverage, outside that profile's stated scope. AP-202 governs live evaluation of Jev semantic lenses and is unrelated.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | Coverage mode drops unreadable bundle inputs before returning a successful outcome | rust/crates/quoin-cli/src/validation.rs:84-91 |
| FND-002 | medium | New `string_field` helper is missing the required `implements` trace for the criteria it supports | rust/crates/quoin-cli/src/validation.rs:456-460 |

## Verdict

FAIL. If an FR or rating file cannot be parsed, `read_bundle_frontmatter` returns it in `parsed.unreadable`; the regular ratings path includes those documents and sets `Outcome::Partial`. The coverage branch passes only `parsed.documents` to `run_coverage`, which omits unreadable paths from the output and does not include them in its `partial` calculation. Thus a high-risk rating hidden in an unreadable record can disappear from the coverage input and the command can exit 0. The new helper is also not discoverable as an implementation of the two criteria under the repository's required trace convention.

## Dispositions

Round 1, checked against head 5442c2e0296903474fd8a2ef4243763f3eacbbab.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | Coverage now includes unreadable documents in JSON and human output and returns Partial when any are present. The new malformed-frontmatter fixture asserts the unreadable path and nonzero exit. |
| FND-002 | fixed | `string_field` now carries bare `implements` attributes for FR-235-AC-4 and FR-235-AC-11; `quire trace --file` lists both implementation bindings. |
