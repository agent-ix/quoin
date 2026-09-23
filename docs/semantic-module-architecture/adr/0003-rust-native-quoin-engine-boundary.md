---
id: ARCH-SM-ADR-0003
title: "Rust-native Quoin engine boundary"
status: accepted
date: 2026-09-12
requirements:
  - StR-009
  - FR-096
  - FR-097
  - FR-098
  - FR-099
  - FR-100
  - FR-101
  - FR-102
  - FR-103
  - FR-112
  - NFR-024
  - NFR-025
  - NFR-026
  - NFR-027
---

# ADR-0003: Rust-native Quoin engine boundary

## Context

Quoin implements its first-party engine, production, planning, validation,
canonicalization and digest, oracle, evidence and qualification behaviour in
TypeScript on an oclif v4 command shell, with supporting `.mjs` scripts, an
evaluation harness, and a Markdown skills tree carrying executable workflow
assets. The measured baseline, taken at `e718d45` over `git ls-files`
intersected with `{.ts, .mjs, .js, .py, .sh, .tsp}` and counted in physical
lines, is **105,814 lines across 364 in-repository files**. The largest single
block is `skills/` at 23,164 lines; `bin/` and the root tool configuration
account for a further 302.

`corpus/` is a **git submodule** pointing at `agent-ix/qa-corpus` at `7b81343`.
It is another repository's tree and is not counted in the in-repository
baseline. Whether it is in this programme's scope at all is an open owner
ruling.

