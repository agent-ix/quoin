<!-- SPDX-License-Identifier: AGPL-3.0-or-later -->
<!-- Copyright (C) 2026 Agent-IX -->

# Engineering Assurance adoption (quoin#373 AC-9)

quoin's Rust workspace consumes `agent-ix/engineering-assurance` (EA) rather
than regrowing assurance capability locally. The governing policy is
quire-research `implementation-language-policy.md`, **"Shared tooling is
consumed, not regrown"**: a repository needing assurance capability CONSUMES
EA; where EA does not fit, the response is a **gap ticket in EA**, never a local
harness.

**The one standing exception** is EA's own migration contract, which assigns the
evidence **store** to Quoin. That edge points from EA to Quoin and is not
inverted here.

This document is the record required by that policy: the
retention test for everything quoin keeps local, and the gaps filed against EA.

## How the dependency is consumed

A **git rev pin**, which is the pattern `agent-ix/quire-cli` already proves
against `quire-rs` (`quire-cli/Cargo.toml:20`), not an invented one. EA is
`publish = false` and quoin#373 rules out crates.io, so a registry version is
not available to either repository. `rust/deny.toml` names the URL under
`allow-git`; `unknown-git = "deny"` refuses anything else.

## The one real consumer call

`rust/crates/quoin-core/tests/tc_library_containment.rs` —
`tc_373_the_library_half_names_no_host_capability`.

It walks every `.rs` file under `crates/quoin-core/src/` except `main.rs` and
runs `engineering_assurance::source_audit::audit_rust_source` with
`RustSourceAuditRole::ReusableLibrary`, failing on any
`ForbiddenCapability` finding.

This is not a smoke test. `.claude/skills/rust-style/SKILL.md` states the rule
in prose — "`main.rs` does four things … and must keep doing only four.
Everything decidable lives in the library so it is unit-testable without
spawning a process" — and prose is not a gate. A `std::fs::read_to_string` in
`ops/` passes every other test and passes clippy. The test was negative-
controlled: appending `std::fs::read_to_string` to `src/protocol.rs` produces

```
left: ["src/protocol.rs: Some(Filesystem)"]
```

quoin did **not** grow a syn-based Rust source auditor to get this. That is the
policy working.

## Retention test

Recorded for everything quoin keeps local that EA also offers. Three parts,
each answered, with reasons — including where the answer is "no".

### 1. `rust/crates/quoin-core/tests/tc_source_conventions.rs:46` — SPDX headers, vs EA `content_rights`

- **Must it be local?** **No.** It is a two-line `head.contains(...)` on each
  file; EA's `content_rights::inspect_content` classifies the same thing over
  caller-supplied paths and bytes with a closed category set.
- **Is it Rust?** Yes, and so is EA's.
- **Should it be common?** **Yes.** Every repository in the ecosystem asserts
  its own SPDX header the same way, and each has written the check again.
- **Disposition: retire in favour of EA.** Not done in this change because the
  `full`-feature blocker (below) makes the import cost disproportionate to a
  two-line check. Tracked, not conceded.

### 2. `tests/arch-boundaries.test.ts:22` — TypeScript import containment, vs EA `source_audit`

- **Must it be local?** **Yes.** It audits **TypeScript** sources via the
  TypeScript compiler API. EA's `source_audit` parses **Rust** with `syn`.
  There is no EA capability for this input language.
- **Is it Rust?** **No** — this is the deciding answer. It cannot move to a Rust
  library while the sources it audits are TypeScript, and #373 retires those
  sources rather than porting the auditor.
- **Should it be common?** Moot while the answer to part two is no. When
  Stage 8 retires the TypeScript, the Rust replacement is EA's — as
  `tc_library_containment.rs` already demonstrates.
- **Disposition: retained, with a stated end date (Stage 8).**

### 3. `src/evidence/store.ts` (and the future `rust/quoin-store`) — evidence store, vs EA `evidence`

