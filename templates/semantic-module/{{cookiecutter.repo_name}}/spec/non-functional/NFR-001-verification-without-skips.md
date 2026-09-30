---
id: NFR-001
title: "Verification without skips"
type: NFR
quality_attribute: reliability
relationships:
  - target: "ix://{{ cookiecutter.org }}/{{ cookiecutter.repo_name }}/FR-003"
    type: "constrains"
---

# NFR-001: Verification without skips

## Statement

The verification suite SHALL report zero skipped tests, failing and naming the
install command whenever a tool it needs is absent.

## Scope

- Applies to: every test in `tests/`, and every leg of `make gate`.
- Operational context: a clean runner with none of the toolchain installed, and a developer machine with all of it.

## Rationale

The Quire wheel exposing `extract_semantic` is a declared dev dependency
resolved from the `internal-pypi` Poetry source (`poetry install`); the
TypeSpec toolchain and `@agent-ix/semantic-core` are resolved by
`make semantic-install`. The obvious handling of an absent optional import —
`pytest.importorskip` — turns every semantic row green in exactly the environment
where none of them ran, and a clean runner that has not yet installed is
precisely that environment. The same reasoning covers the schema toolchain and
the validator: a drift gate that reports success without running is the defect
the gate exists to catch.

## Measurement and Evaluation

| Metric | Target | Threshold | Method |
|--------|--------|-----------|--------|
| Skipped tests in a suite run | 0 | 0 | Manual |
| Absent tools producing a named diagnostic | all | all | Inspection |
| Gate legs that report success without running | 0 | 0 | Inspection |

## Verification

`make test` reports the skip count; it must be zero. Removing the engine, the
grammar package or the schema toolchain must each turn the suite red with a
message naming the command that restores it.

No test in the suite asserts either criterion, so neither carries a trace tag.
AC-1 is verified manually, by reading the skip count from a run's own summary.
AC-2 is verified by inspecting each absent-tool path: `tests/conftest.py` for
the engine and the grammar package, `tests/test_schema_emission.py` for the
schema toolchain, and `scripts/generate-schemas.mjs`, each of which must fail
naming its install command.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| NFR-001-AC-1 | A run of `make test` reports zero skipped tests. | Manual |
| NFR-001-AC-2 | With the engine, the grammar package or the schema toolchain absent, the suite fails with a message naming the command that installs it. | Inspection |

## Dependencies

- **Upstream**: [FR-003](../functional/FR-003-authoring-forms-and-fixtures.md)
