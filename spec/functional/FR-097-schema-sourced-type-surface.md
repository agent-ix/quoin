---
id: FR-097
title: "Source every boundary type from one schema and refuse hand-written duplicates"
type: FR
relationships:
  - target: "ix://agent-ix/quoin/StR-009"
    type: "implements"
  - target: "ix://agent-ix/quoin/US-024"
    type: "implements"
  - target: "ix://agent-ix/quoin/FR-096"
    type: "requires"
---

# FR-097: Source every boundary type from one schema and refuse hand-written duplicates

## Description

Quoin SHALL derive every type that crosses the `quoin-core` boundary from one
declared schema source, SHALL record the generator identity and source-schema
digest of each generated artefact, and SHALL refuse a hand-written declaration
that duplicates a type the schema source publishes.

## Inputs

- The `filament-core-data`-published schema and the types it emits for Rust,
  TypeScript, Python and JSON Schema.
- Repository-owned `schemars` type definitions in `quoin-schemas` for boundary
  types `filament-core-data` does not publish.
- The generated TypeScript type surface at `src/core/types.ts`.

## Outputs

- One generated TypeScript boundary type surface with an asserted content
  digest.
- A generator provenance record per generated artefact, naming the generator
  identity, the generator version and the source-schema digest.
- A refusal naming the duplicated type and its publishing schema source.

## Behavior

- Quoin SHALL generate `src/core/types.ts` from the JSON Schema that
  `quoin-schemas` emits, and SHALL assert the generated file's content digest so
  a hand edit fails the build.
- Quoin SHALL record, for each generated artefact, the generator identity, the
  generator version and the digest of the source schema it was generated from.
- If a first-party TypeScript, Rust or Python declaration duplicates a type that
  `filament-core-data` publishes, then Quoin's type-surface gate SHALL refuse it
  and SHALL name the type and the publishing source.
- Quoin SHALL treat a generated artefact whose recorded source-schema digest does
  not match the current schema as stale and SHALL refuse to use it.
- Quoin SHALL treat a hand edit of a generated artefact as loss of its generated
  status, identified by the provenance record rather than by the artefact's
  directory name.
- Generation, provenance recording and staleness refusal SHALL be delivered at
  stage 0 under the `filament-core-data` Phase A gate; only the retirement of a
  hand-written duplicate is gated on Phase B.
- Retirement of a hand-written duplicate SHALL NOT begin before
  `filament-core-data` publishes the replacing package, and a retired duplicate
  SHALL be replaced by an import of the published type rather than by a second
  generated copy.
- Quoin SHALL consume published TypeScript types from `npm.ix`, published Python
  packages from the internal PyPI, and Rust crates by git-revision pin, and SHALL
  NOT consume them from crates.io or public npmjs.
- User-interface TypeScript and `filament-core-data`-published types and clients
  SHALL remain approved target-language code and SHALL NOT be reported as
  duplicates by this gate.

## Error Conditions

A generated artefact whose digest does not match its recorded provenance, a
source schema that cannot be read, a hand-written declaration duplicating a
published type, and a generated artefact whose source-schema digest is stale each
fail the type-surface gate and are never reported as passing.

## Constraints

| ID | Constraint | Type | Validation |
|----|-----------|------|------------|
| FR-097-CON-1 | Quoin SHALL NOT hand-write a boundary response type. | Design | Test |
| FR-097-CON-2 | Generated status SHALL NOT be inferred from a directory name. | Design | Test |
| FR-097-CON-3 | Quoin SHALL NOT create a second schema source for a type `filament-core-data` publishes. | Architecture | Inspection |

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-097-AC-1 | `make types` regenerates `src/core/types.ts` from `quoin-schemas` and the build fails when the committed file's digest does not match the regenerated one. | Test (TC-1612) |
| FR-097-AC-2 | Every generated artefact carries a provenance record naming generator identity, generator version and source-schema digest, and an artefact missing one fails the gate. | Test (TC-1613) |
| FR-097-AC-3 | A planted hand-written declaration of a `filament-core-data`-published type is refused, naming the type and the publishing source. | Test (TC-1614) |
| FR-097-AC-4 | A hand edit of a generated artefact fails the gate even though the artefact's path is unchanged. | Test (TC-1615) |
| FR-097-AC-5 | A generated artefact whose recorded source-schema digest is stale is refused rather than used. | Test (TC-1616) |
| FR-097-AC-6 | The gate reports no finding against user-interface TypeScript or against an imported `filament-core-data`-published type. | Test (TC-1617) |
| FR-097-AC-7 | `rust/Cargo.toml` and `package.json` declare no crates.io registry source and no public npmjs registry source for a first-party package. | Test (TC-1691) |

## Dependencies

- **Upstream**: [FR-096](./FR-096-versioned-rust-engine-boundary.md); the `filament-core-data` Phase A gate (#7) releases generation and provenance, and the Phase B gate (#11) releases duplicate retirement only.
- **Downstream**: [IT-003](../integration/IT-003-filament-core-data-published-types.md) exercises the published-type boundary; [FR-101](./FR-101-retire-replaced-executable-paths.md) removes the replaced declarations.
