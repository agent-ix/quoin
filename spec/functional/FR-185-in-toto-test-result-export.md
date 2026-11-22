---
id: FR-185
title: "Export retained verification evidence as an in-toto Test Result statement"
type: FR
relationships:
  - target: "ix://agent-ix/engineering-assurance/FR-069"
    type: "references"
  - target: "ix://agent-ix/quoin/FR-068"
    type: "references"
---
# FR-185: Export retained verification evidence as an in-toto Test Result statement

## Description

Quoin SHALL export its retained evidence as an in-toto Statement v1 with the
Test Result v0.1 predicate, so consumers can attest the statement using the
subject supplied by the caller. The consumer contract is
[Engineering Assurance FR-069](ix://agent-ix/engineering-assurance/FR-069),
which this requirement adopts unchanged. The predicate fields conform to the
[in-toto Test Result predicate](https://github.com/in-toto/attestation/blob/main/spec/predicates/test-result.md).

## Inputs

- `--subject <subject.json>`: a non-empty JSON array of in-toto resource
  descriptors supplied by the caller. Each descriptor SHALL include a
  `digest.sha256` value containing exactly 64 lowercase hexadecimal characters.
  Each descriptor SHALL also include a non-empty `annotations.revision` string.
  All descriptors in one export SHALL carry the same revision. Those
  caller-supplied digests bind the statement to artifact content; the revision
  annotation identifies the immutable candidate revision whose evidence is
  being exported. Quoin computes no digest and does not resolve the revision.
- `--required-obligations <obligations.json>`: a non-empty JSON array of unique
  obligation identifiers declared independently of the retained evidence
  bindings. This is the complete population the export claims to assess.
- `--output <statement.json>`: the destination path for the statement.
- `--evidence-dir <DIR>`: the retained evidence store to read; it defaults to
  `spec/evidence`.

## Outputs

- One RFC 8785 canonical JSON in-toto Statement v1 at `--output`, containing the
  supplied `subject` and the Test Result v0.1 predicate derived from the
  evidence store.
- On failure, no output file is created or replaced.

## Behavior

- When a user runs `quoin evidence export-test-result`, Quoin SHALL read only
  the retained run records, `bindings.json`, and the caller-selected
  required-obligations file and SHALL NOT read Git state, access the network,
  or spawn a process.
- The statement SHALL contain exactly `_type`, `subject`, `predicateType` and
  `predicate`, with `_type` equal to
  `https://in-toto.io/Statement/v1` and `predicateType` equal to
  `https://in-toto.io/attestation/test-result/v0.1`.
- The predicate SHALL contain exactly `result`, `configuration`,
  `passedTests`, `warnedTests` and `failedTests`. Its closed deserialization
  type SHALL reject unknown members.
- Quoin SHALL select the latest run for each suite by greatest lexical
  `timestamp`; when timestamps tie, Quoin SHALL select the record with the
  lexicographically greatest store path.
- For each suite used in the export, `configuration` SHALL contain one entry
  with `name` equal to the suite id and annotations `tool` and `timestamp` from
  the selected run for that suite. Entries SHALL be sorted by suite id.
- Quoin SHALL evaluate exactly the obligation identifiers in the required
  population file. An obligation absent from `bindings.json` SHALL appear in
  `failedTests`; an obligation whose binding has no symbols SHALL appear in
  `failedTests` as well. Neither condition SHALL remove an obligation from the
  export population. Quoin SHALL NOT add bindings outside the declared
  population to the result lists.
- For each declared obligation, Quoin SHALL inspect the latest run of every
  bound suite and the symbols named by that binding.
- Every selected run that contributes evidence to an obligation SHALL identify
  the same revision as the `annotations.revision` value on every subject
  descriptor. A missing or different run revision SHALL classify the
  obligation as failed. The exporter SHALL preserve the caller's subject
  descriptors and SHALL NOT infer, resolve, or replace their revision.
- New run records SHALL use evidence-store schema version 2 and carry
  a non-empty `subjectRevision` field with the immutable revision they
  describe. The value SHALL equal the subject's `annotations.revision`.
  Readers SHALL continue to read version-1 run records, but those records have
  no subject-revision commitment and SHALL NOT support a `PASSED` result. A
  version-1 record
  selected for a required obligation SHALL cause that obligation to appear in
  `failedTests`. Writers SHALL emit only the new schema version after the
  transition; they SHALL NOT add a revision field while retaining the version-1
  schema meaning.
- For each bound symbol that has one or more entries in its selected run,
  Quoin SHALL require at least one entry with that symbol whose `traceIds`
  contains the binding's obligation id before the symbol's outcomes can
  support that obligation. An entry for the symbol with absent, empty, or
  non-matching `traceIds` SHALL NOT by itself satisfy the binding. If the
  symbol is present but no entry relates it to the obligation, Quoin SHALL
  classify the obligation as stale and put it in `failedTests`, not
  `passedTests` or `warnedTests`. This failure is represented in the Test
  Result and does not make an otherwise readable run record an invalid
  evidence store. Run records without `traceIds` remain readable, but an
  existing bound symbol in such a record cannot satisfy this relation. This
  rule does not change the existing missing-run, failed-symbol, skipped-symbol,
  or duplicate-outcome rules.
- When a bound symbol has no entry in the selected run, Quoin SHALL classify
  the obligation in `warnedTests` under the existing rule.
- Quoin SHALL reuse the stale-evidence conditions of
  `quoin-auditor::audit::stale::FindingKind::STALE_EVIDENCE`. A bound suite
  without a recorded run, or a latest bound-symbol outcome of `fail` or
  `error`, makes the obligation stale. If a run contains more than one entry
  for a bound symbol, any `fail` or `error` entry makes the obligation stale;
  a passing duplicate SHALL NOT mask it. This export is run-specific: a scan
  record for the suite does not satisfy the run requirement, so a missing run
  makes the obligation stale even when a scan exists.
- If an obligation is stale under the
  `quoin-auditor::audit::stale::FindingKind::STALE_EVIDENCE` conditions, then
  Quoin SHALL put it in `failedTests`.
- When an obligation is not stale and any bound symbol has outcome `skip` or
  is absent from its run, Quoin SHALL put it in `warnedTests`. If a run contains
  more than one entry for a bound symbol, any `skip` entry warns the obligation
  unless a `fail` or `error` entry makes it stale.
- When an obligation is not stale and every bound symbol has outcome `pass`,
  Quoin SHALL put it in `passedTests`.
- This export SHALL NOT define a second staleness rule.
- Quoin SHALL sort and deduplicate the three test lists.
- Quoin SHALL place each obligation in exactly one list.
- `result` SHALL be `FAILED` when `failedTests` is non-empty, `WARNED` when
  `failedTests` is empty and `warnedTests` is non-empty, and `PASSED` otherwise.
- Quoin SHALL copy the caller's subject array without computing or adding any
  digest. Quoin SHALL validate that the array is non-empty. Quoin SHALL require
  every descriptor to have a `digest.sha256` of exactly 64 lowercase
  hexadecimal characters and the same non-empty `annotations.revision` string.
- Quoin SHALL validate the required-obligations input as a non-empty array of
  unique non-empty identifiers. A missing, malformed, or empty required
  population SHALL be a usage error. Quoin SHALL NOT infer that population from
  `bindings.json`.
- Quoin SHALL write the canonical statement atomically by writing a temporary
  file and renaming it to the destination only after successful export.
- The command SHALL exit 2 for usage errors or an invalid subject, exit 3 for
  an unreadable or invalid evidence store with diagnostic code
  `evidence_store_invalid` and the relevant path, and exit 4 when the output
  cannot be written. It SHALL leave no output file on every error.

## Constraints

| ID | Constraint | Type | Validation |
|----|------------|------|------------|
| FR-185-CON-1 | The export SHALL not sign the statement, compute the subject, read Git state, access the network, or spawn a process. | Security | Inspection |
| FR-185-CON-2 | Quoin SHALL NOT vendor the in-toto schema or protobuf files. | Maintainability | Inspection |

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-185-AC-1 | A fixture with correctly traced passing evidence, a correctly traced failing symbol, and an obligation bound to a suite without a run exports `result: FAILED`; only the passing obligation is in `passedTests`, and the other two are in `failedTests`. | Test |
| FR-185-AC-2 | A fixture whose only non-pass outcome is a correctly traced `skip` exports `WARNED` with that obligation in `warnedTests`, including when a correctly traced passing duplicate entry precedes the skip; correctly traced all-pass evidence exports `PASSED` with empty `warnedTests` and `failedTests`. | Test |
| FR-185-AC-3 | The output subject equals the supplied array after canonicalization; a non-hex digest, missing `sha256`, missing or inconsistent revision annotation, empty array or non-array subject exits 2 and writes no output file. | Test |
| FR-185-AC-4 | Repeating an export over the same store and subject yields byte-identical output, and reordering the stored runs or bindings does not change the bytes. | Test |
| FR-185-AC-5 | The predicate has exactly the five specified members, deserializes through its closed type, and rejects a predicate with an additional member. | Test |
| FR-185-AC-6 | An unparsable run record, unreadable runs directory, or missing `bindings.json` exits 3 with `evidence_store_invalid` and the relevant path, and leaves no output file; missing or malformed required-obligations input exits 2. | Test |
| FR-185-AC-7 | A static boundary check establishes that the export path calls no Git, network or process-spawning API. | Inspection |
| FR-185-AC-8 | The consumer-side negative check from Engineering Assurance FR-069-AC-2 is reproducible from Quoin: an export cannot produce `result: PASSED` with a non-empty `failedTests`, including when a correctly traced passing duplicate entry precedes a failing entry for the same bound symbol. | Test |
| FR-185-AC-9 | When a bound symbol has a passing run entry but that entry omits `traceIds` or lists only another obligation, the exporter places the bound obligation in `failedTests`, never `passedTests` or `warnedTests`, and sets `result: FAILED`; the same result applies to a legacy run record with no `traceIds`. A matching `traceIds` entry permits the existing outcome rules to classify the obligation. | Test |
| FR-185-AC-10 | Given a required population containing an obligation omitted from `bindings.json`, the exporter lists that obligation in `failedTests`; adding or removing unrelated bindings does not shrink or expand the declared population. | Test |
| FR-185-AC-11 | An obligation with a binding whose `symbols` array is empty is listed in `failedTests`; an empty required-population file, duplicate identifiers, or a malformed file exits 2 without creating or replacing output. | Test |
| FR-185-AC-12 | A new-version run whose revision matches all subject annotations can support the existing verdict rules; a different or missing revision is listed in `failedTests`. A readable version-1 run remains readable but cannot support `PASSED`. | Test |

## Dependencies

- **Consumer contract**: [Engineering Assurance FR-069](ix://agent-ix/engineering-assurance/FR-069).
- **Command boundary**: [FR-068](./FR-068-producer-facing-change-assurance-cli-surface.md).
