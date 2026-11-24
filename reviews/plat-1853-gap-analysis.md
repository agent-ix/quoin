---
id: SR-1853002
title: "PLAT-1853 PR 1047 gap-analysis disposition"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quoin; PR 1047; FR-030, FR-133, FR-211 and changed Rust files"
review_set: subset
relationships:
  - target: "ix://agent-ix/quoin/FR-211"
    type: references
---

# PLAT-1853 PR 1047 gap-analysis disposition

## Summary

Reconstructed from the full Linear reviewer comment for run acb228ea-2ca8-494b-b8a8-8bcc6123d60c. Inspected pushed fixes through 01c1027bffd022f70bb729afd05d42fba7fb2a6f. All five original findings are fixed. Plan completion: not assessed. Tests were not run.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-030-AC-41 did not cover an invalid stored artifactId error naming the file. | rust/crates/quoin-evidence/src/customer_artifact/tests.rs:227 |
| FND-002 | medium | The AC-28 uppercase-reference case could not fail if the lowercase guard were removed. | rust/crates/quoin-assurance/src/discharge/verify.rs:520,838 |
| FND-003 | medium | FR-211-AC-25/26 lacked a test proving invalid artifact files do not discharge clauses. | rust/crates/quoin-assurance/src/discharge/verify.rs:520-534 |
| FND-004 | low | AC-40's retirement-log assertion was vacuous because no run was retired. | rust/crates/quoin-evidence/src/customer_artifact/tests.rs:194 |
| FND-005 | low | AC-36 named the CLI command but no end-to-end command test existed. | rust/crates/quoin-cli/src/evidence.rs:422-451,549 |

## Verdict

All original findings are fixed in the pushed branch.

## Coverage

Plan completion: not assessed.

## Dispositions

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | 01c1027b tests an invalid-charset stored id and requires the path in the error. |
| FND-002 | fixed | 01c1027b asserts the uppercase reference is refused for non-folded form. |
| FND-003 | fixed | 01c1027b validates references through read_customer_artifact; the added test rejects empty JSON, invalid JSON, mismatched ids, and invalid ids, with a valid control. |
| FND-004 | fixed | 01c1027b seeds two runs, then checks the retirement log lists a run but no customer artifact. |
| FND-005 | fixed | 01c1027b adds tc_3503, which spawns the quoin executable and exercises path input, stdin, replay and refusal. The test exists but was not run. |

## Examined

Full original examined scope, source excerpts, findings metadata and bindings are retained in the source record below.

## Original machine-readable findings record

```yaml
clean: false
scope:
- id: FR-030-AC-36
  path: spec/functional/FR-030-evidence-store.md
  role: examined
  excerpt: '`quoin evidence record-customer-artifact` with a document of exactly `artifactId`, `title`,
    `location`, `suppliedBy` and `suppliedAt` writes `customer-artifacts/<artifact-id>.json` (file name
    lowercased, `artifactId` as submitted), canonical, holding those members and `schemaVersion` only,
    and the store reads it back unchanged.'
- id: FR-030-AC-37
  path: spec/functional/FR-030-evidence-store.md
  role: examined
  excerpt: A customer artifact document with a missing, unknown, blank or non-string member, an over-long
    `title`, `suppliedBy` or `location`, an `artifactId` outside 1 to 64 ASCII letters, digits and `-`
    (`invalid-identifier`), or a `suppliedAt` that is not canonical UTC is refused before any file is
    written.
- id: FR-030-AC-38
  path: spec/functional/FR-030-evidence-store.md
  role: examined
  excerpt: Two artifact ids that differ only in letter case file under one lowercase name and the second
    is refused as `case-collision`; an identical replay is a no-op, and a different document at an occupied
    id is refused with `RecordIntegrity` and the stored bytes unchanged.
- id: FR-030-AC-39
  path: spec/functional/FR-030-evidence-store.md
  role: examined
  excerpt: 'The stored customer artifact holds no digest, hash, size or content of the artifact, and recording
    one never reads, opens or fetches the `location`: with a `location` naming an unreadable path and
    an unreachable URL the record is accepted and the stored bytes equal the submitted members plus `schemaVersion`.'
- id: FR-030-AC-40
  path: spec/functional/FR-030-evidence-store.md
  role: examined
  excerpt: '`quoin evidence gc` leaves every customer artifact in place, `--dry-run` or not, and its retirement
    log names none.'
- id: FR-030-AC-41
  path: spec/functional/FR-030-evidence-store.md
  role: examined
  excerpt: A stored customer artifact file that is not valid JSON, holds an `artifactId` that, lowercased,
    differs from its file name (a mixed-case `artifactId` in a lowercase file name is valid), or breaks
    AC-37 is an error naming the file, and is never read as an absent record.
- id: FR-030-CON-4
  path: spec/functional/FR-030-evidence-store.md
  role: examined
  excerpt: The store SHALL NOT compute or persist a content identity, digest or size of a customer artifact.
- id: FR-030-CON-5
  path: spec/functional/FR-030-evidence-store.md
  role: examined
  excerpt: The store SHALL NOT read the artifact a customer artifact's `location` names.
- id: FR-133-AC-8
  path: spec/functional/FR-133-discharge-facts-in-the-evidence-store.md
  role: examined
  excerpt: A stored fact that cites `evidence://customer-artifacts/<artifact-id>` holds that ref string
    and no member of the artifact record, and no digest of the artifact; after a further customer artifact
    is recorded, the stored fact file bytes are unchanged.
