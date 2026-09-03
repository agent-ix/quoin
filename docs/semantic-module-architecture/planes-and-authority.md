---
id: ARCH-SM-002
title: "Semantic data planes and concern-specific authority"
status: proposed
requirements:
  - FR-046
  - FR-048
---

# Semantic data planes and concern-specific authority

No format is universally canonical. Authority follows the concern. A source may be
authoritative for authored knowledge while a database is authoritative for a related runtime
occurrence and a Markdown report is a useful projection of that occurrence.

## Four data planes

### Meta plane

The Meta plane contains packages, semantic type definitions, mappings, profiles,
transformation definitions, compatibility rules, and version identities. Its typical
authority is the TypeSpec schema/package source (filament-core-data ADR-0005) plus explicit
package metadata; normalized or bundled JSON Schema, language types, wire descriptors, and
registry metadata are outputs or distribution forms.

### Definition plane

The Definition plane contains requirements, architecture, plans, policy, test definitions,
domain definitions, and other durable authored knowledge. Reviewed typed Markdown is the
normal authority in Quoin/Quire modules. Extracted JSON, graph records, search indexes,
embeddings, and rendered views are projections of that knowledge.

### Execution and observation plane

The Execution and observation plane contains runs, events, results, evidence, incidents,
measurements, and other occurrences. The Owning run, evidence, or event store is authoritative
for these observations; a high-rate operational occurrence does not become authored Markdown
merely because humans and LLMs benefit from a text view.

### Presentation plane

The Presentation plane contains documents, reports, dashboards, UI views, tables, and exports.
It is normally derived. A Markdown report is a presentation projection of the referenced
observations unless it is separately declared and reviewed as a new authored artifact.

## Definitions, occurrences, and presentations

A concept has one primary plane for a given object, but its semantic identity is not the plane.
For example:

| Object                                  | Plane                     | Authority                                                                |
| --------------------------------------- | ------------------------- | ------------------------------------------------------------------------ |
| A `TestCase` definition                 | Definition                | Reviewed typed Markdown and its module contract.                         |
| One `TestExecution`                     | Execution and observation | The owning run/evidence store.                                           |
| A run report summarizing that execution | Presentation              | Derived from the run and its evidence, with declared selection and loss. |

These are linked objects, not three interchangeable encodings. Editing the run report does not
mutate the `TestExecution`; changing an execution row does not revise the `TestCase` definition.

Structural kind and semantic role are independent. A record-shaped value may be a definition,
entity, event, command, observation, evidence item, or projection. Generator and parser logic
must not infer domain meaning from `record`, `enum`, a heading, or a table layout alone.

Explicit non-goal: No universal `SemanticObject` runtime envelope. A small shared kernel may
carry semantic references, provenance, time, typed outcomes, support/loss state, and namespaced
extensions. Domain types remain in their owning modules and need not inherit one base entity.

## Authority matrix

| Concern                                | Authoritative source                                    | Derived representations                                                                  | Edit direction                                          | Required provenance                                                               |
| -------------------------------------- | ------------------------------------------------------- | ---------------------------------------------------------------------------------------- | ------------------------------------------------------- | --------------------------------------------------------------------------------- |
| Human/agent-authored durable knowledge | Reviewed typed Markdown plus the owning module contract | extracted JSON, graph records, search indexes, embeddings, and rendered views            | Edit and review Markdown; re-extract                    | artifact identity/version, module/archetype version, extraction tool identity     |
| Shared schema/package definitions      | Accepted schema/package source plus package metadata    | normalized/bundled JSON Schema, generated Rust, TypeScript, and Python, wire descriptors | Edit the accepted source/metadata; regenerate           | package/type identity, semantic version, compiler/emitter identity, source digest |
| Transactional application state        | Owning PostgreSQL database or event store               | APIs, reports, analytical tables, Markdown snapshots                                     | Write through the owning application transaction        | tenant/aggregate identity, schema version, transaction/event position             |
| Operational observations               | Owning run, evidence, or event store                    | reports, dashboards, alerts, Markdown summaries, analytical datasets                     | Append or correct through owning-system policy          | run/source/time/tool/config identities and evidence digest                        |
| Interface payload conformance          | Versioned interface schema package                      | Protobuf, JSON, and Avro payloads and clients                                            | Change the interface package under compatibility policy | package/version, message type, codec/profile                                      |
| Analytical data                        | Source stores remain authoritative                      | Arrow and Parquet batches; CSV and TSV exports                                           | Recompute through a declared query/mapping              | source identities/versions, query/mapping, time, declared loss                    |
| Generated language code                | Never independently authoritative                       | compiled libraries and consumer adapters                                                 | Regenerate; do not hand-author contract drift           | source/package/compiler/emitter digests and versions                              |
| Rendered reports/UI                    | Normally never independently authoritative              | Markdown, HTML, PDF, React views                                                         | Change source or declared presentation mapping          | source/run identities, profile, time, omissions and aggregation                   |

The generated Rust, TypeScript, and Python packages are reproducible outputs and never
independent authorities. Their native ergonomics do not transfer semantic ownership from the
package source or its module owner.

## Representation selection

Representation is selected by boundary, not by global preference:

- Protobuf is a good compact, strongly typed service/IPC projection when schema evolution and
  generated clients fit the boundary.
- JSON is the portable interchange, validation, debugging, and LLM/tool integration projection.
- Avro remains the current implemented shared compatibility contract where it already exists;
  this record neither retires nor regenerates it.
- Arrow and Parquet are analytical projections optimized for columns, batches, and retained
  datasets, not transactional or authored authorities.
- CSV and TSV are deliberately limited export projections and require declared loss, null,
  ordering, and type conventions.
- Markdown is the preferred authored-knowledge and human/LLM presentation form, not a high-rate
  occurrence store or compact wire.

Each transformation declares byte-exact, structural, semantic, or projected equivalence.
Projected equivalence must name declared loss, enrichment, selection, and provenance.

## Conflict rule

If two sources appear authoritative for one concern, promotion stops. The owner must record
whether one is a projection, whether the concerns are distinct, or whether a reviewed ADR and
migration are required. Last-writer-wins is rejected as an authority policy because it hides the
ownership error instead of resolving it.

## Traceability

This record implements FR-046 and FR-048 and specializes `filament-core-data` ARCH-003 and
ARCH-005. The exact source identities and decision status are in the
[external decision ledger](decision-ledger.md).
