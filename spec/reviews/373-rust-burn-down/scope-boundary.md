---
id: SR-157
title: "Scope-boundary review of the issue 373 Rust burn-down artefact batch"
type: SpecReview
analysis: scope-boundary
scope: "ADR-0003, StR-009, US-024, FR-096..FR-103, NFR-024..NFR-027, IT-003"
review_set: all
---

# Scope-boundary review of the issue 373 Rust burn-down artefact batch

## Summary

The batch allocates implementation language, not authority: ADR-0003 states that
it takes no ownership from Quire, `filament-core-data`, `engineering-assurance`,
ix-flow or any module repository, and the requirement set is internally
consistent with that claim on the boundaries it names — `quoin-quire` wraps
`quire-rs` rather than re-implementing it (FR-099-CON-2), the evidence store
stays Quoin's and takes no `engineering-assurance` retention dependency
(FR-100), `corpus/` file dispositions are refused as another repository's tree
(FR-103-CON-2, FR-103-AC-6), and IT-003 is the one place a cross-repository
contract is actually exercised rather than assumed.

Four boundaries are not allocated. The enforcement run that publishes the metric
is claimed by this programme (FR-100) and by quire-research LR08 (NFR-024
Dependencies) with no owner named, which is the reporting cross-over US-024
forbids. `engineering-assurance` is simultaneously "a reference, not a
dependency" in ADR-0003's crate topology and a mandatory supplier in FR-100 and
FR-103-AC-4. The ADR-0001/ADR-0002 collision rule — they allocate authority,
this allocates language — exists only in a `status: proposed` ADR: no
requirement in the batch cites ADR-0001, ADR-0002, issue #286, FR-047, FR-048 or
FR-070..FR-083, so FR-099 ports catalog, module and semantic behaviour whose
owning requirements it never names. And FR-103-CON-3 subordinates a Quoin gate
to the containment programme's matrix, coupling the two programmes the
separation rule exists to keep apart. Six further findings record unmarked
external dependencies, duplicated allocation and one inconsistent submodule
exclusion.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | The enforcement run has two claimants. FR-100 requires Quoin to write the first-party non-Rust line count into its evidence store "on each enforcement run" and NFR-024's Verification assigns the three-class classification and counts to that same run, while NFR-024's Dependencies name quire-research LR08 as the downstream that "emits the metric as a byproduct of the check it already performs". No artefact says which programme owns the run, its allow-set or the published figure, so containment and burn-down can each report the same measurement as its own — the cross-over US-024 and ADR-0003 explicitly forbid. | FR-100, FR-100-AC-7, NFR-024 Verification, NFR-024 Dependencies, US-024, ADR-0003 Consequences |
| FND-002 | high | `engineering-assurance` is allocated two contradictory roles. ADR-0003's crate topology states "`engineering-assurance` is a reference, not a dependency" and that the edge points from Engineering Assurance to Quoin, while FR-100 makes consuming its implementation mandatory before any local one may exist and FR-103-AC-4 requires corpus accounting to be "obtained from `engineering-assurance`, and no local corpus-accounting implementation exists in the workspace" — a hard build dependency, testable only as one. Either the topology is wrong or the two FRs are, and the direction of the dependency edge decides who can break whom. | ADR-0003 Crate topology, FR-100 Behavior, FR-100-CON-2, FR-103-AC-4, FR-103 Inputs |
| FND-003 | high | The rule that resolves boundary (b) lives only in the ADR. ADR-0003 says ADR-0002 governs who owns the behaviour and ADR-0003 governs what language expresses it, but that rule is carried by no requirement: no artefact in the batch cites ADR-0001, ADR-0002, issue #286, FR-047, FR-048 or FR-070..FR-083, and ADR-0003 is `status: proposed`. FR-099 therefore ports catalog, module, semantic and completeness behaviour with no stated subordination to the authority allocation that owns it. | ADR-0003 Context, FR-099, FR-047-AC-2, FR-047-AC-3, FR-048 |
| FND-004 | high | FR-103-CON-3 forbids Quoin to "open a corpus debt row that the containment matrix does not carry", making a Quoin gate conditional on an artefact held by quire-research #56, verified by Inspection only, with no location, format or contract named. This is the reverse coupling of the separation rule: the burn-down's scope becomes a function of the containment programme's matrix, and a row dropped there silently narrows this programme. | FR-103-CON-3, FR-101 Inputs, US-024 Context, NFR-024 Rationale |
| FND-005 | medium | FR-099 names only FR-007, FR-017 and FR-029 as the behaviour it preserves, yet it ports semantic manifest validation and completeness analysis, whose verdicts are owned by the #286 semantic-module contract requirements (FR-046, FR-047, FR-048, FR-070..FR-083). Those requirements are neither cited nor listed upstream, so a ported verdict can drift from its owning requirement with no criterion detecting it, and FR-047-AC-3's allocation of the semantic kernel to `filament-core-data` is never restated as a limit on `quoin-semantic`. | FR-099 Behavior, FR-099 Dependencies, FR-047-AC-2, FR-047-AC-3, FR-070 |
| FND-006 | medium | FR-097 admits two schema sources — `filament-core-data` for what it publishes, `quoin-schemas` for "boundary types `filament-core-data` does not publish" — but states no rule for deciding which source owns a newly introduced type, and no criterion detects the migration case where `filament-core-data` begins publishing a type `quoin-schemas` already owns. FR-097-CON-3 forbids the second source only after the fact. | FR-097 Inputs, FR-097-CON-3, FR-097-AC-3, IT-003 |
| FND-007 | medium | Canonical serialization is allocated twice. FR-100 lists canonical serialization among the capabilities that must be consumed from `engineering-assurance` before a local implementation exists, while FR-098-CON-2, FR-100-CON-4 and FR-100-AC-8 require canonicalization and digest to exist only in `quoin-store` and fail when a second implementation is found. An `engineering-assurance`-supplied serializer is by construction outside `quoin-store`, and NFR-025's digest identity depends on which one wins. | FR-100 Behavior, FR-100-CON-4, FR-100-AC-8, FR-098-CON-2, NFR-025 |
| FND-008 | medium | The `corpus/` submodule is excluded from NFR-024's scope ("Not applied to: paths inside the `corpus/` submodule, which belong to `agent-ix/qa-corpus`") but not from NFR-025's, which asserts zero changed bytes and a 100% replay population over "every accepted-corpus byte reachable from the governed corpus pins". NFR-025 therefore states a guarantee over a tree quoin does not own, with no statement of whether it is verified read-only or assumed. | NFR-025 Scope, NFR-025 Measurement, NFR-024 Scope, FR-103-CON-2 |
| FND-009 | medium | The classification of `skills/**/workflow-assets/**` as executable assertion logic is owned by two requirements — FR-099 (final bullet, FR-099-AC-9, with a recorded disposition per asset) and FR-101 (same bullet, no criterion) — while ADR-0003 defers the disposition itself to delivery stage 8. ix-flow, the external process that executes those assets and whose `specInvariants` decide flow outcomes, is named as the reason but is allocated no boundary, contract or post-port owner anywhere in the batch. | FR-099 Behavior, FR-099-AC-9, FR-101 Behavior, ADR-0003 Capability boundary, US-024 Context |
| FND-010 | medium | The quire-research LR03 capability-gap matrix is consumed as an input by FR-101 and as an upstream by NFR-024 with no location, format, contract test, or classification as assumed or guaranteed, yet FR-101-AC-1's disposition gate and NFR-024's successor-reference check both read against it. Every other cross-repository consumer in this batch is either exercised (IT-003) or refused (FR-103-CON-2); this one is neither. | FR-101 Inputs, FR-101-AC-1, NFR-024 Dependencies |
| FND-011 | low | FR-102 requires a dated owner disposition for `@agent-ix/filament-plan-sync`, a plugin published from another repository, without naming that repository, its maintainers, or any criterion that its consumers are enumerated before a drop disposition lands; FR-102-AC-3 checks only that a disposition exists. | FR-102 Behavior, FR-102-AC-3, FR-102-CON-2 |
| FND-012 | low | The containment/burn-down separation is stated in US-024 and ADR-0003 but not in StR-009, and only NFR-024-AC-3 enforces it, in one direction (no report of this programme records a retention as remediated). No criterion prevents this programme from reporting containment work as its own, which is the half FND-001 makes reachable. | StR-009, US-024 Context, NFR-024-AC-3, ADR-0003 Consequences |

