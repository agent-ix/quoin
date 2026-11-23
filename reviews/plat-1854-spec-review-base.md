---
id: SR-1854001
title: "Base review of FR-029 dropped-declaration refusal (PLAT-1854)"
type: SpecReview
analysis: base
scope: "agent-ix/quoin; spec/functional/FR-029-consume-the-quire-json-contract.md"
review_set: subset
---

# Base review of FR-029 dropped-declaration refusal (PLAT-1854)

## Summary

Ticket: PLAT-1854. PR agent-ix/quoin#1036, branch `plat-1854-spec`, head 3a65d0af, diff `origin/main...HEAD` (one file, +28 lines). Checklist review of the three new Behavior bullets and FR-029-AC-14..18, measured against quoin `origin/main` code and quire-rs `origin/main` (#715). Tool versions: quoin 0.36.8, quire 0.52.0 (engine 0.62.2), module spec-artifacts-process@v0.36.0.

Assurance profiles: `spec/assurance/AP-201-finding-quality.md` and `spec/assurance/AP-202-jev-lens-evaluation.md` both declare `review_policy.mode: recommend`, so no enforced review run is required. Review set: subset (base, integrity, ears-conformance).

`quire validate --scope . spec/functional/FR-029-consume-the-quire-json-contract.md` passes with no diagnostics.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | medium | FR-029-AC-18's oracle is "runs without `QQ-1022`", which a command that fails for any other reason (QQ-1020, a missing module, a crash exit) also satisfies; it should require each command's normal success outcome on the clean scope. | spec/functional/FR-029-consume-the-quire-json-contract.md:98 | wrong-requirement |
| FND-002 | medium | FR-029-AC-16 requires the envelope context to hold "each dropped declaration", but `CoreError.context` is a `BTreeMap<String, String>` (quoin-core/src/error.rs:125); the key name(s) and encoding are unstated, so two implementations would be tested differently. | spec/functional/FR-029-consume-the-quire-json-contract.md:96; rust/crates/quoin-core/src/error.rs:125 | wrong-requirement |
| FND-003 | medium | FR-029-AC-17 says each CLI command is "refused with `QQ-1022`" but does not say what is observed: exit status and where the code appears. Today those CLI paths stringify the `quoin_quire::Error` (analysis_evidence.rs `map_err(to_string)`, dossier `SourceFailure.detail`) and its `Display` carries no code. | spec/functional/FR-029-consume-the-quire-json-contract.md:97; rust/crates/quoin-cli/src/analysis_evidence.rs:97; rust/crates/quoin-cli/src/dossier.rs:251 | wrong-requirement |
| FND-004 | low | FR-029-AC-14 reuses an id the file held before: FR-029-AC-14 was the vendored-schema provenance criterion, now deleted, and committed reviews still refer to it as gone. The file's own convention is "id not reused"; the new rows can start at AC-15. | spec/functional/FR-029-consume-the-quire-json-contract.md:94; reviews/plat-1405-spec-review.md:31 | wrong-requirement |

## Verdict

This method's findings are medium and low. The PR is not mergeable, because of the high finding in the integrity review (SR-1854002).

What holds: the ID format, the table layout and the Withdrawn convention for AC-1..13 are intact. No AC-1..13 statement is contradicted. There is no migration, compatibility or fallback language, no hash, pin or ledger, no paywalled text and no machine path. Every new AC uses Test verification.

Test binding: quoin has no `spec/tests.md`. Tests bind to criteria through `#[trace("FR-…-AC-n")]` tags (949 `FR-0…` trace tags in `rust/`). The 40 files under `spec/test/` are optional TestCase documents, and none covers FR-029. No test-row file is required for this PR.

## Examined

FR-029 Description, Behavior (all four bullets), Constraints CON-1..3, AC-1..AC-18 and Dependencies; FR-096 exit taxonomy (AC-3, AC-4); FR-037-AC-2; quoin-core `ops/quire/taxonomy.rs` and `error.rs`; quoin-quire `error.rs`; `reviews/plat-1403-code-review.md` and `reviews/plat-1405-spec-review.md` (AC-14 history).

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | dec7dd14: AC-23 requires each listed command to exit 0 on the clean scope and produce its normal report, matrix, export, record or dossier. |
| FND-002 | fixed | dec7dd14: Behavior and AC-18 name the context key `dropped_declarations`, with one `module=<module> kind=<kind> item=<item>` line per declaration in engine order. |
| FND-003 | fixed | dec7dd14: Behavior and AC-21 require the Refused exit status, `QQ-1022` and each drop on stderr, and nothing on stdout. This matches the 0/1/2/3/4 taxonomy and the stdout-only-with-payload discipline. |
| FND-004 | fixed | dec7dd14: the new rows are numbered FR-029-AC-15..23, and FR-029-AC-14 is no longer used. |
