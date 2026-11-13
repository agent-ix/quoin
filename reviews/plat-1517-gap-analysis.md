---
id: SR-20261113014536
title: "Gap analysis of quoin PR 840 (FR-143 coverage)"
type: SpecReview
analysis: base
scope: "agent-ix/quoin@084c12bb; spec/functional/FR-143-support-status-and-end-of-support-input.md, spec/functional/FR-144-soup-report.md, rust/crates/quoin-cli/src/soup/support.rs"
review_set: subset
---

# Gap analysis of quoin PR 840

## Summary

Ticket: PLAT-1517. Plan completion: not assessed. Measured against FR-143 at 084c12bb, with
FR-144 as the consumer and agent-ix/spec-objects-security FR-009..FR-012 (FR-010 for
`support_status`/`end_of_support` and the `SoupSupportStatus` members) as context. `quire
matrix --scope .` (quire 0.44.0, engine 0.59.1) reports FR-143-AC-1..AC-9 all `tagged` by one
test each, and CON-1 `method-without-symbol` (Inspection, so expected). All 10 `tc_1517` tests
pass.

Each binding was read to check that its test can fail. AC-1, AC-3, AC-4, AC-5, AC-6 and AC-9
can fail on a real defect: changing `>` to `>=`, letting the first matching entry win, keeping
qualifiers, or ignoring the offset each breaks a named assertion. AC-2 is sound for its
module-level half. AC-7 and AC-8 cannot fail (FND-001). The record-side parser
`record_support` is tested by tc_1517_010, which is not tagged. That is correct, because no
FR-143 criterion owns record parsing. Author gap 5 below gives it a criterion.

No unowned behaviour: every public item of `support.rs` serves an FR-143 behaviour clause.
There are no stubs and no copied cross-repo content. The `SoupSupportStatus` members are
written out in Rust, not copied from a schema file, and they match FR-010 exactly.

The author reported six spec gaps. Each one is resolved below. All are small clarifications,
and they should ride in this PR as a FR-143 amendment (FND-003).

| Gap | Disposition | Recommended FR-143 text |
| --- | --- | --- |
| 1 JSON array only | amend | Description: "The input file is a JSON array of objects `{purl, support_status, end_of_support, source}`." Behavior: "If the file is not a JSON array of objects, then the command SHALL refuse the file." |
| 2 absent end_of_support refused | amend (keep the code) | Behavior: "If an entry lacks `purl`, `support_status`, `end_of_support` or `source`, or its `source` is empty, ..." This matches FR-010: a scalar states "not known" as the literal `unknown`, never by absence |
| 3 extra keys ignored | change the code and amend (code-review FND-002) | Behavior: "If an entry carries a key other than `purl`, `support_status`, `end_of_support` and `source`, then the command SHALL refuse the file naming the entry and the key." |
| 4 `unknown` in an entry is not stated | amend (keep the code) | Behavior: "An entry value of `unknown` states nothing: it neither fills a record's `unknown` or absent value nor disagrees with a stated one." |
| 5 invalid record value refused (`BadRecordValue`) | amend here; PLAT-1518 wires and tests it | Behavior: "If a record's `support_status` or `end_of_support` is outside its FR-010 vocabulary, then the command SHALL refuse naming the record and SHALL NOT treat the value as `unknown`." Add an AC: "A record with `support_status: maintained` refuses the report naming the record." |
| 6 exact-case purl match | amend; also fix code-review FND-003 | Description: "An entry matches a record when the type, namespace and name segments of the two purls, each percent-decoded, are byte-equal (no case folding or type-specific normalisation), ..." |
| (found here) `--as-of` form | amend | FR-143-AC-5 uses `--as-of 2026-06-01`, a date, but FR-144 says `--as-of <instant>`. FR-143 Behavior: "The `--as-of` value SHALL be a calendar date, taken as that UTC date, or an RFC 3339 instant, taken as its UTC calendar date." |

## Verdict

