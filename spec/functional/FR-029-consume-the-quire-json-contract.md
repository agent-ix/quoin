---
id: FR-029
title: "Consume the published quire JSON contract"
type: FR
relationships:
  - target: "ix://agent-ix/quoin/StR-004"
    type: "traces_to"
  - target: "ix://agent-ix/quire-rs/FR-055"
    type: "traces_to"
  - target: "ix://agent-ix/quire-rs/FR-053"
    type: "traces_to"
---

# FR-029: Consume the published quire JSON contract

## Description

`quoin` SHALL validate every quire JSON payload it reads against the schemas
[quire-rs FR-055](ix://agent-ix/quire-rs/FR-055) publishes, and SHALL enforce
the CLI version premise before reading one.

quoin has never parsed quire JSON in code. The `properties --json` and
`coverage --json` shapes existed only as prose in skill markdown, consumed by
agents reading that prose — so when a shape drifted, the failure surfaced as an
agent confused mid-skill rather than as a diagnosable error. quoin's own
`spec/review.md` Finding 8 records exactly this: *"no contract test against
quire"*. The documented version premises were enforced by nothing.

### Validation comes before typing, not after

TypeScript erases at runtime and JSON from a subprocess is `any`, so a type
declaration alone would have caught none of this. Every payload is validated
against the published schema **first**; the models exist to describe what the
validator has already made true. A mismatch surfaces as a violation naming the
offending path rather than as an `undefined` three frames later.

### The schemas are vendored, with recorded provenance

quire-rs is a Rust crate consumed by git tag; quoin is an npm package. There is
no dependency edge along which a schema file could travel, and quoin performs no
network reads on a command path. So the artifacts are copied in with their
source tag, path and **content hash** recorded, and refreshed by a script that
refuses to run against a checkout not at the pinned tag.

That is a copy, and a copy can drift. What keeps it honest is that the hash is
asserted on every test run: an edit to a vendored file without a matching
refresh fails immediately rather than silently teaching quoin a contract quire
does not emit.

### The version premise is checked before anything parses

An older `quire` does not fail — it emits an *older shape*, which a consumer
misreads. That is why the premise is a precondition rather than an error path:
by the time a parse fails, the wrong thing has already been believed. The check
names the found version, the required version, and the consequence.

## Inputs

- `quire coverage --json` and `quire properties --json` payloads
- `quire --version` output
- The vendored schemas under `src/quire/schemas/`

## Outputs

- A validated, typed payload, or a named `ContractViolation` carrying every
  failing path
- A named `VersionPremiseFailure` when the CLI predates the contract

## Behavior

- `validateCoverage` / `validateProperties` SHALL validate against the vendored
  published schema and SHALL return a result rather than throwing, so a caller
  chooses how to report.
- `parseCoverage` / `parseProperties` SHALL parse and validate in one call, so
  no caller can hold an unvalidated payload it might use by accident.
- The parse helpers SHALL return a contract violation on a JSON parse failure
  rather than throwing:
  from the caller's position "quire emitted something unreadable" and "quire
  emitted the wrong shape" are the same actionable fact.
- `checkVersionPremise` SHALL treat an unreadable version as a failure rather
  than a pass, since "no quire on PATH" and "a quire too old" both need
  reporting.
- Version comparison SHALL be numeric. A lexical comparison ranks `0.9.0` above
  `0.21.0`, which is precisely the range this premise spans.
- The eval harness SHALL assert the premise before a run. It keys on exit codes
  that an old binary still produces, so without the check a stale toolchain
  reads as a failing spec.
- The `quire` subprocess SHALL run with an explicit `maxBuffer` sized for real
  corpora. Node's 1 MiB default was exceeded by a measured 1,090,714-byte
  `coverage --json` payload — 4% over — which killed all six commands that
  shell out (#164). The payload grows with spec size, so the default failed
  exactly on the corpora the commands exist to serve.
- A child that never exited SHALL be reported by its cause — `err.code`
  (`ENOBUFS`) or `err.signal` — with no exit status and no child stderr
  appended. On a kill, stderr holds whatever the child happened to write
  before dying (on every real repo, harmless `DuplicateArchetype` first-wins
  warnings), and appending it frames that noise as the diagnosis.

## Constraints

| ID | Constraint | Type | Validation |
|----|------------|------|------------|
| FR-029-CON-1 | quoin SHALL NOT restate the published schemas as hand-written validators. The duplication is the failure being closed; the vendored artifact plus its hash is the whole mechanism. | Architecture | Test |
| FR-029-CON-2 | quoin SHALL NOT fetch a schema at runtime. Every command path stays network-free. | Architecture | Inspection |
| FR-029-CON-3 | The vendored schemas SHALL mirror the publisher's open/closed decision for every vocabulary rather than second-guessing it. Closing `diagnostics[].reason` would reject a newer engine's payload that a consumer could otherwise read. | Architecture | Test |

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-029-AC-1 | Each vendored schema hashes to the value recorded beside it and carries a `$id` naming the pinned contract version. | Test (TC-110) |
| FR-029-AC-2 | A conformant coverage payload and a conformant properties payload each validate. | Test (TC-111) |
| FR-029-AC-3 | A payload missing a required key, carrying an added field, or using a value outside a closed engine enum is rejected, and the report names the failing path. | Test (TC-112) |
| FR-029-AC-4 | Unreadable output is reported as a named contract violation rather than thrown, and the message names the likeliest cause. | Test (TC-113) |
| FR-029-AC-5 | A CLI older than the pinned minimum fails the premise with the found version, the required version and the consequence; an unreadable version is a failure rather than a pass. | Test (TC-114) |
| FR-029-AC-6 | Version comparison is numeric, so `0.21.0` ranks above `0.9.0`. | Test (TC-115) |
| FR-029-AC-7 | A payload omitting every optional key validates, one carrying every optional key validates, and a malformed statement hash is rejected. | Test (TC-116) |
| FR-029-AC-8 | The eval harness's version floor equals the pinned contract minimum, so the two restatements cannot drift. | Test (TC-117) |
| FR-029-AC-9 | A payload emitted by the installed `quire` binary validates against the vendored schema, so the contract is checked against the real emitter and not only against fixtures. | Test (TC-118) |
| FR-029-AC-10 | When the `quire` subprocess exits non-zero, its **stderr** is surfaced in the raised diagnostic. Its own message names the cause — a missing traceability model, a bad `--module` — and discarding it undoes the care FR-029 takes over the version premise one frame later. | Inspection (TC-134), Test (TC-256) |
| FR-029-AC-11 | Every `quire` subprocess call sets an explicit `maxBuffer` sized for real corpora, so a corpus whose `coverage --json` payload exceeds Node's 1 MiB default still runs every command that shells out. | Test (TC-254) |
| FR-029-AC-12 | A child that never exited on its own — killed on a buffer overrun, killed by a signal, or never spawned — is reported by its cause (`ENOBUFS` naming the byte limit, the signal name, the spawn error code), reports no exit status, and appends no child stderr. | Test (TC-254, TC-255, TC-257) |

## Dependencies

- **Upstream**: quire-rs [FR-055](ix://agent-ix/quire-rs/FR-055) (the published schemas), [FR-053](ix://agent-ix/quire-rs/FR-053) (the obligation records the payloads carry)
- **Downstream**: the `quoin evidence` verbs, which are the first production consumer of these models
