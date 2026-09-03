---
id: ARCH-SM-003
title: "Semantic-module ownership and subsystem boundaries"
status: proposed
requirements:
  - FR-047
  - FR-050
---

# Semantic-module ownership and subsystem boundaries

Ownership follows the component that defines a contract or controls an effect. Reading,
validating, distributing, generating, storing, and presenting the same semantic concept do not
make those components co-owners of its definition.

## Responsibility allocation

| Owner                | Owns                                                                                                                                 | Does not own                                                                                                     |
| -------------------- | ------------------------------------------------------------------------------------------------------------------------------------ | ---------------------------------------------------------------------------------------------------------------- |
| Quire                | parse, validate, extract, address, and byte-splice typed Markdown with module-provided contracts                                     | template rendering, cross-language generation, schema-package publication, registry sourcing, application policy |
| Quoin                | catalog discovery, locks, installation, update; authoring-contract discovery, skills, workflows, and evidence-oriented orchestration | parser semantics, compiler implementation, runtime persistence, consumer adapters                                |
| `filament-core-data` | semantic kernel, IR, compatibility rules, compiler, and emitters                                                                     | domain vocabulary content, module distribution, application persistence, UI, ORM, or migration policy            |
| Module repositories  | vocabulary, constraints, archetypes, skeletons, mappings, examples, and semantic versions                                            | shared compiler implementation or consumer persistence policy                                                    |
| Consumers            | application adapters, API and IPC projections, persistence mappings and migrations, runtime state, and UI presentation               | authority to publish a divergent shared definition under the same semantic identity/version                      |

A consumer may add a storage index, UI-only state, or an adapter field that is explicitly local.
It must not fork a shared semantic identity under the same package/type/version and present that
fork as the common contract.

## Dependency direction

```text
module definitions and mappings ──────> filament-core-data compiler contract
          │                                         │
          ├── catalog distribution via Quoin        ├── semantic packages
          │                                         └── schemas/profiles
          │
typed Markdown ─ parse/validate/extract via Quire ──> consumer adapters
semantic packages ──────────────────────────────────> consumer code
runtime stores ─────────────────────────────────────> reports and analytics
```

Quire and the compiler may consume compatible JSON Schema vocabulary and small kernel types,
but neither depends on a consumer UI, ORM, database migration framework, or transport framework.
Generated packages depend on their semantic source and compatible kernel; they do not depend
back on an application adapter.

## Validation levels and capability roles

Accepted Quire ADR-0011 remains governing:

| Level | Subject                    | Execution owner                                                    |
| ----- | -------------------------- | ------------------------------------------------------------------ |
| L0    | Quire and Quoin themselves | Their own repository CI.                                           |
| L1    | The specification corpus   | Deterministic Quire validators and Quoin analyses/evidence policy. |
| L2    | A consumer implementation  | Consumer CI always executes L2 work.                               |

The capability roles remain distinct:

- A **Validator** determines a fact about the specification alone.
- An **Advisor** emits a typed verification obligation.
- A **Generator** proposes a consumer-owned test skeleton or seed.
- An **Auditor** binds consumer evidence back to obligations and applies policy.

Quire owns deterministic spec-only parsing/validation/extraction and obligation facts. Quoin may
orchestrate, advise, generate proposed scaffolds, and audit evidence. Consumer CI always executes
L2 work; neither Quoin nor Quire takes over the consumer's test execution.

## Boundary rules for future work

1. A proposal states its data plane, capability role, authoritative input, output projection,
   and effect owner before choosing a repository.
2. Deterministic typed-Markdown syntax/structure belongs in Quire; module-owned vocabulary rules
   stay in modules.
3. Catalog sourcing, installation, locks, and authoring workflow remain in Quoin.
4. Shared IR, compatibility comparison, and cross-language emitters belong in
   `filament-core-data`.
5. Database, API, IPC, UI, migration, and application policy remain consumer responsibilities.
6. Generated output lands with a source digest and is consumer-owned after generation; silent
   overwrite is forbidden.
7. An ownership move requires a reviewed successor ADR. A dependency introduced by code does
   not silently rewrite this allocation.

## Explicit exclusions

Quire does not become a renderer, package publisher, or code generator. Quoin does not become a
Markdown parser, semantic compiler, application persistence layer, or general job executor.
`filament-core-data` does not become the module registry or runtime database. Modules do not
embed consumer frameworks. Consumers do not redefine common identities in isolation.

## Traceability

This record implements FR-047 and the ownership parts of FR-050. The reconciled source status is
recorded in the [external decision ledger](decision-ledger.md) and the local decision is stated in
[ADR-0002](adr/0002-preserve-quire-quoin-boundaries.md).
