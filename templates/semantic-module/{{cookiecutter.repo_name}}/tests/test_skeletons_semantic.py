"""The skeletons demonstrate the mappings, and the fixtures prove they are enforced.

The rows that consult the engine take the `quire_engine` fixture, which FAILS —
never skips — when the wheel is absent.
"""

from __future__ import annotations

import re

import pytest

PLACEHOLDERS = ("TODO", "FIXME", "XXX", "<fill", "lorem ipsum")
TABLE_HEADER = "| Field | Type | Multiplicity | Constraints |"


def properties_section(text: str) -> str:
    match = re.search(r"^## Properties\n(.*?)(?=^## |\Z)", text, re.S | re.M)
    assert match, "document has no Properties section"
    return match.group(1)


def table_rows(section: str) -> list[list[str]]:
    rows = []
    for line in section.splitlines():
        if not line.startswith("|") or set(line) <= set("|- "):
            continue
        rows.append([cell.strip() for cell in line.strip().strip("|").split("|")])
    return rows[1:] if rows else []


def sysml_fields(section: str) -> list[tuple[str, str, str]]:
    fence = re.search(r"```sysml\n(.*?)```", section, re.S)
    assert fence, "document has no sysml fence"
    fields = []
    for line in fence.group(1).splitlines():
        decl = re.match(
            r"\s*(?:attribute|ref item)\s+(\w+)\s*:\s*(\w+)\[([^\]]+)\]", line
        )
        if decl:
            fields.append((decl.group(1), decl.group(2), decl.group(3)))
    return fields


def extract(engine, module, path):
    """Run the engine over one document and return the semantic-v1 record.

    The request deliberately carries no `path`. A golden record fixes the bytes
    an extraction produces, and an absolute path from the machine that produced
    it would make the golden machine-specific — a diff that says only "somebody
    else ran it". The assertions below name the file themselves.
    """
    return engine.extract_semantic(
        {
            "markdown": path.read_text(),
            "module": module,
            "required": {"properties": True},
        }
    )


def declaration_of(record: dict) -> dict:
    """The declaration record an emitted schema validates: the grammar keys only.

    The engine's `semantic-v1` envelope also carries diagnostics, availability
    and the verbatim clause text. Those describe the extraction, not the
    declaration, and the emitted schemas are sealed — passing the whole envelope
    would fail on keys the schema is right not to admit.
    """
    return {
        key: record[key]
        for key in ("fields", "relations", "clauses", "operations")
        if record.get(key)
    }


def errors_of(record: dict) -> list[dict]:
    return [d for d in record.get("diagnostics", []) if d.get("severity") == "error"]


@pytest.mark.trace("FR-003-AC-1")
def test_every_export_has_a_skeleton_in_the_typed_table_form(exports, skeletons_dir):
    for name in exports:
        path = skeletons_dir / f"{name}.md"
        assert path.is_file(), f"{name} has no authoring skeleton"
        section = properties_section(path.read_text())
        assert TABLE_HEADER in section, (
            f"{path.name}'s Properties table header is not the typed four-column form"
        )
        assert len(table_rows(section)) >= 1, f"{path.name} declares no property"


@pytest.mark.trace("FR-003-AC-2")
def test_every_skeleton_has_a_sysml_alternate_declaring_the_same_fields(
    exports, skeletons_dir
):
    for name in exports:
        table = properties_section((skeletons_dir / f"{name}.md").read_text())
        fence = properties_section((skeletons_dir / f"{name}.sysml.md").read_text())
        from_table = [(row[0], row[1], row[2]) for row in table_rows(table)]
        assert from_table == sysml_fields(fence), (
            f"{name}.md and {name}.sysml.md declare different fields; the two forms "
            "are alternates of one declaration, not two declarations"
        )


@pytest.mark.trace("FR-003-AC-3")
def test_every_skeleton_carries_an_ocl_clause_under_its_own_heading(skeletons_dir):
    for path in sorted(skeletons_dir.glob("*.md")):
        clauses = re.findall(r"^### (\w+)\n\n```ocl\n(.*?)```", path.read_text(), re.S | re.M)
        assert clauses, f"{path.name} carries no ocl clause under a clause heading"
        ids = [clause[0] for clause in clauses]
        assert len(ids) == len(set(ids)), f"{path.name} repeats a clause id"


@pytest.mark.trace("FR-003-AC-4")
def test_no_skeleton_carries_a_placeholder_body(skeletons_dir):
    for path in sorted(skeletons_dir.glob("*.md")):
        body = re.sub(r"<!--.*?-->", "", path.read_text(), flags=re.S)
        for token in PLACEHOLDERS:
            assert token.lower() not in body.lower(), f"{path.name} carries {token!r}"


@pytest.mark.trace("FR-003-AC-5")
def test_each_negative_fixture_declares_its_own_distinct_expectation(
    negative_fixtures, helpers
):
    assert negative_fixtures, "the module declares no negative fixture"
    expectations = []
    for path in negative_fixtures:
        front = helpers.frontmatter(path.read_text())
        assert "expect" in front, f"{path.name} names no expected diagnostic"
        assert "because" in front, f"{path.name} names no reason"
        expectations.append(front["expect"])
    assert len(expectations) == len(set(expectations)), (
        "two negative fixtures expect the same diagnostic, so one of them proves "
        f"nothing the other does not: {expectations}"
    )


