---
id: SR-20261113015348
title: "Gap analysis of quoin PR 841 (FR-142 coverage)"
type: SpecReview
analysis: base
scope: "agent-ix/quoin@3b9ddbe5; spec/functional/FR-142-advisory-join-onto-soup-records.md, spec/functional/FR-144-soup-report.md, rust/crates/quoin-cli/src/soup/advisory.rs"
review_set: subset
---

# Gap analysis of quoin PR 841

## Summary

Ticket: PLAT-1516. Plan completion: not assessed. The audit measured FR-142 at 3b9ddbe5. Context
was FR-033 (the registry), FR-034 (the cargo-audit and cargo-deny adapters), FR-046 and FR-133
(dispositions and discharge facts), FR-141 and the merged PR 838 promote code, and FR-144 as the
consumer. `quire matrix --scope .` (quire 0.44.0, engine 0.59.1) reports every Test criterion
`tagged`: FR-142-AC-1..10, AC-12..15 and CON-1. AC-11 and CON-2 are Inspection and report
`method-without-symbol`, as expected. All 12 tests in the module pass.

Each binding was read to check that its test can fail.

- **Sound bindings.** AC-1, AC-2, AC-3, AC-4, AC-6, AC-7, AC-8, AC-9, AC-13 and AC-15 assert
  literals and fail on a real defect. Examples: dropping the version from the match key,
  treating a vacuous scan as read, using `>` for `>=` on the attestation, flipping the
  newest-scan order, or counting a deny ban as an advisory.
- **AC-3 can fail.** It counts 4 distinct advisories as a literal, alongside the literal
  unmatched list, so its equality is not a self-agreeing re-derivation.
- **AC-10 and CON-1 hold by construction.** `join` takes slices and returns a value, so no
  record can be written. The byte-identity test is cheap and correct.
- **AC-11 holds by inspection.** No new parser or adapter module was added. Scans are typed
  through `select_finding_adapter`, and the purl reading reuses `plan::parse_purl`.
- **CON-2 holds.** Nothing in the join decides whether an advisory blocks a release.
- **Weak bindings.** AC-5, AC-12 and AC-14 are bound by tests that exercise less than the
  criterion says. See FND-001..FND-003.

Reverse gap: one test exercises a spec behaviour that has no criterion (FND-004). Every other
item in `advisory.rs` serves an FR-142 behaviour clause. There are no stubs and no copied
cross-repo content. The repo reads the real cargo-audit fixture and the real
`cargo-deny-failed-real.jsonl` in place.

## Verdict

**CONDITIONAL: six low findings.** Every criterion is tagged, and most bindings can fail. The
findings are three weak test bindings, one untagged behaviour, and two spec questions: an
owner decision on FR-142 and a consistency fix for FR-144. The FR-142 amendments for the
author's spec-silent choices are in the code-review artifact SR-20261113015347. None of these
blocks merge on its own.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | FR-142-AC-5 says recording happens "through `quoin evidence record --tool cargo-deny`". tc_1516_004 only calls `parse_cargo_deny` on the fixture. Its "deny not run" half reuses AC-4's store, which holds no deny scan, so the refusal and the join input are never linked. tc_2706 (quoin-core, FR-034-AC-25) already shows `evidence.record` refuses this fixture and writes no scan. Either drive `evidence.record` into a temp store in the AC-5 test and join the scans actually retained, or reword AC-5: "Recording a cargo-deny log whose summary counts more errors than its diagnostics is refused (FR-034-AC-25), and a join over the scans then retained shows cargo-deny not run and no record free of its advisories." | rust/crates/quoin-cli/src/soup/advisory.rs:962-971 |
| FND-002 | low | FR-142-AC-12 describes a record first accepted for advisory A, then a newer scan that also reports B. tc_1516_008 joins only the newer scan, so it never shows A resolved against the earlier scan. Add the earlier A-only scan (at BEFORE) to the retained set. Assert that A is resolved with the earlier scan alone, and that adding the newer scan unresolves both A and B | rust/crates/quoin-cli/src/soup/advisory.rs:1051-1092 |
| FND-003 | low | FR-142-AC-14 asks for an authored `vulnerability` edge to the record. tc_1516_010 writes a `vulnerability:` key into the soup_record's own frontmatter. That is not how a spec-objects-security `vulnerability` object names a record (its `affects` edge). The AC holds anyway, because `join` has no vulnerability input, but the fixture models the wrong thing. Write a separate `type: vulnerability` file whose `affects` names SOUP-1 into the same directory, and keep the assertions | rust/crates/quoin-cli/src/soup/advisory.rs:1132-1139 |
| FND-004 | low | `a_purl_path_matches_by_purl_ignoring_qualifiers` tests the FR-142 match rule "Where a finding's path is itself a purl, purl equality applies", but no criterion owns that rule and the test carries no `#[trace]`. Add a criterion and tag the test with it: "FR-142-AC-16, A finding whose path is `pkg:cargo/a@1.0.0?arch=x86#sub` attaches to the record with purl `pkg:cargo/a@1.0.0`, Test" | rust/crates/quoin-cli/src/soup/advisory.rs:1207-1217 |
| FND-005 | low | Owner decision needed. FR-142 reads one scan per tool across all suites, the newest. A repository with two cargo-audit suites, for example two lockfiles, has one suite's advisories dropped silently, and its records reported free of cargo-audit on the other suite's evidence. The code follows the spec as written, and AC-15 relies on it, since it uses two suites. Either keep it and say so: "The join reads one scan per tool, the newest across all suites; a repository with several scans of one tool in different suites is read through the newest only." Or read the newest scan per suite and change AC-15 to two scans of one suite | spec/functional/FR-142-advisory-join-onto-soup-records.md |
| FND-006 | low | FR-144-AC-5 expects "a refused scan" to appear after the rows "with their reason". FR-142 reports a refused cargo-deny log as not run, because a refused log leaves no record, so the join can never produce a refused state. Amend FR-144-AC-5 with PLAT-1518: "Unmatched advisories, a not-run scan (including one whose log was refused at recording, FR-142), and a vacuous scan each appear after the rows with their reason." | spec/functional/FR-144-soup-report.md |

## Coverage

- Plan completion: not assessed
- FR-142 criteria: 17. Test criteria: 15, all tagged. Inspection criteria: 2 (AC-11, CON-2),
  both hold by inspection.
- Bindings judged able to fail: 12 of 15 (AC-1..4, AC-6..10, AC-13, AC-15, CON-1). Bound but
  weaker than the criterion: AC-5, AC-12, AC-14.
- Untagged tests: 1 (`a_purl_path_matches_by_purl_ignoring_qualifiers`, FND-004).
- Semantic review: done inline for FR-142, intent↔test↔code per criterion, as part of the
  reviewer brief.
