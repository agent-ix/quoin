---
id: IT-003
title: "Quoin consumes filament-core-data published types across Rust and TypeScript"
type: IT
relationships:
  - target: "ix://agent-ix/quoin/FR-097"
    type: "verifies"
  - target: "ix://agent-ix/quoin/FR-096"
    type: "verifies"
---

# IT-003: Quoin consumes filament-core-data published types across Rust and TypeScript

## Objective

Verify the real integration boundary between Quoin and `filament-core-data` as
the multi-language schema source of truth: a type defined once in
`filament-core-data` is consumed by the Quoin Rust workspace through a
git-revision pin and by the retained TypeScript through the `npm.ix` registry,
both forms describe the same schema, and a document that crosses the `quoin-core`
boundary round-trips through both without a hand-written duplicate declaration.
This exercises the published-package boundary that the unit suite covers only
with locally generated fixtures.

## Target Integration

The component under test is Quoin's schema-sourced type surface — the
`quoin-schemas` crate and the generated `src/core/types.ts`. The external
dependency is `filament-core-data`'s published packages: the Rust crate consumed
by git-revision pin and the TypeScript package consumed from `npm.ix`. The
integration exercised is dependency resolution against those registries followed
by a round trip of one shared type across the `quoin-core` boundary.

## Preconditions

`filament-core-data` has passed its Phase A cross-language compatibility gate
(`filament-core-data` #7) and closed its Phase B publication gate
(`filament-core-data` #11), so generated packages are actually published. An
isolated build environment with no pre-populated Cargo or npm cache, credentials
for `npm.ix`, and read access to the `filament-core-data` repository at the
pinned revision. No crates.io and no public npmjs registry is configured as a
source for a first-party package.

## Inputs

One type that `filament-core-data` publishes and that crosses the `quoin-core`
boundary, together with one instance document of that type drawn from
`tests/fixtures`, and the pinned `filament-core-data` git revision and published
package version recorded in `rust/Cargo.toml` and `package.json`.

## Test Procedure

Each step performs one discrete action and has its own success criterion.

1. Resolve dependencies in the clean environment with
   `cargo fetch --manifest-path rust/Cargo.toml` and `pnpm install --frozen-lockfile`.
   - IT-003-SC-01: both resolve the `filament-core-data` packages, the Rust one
     from the recorded git revision and the TypeScript one from `npm.ix`, and
     neither resolver contacts crates.io or public npmjs for a first-party
     package.
2. Compare the schema each published form describes for the selected type.
   - IT-003-SC-02: the Rust crate's schema and the TypeScript package's schema
     agree on field names, required fields and types, and both carry the same
     source-schema digest in their provenance record.
3. Run the type-surface gate over the repository.
   - IT-003-SC-03: the gate reports no hand-written first-party declaration of
     the selected type, and the generated `src/core/types.ts` matches its
     asserted digest.
4. Send the instance document through the `quoin-core` boundary operation that
   carries the selected type and read the result back in TypeScript.
   - IT-003-SC-04: the document round-trips with a byte-identical canonical
     serialization and no field is lost or renamed.
5. Plant a hand-written TypeScript declaration duplicating the selected type and
   re-run the type-surface gate.
   - IT-003-SC-05: the gate refuses, naming the duplicated type and
     `filament-core-data` as its publishing source.

## Expected Results

Quoin resolves the `filament-core-data`-published type from the internal
registries only, the Rust and TypeScript forms describe the same schema with the
same recorded provenance, the generated type surface matches its asserted digest,
an instance document round-trips across the boundary byte-identically, and a
planted duplicate declaration is refused. The test passes only when every
per-step success criterion holds.

## Metadata

- Priority: High
- Test Case: TC-1709
- Target Integration: `filament-core-data` published Rust crate and `npm.ix` TypeScript package
- Automation: Automated (live registry integration; gated on `filament-core-data` Phase B — see Notes)

## Dependencies

**Upstream**: [FR-097](../functional/FR-097-schema-sourced-type-surface.md) and
[FR-096](../functional/FR-096-versioned-rust-engine-boundary.md), which this
verifies; the `filament-core-data` Phase A and Phase B gates recorded in
[ADR-0003](../../docs/semantic-module-architecture/adr/0003-rust-native-quoin-engine-boundary.md).
**Downstream**: the hand-written-duplicate retirement clauses of
[FR-097](../functional/FR-097-schema-sourced-type-surface.md), which may not
begin before this boundary holds; the rest of
[FR-101](../functional/FR-101-retire-replaced-executable-paths.md) is released by
Phase A and is not gated on this test.

## Notes

As at 2026-09-12 `filament-core-data` publishes nothing: all six of its Rust
crates declare `publish = false`, its release workflows ship the legacy Avro
contract rather than the generated packages, and codegen is local-only in all
three languages. This integration test therefore defines coverage that cannot run
until Phase B closes, and is recorded as spec-ahead-of-code in the same manner as
[IT-001](./IT-001-default-module-reconcile.md) and
[IT-002](./IT-002-github-plugin-install.md). Until then the type surface is
exercised against locally generated schemas, which proves generation but not
publication.

## Traceability

This integration test verifies the schema-sourced type surface and the boundary
type rules, and exercises the stakeholder need for one schema source recorded in
[StR-009](../stakeholder/StR-009-one-implementation-language-for-engine-logic.md).
