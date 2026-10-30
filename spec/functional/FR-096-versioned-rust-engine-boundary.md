---
id: FR-096
title: "Expose a Rust engine boundary"
type: FR
relationships: []
---

# FR-096: Expose a Rust engine boundary

## Description

Quoin SHALL expose its first-party engine, production and qualification
behaviour to its Rust callers (`quoin-cli`) through one `quoin-core` library
boundary that carries structured request and result documents, distinguishes
every non-success state, and admits no Node runtime type. `quoin-core` is a
library linked into the one `quoin` executable: the caller invokes an operation
in process and there is no second executable, child process, executable path or
inter-process stream. The boundary records no protocol version and compares
none: caller and engine are one workspace built together, bound by `Cargo.toml`
and `Cargo.lock`.

## Inputs

- A JSON request document naming one command-shaped operation
  `<domain>.<op>`, handed to the operation as a value (or, for a caller holding
  a byte stream, read from that stream by the bounded request reader).
- The caller-selected repository content, supplied explicitly in the request:
  for an operation that analyses a repository, the file map the caller read —
  not a path for `quoin-core` to walk — together with the directories the caller
  could not list. For an operation that names a configuration root, that root.

## Outputs

- Exactly one JSON result document per operation.
- Diagnostics as structured entries distinct from the result document.
- An outcome drawn from the declared 0/1/2/3/4 taxonomy.

## Behavior

- Each request and result SHALL carry the operation name `<domain>.<op>` as its
  discriminator, and no version number. A request naming an operation
  `quoin-core` does not implement SHALL be refused before any filesystem write
  or downstream action.
- `quoin-core` SHALL return the machine result and the diagnostics as separate
  values, and SHALL NOT mix a diagnostic into the result document.
- `quoin-core` SHALL expose only command-shaped operations `<domain>.<op>`, and
  SHALL NOT expose a primitive function such as canonical-JSON serialization as
  its own boundary operation.
- `quoin-core` SHALL exit 0 for success, 1 for a partial result carrying a valid
  payload, 2 for a refusal, 3 for a malformed or unsupported request, and 4 for
  an internal fault, and a caller SHALL be able to distinguish a non-zero status
  that carries a valid payload from one that carries none.
- The caller SHALL resolve the caller-selected repository and
  configuration roots through their real paths, so that a symbolic link inside a
  permitted root cannot reach outside it.
- No Node runtime type, in-process object model, npm runtime value or oclif type
  SHALL appear in a boundary request or result document.
- For an operation that analyses a repository, the caller SHALL
  supply the repository as content rather than as a path, and the pre-filter it
  applies when reading that content SHALL admit every path `quoin-core` would
  classify, so that no path the analysis would act on is dropped before it
  reaches the boundary. `quoin-core` SHALL re-apply its own classification to
  every path it receives, so that a caller that sends more than the analysis
  needs cannot change a verdict either.
- `quoin-core` SHALL bound the number of bytes its request reader takes from a
  stream while it is reading them, and SHALL refuse a stream that exceeds that bound without
  parsing it. That transport bound SHALL be strictly greater than every
  operation's own request-size bound, so that an operation's refusal remains
  reachable and names the operation that refused.
- Quoin SHALL return a changed boundary interface or compatibility promise to
  specification before the implementation continues.

> **CR-001 (2026-10-02, PLAT-105):** The Rust boundary consumes the authoritative
> `ix-cli-kit` exit taxonomy and canonical JSON utilities through a git revision
> dependency. Domain error classification and diagnostic shape remain in Quoin.
> This adoption preserves statuses 0/1/2/3/4, payload eligibility, compact sorted
> output at every depth, and existing error codes. The duplicated taxonomy and
> key sorter are removed; callers import the shared type directly.

## Error Conditions

Malformed JSON, a request naming an unknown operation, a repository or configuration root escaping the caller-selected root,
and an oversized request each produce a distinguishable non-success outcome and
are never reported as success.

## Constraints