The campaign implementation language policy, owned by
[`agent-ix/quire-research`](https://github.com/agent-ix/quire-research),
requires first-party production and qualification-path implementation to be
Rust. Its earlier text deferred Quoin. **An owner directive of 2026-09-12
withdraws that deferral** and directs a staged port. `engineering-assurance`
completed the same transition and is the reference: ADR-002, StR-003, FR-014
through FR-019, NFR-005.

This decision changes Quoin's implementation language. It does not change
Quoin's architecture, its ownership of the evidence store, the catalog, or the
measurement model, and it takes no ownership from Quire, `filament-core-data`,
`engineering-assurance`, ix-flow, or any module repository.

The enforcement run that publishes the metric is owned by quire-research LR08.
Quoin owns only the retention of its result in Quoin's evidence store. Neither
programme reports the other's work as its own.

Two standing-approved TypeScript categories are **not** program debt and are not
retired by this decision: (a) user-interface code, and (b) types and clients
published by `filament-core-data` from the single multi-language schema source.

This ADR is a sibling of [ADR-0001](0001-authority-by-concern.md) and
[ADR-0002](0002-preserve-quire-quoin-boundaries.md) and does not supersede
either. Those allocate _authority_; this allocates _implementation language_.
Where they collide on catalog and module code, ADR-0002's allocation governs who
owns the behaviour and this ADR governs what language expresses it.

## Decision drivers

1. Put Quoin's first-party engine, production and qualification semantics in
   Rust, staged rather than rewritten at once.
2. Preserve observable behaviour, including non-success and refusal paths, and
   preserve every existing identity domain (digest and canonicalization).
3. Keep one Quoin evidence store, one catalog, and one measurement model. A port
   must not fork them.
4. Replace an existing implementation only after its Rust replacement is
   demonstrated at one candidate revision, and keep the cutover reversible.
5. Consume shared assurance capability from `engineering-assurance` rather than
   re-growing it in Rust.
6. Change no evidence byte and no accepted-corpus byte.

## Options considered

| Option                                                           | Disposition                                                                                                                                                                                                                                    |
| ---------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Subprocess and JSON over stdin/stdout to one `quoin-core` binary | **Selected.** The precedent is already built and hardened in this repository; it converges on deleting the shell; it is diffable by hand; and it survives the filesystem and child-process access the evidence and measurement stores require. |
| `napi-rs` native Node addon                                      | Rejected. It adds an N-API shim layer whose only purpose is to be deleted at the final stage, and it makes the boundary a linked in-process surface that cannot be diffed by replaying a recorded request.                                     |
| WebAssembly (WASM/WASI)                                          | Rejected. The evidence and measurement stores perform directory creation and atomic rename, and the Quire adapter spawns a child process; there is no browser target that would justify the WASI plumbing to restore them.                     |
| A separate Rust repository                                       | Rejected. [FR-101](../../../spec/functional/FR-101-retire-replaced-executable-paths.md)'s one-candidate-revision rule becomes a two-repository pin-and-sync problem, which is the drift class `quire-cli` already hit.                         |
| Keep TypeScript behind a Rust wrapper                            | Rejected. Per policy, a Rust wrapper around non-Rust semantic logic does not satisfy the boundary; it conceals the implementation rather than replacing it.                                                                                    |

### Grounds for the selected boundary

1. **The precedent is built and tested.** `src/quire/exec.ts` already encodes
   four incidents this repository paid for: executable realpath resolution,
   `QUOIN_EXPECTED_QUIRE_SHA256` bytes pinning, a 64 MiB `maxBuffer` (ENOBUFS on
   a 1,090,714-byte payload, agent-ix/quoin#164), and a three-way termination
   taxonomy. `src/core/exec.ts` is a copy-edit of it carrying `QUOIN_CORE` and
   `QUOIN_EXPECTED_CORE_SHA256`.
2. **It converges.** At the final stage `quoin-core` _becomes_ `quoin`: the shell
   is deleted and a binary is renamed. No layer is built to be thrown away.
3. **It is diffable.** `quoin-core evidence.record --json < request.json` can be
   replayed against the retained TypeScript entry point, which is what makes the
   differential harness small enough to be trusted.
4. **It survives the I/O the domain requires.** Stores do directory creation and
   atomic rename; the Quire adapter spawns a child process. The cost is one
   spawn per operation, alongside the Quire spawns already occurring.

### Boundary rules

- The unit of inter-process communication is a **command-shaped operation**
  (`quoin-core <domain>.<op>`), never a function. Exposing a primitive such as
  canonical-JSON serialization over the boundary creates a surface that can never
  be deleted.
- stdout carries the machine payload; stderr carries diagnostics.
- The exit taxonomy is 0/1/2/3/4, and "non-zero status with a valid payload" is
  distinguishable from "non-zero status with no payload".
- TypeScript never hand-writes a response type. `schemars` emits JSON Schema, a
  build step generates `src/core/types.ts`, and the generated file is
  hash-asserted the way `src/quire/contract.ts` is today.

## Crate topology

One Cargo workspace in-repo at `rust/`, beside the retained `src/`:

| Crate                                                                          | Owns                                                            | Stage |
| ------------------------------------------------------------------------------ | --------------------------------------------------------------- | ----- |
| `quoin-schemas`                                                                | vendored and `schemars`-generated schemas, one hash-pinned home | 0     |
| `quoin-core`                                                                   | the boundary binary: dispatch only, no domain logic             | 0     |
| native fixture tests                                                            | retained golden corpora replayed without a Node runtime         | 8     |
| `quoin-quire`                                                                  | facade over `quire-rs`                                          | 1     |
| `quoin-validators`                                                             | validators                                                      | 2     |
| `quoin-semantic`, `quoin-completeness`                                         | semantic and completeness leaves                                | 3     |
| `quoin-store`                                                                  | canonical JSON, JCS, blake3, atomic rename                      | 4     |
| `quoin-evidence`, `quoin-change-assurance`                                     | evidence and change assurance                                   | 4     |
| `quoin-auditor` (including advisor), `quoin-assurance`, `quoin-graph-analysis` | audit, assurance, graph analysis                                | 5     |
| `quoin-measurement{-core,-portfolio,-graph,-operational,-intervention}`        | measurement                                                     | 6     |
| `quoin-config`, `quoin-modules`, `quoin-catalog`                               | configuration, modules, catalog                                 | 7     |
| `quoin-cli`                                                                    | `clap`; becomes `quoin`                                         | 9     |

A crate boundary exists where a dependency cycle must be broken or a stage must
ship independently, not once per source directory — hence auditor and advisor are
one crate and measurement is five.

Outward edges: `quoin-quire` to `quire-rs`; every crate to `ix-trace-rs` for
test tracing; `quoin-modules` to `gix`. **`engineering-assurance` is a reference,
not a dependency** — its own migration contract assigns the evidence store to
Quoin, so that edge points from Engineering Assurance to Quoin, not the reverse.

Two dependency cycles must be broken before any Rust crate exists, because Cargo
forbids them: `src/evidence/index.ts:142` re-exports from
`../change-assurance/schema-assets.js` while `src/change-assurance/store.ts:22`
imports `storeRoot` from `../evidence/store.js`; and `auditor` depends on
`advisor` while `advisor` depends on `auditor`.

## Capability boundary

| Capability                                                                                                                                                              | Rust disposition                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                       |
| ----------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Quire adapter (`src/quire/`)                                                                                                                                            | Port to `quoin-quire` as a facade over `quire-rs`; the vendored-contract machinery dissolves into a Cargo edge.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                        |
| Validators (`src/validators/`)                                                                                                                                          | Port. Deliberately first: it proves the boundary, the error taxonomy, generated types, the differential harness and the delete step on a small surface.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                |
| Semantic and completeness (`src/semantic/`, `src/completeness/`)                                                                                                        | Port. The ajv-to-`jsonschema` verdict-parity question is settled here, on a leaf.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                      |
| Canonicalization, digest and store primitives                                                                                                                           | Port to `quoin-store` as one implementation; digest and canonicalization outputs are unchanged.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                        |
| Evidence and change assurance (`src/evidence/`, `src/change-assurance/`)                                                                                                | Port after the cycle break. Quoin retains sole ownership of the evidence store.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                        |
| Auditor, advisor, assurance, graph analysis                                                                                                                             | Port; auditor and advisor become one crate.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                            |
| Measurement (`src/measurement/`)                                                                                                                                        | Port into five crates. Check `engineering-assurance` for each reporting and accounting capability before writing a local one.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                          |
| Configuration, plugins, modules, catalog                                                                                                                                | Port; this is the only stage needing git and network access.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                           |
| Command surface (`src/commands/`, `src/cli.ts`)                                                                                                                         | Port last. `@oclif/core` retirement is Stage 9, after every logic capability behind it is Rust-native.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                 |
| Corpus tooling reached through the `corpus/` submodule                                                                                                                  | The submodule points at `agent-ix/qa-corpus`, which this repository does not own. Only the Quoin-side accounting and selection are in scope here; the `qa-corpus` side is pending an owner scope ruling and carries no disposition from this ADR.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                      |
| Foreign-language sample inputs that determine no assertion                                                                                                              | Retain as inert data under the `inert` category of `.language-allowances.yaml`, identified by the manifest rather than by a directory name.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                            |
| `skills/**/workflow-assets/**`                                                                                                                                          | **Not inert.** `src/flows.ts:57` spawns ix-flow against these assets and their `specInvariants` decide whether a review, matrix or plan flow passes, which makes them assertion logic in scope. Measured at `e718d45`, 22,575 of the 23,164 lines under `skills/` are three byte-identical copies of `workflow-assets/dist/index.js` and its declaration file — a build product of `ix-spec-workflows` — while `scripts/invariants.js` is 139 hand-written lines re-exporting `specInvariants` from it. The shim is first-party logic in scope; the bundle claims the `generated` exception only once its generator identity and source digest are recorded, and is governed as first-party source until then. Whether the bundle belongs in this programme's baseline at all is an open owner ruling. |
| Markdown skill prose outside `workflow-assets/`                                                                                                                         | Retain as data, shipped through `files:`. Skill-vocabulary-drift and skill-contract assertions port to Rust tests reading the same Markdown.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                           |
| Schemas, manifests, specifications, plans, reviews, fixtures                                                                                                            | Retain in their data or documentation formats.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                         |
| User-interface TypeScript                                                                                                                                               | Retain. Standing-approved; not program debt.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                           |
| `filament-core-data`-published types and clients                                                                                                                        | Retain. Standing-approved; not program debt. Hand-written duplicates of them are refused ([FR-097](../../../spec/functional/FR-097-schema-sourced-type-surface.md)).                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                   |
| `@agent-ix/ix-cli-core`                                                                                                                                                 | Do not port. Quoin uses nine of its symbols and none of its authentication, secrets or marketplace surface; those nine behaviours are reimplemented natively.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                          |
| Shared assurance capability (advisory floors, package audit, bounded producer execution, evaluation and evidence reporting, corpus accounting, canonical serialization) | Consume from `engineering-assurance`. When it does not fit, file a gap ticket in `engineering-assurance` rather than growing a local substitute.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                       |

## Decision

Quoin's first-party engine, production and qualification behaviour is delivered
by one in-repository Cargo workspace at `rust/` and the `quoin` binary built
from `quoin-cli`. The temporary `quoin-core` subprocess boundary is no longer
on the production path: `quoin-cli` invokes its runtime in-process. Its
remaining development-only executable target is explicitly tracked for deletion
by #521 before promotion. No first-party TypeScript runtime, command shell,
test oracle, evaluation harness, release, or install-smoke path remains.

The Rust workspace adopts the **`/rust-review` skill**
(`agent-ix` marketplace, `skills/rust-review/SKILL.md`) as its standing review
guide for Rust style, idioms and smells. Per that skill's own §0, a repository's
own idiom document outranks it. Quoin has no `rust-style` document today; Stage 0
creates `.claude/skills/rust-style/SKILL.md` for this workspace, and until it
exists `/rust-review` is the authority.
[NFR-027](../../../spec/non-functional/NFR-027-rust-implementation-idioms-and-gates.md)
makes the enforceable subset of that guidance a requirement rather than a
convention, and §12 of the skill — gates are run, not assumed — is why every
criterion in this set names a command rather than an intent.

## Port and cutover order — three normative stages

Quoin's port runs in three ordered stages. This ADR's ten delivery stages sit
inside them.

1. **Extract deterministic domain logic into Rust crates behind one stable,
   versioned boundary.** Delivery stages 0 through 7: workspace and boundary
   (0), Quire adapter collapse (1), validators (2), semantic and completeness
   (3), cycle break, store, evidence and change assurance (4), auditor, advisor,
   assurance and graph analysis (5), measurement (6), configuration, plugins,
   modules and catalog (7, parallel from stage 2). Corpus consolidation runs in
   parallel and independently.
2. **Reduce the TypeScript surface to a thin caller of that boundary.** Delivery
   stage 8: command residue, and deletion of `src/quire/exec.ts`.
3. **Retire the oclif command shell and `@oclif/core`.** Delivery stage 9,
   completed in `33ca665` after native fixture replay. The resulting executable
   is `quoin`; the plugin and hook were withdrawn under #396. The native
   `quoin sync` command that briefly replaced it wired the dependency's own
   test doubles into production and pulled the private AGPL
   `agent-ix/filament-ide-rs` repository into the graph; it was removed
   outright rather than kept as a non-functional stand-in. Plan syncing will
   be designed separately, once Linear integration happens.

Each delivery stage ends in two tickets that are never merged together: a
reversible cutover, and a deletion of the retained TypeScript together with its
tests. Deletion is always last.

The delivery-stage issues 0 through 9 were created on 2026-09-12 and the
executable-path matrix names them, because
[NFR-024](../../../spec/non-functional/NFR-024-bounded-staged-coexistence.md)
defines a valid successor reference as one of them and reports an epic-level
reference as provisional.

## Gate

`filament-core-data` is the multi-language schema source of truth, and its gate
is two-phase:

- **Phase A** — `filament-core-data` #7, cross-language compatibility, passes.
  This releases delivery stages 0 through 3. None of them retires a hand-written
  type.
- **Phase B** — `filament-core-data` #11, generated packages actually publish,
  closes. This gates type retirement only, that is
  [FR-097](../../../spec/functional/FR-097-schema-sourced-type-surface.md) and
  [IT-003](../../../spec/integration/IT-003-filament-core-data-published-types.md).

## Registries

No crates.io and no public npmjs. Rust crates are unpublished and consumed by
git-revision pin, the pattern `quire-cli` already uses against `quire-rs`.
TypeScript packages publish to `npm.ix` / GitHub Packages. Python packages
publish to the internal PyPI. Any requirement in this set that touches
publication carries this restriction.

**Amendment, 2026-09-23 (owner ruling):** the native `quoin` binary is
distributed on public npmjs again, as a launcher package `@agent-ix/quoin`
plus one platform package per supported target, packaged from the same
GitHub Release assets FR-102 already publishes rather than rebuilt
(FR-112). `agent-ix/nodejs-actions/publish-native-npm` owns the packaging
and publish steps; quoin's own workspace carries none. Rust crates stay
off crates.io, and no other package in this set gains a public-npmjs
publication target — this amendment covers the native binary only.

## Consequences

- The repository gains an in-tree Rust workspace while retaining its schemas,
  skills, specifications, corpora and fixtures as data.
- TypeScript and Rust coexisted only during staged cutovers. The final Stage 9
  deletion removed the retained runtime and Node test/oracle harnesses; native
  checked-in fixtures are now the parity evidence.
- The baseline TypeScript suite was a temporary oracle, not debt. Each retained
  behaviour is now covered by tracking-tagged Rust tests or native fixture
  replay, and the old tests were deleted with their implementation.
- Evidence bytes and accepted-corpus bytes do not change
  ([NFR-025](../../../spec/non-functional/NFR-025-immutable-evidence-and-corpus-bytes.md)).
- No second catalog, evidence model, runner or measurement model is created.
- Containment (quire-research #56) and burn-down (this programme) stay separate:
  neither reports the other's work as its own.

## Revisit triggers

Reopen this decision if `quire-rs` cannot expose a usable structured interface
for a capability `quoin-quire` must wrap; if the evidence store's on-disk format
cannot be preserved byte-for-byte through a Rust implementation; if digest or
canonicalization agreement cannot be demonstrated over every reachable store; if
the `filament-core-data` gate does not reach Phase A; or if a required tool
demonstrably cannot run on the toolchain floor in
[NFR-026](../../../spec/non-functional/NFR-026-rust-toolchain-floor.md).
Formatting differences and repairable lint findings are not tool
incompatibilities.

## Open questions for the owner

- The `@agent-ix/filament-plan-sync` oclif plugin and the `command_not_found`
  hook were withdrawn at Stage 9 under #396. Its native `quoin sync`
  replacement was removed in turn: quoin does not depend on
  `filament-ide-rs`, and plan syncing will be designed separately. This is no
  longer an open question.
- `filament-core-data` and `quire-rs` pin Rust 1.94.1 today, not 1.98.1. See
  [NFR-026](../../../spec/non-functional/NFR-026-rust-toolchain-floor.md).
- The `corpus/` submodule points at `agent-ix/qa-corpus`, a repository this
  programme does not own. Whether the burn-down has any scope over it needs an
  owner ruling; until then
  [FR-103](../../../spec/functional/FR-103-corpus-consolidation.md) is confined
  to the Quoin side of the boundary.
- Whether the 22,575 lines of vendored `ix-spec-workflows` bundle under
  `skills/**/workflow-assets/dist/` belong in this programme's baseline at all,
  given that the same argument excludes the `corpus/` submodule. Until that
  ruling, the bundle is governed as first-party source and only the 139-line
  invariant shim is treated as hand-written logic.
- The prior npm package was removed at Stage 9 and, per the 2026-09-23
  amendment above, is restored as a repackaging of GitHub Release delivery
  rather than an independent build; the first tagged-release smoke of each
  channel remains a promotion/release operation.
- Whether `engineering-assurance` is a build dependency of this workspace or a
  reference only. This ADR's crate topology says reference;
  [FR-100](../../../spec/functional/FR-100-rust-evidence-measurement-change-assurance.md)
  and [FR-103](../../../spec/functional/FR-103-corpus-consolidation.md) make
  consuming it mandatory. The direction of that edge decides who can break whom.
