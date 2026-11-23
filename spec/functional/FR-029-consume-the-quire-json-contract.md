---
id: FR-029
title: "Consume the published quire JSON contract"
type: FR
relationships:
  - target: "ix://agent-ix/quoin/StR-004"
    type: "traces_to"
  - target: "ix://agent-ix/quire-rs/FR-055"
    type: "traces_to"
  - target: "ix://agent-ix/quire-rs/FR-053"
    type: "traces_to"
---

# FR-029: Consume the published quire JSON contract

## Description

`quoin` SHALL validate every quire payload it reads against the schema and
types of the `quire-rs` engine it links (`quoin-quire`, at the version
`Cargo.lock` records), and SHALL return a result naming every failing path
rather than panicking.

Quoin links `quire-rs` in process. It spawns no `quire` subprocess, reads no
`quire --version`, records no minimum version and compares none: the locked
crate is the whole provenance, and Cargo already refuses a mismatched build.
For the `assurance-v1` export it validates against the schema the crate
exposes (`quire_rs::assurance::ASSURANCE_V1_SCHEMA`); coverage, properties and
other payloads are read into the engine's own types. Quoin keeps no copy of a
schema, reads none from the network, and hashes none.

## Inputs

- A coverage, properties or assurance payload produced by the linked engine,
  or supplied as JSON text.

## Outputs

- A validated, typed payload, or a violation naming every failing path.

## Behavior

- The parse functions in `quoin-quire` SHALL parse and validate in one call, so
  no caller holds an unvalidated payload, and SHALL return a violation on a JSON
  parse failure rather than panicking.

- When the linked engine reports that a traceability declaration of the
  selected modules failed to load (quire-rs
  [FR-058](ix://agent-ix/quire-rs/FR-058) AC-47), `quoin-quire` SHALL return the
  typed error `Error::TraceabilityDeclarationsDropped` with the stable code
  `QQ-1022`. The error SHALL carry every dropped declaration as a typed
  `(module, kind, item)` record, in the engine's order, and SHALL render each one
  on its own line in its message. `quoin-quire` SHALL name a whole unreadable
  `traceability:` section with the item `traceability_section`.
- `QQ-1022` SHALL map to the same exit-taxonomy class as `QQ-1020`
  (`TraceabilityModelUndeclared`), `Refused`, and its envelope SHALL carry
  `quire_code` `QQ-1022` and the dropped declarations in its context. The engine's
  coverage error is mapped by an exhaustive match, so a new engine variant fails
  the build instead of falling into a catch-all.
- Every quoin command that computes over the merged traceability model SHALL
  refuse with `QQ-1022` when any declaration was dropped, before computing and
  without emitting a report, matrix, export or dossier. These commands are:
  the `quire.coverage`, `quire.design_matrix` and `quire.design_join` core
  operations; `quoin matrix`; `quoin assurance`; `quoin formal-rules coverage`;
  `quoin formal-rules status`; `quoin dossier generate`, `check` and `export`;
  and the `quoin-suite-trace` annotator. A drop never degrades into the absent
  model refusal `QQ-1020`: a model that has declarations, some dropped, is a
  different condition from a model nobody declared.

## Constraints

| ID | Constraint | Type | Validation |
|----|------------|------|------------|
| FR-029-CON-1 | quoin SHALL NOT restate the published schemas as hand-written validators. The duplication is the failure being closed; using the linked engine's schema and types is the whole mechanism. | Architecture | Test |
| FR-029-CON-2 | quoin SHALL NOT fetch a schema at runtime. Every command path stays network-free. | Architecture | Inspection |
| FR-029-CON-3 | The schemas quoin validates against SHALL mirror the publisher's open/closed decision for every vocabulary rather than second-guessing it. Closing `diagnostics[].reason` would reject a newer engine's payload that a consumer could otherwise read. | Architecture | Test |

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-029-AC-1 | Withdrawn: required each schema copy to hash to a recorded value. quoin keeps no copy, so nothing remains to check (id not reused). | Withdrawn |
| FR-029-AC-2 | A conformant coverage payload and a conformant properties payload each validate. | Test |
| FR-029-AC-3 | A payload missing a required key, carrying an added field, or using a value outside a closed engine enum is rejected, and the report names the failing path. | Test |
| FR-029-AC-4 | Unreadable JSON is reported as a named violation rather than a panic. | Test |
| FR-029-AC-5 | Withdrawn: a minimum-version premise on a `quire` subprocess. Quoin links the engine in process; there is no version to read or compare (id not reused). | Withdrawn |
| FR-029-AC-6 | Withdrawn with AC-5: no version comparison remains (id not reused). | Withdrawn |
| FR-029-AC-7 | Withdrawn: described the former TypeScript validators' optional-key fixtures (id not reused). | Withdrawn |
| FR-029-AC-8 | Withdrawn earlier; id not reused. | Withdrawn |
| FR-029-AC-9 | Withdrawn: described payloads emitted by a selected `QUIRE` binary against TypeScript validators; the engine is linked in process (id not reused). | Withdrawn |
| FR-029-AC-10 | Withdrawn: surfaced a subprocess's stderr; no subprocess exists (id not reused). | Withdrawn |
| FR-029-AC-11 | Withdrawn: a subprocess `maxBuffer`; no subprocess exists (id not reused). | Withdrawn |
| FR-029-AC-12 | Withdrawn: subprocess death causes; no subprocess exists (id not reused). | Withdrawn |
| FR-029-AC-13 | Withdrawn: TypeScript interfaces against the coverage schema; the types are the engine's own Rust types (id not reused). | Withdrawn |
| FR-029-AC-14 | A coverage run over modules where one declaration fails to load returns `TraceabilityDeclarationsDropped` with code `QQ-1022`, not `QQ-1020`, carrying one `(module, kind, item)` record per dropped declaration. | Test |
| FR-029-AC-15 | The error message renders every dropped declaration, one per line, naming its module, kind and item; an unreadable `traceability:` section appears with the item `traceability_section`. | Test |
| FR-029-AC-16 | `QQ-1022` maps to `Refused`, the class of `QQ-1020`, and the envelope context holds `quire_code` `QQ-1022` and each dropped declaration. | Test |
| FR-029-AC-17 | For each of `quire.coverage`, `quire.design_matrix`, `quire.design_join`, `quoin matrix`, `quoin assurance`, `quoin formal-rules coverage`, `quoin formal-rules status`, `quoin dossier generate`, `quoin dossier check`, `quoin dossier export` and `quoin-suite-trace`, a scope with a dropped declaration is refused with `QQ-1022` naming each drop, and no output document is written. | Test |
| FR-029-AC-18 | The same scope with no dropped declaration runs every command in AC-17 without `QQ-1022`. | Test |

## Dependencies

- **Upstream**: quire-rs [FR-055](ix://agent-ix/quire-rs/FR-055) (the published schemas), [FR-053](ix://agent-ix/quire-rs/FR-053) (the obligation records the payloads carry)
- **Downstream**: the `quoin evidence` verbs, which are the first production consumer of these models