**CONDITIONAL (one medium, two low).** Every FR-143 criterion is tagged. Two are tagged by tests
that cannot fail, so the matrix overstates coverage. Mergeable once FND-001 is fixed (untag the
two tests) and the FR-143 amendment (FND-003) rides in the PR.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The tests for FR-143-AC-7 (no network) and FR-143-AC-8 (wall-clock independence) cannot fail. tc_1517_009 asserts only that an `unknown` date is not flagged. tc_1517_008 compares two calls to a pure function in one process, with no clock varied. Both are command-level criteria, and the command does not exist until PLAT-1518. Remove the two `#[trace]` tags in this PR (delete the tests or leave them untagged), and bind AC-7/AC-8 in PLAT-1518 against the real `quoin soup report` process. Example: run once with network access blocked (`unshare -rn`, or proxy variables pointing at a closed port) and once without, then compare bytes, and run twice with the same `--as-of` under two different clocks (`faketime`, or a far-future `--as-of` against today's clock) and compare bytes | rust/crates/quoin-cli/src/soup/support.rs:738-762 |
| FND-002 | low | FR-143-AC-1 "produces no report" and FR-143-AC-2 "the record file is unchanged" are command-level clauses, and this PR cannot exercise them. The module returns `Err` and does no I/O. The tags are partial. PLAT-1518's command test must also tag AC-1 and AC-2, asserting no stdout payload on a refused file and byte-identical record files after a run | rust/crates/quoin-cli/src/soup/support.rs:516-611 |
| FND-003 | low | FR-143 leaves unstated six behaviours the code had to decide (author gaps 1-6), and its `--as-of` form disagrees with FR-144. Add the amendment text in the Summary table to FR-143 in this PR, and add a refusal case for gap 3 to tc_1517_001 | spec/functional/FR-143-support-status-and-end-of-support-input.md:24-50 |

## Coverage

- Criteria examined: FR-143-AC-1..AC-9 (9/9 tagged; 7 sound, 2 vacuous per FND-001), FR-143-CON-1 (Inspection: the module checks only presence and non-emptiness of `source`, which holds).
- Reverse gap: none; no unowned behaviour.
- Stubs / inflation: none in production; inflation per FND-001.
- Semantic review: run (caller requested intent checks).
- Plan completion: not assessed

## Examined scope

```yaml
clean: false
scope:
- {id: FR-143-AC-1, path: spec/functional/FR-143-support-status-and-end-of-support-input.md, role: examined, excerpt: "An entry with no or empty `source`, an invalid status, an `end_of_support` of `soon`, or a duplicate each refuses the whole file, names the entry, and produces no report."}
- {id: FR-143-AC-2, path: spec/functional/FR-143-support-status-and-end-of-support-input.md, role: examined, excerpt: "A record with `end_of_support: unknown` and a matching entry shows the entry's date labelled with its source, and the record file is unchanged."}
- {id: FR-143-AC-3, path: spec/functional/FR-143-support-status-and-end-of-support-input.md, role: examined, excerpt: "A record with a stated date and a different entry date shows both, flags the disagreement, and treats the record's date as authoritative."}
- {id: FR-143-AC-4, path: spec/functional/FR-143-support-status-and-end-of-support-input.md, role: examined, excerpt: "A record with `unknown` status and no matching entry shows `unknown` for both fields and carries no past-end-of-support flag."}
- {id: FR-143-AC-5, path: spec/functional/FR-143-support-status-and-end-of-support-input.md, role: examined, excerpt: "With `--as-of 2026-06-01`, a component whose end of support is 2026-05-31 is flagged past end of support and one whose date is 2026-06-01 or later is not."}
- {id: FR-143-AC-6, path: spec/functional/FR-143-support-status-and-end-of-support-input.md, role: examined, excerpt: "A versioned entry takes precedence over a version-less entry for the same package; a version-less entry matches a record of any version, and an entry for a different version matches nothing; `pkg:cargo/a` and `pkg:cargo/a@1` in one file are not duplicates."}
- {id: FR-143-AC-7, path: spec/functional/FR-143-support-status-and-end-of-support-input.md, role: examined, excerpt: "The command run with no network available produces the same output as with it."}
- {id: FR-143-AC-8, path: spec/functional/FR-143-support-status-and-end-of-support-input.md, role: examined, excerpt: "Two runs with different wall-clock times and the same `--as-of` produce identical output."}
- {id: FR-143-AC-9, path: spec/functional/FR-143-support-status-and-end-of-support-input.md, role: examined, excerpt: "With `--as-of 2026-05-31T12:00:00Z` a component ending 2026-05-31 is not flagged, and with `--as-of 2026-06-01T00:00:00Z` it is flagged; an `--as-of` with a non-UTC offset is compared by its UTC date."}
- {id: FR-143-CON-1, path: spec/functional/FR-143-support-status-and-end-of-support-input.md, role: examined, excerpt: "The author of the support file SHALL cite only public sources; the command checks only that each `source` is present and non-empty."}
- {id: FR-144, path: spec/functional/FR-144-soup-report.md, role: context_only, excerpt: "When a user runs `quoin soup report --repo <repo> --binding <clause-binding-v1 report> --as-of <instant>`, the command SHALL render one row for each `soup_record` in the repository."}
- {id: FR-010, path: "agent-ix/spec-objects-security:spec/functional/FR-010-soup-record-fields.md", role: context_only, excerpt: "The `soup_record` schema SHALL admit `support_status` only from the closed vocabulary `SoupSupportStatus`, whose members are exactly `supported`, `community_supported`, `unsupported`, `unknown`."}
```