- id: FR-211-AC-25
  path: spec/functional/FR-211-discharge-resolves-evidence-and-approvals.md
  role: examined
  excerpt: 'A clause whose every `expectedOutputs` entry is `kind: record` or `kind: document` with `producers`
    of `[customer-artifact]` is discharged by a direct fact whose `evidenceRefs` is `evidence://customer-artifacts/<artifact-id>`
    naming a retained customer artifact (FR-030).'
- id: FR-211-AC-26
  path: spec/functional/FR-211-discharge-resolves-evidence-and-approvals.md
  role: examined
  excerpt: 'The same clause with no `evidenceRefs` entry resolving to a retained customer artifact stays
    open: a ref naming no customer artifact is `unresolved`, and a ref to a retained run or other kind
    outside the `producers` is `kind-not-expected`.'
- id: FR-211-AC-28
  path: spec/functional/FR-211-discharge-resolves-evidence-and-approvals.md
  role: examined
  excerpt: A ref `evidence://customer-artifacts/<artifact-id>` whose `artifact-id` segment is not lowercase,
    is empty, or holds a `.` or `..` segment is `unresolved`; the family is citable only as `customer-artifacts/<artifact-id>`.
- id: FR-211-AC-29
  path: spec/functional/FR-211-discharge-resolves-evidence-and-approvals.md
  role: examined
  excerpt: 'Resolving a ref to a customer artifact reads only the retained record: with the record''s
    `location` naming an unreadable path and an unreachable URL the clause still discharges, and the report
    carries no digest, hash or size of the artifact and no statement that the artifact is adequate.'
- id: FR-211-AC-30
  path: spec/functional/FR-211-discharge-resolves-evidence-and-approvals.md
  role: examined
  excerpt: A customer artifact that `quoin evidence gc` leaves in place keeps its clause discharged on
    the next report, and a stored fact citing it is unchanged.
findings:
- fnd: FND-001
  severity: medium
  confidence: high
  method: gap-analysis
  check: coverage
  artifact_id: FR-030-AC-41
  related_ids:
  - FR-030-AC-37
  path: rust/crates/quoin-evidence/src/customer_artifact/tests.rs
  lines: '227'
  excerpt: A stored customer artifact file that is not valid JSON, holds an `artifactId` that, lowercased,
    differs from its file name (a mixed-case `artifactId` in a lowercase file name is valid), or breaks
    AC-37 is an error naming the file, and is never read as an absent record.
  finding: 'FR-030-AC-41 (breaks AC-37 is an error naming the file) is not met for an inadmissible stored
    artifactId: the error is QE-E011 with no path, and the AC-41 test has no such case.'
- fnd: FND-002
  severity: medium
  confidence: high
  method: gap-analysis
  check: test-intent
  artifact_id: FR-211-AC-28
  related_ids: []
  path: rust/crates/quoin-assurance/src/discharge/verify.rs
  lines: 520, 838
  excerpt: A ref `evidence://customer-artifacts/<artifact-id>` whose `artifact-id` segment is not lowercase,
    is empty, or holds a `.` or `..` segment is `unresolved`; the family is citable only as `customer-artifacts/<artifact-id>`.
  finding: 'FR-211-AC-28''s uppercase-segment case cannot fail as tested: removing the lowercase guard
    leaves the tagged test green on the case-sensitive MemoryEvidence.'
