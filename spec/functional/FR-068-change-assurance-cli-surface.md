---
id: FR-068
title: "Producer-facing change assurance CLI surface"
type: FR
verification_method: test
evidence:
  - kind: test_case
    ref: tests/change-assurance-command.test.ts
relationships:
  - target: "ix://agent-ix/quoin/StR-001"
    type: "satisfies"
  - target: "ix://agent-ix/quoin/US-017"
    type: "implements"
  - target: "ix://agent-ix/quoin/FR-063"
    type: "requires"
  - target: "ix://agent-ix/quoin/FR-064"
    type: "requires"
  - target: "ix://agent-ix/quoin/FR-065"
    type: "requires"
---
# FR-068: Producer-facing change assurance CLI surface

## Description

Quoin SHALL expose the implemented FR-063 record, FR-064 attestation, retained
output intake, and FR-065 receipt contracts through supported
`quoin change-assurance` commands. Every command SHALL consume explicit caller
inputs and retained bytes only, and SHALL NOT become a producer, runner, or
scheduler for the work an attestation describes.

## Inputs

- `seal-record`: one JSON change-assurance record body without `digest`, from a
  named file or `-` for standard input.
- `seal-attestation`: one JSON proof-attestation body without `digest` and
  without `retained_output`, plus `--output`, the path of the retained result
  file the attestation describes, and `--media-type`, its declared media type.
- `intake`: `--attestation`, an exact sealed attestation JSON file, and
  `--output`, the exact retained result file it names.
- `receipt`: `--record`, a stored record digest; `--candidate-revision`;
  repeated `--select <proof-id>=<attestation-digest>`; repeated
  `--parent <record-digest>`; `--decisions`, a retained ix-flow decision
  history file; and optional `--audits`, retained FR-032 audit reports.
- `verify-receipt`: one sealed receipt JSON file.
- `schema`: an optional `--name` naming one of the three normative assets.
- `recover`: no input beyond `--repo`.

`--repo` names the repository root holding the evidence store and defaults to
the working directory.

## Outputs

- `seal-record` writes the sealed record into the store and reports its digest
  and retained path.
- `seal-attestation` emits the sealed attestation JSON on standard output; it
  writes nothing.
- `intake` reports the retained attestation directory.
- `receipt` emits the FR-065 verification receipt.
- `verify-receipt` reports the verified receipt digest and outcome.
- `schema` emits one normative schema, or the three asset names when `--name`
  is omitted.
- `recover` reports the number of removed staging directories.

Every command accepts `--json` and emits canonical JSON on that flag, except
`schema --name`, which emits the packaged asset verbatim because a
re-serialization would no longer be the file consumers validate against.

## Behavior

- `seal-attestation` SHALL derive `retained_output.digest` and
  `retained_output.size_bytes` from the bytes of the named output file and take
  `media_type` from the caller. It SHALL NOT derive any other field.
- `intake` SHALL retain the exact canonical attestation bytes and the exact
  output bytes as one atomic pair, and SHALL refuse an attestation whose
  recorded output digest or size does not match the supplied bytes.
- Re-running `intake` with byte-identical inputs SHALL succeed and retain
  nothing new. A digest collision with differing bytes SHALL be refused.
- `receipt` SHALL assemble its FR-065 verification input from the named stored
  record, the named parents, the explicitly selected attestations and their
  retained outputs, the supplied decision history, and the supplied audit
  reports. A stored attestation that is not selected SHALL have no effect.
- `receipt` SHALL preserve `unavailable` and `not_computed` results and missing
  evidence as their own outcomes and reasons, and SHALL NOT convert any of them
  into a pass or a failure.
- Command exit status SHALL be `0` for a `valid` receipt, `1` for an `invalid`
  or `incomplete` receipt, and `2` for a usage, parse, or integrity error. The
  emitted receipt remains the machine-readable result in every case.
- `recover` SHALL remove only staging directories left by an interrupted
  intake, and SHALL report the count it removed.

## Error Conditions

A missing, unreadable, or non-JSON input, an input carrying a `digest` the
caller must not supply, an unknown schema name, a `--select` mapping naming an
unstored attestation, a record digest naming no stored record, and an output
whose bytes contradict the attestation each fail with exit status `2` and a
message naming the offending input. No partial record, attestation, output, or
receipt is retained on any of these paths.

## Constraints

| ID | Constraint | Type | Validation |
|----|------------|------|------------|
| FR-068-CON-1 | No command SHALL execute the attested command, spawn a process, invoke Git, or perform a network request. | Architecture | Inspection |
| FR-068-CON-2 | No command SHALL infer a candidate revision, proof id, command binding, tool or configuration identity, result, environment, or decision. Content digests and sizes computed from named bytes are not inferences. | Integrity | Test |
| FR-068-CON-3 | No command output or help text SHALL make an identity, authorization, non-repudiation, or certification claim. | Responsibility | Inspection |
| FR-068-CON-4 | The library contracts, stored layout, and schema assets SHALL be unchanged by this surface, and existing retained records SHALL remain readable. | Compatibility | Test |

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-068-AC-1 | `seal-record` seals an explicit record body, retains it, and reports its digest and path; a body carrying `digest` or an undeclared field is refused with exit 2 and nothing is retained. | Test (TC-1317) |
| FR-068-AC-2 | `seal-attestation` derives only media type, digest, and size for the retained output from the named file and the caller's flag, and refuses a body that supplies `retained_output` or `digest`. | Test (TC-1318) |
| FR-068-AC-3 | `intake` retains exact attestation and output bytes atomically; a contradicting digest or size is refused, an identical re-intake is idempotent, and a colliding digest with differing bytes is refused. | Test (TC-1319) |
| FR-068-AC-4 | `receipt` builds its verification input only from the named record, parents, explicit selections, decision history, and audits; an unselected stored attestation changes no receipt field. | Test (TC-1320) |
| FR-068-AC-5 | `unavailable`, `not_computed`, and missing evidence appear as their own receipt outcomes and reasons and are never converted to a pass or a failure. | Property (TC-1321) |
| FR-068-AC-6 | Exit status is 0 for `valid`, 1 for `invalid` and `incomplete`, and 2 for usage, parse, and integrity errors, with the receipt still emitted for the first two. | Test (TC-1322) |
| FR-068-AC-7 | `verify-receipt` accepts a sealed receipt and refuses one whose semantic field or digest was altered. | Test (TC-1323) |
| FR-068-AC-8 | `schema` lists the three normative asset names and emits each by name byte-identically to the packaged asset; an unknown name is refused with exit 2. | Test (TC-1324) |
| FR-068-AC-9 | `recover` removes only interrupted-intake staging directories, reports the count, and leaves retained records, attestations, and outputs untouched. | Test (TC-1325) |
| FR-068-AC-10 | Golden fixtures for a sealed record, a sealed attestation, and a valid receipt reproduce byte-identical canonical JSON through the CLI (CON-4). | Integration (TC-1326) |
| FR-068-AC-11 | Static boundaries prove no command executes an attested command, spawns a process, or performs Git or network work, and that no output or help text makes an identity, authorization, non-repudiation, or certification claim (CON-1, CON-3). | Inspection (TC-1327) |

## Dependencies

- **Upstream**: [FR-063](./FR-063-change-assurance-record-integrity.md),
  [FR-064](./FR-064-proof-attestation.md), and
  [FR-065](./FR-065-change-assurance-verification.md).
- **Downstream**: `agent-ix/engineering-assurance#9` consumes these commands for
  PGM-01 compatibility fixtures; `agent-ix/quoin#323` inventories native-result
  adapters against this surface.
