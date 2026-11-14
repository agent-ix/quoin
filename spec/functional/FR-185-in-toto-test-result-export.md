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
  Those caller-supplied digests bind the statement to the exact artifact
  content; Quoin computes no digest and records no other digest.
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
  the retained run records and `bindings.json` from the selected evidence
  directory and SHALL NOT read Git state, access the network, or spawn a
  process.
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
- For each obligation id in `bindings.json`, Quoin SHALL inspect the latest
  run of every bound suite and the symbols named by that binding.
- Quoin SHALL reuse the stale-evidence conditions of
  `quoin-auditor::audit::stale::FindingKind::STALE_EVIDENCE`. A bound suite
  without a recorded run, or a latest bound-symbol outcome of `fail` or
  `error`, makes the obligation stale. This export is run-specific: a scan
  record for the suite does not satisfy the run requirement, so a missing run
  makes the obligation stale even when a scan exists.
- If an obligation is stale under the
  `quoin-auditor::audit::stale::FindingKind::STALE_EVIDENCE` conditions, then
  Quoin SHALL put it in `failedTests`.
- When an obligation is not stale and any bound symbol has outcome `skip` or
  is absent from its run, Quoin SHALL put it in `warnedTests`.
- When an obligation is not stale and every bound symbol has outcome `pass`,
  Quoin SHALL put it in `passedTests`.
- This export SHALL NOT define a second staleness rule.
- Quoin SHALL sort and deduplicate the three test lists.
- Quoin SHALL place each obligation in exactly one list.
- `result` SHALL be `FAILED` when `failedTests` is non-empty, `WARNED` when
  `failedTests` is empty and `warnedTests` is non-empty, and `PASSED` otherwise.
- Quoin SHALL copy the caller's subject array without computing or adding any
  digest. Quoin SHALL validate that the array is non-empty and that every
  descriptor has a `digest.sha256` of exactly 64 lowercase hexadecimal
  characters.
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
| FR-185-AC-1 | A fixture with a passing obligation, a failing symbol obligation and an obligation bound to a suite without a run exports `result: FAILED`; only the passing obligation is in `passedTests`, and the other two are in `failedTests`. | Test |
| FR-185-AC-2 | A fixture whose only non-pass outcome is `skip` exports `WARNED` with that obligation in `warnedTests`; an all-pass fixture exports `PASSED` with empty `warnedTests` and `failedTests`. | Test |
| FR-185-AC-3 | The output subject equals the supplied array after canonicalization; a non-hex digest, missing `sha256`, empty array or non-array subject exits 2 and writes no output file. | Test |
| FR-185-AC-4 | Repeating an export over the same store and subject yields byte-identical output, and reordering the stored runs or bindings does not change the bytes. | Test |
| FR-185-AC-5 | The predicate has exactly the five specified members, deserializes through its closed type, and rejects a predicate with an additional member. | Test |
| FR-185-AC-6 | An unparsable run record or missing `bindings.json` exits 3 with `evidence_store_invalid` and the relevant path, and leaves no output file. | Test |
| FR-185-AC-7 | A static boundary check establishes that the export path calls no Git, network or process-spawning API. | Inspection |
| FR-185-AC-8 | The consumer-side negative check from Engineering Assurance FR-069-AC-2 is reproducible from Quoin: an export cannot produce `result: PASSED` with a non-empty `failedTests`. | Test |

## Dependencies

- **Consumer contract**: [Engineering Assurance FR-069](ix://agent-ix/engineering-assurance/FR-069).
- **Command boundary**: [FR-068](./FR-068-producer-facing-change-assurance-cli-surface.md).
