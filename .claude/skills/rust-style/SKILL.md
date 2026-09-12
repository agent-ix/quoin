---
name: rust-style
description: The Rust idioms quoin's `rust/` workspace is built on — the boundary contract (exit taxonomy, canonical JSON, stream discipline), error envelopes with stable codes, the workspace lint policy, untrusted-input hardening on stdin, the tc_NNN/Trace test convention, and the gates that must actually be run. Use whenever writing or reviewing Rust in this repository, adding an operation to `quoin-core`, touching the error catalogue, or changing anything under `rust/`.
---

# quoin Rust Style

The idioms **this workspace already uses**. Everything below is true of the
code in `rust/` as it stands; where a lint or a test enforces a rule, the
enforcement is named. Nothing here is aspirational — if you find a statement
that the tree contradicts, the tree is the fact and this document is the
defect (filament-ide-rs SR-151 FND-003 is the precedent: a "do not" in an idiom
doc that had been false for two crates, which a reviewer nearly enforced).

This document **outranks** the ecosystem-wide `rust-review` and `rust-style`
skills for this repository. Where it is silent, those apply.

Governing tickets: quoin#373 (the burn-down EPIC), quoin#375 (Stage 0). The
spec ids are FR-096 (the versioned boundary), FR-097 (schema-sourced types),
FR-101 (retire-after-parity), NFR-024 (bounded coexistence) and NFR-026 (the
toolchain floor).

## The workspace

`rust/` sits **inside** this repository, beside the retained `src/`. That is
not a convenience: FR-101 requires old and new implementations to be exercised
at ONE candidate revision, and a second repository turns that into a
pin-and-sync problem. Do not propose extracting it.

Three crates exist:

| crate | what it is |
|---|---|
| `quoin-core` | the boundary binary, plus the library the binary is a shell over |
| `quoin-schemas` | where generated types will land (FR-097); holds `PROTOCOL_VERSION` today |
| `quoin-difftest` | the dev-only differential harness |

**Do not add an empty crate to "reserve" a name from #373's topology.** An
empty crate is noise the gates still have to walk. A crate appears in the same
commit as the code that fills it.

## The boundary contract

Every rule in this section is asserted by `crates/quoin-core/tests/tc_boundary.rs`.

- **One invocation, one operation.** `quoin-core <domain>.<op>`, JSON request
  on stdin, JSON payload on stdout. The unit is a command-shaped operation,
  never a function.
- **stdout is the payload and nothing else.** No log lines, no progress, no
  half-written object. `main.rs` serialises the whole payload into memory
  before a byte reaches stdout, and writes stdout *only* when the outcome
  carries a payload.
- **stderr is a JSON array of diagnostics.** Not prose.
- **The exit taxonomy is 0/1/2/3/4** and lives in one place —
  `protocol::Outcome`. The load-bearing member is `Partial` (1): a non-zero
  status whose stdout is a complete, valid payload. That case is not
  hypothetical; it is `runQuireAllowFailure`'s reason for existing
  (agent-ix/quoin#103, where treating a qualified result as total failure
  silently discarded a whole analysis axis). Ask `Outcome::carries_payload()`,
  never `status == 0`.
- **An operation picks a CODE, never an exit number.** `CoreErrorCode::outcome()`
  is the only mapping from code to status, so a new operation cannot invent a
  second meaning for status 2.
- **Canonical JSON on the way out**: object keys sorted at every depth, no
  insignificant whitespace, via `protocol::canonical_json`. On the Rust side
  this falls out of `serde_json::Map` being a `BTreeMap` — the `preserve_order`
  feature is deliberately **not** enabled, and enabling it silently breaks the
  comparison `quoin-difftest` performs.

## Error envelopes

`crates/quoin-core/src/error.rs` is the pattern; copy it into the next crate
that needs one rather than inventing a second shape.

- A `Copy` code enum with `as_str(self) -> &'static str`, `all() -> &'static [Self]`,
  `from_code(&str) -> Option<Self>` and `outcome(self) -> Outcome`.
- **Codes are the API. Never rename one, never reuse one.** A consumer that
  learned `CORE_UNKNOWN_OP` keeps it forever. Two unit tests hold this: every
  code round-trips through its spelling, and no two codes share one.
