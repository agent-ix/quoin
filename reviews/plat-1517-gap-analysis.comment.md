<!-- reviewer repo=agent-ix/quoin visibility=public quoin=0.30.1 module=spec-artifacts-process@v0.31.0 id=SR-20261113014536 method=gap-analysis lang=rust pr=quoin#840 model=claude-opus-5-5 run=bee49e35-0666-45b9-ba47-c1838e0c67e0 -->

Gap analysis of quoin#840 against FR-143 (FR-144 consumer, spec-objects-security FR-010 fields). Verdict: CONDITIONAL. All 9 ACs are tagged (`quire matrix`), and 7 of them by tests that can fail. Plan completion: not assessed.

| FND | Severity | Check | Summary |
| --- | --- | --- | --- |
| FND-001 | medium | coverage | Tests for AC-7 (no network) and AC-8 (wall clock) cannot fail; untag now, bind at command level in PLAT-1518 |
| FND-002 | low | coverage | AC-1 "produces no report" and AC-2 "record file unchanged" are command-level; PLAT-1518 must also tag them |
| FND-003 | low | ambiguous | FR-143 leaves author gaps 1-6 unstated and its `--as-of` form disagrees with FR-144; amend FR-143 in this PR |

**FR-143 amendments to ride in this PR (author gaps):**

| Gap | Disposition | Text |
| --- | --- | --- |
| 1 JSON array | amend | Description: "The input file is a JSON array of objects `{purl, support_status, end_of_support, source}`." Behavior: "If the file is not a JSON array of objects, then the command SHALL refuse the file." |
| 2 absent end_of_support | amend, keep code | "If an entry lacks `purl`, `support_status`, `end_of_support` or `source`, or its `source` is empty, ..." (FR-010: `unknown` is stated, never absent) |
| 3 extra keys | change code + amend | "If an entry carries a key other than `purl`, `support_status`, `end_of_support` and `source`, then the command SHALL refuse the file naming the entry and the key." |
| 4 entry `unknown` | amend, keep code | "An entry value of `unknown` states nothing: it neither fills a record's `unknown` or absent value nor disagrees with a stated one." |
| 5 bad record value | amend here, PLAT-1518 wires/tests | "If a record's `support_status` or `end_of_support` is outside its FR-010 vocabulary, then the command SHALL refuse naming the record and SHALL NOT treat the value as `unknown`." + AC: "A record with `support_status: maintained` refuses the report naming the record." |
| 6 exact case | amend (+ code-review FND-003) | "An entry matches a record when the type, namespace and name segments of the two purls, each percent-decoded, are byte-equal (no case folding or type-specific normalisation), ..." |
| `--as-of` form | amend | "The `--as-of` value SHALL be a calendar date, taken as that UTC date, or an RFC 3339 instant, taken as its UTC calendar date." (AC-5 uses a date; FR-144 says `<instant>`) |

+++ [reviewer data]

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
  - {id: FR-143, path: spec/functional/FR-143-support-status-and-end-of-support-input.md, role: examined, excerpt: "The input file is a list of entries `{purl, support_status, end_of_support, source}`."}
  - {id: FR-144, path: spec/functional/FR-144-soup-report.md, role: context_only, excerpt: "When a user runs `quoin soup report --repo <repo> --binding <clause-binding-v1 report> --as-of <instant>`, the command SHALL render one row for each `soup_record` in the repository."}
  - {id: FR-010, path: "agent-ix/spec-objects-security:spec/functional/FR-010-soup-record-fields.md", role: context_only, excerpt: "The `soup_record` schema SHALL admit `support_status` only from the closed vocabulary `SoupSupportStatus`, whose members are exactly `supported`, `community_supported`, `unsupported`, `unknown`."}
findings:
  - fnd: FND-001
    severity: medium
    confidence: high
    method: gap-analysis
    check: coverage
    artifact_id: FR-143-AC-7
    related_ids: [FR-143-AC-8]
    path: rust/crates/quoin-cli/src/soup/support.rs
    lines: "738-762"
    excerpt: |-
      The command run with no network available produces the same output as with it.
    finding: "The tests bound to FR-143-AC-7 (tc_1517_009) and FR-143-AC-8 (tc_1517_008) cannot fail. One asserts only that an unknown date is not flagged; the other compares two in-process calls of a pure function with no clock varied. Both criteria are command-level, and the command arrives in PLAT-1518. Remove the two #[trace] tags now and bind AC-7/AC-8 in PLAT-1518 against the real process: run once with network blocked (unshare -rn or proxy at a closed port) and once without and compare bytes; run twice with the same --as-of under different clocks and compare bytes."
  - fnd: FND-002
    severity: low
    confidence: high
    method: gap-analysis
    check: coverage
    artifact_id: FR-143-AC-1
    related_ids: [FR-143-AC-2]
    path: rust/crates/quoin-cli/src/soup/support.rs
    lines: "516-611"
    excerpt: |-
      An entry with no or empty `source`, an invalid status, an `end_of_support` of `soon`, or a duplicate each refuses the whole file, names the entry, and produces no report.
    finding: "FR-143-AC-1 'produces no report' and FR-143-AC-2 'the record file is unchanged' are command-level clauses this module cannot exercise (it returns Err and does no I/O), so the tags are partial. PLAT-1518's command test must also tag AC-1 and AC-2, asserting no stdout payload on a refused file and byte-identical record files after a run."
  - fnd: FND-003
    severity: low
    confidence: high
    method: gap-analysis
    check: ambiguous
    artifact_id: FR-143
    related_ids: [FR-144, FR-143-AC-5]
    path: spec/functional/FR-143-support-status-and-end-of-support-input.md
    lines: "24-50"
    excerpt: |-
      The input file is a list of entries `{purl, support_status, end_of_support, source}`.
    finding: "FR-143 leaves six behaviours the code decided unstated (author gaps 1-6: JSON array, required end_of_support, extra keys, entry unknown, invalid record value, exact-case segment match), and its --as-of form (AC-5 uses a date) disagrees with FR-144's --as-of <instant>. Add the amendment text in the gap table above to FR-143 in this PR; add a gap-3 refusal case to tc_1517_001."
bindings:
  - {test_id: tc_1517_001, ac_id: FR-143-AC-1, trace: correct}
  - {test_id: tc_1517_002, ac_id: FR-143-AC-2, trace: correct}
  - {test_id: tc_1517_003, ac_id: FR-143-AC-3, trace: correct}
  - {test_id: tc_1517_004, ac_id: FR-143-AC-4, trace: correct}
  - {test_id: tc_1517_005, ac_id: FR-143-AC-5, trace: correct}
  - {test_id: tc_1517_006, ac_id: FR-143-AC-6, trace: correct}
  - {test_id: tc_1517_007, ac_id: FR-143-AC-9, trace: correct}
  - {test_id: tc_1517_009, ac_id: FR-143-AC-7, trace: correct}
  - {test_id: tc_1517_008, ac_id: FR-143-AC-8, trace: correct}
```

+++
