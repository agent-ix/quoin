---
id: FR-029
title: "Consume the published quire JSON contract"
type: FR
relationships:
  - target: "ix://agent-ix/quire-rs/FR-058"
    type: "traces_to"
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
- When the linked engine reports that traceability declarations of the selected
  modules failed to load, whether through a coverage error or through a
  relation-report error, `quoin-quire` SHALL return the typed error
  `Error::TraceabilityDeclarationsDropped` with the stable code `QQ-1022`.
- `Error::TraceabilityDeclarationsDropped` SHALL carry every dropped declaration
  as a typed `(module, kind, item)` record in the engine's order, with `kind`
  as the engine's kind token.
- `Error::TraceabilityDeclarationsDropped` SHALL render each dropped declaration
  on its own line of its message.
- When a whole `traceability:` section cannot be read, `quoin-quire` SHALL
  report it as the record with kind `traceability_section` and item
  `traceability`, exactly as the engine records it.
- `QQ-1022` SHALL map to the exit-taxonomy class `Refused`, the class of
  `QQ-1020` (`TraceabilityModelUndeclared`).
- The envelope for `QQ-1022` SHALL carry the context key `quire_code` with the
  value `QQ-1022`.
- The envelope for `QQ-1022` SHALL carry the context key `dropped_declarations`,
  whose value is one line per dropped declaration in the engine's order, each
  line written `module=<module> kind=<kind> item=<item>`.
- `quoin-quire` SHALL map the engine's coverage errors with an exhaustive
  match, so that a new engine variant fails the build.
- `quoin-quire` SHALL map both engine drop errors, the coverage error and the
  relation-report error, to `QQ-1022`, and SHALL NOT map either to the
  design-join code `QQ-1110`.
- If any declaration was dropped, then every operation or command that computes
  over the merged traceability model SHALL refuse with `QQ-1022` before
  computing and SHALL emit no report, matrix, export, record or dossier.
- If any declaration was dropped, then a command SHALL report `QQ-1022`, not the
  absent-model code `QQ-1020`, including when every declaration was dropped and
  the merged model is empty.
- A command that refuses with `QQ-1022` SHALL exit with the `Refused` exit
  status, SHALL write the code `QQ-1022` and each dropped declaration to stderr,
  and SHALL write nothing to stdout.
- Each code path that reads the merged model itself (the trace-tag walk behind
  `quoin-suite-trace`, `quoin formal-rules status`, the formal-rules declaration
  read behind `quoin assurance`, and the `quoin-quire` evidence compute, which
  re-reads the model after the shared walk) SHALL ask the engine whether
  declarations were dropped before it reads the model, and SHALL do so before
  any absent-model check.
- If any declaration was dropped, then `quoin_quire::assurance::build` SHALL
  refuse with `QQ-1022`.
- The commands governed by the refusal are those that reach the merged model
  through `quire.coverage`, `quire.design_matrix`, `quire.design_join`, the
  flow-report compute, the evidence compute, the assurance export or the
  trace-tag walk. At the time of writing they are `quoin matrix`, `quoin advise`,
  `quoin anomaly check`, `quoin evidence record`, `quoin evidence audit`,
  `quoin class obligations`, `quoin soup promote --discharges`,
  `quoin release snapshot`, `quoin assurance`, `quoin formal-rules coverage`,
  `quoin formal-rules status`, `quoin dossier generate`, `check` and `export`,
  and `quoin-suite-trace`; the list is illustrative, the rule decides. `quoin
  completeness` and `quoin validate` do not read the model and are not governed.

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
| FR-029-AC-15 | A coverage run over modules where one declaration fails to load returns `TraceabilityDeclarationsDropped` with code `QQ-1022`, not `QQ-1020`, carrying one `(module, kind, item)` record per dropped declaration. | Test |
| FR-029-AC-16 | The message of the error renders every dropped declaration on its own line with its module, kind and item. | Test |
| FR-029-AC-17 | A module whose whole `traceability:` section cannot be read yields the record with kind `traceability_section` and item `traceability`. | Test |
| FR-029-AC-18 | `QQ-1022` maps to `Refused`, and its envelope carries `quire_code` `QQ-1022` and a `dropped_declarations` value with one `module=<module> kind=<kind> item=<item>` line per dropped declaration in engine order. | Test |
| FR-029-AC-19 | A dropped declaration reported by the engine's relation-report error maps to `QQ-1022` through `quire.design_join` and the flow-report compute, and never to `QQ-1110`. | Test |
| FR-029-AC-20 | A scope where every declaration is dropped, so the merged model is empty, is refused with `QQ-1022` by each of the coverage, evidence, flow-report and assurance-export computes and by `quoin formal-rules status`, never with `QQ-1020`. | Test |
| FR-029-AC-21 | For each command named in the Behavior list, a scope with a dropped declaration exits with the `Refused` status, writes `QQ-1022` and each dropped declaration to stderr, writes nothing to stdout and writes no output document. | Test |
| FR-029-AC-22 | The trace-tag walk behind `quoin-suite-trace`, `quoin formal-rules status`, the formal-rules declaration read behind `quoin assurance`, the evidence compute and `quoin_quire::assurance::build` each refuse with `QQ-1022` on a scope with a dropped declaration. | Test |
| FR-029-AC-23 | For each command named in the Behavior list, the same scope without a dropped declaration exits 0 and produces that command's normal output: its report, matrix, export, record or dossier. | Test |

## Dependencies

- **Upstream**: quire-rs [FR-055](ix://agent-ix/quire-rs/FR-055) (the published schemas), quire-rs [FR-058](ix://agent-ix/quire-rs/FR-058) (the dropped-declaration refusal, AC-47), [FR-053](ix://agent-ix/quire-rs/FR-053) (the obligation records the payloads carry)
- **Downstream**: the `quoin evidence` verbs, which are the first production consumer of these models