| ID | Constraint | Type | Validation |
|----|-----------|------|------------|
| FR-096-CON-1 | The boundary SHALL NOT expose a function-shaped operation. | Architecture | Test |
| FR-096-CON-2 | The boundary SHALL NOT derive a verdict from unstructured stdout or stderr text. | Responsibility | Test |
| FR-096-CON-3 | The Rust workspace SHALL remain in this repository. | Architecture | Test |
| FR-096-CON-4 | Quoin SHALL NOT publish any crate of the workspace to crates.io; consumers depend on the workspace's `Cargo.toml` and `Cargo.lock`. | Distribution | Inspection |
| FR-096-CON-5 | `quoin-core` SHALL remain a library and SHALL NOT gain an executable target. | Architecture | Test |

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-096-AC-1 | `cargo build --workspace --manifest-path rust/Cargo.toml` produces the `quoin` executable with `quoin-core` linked in as a library, from a workspace inside this repository, requiring no additional repository. | Test (TC-1605) |
| FR-096-AC-2 | Every boundary operation returns exactly one JSON result document and keeps its diagnostics separate from it. | Property (TC-1606) |
| FR-096-AC-3 | An unknown operation, malformed JSON and an escaping root each refuse before any filesystem write, returning the declared exit status. | Property (TC-1607) |
| FR-096-AC-4 | A refusal carrying a valid result payload is distinguishable by the caller from a fault carrying no payload, for every declared non-zero exit status. | Test (TC-1608) |
| FR-096-AC-5 | Withdrawn: `QUOIN_CORE` executable-path resolution. `quoin-core` is a linked library and has no executable to resolve (id not reused). | Withdrawn (TC-1609) |
| FR-096-AC-6 | Withdrawn: the output-buffer ceiling and the exit/signal/spawn-failure classification of a child process. There is no child process (id not reused). | Withdrawn (TC-1610) |
| FR-096-AC-7 | A static check over the generated boundary type surface finds no Node, npm or oclif runtime type, and fails when one is planted. | Test (TC-1611) |
| FR-096-AC-8 | Over one materialised repository, the verdict reached through the caller's snapshot equals the verdict reached by `quoin-core` reading the same tree from disk, including for a directory the caller's pre-filter does not exclude and for one both sides do; narrowing the caller's pre-filter by one directory fails the comparison. | Test (TC-1713) |
| FR-096-AC-9 | A request stream one byte past the transport bound is refused with the declared refusal status, without the request being parsed and without the stream being read beyond that bound; a stream of exactly the bound is accepted; and the transport bound is strictly greater than every declared per-operation request bound. | Test (TC-1714) |
| FR-096-AC-10 | The native boundary uses the shared outcome type, emits the existing 0/1/2/3/4 status and payload eligibility for every outcome, and emits compact lexicographically sorted JSON including objects nested inside arrays when insertion-order maps are enabled. Serialization errors retain the Quoin Io code. | Test |
| FR-096-AC-11 | Withdrawn: the `PATH` resolution diagnostic for a missing `quoin-core`. There is no executable to find (id not reused). | Withdrawn (TC-1980) |
| FR-096-AC-12 | Withdrawn: the resolution-versus-spawn-failure distinction. There is no spawn (id not reused). | Withdrawn (TC-1981) |
| FR-096-AC-13 | Withdrawn: `core.ping` declared an `expect_protocol` and returned `ProtocolSkew` on a differing recorded revision. The boundary records no protocol revision; caller and engine are bound by `Cargo.lock` (id not reused). | Withdrawn (TC-1982) |

## Dependencies

- **Downstream**: [FR-098](./FR-098-semantic-and-identity-parity.md) through [FR-103](./FR-103-corpus-consolidation.md) all cross this boundary.


> **PLAT-989 change note (2026-10-05):** boundary canonical JSON SHALL be
> encoded through `quire-canonical`, with RFC 8785 UTF-16 member ordering and
> ECMAScript number spelling. The shared CLI kit continues to supply exit
> outcomes and diagnostics; its general sorted JSON utility is not an RFC 8785
> encoder. Encoding refusals retain the boundary `Io` code.
