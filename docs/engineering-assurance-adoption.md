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

This document is the record required by that policy: the capability census, the
retention test for everything quoin keeps local, and the gaps filed against EA.

## How the dependency is consumed

`rust/Cargo.toml`:

```toml
engineering-assurance = { version = "=0.3.1", git = "https://github.com/agent-ix/engineering-assurance", rev = "72b0fcd" }
```

A **git rev pin**, which is the pattern `agent-ix/quire-cli` already proves
against `quire-rs` (`quire-cli/Cargo.toml:20`), not an invented one. EA is
`publish = false` and quoin#373 rules out crates.io, so a registry version is
not available to either repository. `rust/deny.toml` names the URL under
`allow-git`; `unknown-git = "deny"` refuses anything else.

EA is a **dev-dependency of `quoin-core`**, not a runtime dependency. Today's
single consumer is a test (below). It moves to `[dependencies]` in the first
stage that ships EA-backed behaviour in the binary — and see "Known blockers"
before that happens.

### The pin collision, and how it was resolved

EA and quoin both use exact `=` pins, and they disagreed. `cargo build` with EA
added and quoin's pins unchanged:

```
error: failed to select a version for `serde`.
    ... required by package `engineering-assurance v0.3.1 (https://github.com/agent-ix/engineering-assurance?rev=72b0fcd#72b0fcd1)`
    ... which satisfies git dependency `engineering-assurance` of package `quoin-core v0.1.0 (.../rust/crates/quoin-core)`
    ... which satisfies path dependency `quoin-core` (locked to 0.1.0) of package `quoin-difftest v0.1.0 (.../rust/crates/quoin-difftest)`
versions that meet the requirements `=1.0.229` are: 1.0.229

all possible versions conflict with previously selected packages

  previously selected package `serde v1.0.228`
    ... which satisfies dependency `serde = "=1.0.228"` (locked to 1.0.228) of package `quoin-core v0.1.0 (.../rust/crates/quoin-core)`

failed to select a version for `serde` which could resolve this conflict
```

Two `=` requirements on one semver-compatible range are unsatisfiable; the same
collision stood on `serde_json` and `thiserror`. **quoin moved up to EA's pins**
— EA is the shared home and the ecosystem follows it:

| crate      | quoin before | quoin now (= EA) |
| ---------- | ------------ | ---------------- |
| serde      | `=1.0.228`   | `=1.0.229`       |
| serde_json | `=1.0.150`   | `=1.0.151`       |
| thiserror  | `=2.0.18`    | `=2.0.20`        |

`jsonschema` already agreed at `=0.56.0`.

## Capability census

Every EA public module against quoin. Paths are repository-relative.

| EA module              | quoin local equivalent                                                                                                                                                                                                                                                                                                                                                       | disposition                                                                                                                                                                                                                                   |
| ---------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `compatibility`        | `scripts/check-tool-drift.mjs:13` `auditToolDrift()`; `scripts/check-version-agreement.mjs:16`; `quality/verification-stack-lock.json`                                                                                                                                                                                                                                       | **consume** at Stage 5. quoin classifies pinned-tool drift, not a shared compatibility matrix; the matrix belongs in EA.                                                                                                                      |
| `compatibility_corpus` | `scripts/verification-declarations.mjs:225` `verifyDeclarations()`, `:120` `committedTree()`; `src/measurement/fixture-corpus.ts:15`                                                                                                                                                                                                                                         | **consume**. "Corpus accounting" is named verbatim in #373's consume-from-EA clause.                                                                                                                                                          |
| `content_rights`       | `rust/crates/quoin-core/tests/tc_source_conventions.rs:46` `tc_375_every_rust_file_carries_the_agpl_spdx_header`                                                                                                                                                                                                                                                             | **consume** (retention test below). `src/evidence/adapters/sbom.ts:82` only _reads_ SPDX SBOM documents; there is no second TS checker.                                                                                                       |
| `discovery`            | none. `skills/` (28 dirs), `.claude/skills/rust-style/` (a directory **copy**, not a symlink), `.claude-plugin/plugin.json`, `.codex-plugin/plugin.json`, `.agents/plugins/marketplace.json` exist; nothing checks they reference one canonical skill. Closest: `tests/skill-contracts.test.ts:18` (frontmatter loads), `scripts/release-drift.js:45` (plugin version drift) | **consume**. The copies-vs-references condition EA detects is live in quoin today and unchecked.                                                                                                                                              |
| `evaluation`           | `src/completeness/assess.ts:117` `assessVocabulary()`, `:229` `verdictFor()`; `src/measurement/agent-eval-intervention.ts:182`                                                                                                                                                                                                                                               | **consume** at Stage 6. "evaluation/evidence reporting" is in #373's clause.                                                                                                                                                                  |
| `evaluation_reports`   | `src/evidence/adapters/agent-eval.ts:31` `parseAgentEval()`; version gate `src/measurement/agent-eval-intervention.ts:183`                                                                                                                                                                                                                                                   | **consume** at Stage 6.                                                                                                                                                                                                                       |
| `evidence`             | `src/evidence/store.ts:50` `storeRoot()`, `:146` `canonicalJson()`, `:219` `writeRun()`; `src/evidence/record.ts`; `src/evidence/trust.ts`                                                                                                                                                                                                                                   | **quoin owns it.** EA's migration contract assigns the evidence STORE to Quoin; the edge points EA → Quoin. `rust/quoin-store` is quoin's.                                                                                                    |
| `manifest`             | `src/semantic/manifest.ts:136` `readSemanticBlock()`, `:71` (ajv); `src/quire/validate.ts:50`; `src/semantic/package-manifest.ts:95`; `rust/crates/quoin-schemas/src/lib.rs`                                                                                                                                                                                                 | **quoin owns it** (retention test below). quoin's manifests are quoin's own schema surface, and this is Stage 3 work that precedes EA-consuming stages.                                                                                       |
| `onboarding`           | none found (zero case-insensitive hits for "onboarding" across `src/`, `scripts/`, `tests/`, `bin/`, `quality/`)                                                                                                                                                                                                                                                             | **not needed.** quoin is not an onboarding host.                                                                                                                                                                                              |
| `package_audit`        | none found. No `npm pack` inspection, no archive reader.                                                                                                                                                                                                                                                                                                                     | **consume** — named verbatim in #373. Clean adopt, nothing to retire.                                                                                                                                                                         |
| `package_lifecycle`    | `scripts/release-drift.js:57`, `:212`; `package.json` `publish:dry-run` / `prepublishOnly`                                                                                                                                                                                                                                                                                   | **consume** at Stage 8.                                                                                                                                                                                                                       |
| `package_membership`   | `scripts/release-drift.js:57` derives the path set from `package.json.files` but compares against **git**, not an archive's members                                                                                                                                                                                                                                          | **consume** at Stage 8. The archive-member half does not exist in quoin.                                                                                                                                                                      |
| `producer_execution`   | `src/core/exec.ts:48` `QUOIN_CORE_MAX_BUFFER`, `:58` `CORE_EXIT`; `src/quire/exec.ts:112`; `src/measurement/engine-run.ts:136` `runBatch()`                                                                                                                                                                                                                                  | **consume** at Stage 6 (retention test below). quoin bounds capture and classifies termination for **fixed, pinned** binaries; it has no capability-rooted artifact access, environment isolation, cancellation or process-group confinement. |
| `semantics`            | `src/semantic/contract.ts:23`, `package-manifest.ts:30` `typeIdentity()`, `:112` `exportDigests()`, `:130` `registryPin()`                                                                                                                                                                                                                                                   | **quoin owns it** (retention test below). These are quoin's semantic-module contract, not EA's verification semantics; the names collide, the domains do not.                                                                                 |
| `source_audit`         | containment: `tests/arch-boundaries.test.ts:22` (TypeScript compiler API, TS sources only). Trace tags: **none** — quoin never parses `Trace:` out of source; `quire coverage --json` does (`src/quire/exec.ts:100`, `src/quire/validate.ts:85`)                                                                                                                             | **consume** — and this is the module the one real call routes through today. The `RequirementTests` role does not fit (gap filed).                                                                                                            |
| `structured_yaml`      | `src/modules.ts:5`+`:20`; `src/catalog.ts:5`; `src/plugins.ts:4`; 7 more call sites — all direct `yaml.parse`, no unambiguity layer                                                                                                                                                                                                                                          | **consume** when the Rust side needs YAML. #373 records the TS side as a verified non-risk ("quoin only calls `yaml.parse`, never `stringify`").                                                                                              |
| `workflow`             | `src/flows.ts:74` `spawn("ix-flow", …)`, `:22` `startSpecFlow()`; `src/flow-command.ts:14`                                                                                                                                                                                                                                                                                   | **consume** at Stage 8.                                                                                                                                                                                                                       |
| `workflow_invariants`  | `src/graph-analysis/analysis.ts:154` `analyzeFanOut()`, `:263` `analyzeChangeImpact()`; `src/auditor/combinatorial.ts`; `src/change-assurance/integrity.ts`                                                                                                                                                                                                                  | **not needed.** quoin evaluates evidence-graph invariants, not ix-flow workflow invariants. Same shape, different closed projection; adopting EA's types would mean adopting EA's domain.                                                     |

Counts: **10 consume**, **4 quoin owns it** (three by retention test, one by
EA's own migration contract), **2 not needed**, plus `source_audit` consumed
today and `content_rights` pending. Nothing is padded: `onboarding` and
`workflow_invariants` are the only genuine "not needed" entries.

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

### 6. `scripts/check-tool-drift.mjs:13` — pinned-tool drift, vs EA `compatibility`

- **Must it be local?** **No** for the classification; **yes** for the pin set
  (`quality/verification-stack-lock.json` is quoin's own lock).
- **Is it Rust?** **No**, it is a `.mjs` script.
- **Should it be common?** **Yes.** Classifying observed versions against a
  reviewed matrix is EA's `compatibility` module exactly.
- **Disposition: consume at Stage 5, keeping the lock file local.**

## Known blockers, filed against EA

These are why adoption is one test and not ten. Each is an EA ticket, never a
local copy.

| #   | EA issue                           |
| --- | ---------------------------------- |
| 1   | agent-ix/engineering-assurance#99  |
| 2   | agent-ix/engineering-assurance#100 |
| 3   | agent-ix/engineering-assurance#101 |
| 4   | agent-ix/engineering-assurance#102 |
| 5   | agent-ix/engineering-assurance#103 |

1. **17 of 19 public modules sit behind a single `full` feature**, and `full`
   pulls `cap-std`, `clap`, `flate2`, `jsonschema`, `regex`, `syn`, `tar`,
   `time`, `unicode-casefold`, `yaml_serde` and `zip`. A consumer that wants
   only `evidence` — or, as here, only `source_audit` — imports a CLI's entire
   dependency tree. EA needs per-capability features.

2. **`--no-default-features` and `--no-default-features --features
producer-execution` do not compile** at `72b0fcd`. `pub mod evaluation;` is
   unconditional in `src/lib.rs` while `src/evaluation.rs` imports `time`,
   `serde`, `serde_json` and `thiserror`, all `full`-only optionals. `full` is
   the only feature set that builds. This is the defect EA#78 closed; it stands
   again at `origin/main`.

3. **EA's `full` graph carries duplicate versions** that a consumer with
   `bans.multiple-versions = "deny"` cannot resolve. Measured against
   `x86_64-unknown-linux-gnu` only: `getrandom` 0.3.4 and 0.4.3,
   `io-lifetimes` 2.0.4 and 3.0.1, `syn` 2.0.119 and 3.0.5. Across all targets,
   14 crates duplicate (the remainder are the `windows-sys` 0.59/0.60 split
   under `cap-std`). **This is why EA is a dev-dependency here.** `cargo deny`'s
   graph at that placement is 5 crates and does not reach EA at all; the moment
   EA moves to `[dependencies]`, `make rust-deny` goes red until this is fixed.

4. **Exact `=` pins make EA and every consumer mutually unsatisfiable** until
   one side moves, as the `serde` error above shows. quoin moved up, which is
   the right direction, but it is a manual step for every consumer on every EA
   bump.

5. **`source_audit`'s `RequirementTests` role assumes `ix-trace-rs`
   `#[trace(...)]` attributes.** quoin's convention, stated in
   `.claude/skills/rust-style/SKILL.md`, is a `/// Trace:` doc line with
   comma-separated criteria. Measured over quoin's three integration-test files
   at `72b0fcd`: **13 findings, 13 false positives, 0 true positives** — one
   `TraceImportMissing` per file and one `TestTraceMissing` per test, on tests
   that every one of them carries a `/// Trace:` line. The half of
   `source_audit` quoin most needs is unreachable.
