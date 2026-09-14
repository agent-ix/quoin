<!--
SPDX-License-Identifier: AGPL-3.0-or-later
Copyright (C) 2026 Agent-IX
-->

# `quoin-quire` vs the retained TypeScript

What this crate promises against `src/quire/`, and what it deliberately does
not. Written for quoin#474, which added `schema::validate_assurance` — the Rust
successor to `validateAssurance` (`src/quire/validate.ts:99`).

Nothing in `src/quire/` was edited. `src/quire/validate.ts` remains the oracle;
this is a port, not a cutover, and no TypeScript was deleted.

## 1. Verdicts are contractual. Diagnostics are not.

`validate_assurance` and `validateAssurance` return the same **verdict** for the
same document. That is the whole contract.

Three things are explicitly **not** promised, and quoin#403 asked about the
third:

- the **text** of a violation,
- the **number** of violations,
- the **order** of violations.

ajv with `allErrors: true` and the `jsonschema` crate walk a failing document
differently, and both orders are incidental to their implementations rather than
stated by Draft 2020-12. Promising an order would be promising a property
neither library documents. No test in this crate asserts any of the three —
`tests/tc_474_assurance_parity.rs` reads only `oracle_valid`.

`Error::AssuranceContract` still carries one line per failing instance path in
`<path>: <reason>` form, because that is the shape `ContractViolation.errors`
had and a caller rendering it should not have to change. Its content is a
diagnostic aid, not an interface.

## 2. `format` is annotated, never asserted

`src/quire/validate.ts:99` builds `new Ajv2020({ allErrors: true, strict:
false })` and registers **no** format checks. `assurance-v1` declares one
`format` — `uuid`, on `artifact.uuid` — and under that ajv it is an annotation:
ajv logs _unknown format "uuid" ignored in schema at path "#/properties/uuid"_
and accepts `"not-a-uuid"`.

So this crate compiles with `SchemaValidator::compile_vendored`, which registers
nothing and asserts nothing — **not** `compile_with_formats`, which the two
_measurement_ schemas need because their ajv instances each register `date-time`
(see `quoin-jsonschema/DIVERGENCE.md` §4). Reaching for the asserting
constructor because the schema _has_ a format would refuse documents the oracle
accepts. That is a verdict difference, which is a defect and not a nuance.

The choice is measured from both sides in
`tests/tc_474_assurance_schema.rs::tc_474_012`: the same document with
`uuid: "not-a-uuid"` is accepted by `validate_assurance`, and refused by a
build of the same vendored schema with a `uuid` check registered. An inversion
of the constructor turns one of the two assertions red.

## 3. `ValidAssuranceDocument` is not `quoin_jsonschema::ValidDocument`

The **validator** is shared: `quoin-jsonschema` is the one ajv-parity JSON
Schema validator in this workspace (quoin#470) and this crate calls it. The
`jsonschema` pin lives in that crate's manifest and nowhere else.

Its `ValidDocument` is not reused, because it is indexed by `VendoredSchema` —
the closed enum of the two **measurement** documents, whose `compile()`
hardcodes a `date-time` `FormatCheck`. `assurance-v1` is a quire _output_
contract owned by this crate; widening that enum would file it under the
measurement crate's namespace and hand it exactly the format policy §2 says it
must not have. Only the ~15-line proof token is local.

## 4. Declared verdict divergences: none

`tests/tc_474_assurance_parity.rs::DECLARED_DIVERGENCES` is **empty**, and that
is a measured result rather than an absence of measurement: an undeclared
disagreement fails the test, and a declared one that stopped disagreeing fails
it too.

The corpus is 217 documents — two base exports produced by the linked engine
over the committed fixture corpora, each put through a fixed named mutation
ladder — of which ajv accepts 28 and refuses 189. Both bases carry both
verdicts. The ladder deliberately includes every arm of the only lookahead in
the document (`locator.path`'s `^(?!/)(?!.*(?:^|/)\.\.(?:/|$)).+$`), including
three arms that must be **accepted**, so a regex that refused everything could
not pass as agreement.

## 5. `validate_assurance` is the weaker of the two readers

A caller that can state its premises wants `assurance::read`, which is
`quire_rs::read_assurance_export` — fail-closed, and it also checks the caller's
module set and schema digests. `validate_assurance` answers the one question a
caller with no premises can ask: _is this the published shape?_ That caller is
`src/measurement/graph-adapters.ts` and `src/graph-analysis/load.ts`, reading
exports that arrived from other repositories, and it is why one vendored schema
survived here when the other four did not (see `src/payload.rs`).