- **Must it be local?** **Yes.** EA's own migration contract assigns the
  evidence store to Quoin. This is the standing exception, and inverting it
  would make EA both the producer and the retainer of its own evidence.
- **Is it Rust?** Not yet; `rust/quoin-store` is Stage 4.
- **Should it be common?** **Yes — and it already is, in the other direction.**
  EA consumes Quoin's store. That is the edge, and it is not a "no".
- **Disposition: quoin owns it, permanently.**

### 4. `src/core/exec.ts` / `src/quire/exec.ts` — bounded subprocess execution, vs EA `producer_execution`

- **Must it be local?** **Partly, today.** `src/core/exec.ts` is the _caller_ of
  the quoin-core boundary: it is the TypeScript side of the FR-101 coexistence
  and cannot be EA's, because EA does not know quoin's exit taxonomy. The
  _measurement_ producer runs (`src/measurement/engine-run.ts:136`) are a
  different matter and have no such claim.
- **Is it Rust?** **No**, both are TypeScript today.
- **Should it be common?** **Yes**, for the measurement half. "Bounded producer
  execution" is named verbatim in #373's consume-from-EA clause, and quoin's
  version has no capability-rooted artifact access, no environment isolation,
  no cancellation and no process-group confinement — four properties EA already
  owns and quoin would otherwise write badly.
- **Disposition: `src/core/exec.ts` retained as the boundary caller; the
  measurement executor consumes EA at Stage 6.**

### 5. `src/semantic/` — semantic-module contract, vs EA `semantics`

- **Must it be local?** **Yes.** EA's `semantics` validates references to
  externally authoritative verification records. quoin's `src/semantic/` owns
  the semantic-module manifest, its schema identity and its export digests —
  quoin's own product surface, which EA consumes rather than defines.
- **Is it Rust?** Not yet; Stage 3.
- **Should it be common?** **No**, and the reason is stated rather than assumed:
  it is quoin's domain vocabulary, and a shared library that owned it would make
  every consumer of EA a consumer of quoin's module format.
- **Disposition: quoin owns it.**

## Known blockers, filed against EA

These are why adoption is one test and not ten. Each is an EA ticket, never a
local copy.

| #   | EA issue                           |
| --- | ---------------------------------- |
| 1   | agent-ix/engineering-assurance#99  |
| 2   | agent-ix/engineering-assurance#100 |
| 4   | agent-ix/engineering-assurance#102 |
| 5   | agent-ix/engineering-assurance#103 |

1. **17 of 19 public modules sit behind a single `full` feature**, and `full`
   pulls `cap-std`, `clap`, `flate2`, `jsonschema`, `regex`, `syn`, `tar`,
   `time`, `unicode-casefold`, `yaml_serde` and `zip`. A consumer that wants
   only `evidence` — or, as here, only `source_audit` — imports a CLI's entire
   dependency tree. EA needs per-capability features.

2. **`--no-default-features` and `--no-default-features --features
producer-execution` do not compile**. `pub mod evaluation;` is
   unconditional in `src/lib.rs` while `src/evaluation.rs` imports `time`,
   `serde`, `serde_json` and `thiserror`, all `full`-only optionals. `full` is
   the only feature set that builds. This is the defect EA#78 closed; it stands
   again at `origin/main`.

4. **Exact `=` pins make EA and every consumer mutually unsatisfiable** until
   one side moves. quoin moved up, which is
   the right direction, but it is a manual step for every consumer on every EA
   bump.

5. **`source_audit`'s `RequirementTests` role assumes `ix-trace-rs`
   `#[trace(...)]` attributes.** quoin's convention, stated in
   `.claude/skills/rust-style/SKILL.md`, is a `/// Trace:` doc line with
   comma-separated criteria. Measured over quoin's three integration-test files:
   **13 findings, 13 false positives, 0 true positives** — one
   `TraceImportMissing` per file and one `TestTraceMissing` per test, on tests
   that every one of them carries a `/// Trace:` line. The half of
   `source_audit` quoin most needs is unreachable.