@pytest.mark.trace("FR-003-AC-6")
def test_the_both_forms_fixture_carries_both_forms(negative_fixtures, helpers):
    both = [
        path
        for path in negative_fixtures
        if helpers.frontmatter(path.read_text())["expect"].endswith("both-forms-present")
    ]
    assert both, "no fixture exercises the both-Properties-forms refusal"
    section = properties_section(both[0].read_text())
    assert TABLE_HEADER in section and "```sysml" in section


@pytest.mark.trace("FR-003-AC-7")
def test_every_legacy_fixture_uses_the_pre_contract_form(legacy_fixtures, helpers):
    assert legacy_fixtures, "the module declares no legacy-form fixture"
    for path in legacy_fixtures:
        front = helpers.frontmatter(path.read_text())
        assert front["expect"] == "semantic.legacy-form", path.name
        section = properties_section(path.read_text())
        assert TABLE_HEADER not in section and "```sysml" not in section, (
            f"{path.name} uses a current form; a legacy fixture must use the "
            "pre-contract free-text form or it exercises nothing"
        )


@pytest.mark.trace("FR-003-AC-8")
def test_every_skeleton_frontmatter_names_a_declared_type(
    skeletons_dir, exports, helpers
):
    for path in sorted(skeletons_dir.glob("*.md")):
        front = helpers.frontmatter(path.read_text())
        assert front["type"] in set(exports), (
            f"{path.name} declares type {front['type']!r}, which the manifest does "
            "not export"
        )
        assert front["title"], f"{path.name} has no title"


@pytest.mark.trace("FR-003-AC-9")
def test_every_skeleton_extracts_and_validates_against_its_emitted_schema(
    semantic_module, schema_registry, quire_engine, skeletons_dir, exports, model_of
):
    """The engine extracts each skeleton, and the record validates.

    This is the row the whole suite exists for: the only one that proves the
    Markdown, the manifest locators and the emitted schema agree. It fails
    loudly without the engine rather than reporting green.
    """
    for name in exports:
        path = skeletons_dir / f"{name}.md"
        record = extract(quire_engine, semantic_module, path)
        assert not errors_of(record), f"{path.name}: {errors_of(record)}"
        schema_registry(model_of[name]).validate(declaration_of(record))


@pytest.mark.trace("FR-003-AC-10")
def test_the_two_property_forms_extract_to_the_same_fields(
    semantic_module, quire_engine, skeletons_dir, exports
):
    for name in exports:
        table = extract(quire_engine, semantic_module, skeletons_dir / f"{name}.md")
        fence = extract(
            quire_engine, semantic_module, skeletons_dir / f"{name}.sysml.md"
        )
        assert table["fields"] == fence["fields"], (
            f"{name}.md and {name}.sysml.md extract to different declarations; "
            "the two forms are alternates of one declaration"
        )


@pytest.mark.trace("FR-003-AC-11")
def test_every_legacy_fixture_yields_no_error_under_warning(
    semantic_module, quire_engine, legacy_fixtures
):
    for path in legacy_fixtures:
        record = extract(quire_engine, semantic_module, path)
        assert not errors_of(record), (
            f"{path.name} is an ERROR under legacy_forms: warning. The contract "
            "keeps the pre-contract form valid until a human promotes the module."
        )


# ---------------------------------------------------------------------------
# Markdown mappings and golden records.
#
# An object-only module ships no mapping file, and these rows are then vacuous
# by construction rather than skipped: the loop runs zero times because the
# module genuinely declares nothing to map. That is a different thing from a
# skip, which would hide a mapping file that failed to load.
# ---------------------------------------------------------------------------


def mapped_models(mappings) -> dict:
    return (mappings or {}).get("models") or {}


@pytest.mark.trace("FR-003-AC-12")
def test_every_mapped_model_names_a_schema_a_skeleton_and_a_golden(
    mappings, package_root, model_of, exports
):
    models = mapped_models(mappings)
    if mappings is not None:
        assert models, "mappings.yaml declares no models"
    for name, mapping in models.items():
        assert name in model_of.values(), f"{name} is mapped but not exported"
        for key in ("schema", "skeleton", "example"):
            path = package_root / mapping[key]
            assert path.is_file(), f"{name}.{key} names {mapping[key]}, which is absent"
        assert mapping["authority"] == "markdown"


@pytest.mark.trace("FR-003-AC-13")
def test_every_golden_record_matches_what_its_skeleton_extracts_to(
    mappings, package_root, semantic_module, quire_engine
):
    import json

    for name, mapping in mapped_models(mappings).items():
        skeleton = package_root / mapping["skeleton"]
        golden = package_root / mapping["example"]
        record = declaration_of(extract(quire_engine, semantic_module, skeleton))
        expected = json.loads(golden.read_text())
        assert record == expected, (
            f"{mapping['example']} no longer matches what {mapping['skeleton']} "
            "extracts to. Regenerate the golden deliberately, in the commit that "
            "changed the skeleton or the mapping — never to make this row pass."
        )


@pytest.mark.trace("FR-003-AC-14")
def test_every_golden_record_uses_the_declared_serialization(mappings, package_root):
    import json

    for mapping in mapped_models(mappings).values():
        golden = package_root / mapping["example"]
        text = golden.read_text()
        assert text.endswith("\n"), f"{mapping['example']} has no trailing newline"
        canonical = json.dumps(json.loads(text), indent=2, sort_keys=True) + "\n"
        assert text == canonical, (
            f"{mapping['example']} is not sorted-key two-space JSON, so a diff on "
            "it would show formatting rather than content"
        )