## Boundary allocation

In scope for this programme: the implementation language of Quoin's first-party
engine, production, planning, validation, canonicalization, digest, oracle,
evidence and qualification behaviour; the `quoin-core` boundary protocol and its
generated type surface; the differential and digest-replay parity evidence; the
disposition of every first-party executable path in this repository; the
Quoin-side corpus accounting and selection; the Rust toolchain floor and gates.

Out of scope and retained by others: who owns a behaviour (ADR-0001, ADR-0002,
FR-047, FR-048, issue #286); Quire parsing, extraction and validation semantics
(`quire-rs`); the semantic kernel, IR, compiler, emitters and published packages
(`filament-core-data`); shared assurance capability
(`engineering-assurance`); the contents of `agent-ix/qa-corpus`; workflow-asset
execution (ix-flow); the containment programme's matrix and metric
(quire-research #56).

## External dependencies

| Dependency | Type | Assumed or Guaranteed | Contract |
| --- | --- | --- | --- |
| `quire-rs` | Cargo git-pin | Guaranteed | FR-099-AC-2 verdict differential over the corpus |
| `filament-core-data` published types | Cargo git-pin + `npm.ix` | Guaranteed | IT-003, gated on Phase A (#7) and Phase B (#11) |
| `engineering-assurance` shared capability | Library/package, direction disputed | Contested — see FND-002 | None; FR-103-AC-4 asserts consumption, no contract test |
| `agent-ix/qa-corpus` (`corpus/` submodule) | Git submodule, pinned | Assumed | FR-103-AC-7 revision pin; FR-103-AC-6 refuses dispositions inside it |
| quire-research LR03 capability-gap matrix | Document in another repository | Unmarked — see FND-010 | None |
| quire-research LR08 enforcement / #56 containment matrix | Programme artefact in another repository | Unmarked — see FND-001, FND-004 | None |
| ix-flow + `skills/**/workflow-assets/**` | Spawned process | Assumed | None; disposition deferred to delivery stage 8 |
| `@agent-ix/filament-plan-sync`, `command_not_found` hook | Published oclif extension | Assumed | FR-102-AC-3 disposition exists; FR-102-AC-4 resolution if retained |
| `@agent-ix/ix-cli-core` | npm package, being retired | Guaranteed | FR-099-AC-8: no dependency, nine named Rust replacements |
| `gix`, `ix-trace-rs` | Cargo crates | Assumed | None stated beyond the crate topology |
| `npm.ix`, internal PyPI (no crates.io, no public npmjs) | Registries | Guaranteed | FR-097-AC-7, FR-102-AC-7 by inspection of the manifests |

## Responsibility allocation

| Requirement | Owning Component | Class |
| --- | --- | --- |
| StR-009 | Quoin repository (programme) | cross-cutting |
| US-024 | Quoin repository (programme) | cross-cutting |
| FR-096 | `quoin-core` boundary and `src/core/exec.ts` caller | infrastructure |
| FR-097 | `quoin-schemas` and the type-surface gate | cross-cutting |
| FR-098 | `quoin-difftest` with `quoin-store` | cross-cutting |
| FR-099 | `quoin-quire`, `quoin-validators`, `quoin-semantic`, `quoin-completeness`, `quoin-config`, `quoin-modules`, `quoin-catalog` | core |
| FR-100 | `quoin-store`, `quoin-evidence`, `quoin-change-assurance`, `quoin-auditor`, `quoin-assurance`, `quoin-graph-analysis`, `quoin-measurement-*` | core |
| FR-101 | Programme governance: path inventory and `quoin-core lint.language` | cross-cutting |
| FR-102 | `quoin-cli` | infrastructure |
| FR-103 | `quoin-measurement-core` corpus accounting with programme governance | infrastructure |
| NFR-024 | Enforcement run and `.language-allowances.yaml` | cross-cutting |
| NFR-025 | `quoin-store` and the Quoin evidence store | infrastructure |
| NFR-026 | `rust/` Cargo workspace toolchain declaration | infrastructure |
| NFR-027 | `rust/` Cargo workspace lint, error and test gates | cross-cutting |
| IT-003 | `quoin-schemas` and the generated `src/core/types.ts` surface | cross-cutting |

FR-101's owner is a programme function rather than a crate because its subject is
the inventory of paths, not a runtime capability; FND-001 is the open question of
whether that function is this programme's or quire-research LR08's.