- An envelope struct deriving `thiserror::Error` with
  `#[error("{code}: {message}")]`, holding `{ code, message: Box<str>,
  context: BTreeMap<String, String> }`. `Box<str>` because the message is
  immutable; `BTreeMap` because the serialised context must be byte-stable.
- **The variant set is the API, not the message.** Each condition a caller must
  distinguish gets its own code. If you find yourself writing a second `Refused`
  with a different sentence to mean a different thing, that is a new code.
- Context is added with the consuming builder `with_context(mut self, …) -> Self`,
  so an envelope is built in one expression and immutable after.

## Lints, and the one place they are set

`rust/Cargo.toml`'s `[workspace.lints]` is the policy; every crate opts in with
`[lints] workspace = true`. **There is no exception**, and
`tc_375_every_crate_opts_into_the_workspace_lint_policy` fails if one appears
without this document and the root manifest being updated together.

The policy lives in the manifest rather than only in CI flags so that
`cargo clippy` on a developer machine enforces what the gate enforces.

- `unsafe_code = "forbid"` — workspace-wide, no audited-exception carve-out of
  the kind `filament-app` holds. If you need one, it is a design conversation,
  not a `#[allow]`.
- `clippy::all` and `clippy::pedantic` at `warn`, fatal in the gate via
  `-D warnings`.
- **The panic surface is linted, not policed by review**: `unwrap_used`,
  `expect_used`, `panic`, `indexing_slicing`. A binary that reads untrusted
  JSON off stdin must turn a bad input into an exit status, not into a killed
  process.
- **Casts across the wire boundary are linted**: `cast_possible_truncation`,
  `cast_sign_loss`, `cast_possible_wrap`. Use `try_from` with an explicit
  fallback. `quoin-difftest` converts an exit `i32` to `u8` with
  `u8::try_from(status).ok()` for exactly this reason.
- `missing_docs` and `unreachable_pub` — a `///` on every public item, and
  `pub` only where a consumer reaches.

### Tests may panic; production may not

A test module carries, and only ever carries, this:

```rust
#[allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]
mod tests { … }
```

The `reason =` is required — a bare `#[allow]` with no reason is a finding
(rust-review §5). Never widen this to `expect_used` or `panic` without adding
the reason here too, and never put an `#[allow]` at a crate root to silence one
site.

## Untrusted input

stdin is untrusted. Every rule here is asserted in `ops/core.rs`'s unit tests.

- **`#[serde(deny_unknown_fields)]` on every request type.** A field the
  boundary silently drops is a field the caller believes it sent. A misspelled
  `eco` exits 3 with `CORE_BAD_REQUEST` rather than being ignored.
- **Every accumulator from the stream has a ceiling, and a named refusal at
  it.** `MAX_ECHO_BYTES` is 4 KiB and an oversized token is `CORE_REFUSED`
  (exit 2), not truncated and not accepted. A new operation that reads a list,
  a cursor or a retry count states its bound the same way.
- **Bounds are in BYTES on both sides.** The TypeScript oracle measures
  `Buffer.byteLength(echo, "utf8")`, not `echo.length`: `"é"` is one UTF-16
  unit and two UTF-8 bytes, so comparing `.length` would make the boundary's
  behaviour depend on the caller's alphabet. There is a test for this.
- **A request must be a JSON object.** A bare array or scalar is
  `CORE_BAD_JSON` with the observed type named, not coerced.
- **The type is the schema on the way out too.** Payloads are `Serialize`
  structs (`PingPayload`), never a `serde_json::Value` assembled by inserting
  string keys across branches.

## Dispatch holds no logic

`dispatch.rs` is an exhaustive `match` on the operation name and nothing else.
"Which operations exist" is one fact in one place, checked by the compiler,
rather than a registry assembled at run time. `OPERATIONS` is exported so a
caller can enumerate the surface instead of discovering it by trying names, and
the `CORE_UNKNOWN_OP` diagnostic names the ones that do exist.

