"""The manifest declares the semantic-module contract, and its digests are true.

Nothing here consults the engine: these are facts about the committed bytes, and
a fact about bytes should not need a wheel to check.
"""

from __future__ import annotations

import pytest
import yaml


@pytest.mark.trace("FR-001-AC-1")
def test_semantic_block_carries_exactly_the_admitted_keys(
    semantic_block, semantic_keys
):
    assert set(semantic_block) == set(semantic_keys)


@pytest.mark.trace("FR-001-AC-2")
def test_contract_version_and_posture_are_the_declared_defaults(semantic_block):
    assert semantic_block["contract_version"] == "1.0.0"
    assert semantic_block["compatibility_posture"] == "additive"
    assert semantic_block["legacy_forms"] == "warning"
    # `sweep_report` is required only when `legacy_forms` is promoted to `error`.
    assert "sweep_report" not in semantic_block


@pytest.mark.trace("FR-001-AC-3")
def test_exports_and_declared_type_names_are_the_same_set(
    manifest, semantic_block, helpers, exports
):
    declared = [entry["name"] for entry in helpers.declared_types(manifest)]
    assert sorted(semantic_block["exports"]) == sorted(declared)
    assert sorted(declared) == sorted(exports)


@pytest.mark.trace("FR-001-AC-4")
def test_no_type_name_is_declared_twice(manifest, helpers):
    declared = [entry["name"] for entry in helpers.declared_types(manifest)]
    assert len(declared) == len(set(declared)), f"duplicate type name in {declared}"


@pytest.mark.trace("FR-001-AC-5")
def test_every_export_references_its_schema_by_path_and_digest(
    manifest, helpers, model_of
):
    for entry in helpers.declared_types(manifest):
        reference = entry["data_schema"]
        assert set(reference) == {"schema", "digest"}, (
            f"{entry['name']} carries {sorted(reference)}; the contract is a "
            "schema-and-digest reference, never an inline schema"
        )
        assert reference["schema"].endswith(f"{model_of[entry['name']]}.json")


@pytest.mark.trace("FR-001-AC-6")
def test_no_export_carries_the_placeholder_contract(manifest, helpers):
    for entry in helpers.declared_types(manifest):
        assert entry["data_schema"] != {"type": "object"}


@pytest.mark.trace("FR-001-AC-7")
def test_every_digest_equals_the_bytes_of_the_file_it_names(
    manifest, helpers, package_root
):
    for entry in helpers.declared_types(manifest):
        reference = entry["data_schema"]
        path = package_root / reference["schema"]
        assert (
            path.is_file()
        ), f"{reference['schema']} is referenced but absent. Run `make schemas`."
        assert reference["digest"] == helpers.sha256_of(path), (
            f"{entry['name']}'s digest does not match {reference['schema']}. "
            "Run `make schemas` and commit the result."
        )


@pytest.mark.trace("FR-001-AC-8")
def test_imports_are_a_mapping_to_exact_versions(semantic_block):
    imports = semantic_block["imports"]
    assert isinstance(imports, dict)
    for identity, version in imports.items():
        assert identity.count("/") == 1, identity
        assert (
            version.count(".") >= 2
        ), f"{identity} is pinned to {version!r}, not exact"


@pytest.mark.trace("FR-001-AC-9")
def test_the_manifest_keeps_its_comments_and_is_not_reserialized(package_root):
    text = (package_root / "manifest.yaml").read_text()
    assert "# The semantic-module contract" in text, (
        "the manifest's explanatory comments are gone, which means something "
        "round-tripped it through a YAML parser instead of rewriting the digest "
        "lines textually"
    )
    assert yaml.safe_load(text) is not None
