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

## Constraints

| ID | Constraint | Type | Validation |
|----|------------|------|------------|
| FR-029-CON-1 | quoin SHALL NOT restate the published schemas as hand-written validators. The duplication is the failure being closed; using the linked engine's schema and types is the whole mechanism. | Architecture | Test |
| FR-029-CON-2 | quoin SHALL NOT fetch a schema at runtime. Every command path stays network-free. | Architecture | Inspection |
| FR-029-CON-3 | The schemas quoin validates against SHALL mirror the publisher's open/closed decision for every vocabulary rather than second-guessing it. Closing `diagnostics[].reason` would reject a newer engine's payload that a consumer could otherwise read. | Architecture | Test |

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-029-AC-1 | Withdrawn: required each schema copy to hash to a recorded value. quoin keeps no copy, so nothing remains to check (id not reused). | Withdrawn (TC-110) |
| FR-029-AC-2 | A conformant coverage payload and a conformant properties payload each validate. | Test (TC-111) |
| FR-029-AC-3 | A payload missing a required key, carrying an added field, or using a value outside a closed engine enum is rejected, and the report names the failing path. | Test (TC-112) |
| FR-029-AC-4 | Unreadable JSON is reported as a named violation rather than a panic. | Test (TC-113) |
| FR-029-AC-5 | Withdrawn: a minimum-version premise on a `quire` subprocess. Quoin links the engine in process; there is no version to read or compare (id not reused). | Withdrawn (TC-114) |
| FR-029-AC-6 | Withdrawn with AC-5: no version comparison remains (id not reused). | Withdrawn (TC-115) |
| FR-029-AC-7 | Withdrawn: described the former TypeScript validators' optional-key fixtures (id not reused). | Withdrawn (TC-116) |
| FR-029-AC-8 | Withdrawn earlier; id not reused. | Withdrawn |
| FR-029-AC-9 | Withdrawn: described payloads emitted by a selected `QUIRE` binary against TypeScript validators; the engine is linked in process (id not reused). | Withdrawn (TC-118) |
| FR-029-AC-10 | Withdrawn: surfaced a subprocess's stderr; no subprocess exists (id not reused). | Withdrawn (TC-134, TC-256) |
| FR-029-AC-11 | Withdrawn: a subprocess `maxBuffer`; no subprocess exists (id not reused). | Withdrawn (TC-254) |
| FR-029-AC-12 | Withdrawn: subprocess death causes; no subprocess exists (id not reused). | Withdrawn (TC-254, TC-255, TC-257) |
| FR-029-AC-13 | Withdrawn: TypeScript interfaces against the coverage schema; the types are the engine's own Rust types (id not reused). | Withdrawn (TC-272) |

## Dependencies

- **Upstream**: quire-rs [FR-055](ix://agent-ix/quire-rs/FR-055) (the published schemas), [FR-053](ix://agent-ix/quire-rs/FR-053) (the obligation records the payloads carry)
- **Downstream**: the `quoin evidence` verbs, which are the first production consumer of these models
