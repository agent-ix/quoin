---
id: SR-1853001
title: "PLAT-1853 PR 1047 code-review disposition"
type: SpecReview
analysis: code-review
scope: "agent-ix/quoin; PR 1047; Rust changes for FR-030 and FR-211"
review_set: subset
relationships:
  - target: "ix://agent-ix/quoin/FR-211"
    type: references
---

# PLAT-1853 PR 1047 code-review disposition

## Summary

Reconstructed from the full Linear reviewer comment for run acb228ea-2ca8-494b-b8a8-8bcc6123d60c. Inspected pushed fixes through 01c1027bffd022f70bb729afd05d42fba7fb2a6f. All seven original findings are fixed by code inspection. Tests were not run.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Invalid stored artifactId errors did not name the stored file, contrary to FR-030-AC-41. | rust/crates/quoin-evidence/src/customer_artifact/mod.rs:174-206 |
| FND-002 | medium | Any parseable JSON under customer-artifacts could be treated as a retained artifact without record validation. | rust/crates/quoin-assurance/src/discharge/verify.rs:520-534 |
| FND-003 | medium | The uppercase reference test could not fail if the lowercase guard were removed. | rust/crates/quoin-assurance/src/discharge/verify.rs:838-855 |
| FND-004 | medium | The customer-artifact size bound was missing from the pre-host refusal test. | rust/crates/quoin-core/src/ops/evidence/records.rs:552 |
| FND-005 | low | CLI artifact input was read with unbounded json_input. | rust/crates/quoin-cli/src/evidence.rs:424 |
| FND-006 | low | CustomerArtifactRefusal::ALL was unused. | rust/crates/quoin-evidence/src/customer_artifact/mod.rs:42-43 |
| FND-007 | low | The GC retirement-log assertion had no run to retire. | rust/crates/quoin-evidence/src/customer_artifact/tests.rs:194-225 |

## Verdict

All original findings are fixed in the pushed branch.

## Dispositions

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | 01c1027b maps stored CustomerArtifactRefused errors to InvalidRecord with the path in the detail; the AC-41 test includes an invalid-charset stored id and asserts the path appears in the error. |
| FND-002 | fixed | 01c1027b makes resolve_reference call read_customer_artifact and resolve only Ok(Some(_)); a new test rejects empty, malformed, mismatched-id and invalid-charset files, with a valid control. |
| FND-003 | fixed | 01c1027b adds an explicit assertion for the folded-form refusal reason after the uppercase reference. |
| FND-004 | fixed | 01c1027b adds evidence.record_customer_artifact at MAX_CUSTOMER_ARTIFACT_BYTES to tc_412 using the RefusingHost path. |
| FND-005 | fixed | 01c1027b reads --artifact with bounded_input and MAX_CUSTOMER_ARTIFACT_INPUT_BYTES before parsing JSON. |
| FND-006 | fixed | 01c1027b adds a test iterating CustomerArtifactRefusal::ALL and asserting both contractual spellings. |
| FND-007 | fixed | 01c1027b seeds two runs, verifies a retirement log exists, and checks it lists a retired run but no customer artifact. |

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
  method: code-review
  check: code-bug
  artifact_id: FR-030-AC-41
  related_ids:
  - FR-030-AC-37
  path: rust/crates/quoin-evidence/src/customer_artifact/mod.rs
  lines: 174-206
  excerpt: A stored customer artifact file that is not valid JSON, holds an `artifactId` that, lowercased,
    differs from its file name (a mixed-case `artifactId` in a lowercase file name is valid), or breaks
    AC-37 is an error naming the file, and is never read as an absent record.
  finding: A stored customer artifact file whose artifactId is outside the identifier charset is reported
    as QE-E011 invalid-identifier (BadRequest) with no file path, because read_file's named wrapper only
    rewrites InvalidRecord; FR-030-AC-41 requires an error naming the file, and a valid submission at
    that name is told its own id is refused. Reproduced by probe.
