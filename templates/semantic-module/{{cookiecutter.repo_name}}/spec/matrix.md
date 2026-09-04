---
id: TM-001
title: "{{ cookiecutter.repo_name }} Test Matrix"
type: TestMatrix
---

# {{ cookiecutter.repo_name }} Test Matrix

Tests live in `tests/` and run under pytest (`make test`). Three modules carry
them: `test_manifest_semantic.py` (facts about the committed bytes),
`test_schema_emission.py` (the emit pipeline and the drift gate), and
`test_skeletons_semantic.py` (the authoring forms, through the engine).

Nothing here skips. When the engine, the grammar package or the schema toolchain
is absent, the rows that need it **fail** and name the command that installs it.
A skipped row would report green for a check that did not run.

## Status vocabulary

`✅` complete · `❌` failed · `🚧` in progress, with the reason after the marker ·
`⛔` retired. The source is `spec_artifacts_process/manifest.yaml`. `⚠️` is not a
marker: it was admitted by an older contract, classed by the traceability model
as nothing, and every row that carried it was exempt from the status-lie check by
construction.

## Test Case Summary

| Test ID | Title | Type | Priority | Traces To | Status |
| --- | --- | --- | --- | --- | --- |
| TC-001 | The semantic block carries exactly the nine admitted keys | Unit | P0 | FR-001-AC-1 | ✅ |
| TC-002 | Contract version, posture and legacy_forms are the declared defaults, and sweep_report is absent | Unit | P0 | FR-001-AC-2 | ✅ |
| TC-003 | semantic.exports and the declared type names are the same set | Unit | P0 | FR-001-AC-3 | ✅ |
| TC-004 | No type name is declared twice | Property | P0 | FR-001-AC-4 | ✅ |
| TC-005 | Every exported type references its schema as a schema-and-digest pair | Unit | P0 | FR-001-AC-5 | ✅ |
| TC-006 | No exported type carries the placeholder object contract | Static | P0 | FR-001-AC-6 | ✅ |
| TC-007 | Every digest equals the SHA-256 of the file it names | Property | P0 | FR-001-AC-7 | ✅ |
| TC-008 | Every imports entry is pinned to an exact version | Unit | P1 | FR-001-AC-8 | ✅ |
| TC-009 | The manifest keeps its comments, proving a textual digest rewrite | Unit | P1 | FR-001-AC-9 | ✅ |
| TC-010 | One schema is emitted for every exported type | Unit | P0 | FR-002-AC-1 | ✅ |
| TC-011 | No emitted schema is an empty object contract | Static | P0 | FR-002-AC-2 | ✅ |
| TC-012 | Every reference in every emitted schema is absolute | Property | P0 | FR-002-AC-3 | ✅ |
| TC-013 | toolchain.json records what produced the bytes | Unit | P1 | FR-002-AC-4 | ✅ |
| TC-014 | Check mode is green against the committed output | Integration | P0 | FR-002-AC-5 | ✅ |
| TC-015 | Check mode is red, naming the file, after one emitted byte changes | Integration | P0 | FR-002-AC-6 | ✅ |
| TC-016 | The package metadata declares no dependency on the engine | Static | P0 | FR-002-AC-7 | ✅ |
| TC-017 | No .npmrc exists at any depth | Static | P0 | FR-002-AC-8 | ✅ |
| TC-018 | Every exported type has a typed-table skeleton with at least one row | Property | P0 | FR-003-AC-1 | ✅ |
| TC-019 | Every sysml alternate declares the same fields as its table | Property | P0 | FR-003-AC-2 | ✅ |
| TC-020 | Every skeleton carries a distinct-id ocl clause under its own heading | Property | P0 | FR-003-AC-3 | ✅ |
| TC-021 | No skeleton body carries a placeholder token | Static | P1 | FR-003-AC-4 | ✅ |
| TC-022 | Every negative fixture declares a distinct expect and a because | Property | P0 | FR-003-AC-5 | ✅ |
| TC-023 | The both-forms fixture carries both Properties forms in one document | Unit | P0 | FR-003-AC-6 | ✅ |
| TC-024 | Every legacy fixture uses the pre-contract free-text form | Unit | P0 | FR-003-AC-7 | ✅ |
| TC-025 | Every skeleton's frontmatter names a declared type | Unit | P1 | FR-003-AC-8 | ✅ |
| TC-026 | Every skeleton extracts with no error and validates against its emitted schema | Integration | P0 | FR-003-AC-9 | ✅ |
| TC-027 | A skeleton and its alternate extract to identical fields | Property | P0 | FR-003-AC-10 | ✅ |
| TC-028 | Every legacy fixture extracts with no error under legacy_forms warning | Integration | P0 | FR-003-AC-11 | ✅ |
| TC-029 | A suite run reports zero skipped tests | Integration | P0 | NFR-001-AC-1 | 🚧 the run's skip count is read by eye until a reporter assertion is added |
| TC-030 | Each absent tool produces a diagnostic naming the command that installs it | Unit | P0 | NFR-001-AC-2 | 🚧 exercised for the engine and the toolchain; the validator leg is not yet driven with quire absent |
| TC-031 | Every declared generated target other than json-schema is emitted | Integration | P1 | FR-002-AC-1 | 🚧 no emitter exists for rust, typescript, python-pydantic-v2 or python-dataclass; agent-ix/filament-core-data#11 owns them, so the targets are declared and not emitted |

## Functional Requirement Coverage

| Functional Req | Acceptance Criteria | Test Cases | Coverage Status |
| --- | --- | --- | --- |
| FR-001 | FR-001-AC-1..9 | TC-001..TC-009 | ✅ Covered |
| FR-002 | FR-002-AC-1..8 | TC-010..TC-017 | ✅ Covered |
| FR-003 | FR-003-AC-1..11 | TC-018..TC-028 | ✅ Covered |
| NFR-001 | NFR-001-AC-1..2 | TC-029, TC-030 | 🚧 The engine and toolchain paths fail with a named command today. The skip count is not yet asserted programmatically and the validator-absent path is not yet driven. |

## Stakeholder Requirement Coverage

| Stakeholder Req | Trace to US/FR | Test/Validation | Coverage Status |
| --- | --- | --- | --- |
| StR-001-VC-1 | US-001; FR-001 | TC-005, TC-006, TC-007 | ✅ Covered |
| StR-001-VC-2 | US-001; FR-003 | TC-026 | ✅ Covered |

## Use Case Coverage

| Use Case | Coverage | Test / Evidence |
| --- | --- | --- |
| US-001 | ✅ Covered | TC-010 and TC-007 realise EX-1; TC-027 realises EX-2; TC-022 realises EX-3. |

## Gaps

Three rows are `🚧`, and each says why on the row. Two are verification gaps in
this repository (the skip count is not asserted programmatically; the
validator-absent path is not driven). The third is not this repository's to
close: no emitter exists for the non-JSON-Schema targets, so declaring them and
claiming they are emitted would be a status lie. A `🚧` row with a reason is the
honest form; `✅` would not be, and `⚠️` is not a marker the archetype admits.