`main.rs` does four things — argv, stdin, dispatch, two writes and an exit —
and must keep doing only four. Everything decidable lives in the library so it
is unit-testable without spawning a process.

## Testing

- **Unit tests beside the code** in `#[cfg(test)] mod tests`, for the decisions.
  **Integration tests in `tests/`** reaching only the public API and, for the
  boundary, a real subprocess with real pipes — the stream and exit-status
  properties a library test cannot see are exactly the ones `src/core/exec.ts`
  depends on.
- **Integration test functions are named `tc_NNN_<description>`** and carry a
  `/// Trace:` doc line with the requirement criteria they cover, comma
  separated.

  **The keyword is `Trace:` and the separator is a comma.** A `Tracing:` line,
  or a `;`-separated one, binds nothing and the matrix row it names stays
  unbacked. filament-ide-rs wrote `Tracing:` 643 times before this was
  measured, which is why 51% of its matrix rows read as untested while the
  tests existed.

  **Tag a CRITERION, never a bare requirement id.** `FR-096` binds NOTHING.
  `scripts/check-trace-tags.mjs` resolves ids of the form
  `<TYPE>-<n>-<KIND>-<k>` — `FR-096-AC-2`, `NFR-027-AC-2`, `NFR-026-M-2` — and
  a line carrying only `FR-096` is reported as "a Trace tag naming no
  criterion". The matrix is backed at criterion granularity, so a requirement
  id is not a coarser binding, it is no binding at all: the row stays unbacked
  and the test's evidence is lost. This is not hypothetical and it is not
  filament-ide-rs's mistake — **Stage 0 of this workspace shipped nine such
  tags** (`/// Trace: FR-096`, `/// Trace: NFR-024`, `/// Trace: NFR-026`),
  copied from the example that used to sit two paragraphs below this one, while
  30 tests passed and every Stage-0 matrix row read "none implemented"
  (agent-ix/quoin#390). `-M-` is a real kind: NFR metric-table rows carry no id
  and the engine mints `NFR-<n>-M-<k>` per row in document order.

  **Bind to what the test proves, and leave it untagged when nothing states
  it.** A tag that resolves to the wrong criterion is worse than no tag — it
  reads as coverage. If the assertion contradicts the criterion, or no
  criterion states the property, write the reason in an `/// Unbound:` block
  above the test and report the gap; do not reach for the nearest id.

  Only requirement criteria belong on that line — `FR-`, `NFR-`, `StR-`, `US-`,
  `IT-` and their `-AC-`/`-CON-`/`-VC-`/`-M-`/`-EX-`/`-SC-` forms. A bare
  `TC-<n>` is a matrix row, not a criterion: the checker cannot resolve one and
  now counts them in its report (57 sit on Trace lines in `tests/` today), so a
  `TC-` id may accompany a criterion id but never stand in for one. Issue
  numbers, `Task-`, `Plan-` and `REV-` go on a sibling `/// Provenance:` line:
  they are real artifacts, but on the trace line they only mint untracked
  symbols.

  ```rust
  /// Trace: FR-096-AC-4
  /// Provenance: quoin#375, agent-ix/quoin#103
  #[test]
  fn tc_375_exit_1_still_carries_a_complete_payload() { … }
  ```

- **Assert the literal, not the re-derivation.** `tc_375_stdout_is_canonical_json_one_line`
  writes the expected bytes out rather than calling the canonicaliser again; a
  test that re-derives its expectation agrees with itself no matter what the
  code does.
- **Source-inspection tests are a last resort, and `tc_source_conventions.rs`
  is where the resort is legitimate**: no runtime path can observe whether a
  file carries an SPDX header or whether a crate opted into the lint policy.
  Do not reach for the pattern where a behavioural test is possible.

## Dependencies and the supply chain

- **Exact pins.** `serde = "=1.0.228"`, not `"1.0"`. A caret range makes the
  gate a claim about whatever resolved that morning. Matches quire-corpus.
- **Shared versions in `[workspace.dependencies]`**, consumed as
  `{ workspace = true }`.
- `deny.toml`'s licence allow-list holds **exactly** the licences the current
  graph carries — a new dependency adds its licence in the same commit. A
  generous list is also a noisy one (cargo-deny warns on an unmatched
  allowance) and enforces less.
- `allow-git = []`, with `unknown-git = "deny"`. A git dependency is listed
  there or it is refused.
- `multiple-versions = "deny"` while the graph is this small. Relaxing it needs
  the reason written into `deny.toml`.
- #373's "no crates.io, no public npmjs" governs where quoin **publishes**.
  Dependencies are consumed from crates.io exactly as quire-corpus's are. Every
  crate here carries `publish = false`.

## Headers and docs

- **Every `.rs` file** opens with:
  ```rust
  // SPDX-License-Identifier: AGPL-3.0-or-later
  // Copyright (C) 2026 Agent-IX
  ```
  Enforced by `tc_375_every_rust_file_carries_the_agpl_spdx_header`, which walks
  the workspace. Note the identifier ends `-or-later` — quoin's
  licence, which is not filament-ide-rs's `AGPL-3.0-only`; copying a header
  across repositories gets this wrong.
- A `//!` module header saying what the module owns and citing the governing
  requirement or ticket; `///` on every public item (enforced by `missing_docs`).
- **Comments state the incident, not the intention.** The comments in this
  workspace that matter name a number: #103, #164, #106, the 1,090,714-byte
  payload. A comment that says "handle errors carefully" is worth nothing; one
  that says which failure it is scar tissue from survives the next refactor.

## The toolchain

**1.98.1**, stated in four places that must agree: `rust/rust-toolchain.toml`,
`rust/clippy.toml`'s `msrv`, `rust/Cargo.toml`'s `rust-version`, and the
`toolchain:` pins in `.github/workflows/build-test.yml`.
`tc_375_the_pinned_channel_and_the_declared_msrv_agree` fails if they drift.

NFR-026 states the floor as "consistent with quire-corpus and
filament-core-data". **Those two do not agree today**: quire-corpus pins
1.98.1, filament-core-data pins 1.94.1. quoin follows the stated floor.

`rustup` selects a toolchain from the **working directory**, so
`cargo --manifest-path rust/Cargo.toml` run from the repository root ignores
`rust/rust-toolchain.toml` entirely and builds with whatever `rustup default`
happens to be. Every Makefile recipe therefore enters `rust/` first. If you
invoke cargo by hand, invoke it from `rust/`.

## Gates — run them, don't assume them

```bash
make rust-gate        # everything below, cheapest failure first
make rust-lint        # fmt --check + clippy -D warnings — the fast leg
make rust-test
make rust-deny
make rust-e2e         # src/core/exec.ts against the REAL binary
make rust-difftest    # needs both trees built; `make build` is a prerequisite
```

**Always name the target directory.** Every recipe passes
`--target-dir rust/target` explicitly. An inherited `CARGO_TARGET_DIR` moves
the artifact somewhere else and leaves whatever was in `rust/target` before —
which is how this repository once measured a binary four days and one engine
older than the build that was supposed to have produced it (see the
`bench-tier1` comment in the Makefile). Discount any clean **or** failed result
from a target directory you did not name.

`make rust-e2e` sets `QUOIN_CORE` and runs `tests/core-exec-e2e.test.ts`, which
skips itself when that variable is unset. It exists because the rest of the
TypeScript caller's suite runs against fake binaries — the only way to produce
an ENOBUFS death or a SIGTERM on demand — and a fake agrees with whatever the
test wrote into it. It found a real defect on its first run: `runCoreAllowFailure`
resolved the executable INSIDE the try block that classifies a termination, so
a digest mismatch (no status, no signal, no errno) fell through to
"could not be run (undefined)" and the pinning diagnostic was lost. Resolution
now happens before the try. **The same shape is latent in `src/quire/exec.ts`**
and was left alone — it is not this ticket's code.

`make rust-difftest` is FR-101's check, not a formality: it feeds the same
request to the retained TypeScript and to `quoin-core` and compares canonical
stdout, exit status, and the normalised diagnostic shape — `(code, context
keys)`, deliberately **not** the message text, because #373 records that
verdicts are contractual and error prose is not. When Stage 1 lands
`quoin-quire` it gains cases, not a framework.
