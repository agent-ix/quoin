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
- `--execution-provenance <provenance.json>`: the versioned, trust-verified
  DSSE envelope produced by the Engineering Assurance release-evidence
  verifier under its existing trust policy. Its version-2 payload contains the
  immutable `sourceRevision`, the complete computed Quire Test Matrix for that
  revision, and a `runs` list of suite, record path, and SHA-256 for each exact
  run-record byte sequence. A caller-selected obligation list is not an
  accepted population source.
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
  the retained run records, `bindings.json`, and the execution-provenance file
  and SHALL NOT read Git state, access the network, or spawn a process.
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
- Quoin SHALL derive the required population from every active obligation in
  the computed Test Matrix for the immutable source revision identified by the
  execution provenance and the maintained suite declarations for that
  revision. Quoin SHALL NOT accept a caller-selected subset or infer the
  population from `bindings.json`.
- The trusted Engineering Assurance release-evidence verifier SHALL sign the
  version-2 execution provenance payload as a DSSE envelope under its existing
  trust policy. The payload SHALL commit to the source revision, complete Test
  Matrix, and exact run-record bytes as one verified execution; its source
  revision SHALL be the revision used to compute the matrix and execute the
  listed suites.
- Quoin SHALL verify the provenance signature and every committed matrix and
  run-record digest against the trusted verifier identity from the existing
  trust policy before using the population or run records.
- For every required obligation, Quoin SHALL include exactly one verdict in
  the Test Result lists. An obligation absent from `bindings.json`, or whose
  binding has no symbols, SHALL appear in `failedTests`. An obligation SHALL
  NOT disappear because its run, binding, or symbols are missing. Bindings
  outside the required population SHALL NOT add obligations to result lists.
- For each declared obligation, Quoin SHALL inspect the latest run of every
  bound suite and the symbols named by that binding.
- Each subject descriptor SHALL carry a non-empty `annotations.revision` equal
  to the source revision in the verified execution provenance. Quoin SHALL
  preserve caller subject descriptors and SHALL NOT resolve or replace their
  revision.
- Quoin SHALL consume a new versioned execution-provenance contract. The
  contract SHALL bind the subject revision, complete computed matrix, and
  exact run records without changing the RunRecord schema. FR-030 SHALL
  continue to prohibit commit, revision, content digest, CI association,
  signer, or report fields in RunRecord.
- Readers SHALL continue to read version-1 RunRecord values. Version-1 run
  records without matching verified execution provenance SHALL remain
  readable but SHALL NOT support a `PASSED` result. Missing, unverified,
  wrong-revision, or incomplete provenance SHALL classify affected required
  obligations as failed; if completeness or authenticity cannot be established,
  Quoin SHALL refuse the export without creating or replacing output.
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
- Quoin SHALL refuse to emit `PASSED` unless the authenticated execution
  provenance matches the subject revision and proves the complete computed
  Test Matrix population. Missing or malformed provenance SHALL be an export
  error and SHALL leave no output file.
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
| FR-185-AC-6 | An unparsable run record, unreadable runs directory, or missing `bindings.json` exits 3 with `evidence_store_invalid` and the relevant path, and leaves no output file; missing or malformed execution provenance leaves no output file. | Test |
| FR-185-AC-7 | A static boundary check establishes that the export path calls no Git, network or process-spawning API. | Inspection |
| FR-185-AC-8 | The consumer-side negative check from Engineering Assurance FR-069-AC-2 is reproducible from Quoin: an export cannot produce `result: PASSED` with a non-empty `failedTests`, including when a correctly traced passing duplicate entry precedes a failing entry for the same bound symbol. | Test |
| FR-185-AC-9 | When a bound symbol has a passing run entry but that entry omits `traceIds` or lists only another obligation, the exporter places the bound obligation in `failedTests`, never `passedTests` or `warnedTests`, and sets `result: FAILED`; the same result applies to a legacy run record with no `traceIds`. A matching `traceIds` entry permits the existing outcome rules to classify the obligation. | Test |
| FR-185-AC-10 | For the complete computed Test Matrix population in verified provenance, an obligation omitted from `bindings.json` is listed in `failedTests`; adding or removing unrelated bindings does not shrink or expand the matrix population. | Test |
| FR-185-AC-11 | An obligation whose binding has no symbols is listed in `failedTests`; provenance with an incomplete Test Matrix or run list fails signature/digest verification and cannot produce `PASSED`. | Test |
| FR-185-AC-12 | A valid provenance signature from the trusted Engineering Assurance verifier, with matrix and run-record digests matching the supplied bytes, and a subject revision matching the signed source revision permits the existing verdict rules; missing, invalid-signature, substituted, incomplete, or mismatched provenance cannot produce `PASSED`. Existing version-1 RunRecord values remain readable without acquiring revision fields. | Test |

## Dependencies

- **Consumer contract**: [Engineering Assurance FR-069](ix://agent-ix/engineering-assurance/FR-069).
- **Command boundary**: [FR-068](./FR-068-producer-facing-change-assurance-cli-surface.md).