- fnd: FND-003
  severity: medium
  confidence: high
  method: gap-analysis
  check: coverage
  artifact_id: FR-211-AC-25
  related_ids:
  - FR-211-AC-26
  path: rust/crates/quoin-assurance/src/discharge/verify.rs
  lines: 520-534
  excerpt: 'A clause whose every `expectedOutputs` entry is `kind: record` or `kind: document` with `producers`
    of `[customer-artifact]` is discharged by a direct fact whose `evidenceRefs` is `evidence://customer-artifacts/<artifact-id>`
    naming a retained customer artifact (FR-030).'
  finding: FR-211-AC-25/AC-26 (naming a retained customer artifact) have no test that an invalid file
    under customer-artifacts/ is not a retained customer artifact; a probe shows {} discharges the clause.
- fnd: FND-004
  severity: low
  confidence: high
  method: gap-analysis
  check: test-intent
  artifact_id: FR-030-AC-40
  related_ids: []
  path: rust/crates/quoin-evidence/src/customer_artifact/tests.rs
  lines: '194'
  excerpt: '`quoin evidence gc` leaves every customer artifact in place, `--dry-run` or not, and its retirement
    log names none.'
  finding: 'FR-030-AC-40''s ''its retirement log names none'' half is vacuous: the test store holds no
    runs, so gc never writes a retirement log to inspect.'
- fnd: FND-005
  severity: low
  confidence: medium
  method: gap-analysis
  check: coverage
  artifact_id: FR-030-AC-36
  related_ids: []
  path: rust/crates/quoin-cli/src/evidence.rs
  lines: 422-451, 549
  excerpt: '`quoin evidence record-customer-artifact` with a document of exactly `artifactId`, `title`,
    `location`, `suppliedBy` and `suppliedAt` writes `customer-artifacts/<artifact-id>.json` (file name
    lowercased, `artifactId` as submitted), canonical, holding those members and `schemaVersion` only,
    and the store reads it back unchanged.'
  finding: FR-030-AC-36 names quoin evidence record-customer-artifact, but its CLI binder tc_373 only
    parses argv; no test runs the CLI command end to end (the core op is covered by tc_3501).
bindings:
- test_id: a_document_is_filed_canonical_under_a_lowercased_name_and_reads_back
  ac_id: FR-030-AC-36
  trace: correct
- test_id: tc_3501_the_customer_artifact_operation_files_one_record_or_refuses_by_taxonomy
  ac_id: FR-030-AC-36
  trace: correct
- test_id: tc_373_evidence_store_command_grammar_is_retained
  ac_id: FR-030-AC-36
  trace: correct
- test_id: an_invalid_document_is_refused_before_any_file_is_written
  ac_id: FR-030-AC-37
  trace: correct
- test_id: tc_3501_the_customer_artifact_operation_files_one_record_or_refuses_by_taxonomy
  ac_id: FR-030-AC-37
  trace: correct
- test_id: the_store_is_append_only_at_a_file_name
  ac_id: FR-030-AC-38
  trace: correct
- test_id: tc_3501_the_customer_artifact_operation_files_one_record_or_refuses_by_taxonomy
  ac_id: FR-030-AC-38
  trace: correct
- test_id: the_record_holds_only_the_submitted_members_and_never_touches_the_location
  ac_id: FR-030-AC-39
  trace: correct
- test_id: the_record_holds_only_the_submitted_members_and_never_touches_the_location
  ac_id: FR-030-CON-4
  trace: correct
- test_id: the_record_holds_only_the_submitted_members_and_never_touches_the_location
  ac_id: FR-030-CON-5
  trace: correct
- test_id: gc_never_collects_a_customer_artifact
  ac_id: FR-030-AC-40
  trace: correct
- test_id: an_invalid_stored_file_is_an_error_naming_it_never_an_absent_record
  ac_id: FR-030-AC-41
  trace: correct
- test_id: tc_3502_a_fact_citing_a_customer_artifact_holds_only_the_ref
  ac_id: FR-133-AC-8
  trace: correct
- test_id: a_customer_artifact_ref_discharges_a_clause_whose_producers_name_the_kind
  ac_id: FR-211-AC-25
  trace: correct
- test_id: without_a_retained_customer_artifact_of_the_kind_the_clause_stays_open
  ac_id: FR-211-AC-26
  trace: correct
- test_id: a_customer_artifact_ref_is_citable_only_as_a_lowercase_single_segment
  ac_id: FR-211-AC-28
  trace: correct
- test_id: resolving_a_customer_artifact_reads_only_the_record_and_reports_no_digest
  ac_id: FR-211-AC-29
  trace: correct
- test_id: a_customer_artifact_survives_gc_and_keeps_its_clause_discharged
  ac_id: FR-211-AC-30
  trace: correct
```