- fnd: FND-002
  severity: medium
  confidence: high
  method: code-review
  check: code-bug
  artifact_id: FR-211-AC-25
  related_ids:
  - FR-211-AC-26
  - FR-030-AC-41
  path: rust/crates/quoin-assurance/src/discharge/verify.rs
  lines: 520-534
  excerpt: 'A clause whose every `expectedOutputs` entry is `kind: record` or `kind: document` with `producers`
    of `[customer-artifact]` is discharged by a direct fact whose `evidenceRefs` is `evidence://customer-artifacts/<artifact-id>`
    naming a retained customer artifact (FR-030).'
  finding: 'resolve_reference treats any parseable JSON at customer-artifacts/<id>.json as a retained
    customer artifact: {} or a record whose artifactId differs from its file name discharges a clause
    whose producers name customer-artifact; it should resolve through read_customer_artifact and fail
    on an invalid record. Reproduced by probe.'
- fnd: FND-003
  severity: medium
  confidence: high
  method: code-review
  check: test-intent
  artifact_id: FR-211-AC-28
  related_ids: []
  path: rust/crates/quoin-assurance/src/discharge/verify.rs
  lines: 838-855
  excerpt: A ref `evidence://customer-artifacts/<artifact-id>` whose `artifact-id` segment is not lowercase,
    is empty, or holds a `.` or `..` segment is `unresolved`; the family is citable only as `customer-artifacts/<artifact-id>`.
  finding: 'The AC-28 uppercase case is vacuous: MemoryEvidence is case-sensitive, so removing the id.to_ascii_lowercase()
    == *id guard leaves the test green (mutation survived); on a case-insensitive filesystem the uppercase
    ref would resolve.'
- fnd: FND-004
  severity: medium
  confidence: high
  method: code-review
  check: coverage
  artifact_id: rust/crates/quoin-core/src/ops/evidence/records.rs
  related_ids:
  - FR-030
  path: rust/crates/quoin-core/src/ops/evidence/records.rs
  lines: '552'
  excerpt: '    bound(request, op, MAX_CUSTOMER_ARTIFACT_BYTES)?;'
  finding: 'MAX_CUSTOMER_ARTIFACT_BYTES has no refusal test: evidence.record_customer_artifact is missing
    from tc_412_each_bound_refuses_before_the_host_is_consulted, which lists every sibling evidence operation,
    so deleting the bound call survives.'
- fnd: FND-005
  severity: low
  confidence: medium
  method: code-review
  check: other
  artifact_id: rust/crates/quoin-cli/src/evidence.rs
  related_ids: []
  path: rust/crates/quoin-cli/src/evidence.rs
  lines: '424'
  excerpt: '    let document = json_input(&required(arguments, "artifact")?)?;'
  finding: The CLI reads --artifact (file or stdin) through the unbounded json_input although bounded_input
    exists; an oversized stdin is read fully into memory before the 16 KiB core bound refuses it.
- fnd: FND-006
  severity: low
  confidence: high
  method: code-review
  check: other
  artifact_id: rust/crates/quoin-evidence/src/customer_artifact/mod.rs
  related_ids: []
  path: rust/crates/quoin-evidence/src/customer_artifact/mod.rs
  lines: 42-43
  excerpt: '    pub const ALL: &''static [Self] = &[Self::InvalidIdentifier, Self::CaseCollision];'
  finding: CustomerArtifactRefusal::ALL is a public constant no code or test uses; assert the kind spellings
    with it (as ScenarioRefusal::ALL is) or delete it.
- fnd: FND-007
  severity: low
  confidence: high
  method: code-review
  check: test-intent
  artifact_id: FR-030-AC-40
  related_ids: []
  path: rust/crates/quoin-evidence/src/customer_artifact/tests.rs
  lines: 194-225
  excerpt: '`quoin evidence gc` leaves every customer artifact in place, `--dry-run` or not, and its retirement
    log names none.'
  finding: 'The AC-40 test''s retirement-log assertion is vacuous: the store holds no runs, so gc never
    writes a log; seed a superseded run so a log is written, then assert it names no customer artifact.'
bindings: []
```

