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
- `--execution-provenance <provenance.json>`: the version-2 DSSE envelope
  produced by the signed source release workflow and verified by the
  Engineering Assurance release-evidence verifier under its existing trust
  policy. Its payload contains `sourceRevision`, `suiteDeclarationsSha256`, the
  complete computed Quire Test Matrix for that revision, and `runs`: a list of
  suite id, record path, and SHA-256 or `null` when the expected run record is
  absent. A caller-selected obligation list is not an accepted population
  source.
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
  the retained run records, `bindings.json`, the maintained suite declarations
  at `spec/evidence/suites.md`, and the execution-provenance file. Quoin SHALL
  NOT read Git state, access the network, or spawn a process.
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
  revision. Quoin SHALL verify the exact suite-declaration bytes against
  `suiteDeclarationsSha256` in the signed payload before deriving this
  population. Quoin SHALL NOT accept a caller-selected subset or infer the
  population from `bindings.json`.
- The signed source release workflow SHALL produce the version-2 DSSE
  execution-provenance envelope defined by FR-186. Its payload SHALL contain
  the source revision, suite-declaration digest, complete Test Matrix, and
  expected suite/run paths and digests. An absent expected run SHALL be
  represented by a `null` run-record digest. The source revision SHALL be the
  revision used to compute the matrix and execute the declared suites.
- The Engineering Assurance verifier SHALL authenticate the envelope's source
  workflow identity and validate its revision and artifact provenance under
  the FR-069 trust policy before passing the envelope to Quoin. Quoin SHALL
  verify each run-record digest locally before using its population or run
  records. The signature SHALL cover the complete `testMatrix` payload bytes.
  Quoin does not itself verify the DSSE signature: authentication is the
  Engineering Assurance verifier's, performed before the envelope reaches
  Quoin, so Quoin's population is authenticated only to the extent the caller
  ran that verifier. Quoin checks the envelope's shape (payload type, a
  non-empty `signatures` list, the closed payload fields) and refuses a
  malformed one.
- The required population SHALL be the set of non-empty string `criterion`
  members of the `testMatrix` rows. A matrix with no such row SHALL refuse the
  export; an empty population SHALL NOT yield `PASSED`.
- Quoin SHALL take a suite's run only from the record the provenance names for
  that suite by `recordPath`, after its bytes match the signed digest. A suite
  the provenance does not list has no run. `recordPath` SHALL be a relative
  path with no root or parent components.
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
- Readers SHALL continue to read version-1 RunRecord values. Quoin SHALL permit
  a version-1 record to support a verdict only when valid execution provenance
  binds its exact bytes.
- Quoin SHALL refuse the export without creating or replacing output when the
  provenance envelope is absent, malformed, has an invalid signature, has an
  untrusted source identity, or names a revision that differs from any subject
  descriptor. Quoin SHALL also refuse when a run entry has a non-null digest
  but its referenced run record is absent or its exact bytes do not match that
  digest, or when the signed matrix payload is altered.
- Quoin SHALL refuse the export when `spec/evidence/suites.md` is absent or its
  bytes do not match the suite-declaration digest in valid provenance.
- When provenance is valid and an expected run entry has a signed null digest,
  Quoin SHALL include each obligation requiring that run in `failedTests`.
- When provenance is valid but a required obligation has no binding or has an
  empty symbol set, Quoin SHALL include that obligation in `failedTests`.
- Quoin SHALL NOT refuse the export solely because an expected run entry has a
  signed null digest or an obligation has no binding or symbols.
- Quoin SHALL refuse when provenance authenticity, subject identity, or
  artifact integrity cannot be established.
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
| FR-185-AC-11 | Quoin verifies the maintained suite-declaration file against the signed `suiteDeclarationsSha256`; a missing or changed file refuses export. A source-workflow test compares the signed Test Matrix to the complete Quire matrix at `sourceRevision` and detects an omitted active obligation even when the DSSE signature is valid. | Test |
| FR-185-AC-12 | A valid source-workflow provenance signature, authenticated by the Engineering Assurance verifier, with matching suite-declaration digest, complete matrix, matching present run-record digests, and matching subject revision permits verdict evaluation. A signed null run digest lists affected obligations in `failedTests`; a non-null digest whose run record is missing or changed refuses export without output. Missing, malformed, invalid-signature, untrusted, or subject-mismatched provenance also refuses export. Version-1 RunRecord values remain readable without acquiring revision fields. | Test |

## Dependencies

- **Consumer contract**: [Engineering Assurance FR-069](ix://agent-ix/engineering-assurance/FR-069).
- **Command boundary**: [FR-068](./FR-068-producer-facing-change-assurance-cli-surface.md).
