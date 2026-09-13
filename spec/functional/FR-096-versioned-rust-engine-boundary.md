---
id: FR-096
title: "Expose a versioned Rust engine boundary"
type: FR
relationships:
  - target: "ix://agent-ix/quoin/StR-009"
    type: "implements"
  - target: "ix://agent-ix/quoin/US-024"
    type: "implements"
---

# FR-096: Expose a versioned Rust engine boundary

## Description

Quoin SHALL expose its first-party engine, production and qualification
behaviour to the retained TypeScript through one versioned `quoin-core`
subprocess boundary that carries structured request and result documents,
distinguishes every non-success state, and admits no Node runtime type.

## Inputs

- A versioned JSON request document naming one command-shaped operation
  `<domain>.<op>`, supplied on stdin.
- The caller-selected repository content, supplied explicitly in the request:
  for an operation that analyses a repository, the file map the caller read —
  not a path for `quoin-core` to walk — together with the directories the caller
  could not list. For an operation that names a configuration root, that root.
- The `QUOIN_CORE` executable path and the `QUOIN_EXPECTED_CORE_SHA256` expected
  digest.

## Outputs

- Exactly one versioned JSON result document on stdout per invocation.
- Diagnostics on stderr.
- A process exit status drawn from the declared taxonomy.

## Behavior

- Each boundary protocol SHALL carry a `quoin-core.<domain>.<op>/vN`
  discriminator in both request and result.
- When a request declares a protocol version `quoin-core` does not implement,
  `quoin-core` SHALL refuse the request before performing any filesystem write
  or downstream action.
- `quoin-core` SHALL write the machine result only to stdout and diagnostics
  only to stderr.
- `quoin-core` SHALL expose only command-shaped operations `<domain>.<op>`, and
  SHALL NOT expose a primitive function such as canonical-JSON serialization as
  its own boundary operation.
- `quoin-core` SHALL exit 0 for success, 1 for a partial result carrying a valid
  payload, 2 for a refusal, 3 for a malformed or unsupported request, and 4 for
  an internal fault, and a caller SHALL be able to distinguish a non-zero status
  that carries a valid payload from one that carries none.
- The TypeScript caller SHALL resolve the `quoin-core` executable through its
  real path.
- If the executable's bytes do not match `QUOIN_EXPECTED_CORE_SHA256`, or that
  variable is unset, then the TypeScript caller SHALL refuse to invoke it.
- The TypeScript caller SHALL resolve the caller-selected repository and
  configuration roots through their real paths, so that a symbolic link inside a
  permitted root cannot reach outside it.
- The TypeScript caller SHALL declare an output-buffer ceiling strictly
  greater than 67,108,864 bytes, so that a 67,108,864-byte result payload is
  returned without truncation or an operating-system buffer error. The ceiling
  is strictly greater because the retained caller's 64 MiB `maxBuffer` is the
  exact size at which agent-ix/quoin#164 recurred, and a framing byte puts a
  payload of that size over it.
- The TypeScript caller SHALL classify a child process that exits, that is
  terminated by a signal, and that fails to spawn as three distinguishable
  outcomes.
- No Node runtime type, in-process object model, npm runtime value or oclif type
  SHALL appear in a boundary request or result document.
- For an operation that analyses a repository, the TypeScript caller SHALL
  supply the repository as content rather than as a path, and the pre-filter it
  applies when reading that content SHALL admit every path `quoin-core` would
  classify, so that no path the analysis would act on is dropped before it
  reaches the boundary. `quoin-core` SHALL re-apply its own classification to
  every path it receives, so that a caller that sends more than the analysis
  needs cannot change a verdict either.
- `quoin-core` SHALL bound the number of bytes it reads from stdin while it is
  reading them, and SHALL refuse a stream that exceeds that bound without
  parsing it. That transport bound SHALL be strictly greater than every
  operation's own request-size bound, so that an operation's refusal remains
  reachable and names the operation that refused.
- Quoin SHALL return a changed boundary interface or compatibility promise to
  specification before the implementation continues.

## Error Conditions

Malformed JSON, an unknown protocol version, a request naming an unknown
operation, a repository or configuration root escaping the caller-selected root,
an executable whose digest does not match the expected digest, an oversized
payload, a signal termination and a spawn failure each produce a distinguishable
non-success outcome and are never reported as success.

## Constraints

| ID | Constraint | Type | Validation |
|----|-----------|------|------------|
| FR-096-CON-1 | The boundary SHALL NOT expose a function-shaped operation. | Architecture | Test |
| FR-096-CON-2 | The boundary SHALL NOT derive a verdict from unstructured stdout or stderr text. | Responsibility | Test |
| FR-096-CON-3 | The Rust workspace SHALL remain in this repository. | Architecture | Test |
| FR-096-CON-4 | No crate of the workspace SHALL be published to crates.io; consumers pin by git revision. | Distribution | Inspection |

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-096-AC-1 | `cargo build --workspace --manifest-path rust/Cargo.toml` produces the `quoin-core` binary from a workspace inside this repository, requiring no additional repository. | Test (TC-1605) |
| FR-096-AC-2 | Every boundary operation emits exactly one declared-version JSON document on stdout and sends all diagnostics to stderr. | Property (TC-1606) |
| FR-096-AC-3 | An unknown protocol version, an unknown operation, malformed JSON and an escaping root each refuse before any filesystem write, returning the declared exit status. | Property (TC-1607) |
| FR-096-AC-4 | A refusal carrying a valid result payload is distinguishable by the caller from a fault carrying no payload, for every declared non-zero exit status. | Test (TC-1608) |
| FR-096-AC-5 | With `QUOIN_EXPECTED_CORE_SHA256` set and the binary's bytes altered, the caller refuses to invoke it and names the expected and observed digests; with the variable unset the caller refuses rather than invoking unpinned. | Test (TC-1609) |
| FR-096-AC-6 | A result payload of 67,108,864 bytes is returned to the caller intact under a declared buffer ceiling strictly greater than that size, and process exit, signal termination and spawn failure are reported as three distinct outcomes. | Test (TC-1610) |
| FR-096-AC-7 | A static check over the generated boundary type surface finds no Node, npm or oclif runtime type, and fails when one is planted. | Test (TC-1611) |
| FR-096-AC-8 | Over one materialised repository, the verdict reached through the caller's snapshot equals the verdict reached by `quoin-core` reading the same tree from disk, including for a directory the caller's pre-filter does not exclude and for one both sides do; narrowing the caller's pre-filter by one directory fails the comparison. | Test (TC-1713) |
| FR-096-AC-9 | A stdin stream one byte past the transport bound is refused with the declared refusal status, without the request being parsed and without the stream being read beyond that bound; a stream of exactly the bound is accepted; and the transport bound is strictly greater than every declared per-operation request bound. | Test (TC-1714) |

## Dependencies

- **Upstream**: accepted [ADR-0003](../../docs/semantic-module-architecture/adr/0003-rust-native-quoin-engine-boundary.md) and [StR-009](../stakeholder/StR-009-one-implementation-language-for-engine-logic.md).
- **Downstream**: [FR-097](./FR-097-schema-sourced-type-surface.md) through [FR-103](./FR-103-corpus-consolidation.md) all cross this boundary.
