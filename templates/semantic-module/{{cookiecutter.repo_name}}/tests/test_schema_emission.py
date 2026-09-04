"""The emitted schemas are real, complete, and reproducible from the source."""

from __future__ import annotations

import json
import os
import subprocess

import pytest


def run_generate(repo_root, *args: str) -> subprocess.CompletedProcess:
    return subprocess.run(
        ["node", "scripts/generate-schemas.mjs", *args],
        cwd=repo_root,
        capture_output=True,
        text=True,
        check=False,
    )


def require_toolchain(repo_root):
    """Fail — never skip — when the schema toolchain is absent.

    A drift gate that reports green without running is the defect the gate
    exists to catch.
    """
    if not (repo_root / "node_modules" / "@typespec" / "compiler").is_dir():
        pytest.fail(
            "the schema toolchain is not installed, so this check cannot run. "
            "Run `make install`. This is a failure and not a skip."
        )


@pytest.mark.trace("FR-002-AC-1")
def test_one_schema_is_emitted_for_every_exported_type(
    manifest, helpers, schemas_dir, model_of
):
    for entry in helpers.declared_types(manifest):
        path = schemas_dir / f"{model_of[entry['name']]}.json"
        assert path.is_file(), f"{path.name} is missing; run `make schemas`"


@pytest.mark.trace("FR-002-AC-2")
def test_no_emitted_schema_is_the_placeholder_contract(schemas_dir):
    for path in schemas_dir.glob("*.json"):
        if path.name == "toolchain.json":
            continue
        schema = json.loads(path.read_text())
        assert schema.get("properties") or schema.get("$ref"), (
            f"{path.name} declares no properties; an empty object contract is the "
            "placeholder this module exists to replace"
        )


@pytest.mark.trace("FR-002-AC-3")
def test_every_reference_is_absolute(schemas_dir):
    for path in schemas_dir.glob("*.json"):
        if path.name == "toolchain.json":
            continue
        for line in path.read_text().splitlines():
            for token in ('"$ref":', '"$id":'):
                if token in line:
                    value = line.split(":", 1)[1].strip().strip(",").strip('"')
                    assert value.startswith("https://"), f"{path.name}: {value}"


@pytest.mark.trace("FR-002-AC-4")
def test_the_toolchain_records_what_produced_the_bytes(schemas_dir):
    toolchain = json.loads((schemas_dir / "toolchain.json").read_text())
    for key in ("compiler", "emitter", "semanticCore", "base", "files", "digest"):
        assert key in toolchain, f"toolchain.json declares no {key}"
    assert toolchain["digest"].startswith("sha256:")


@pytest.mark.trace("FR-002-AC-5")
def test_check_mode_is_green_against_the_committed_output(repo_root):
    require_toolchain(repo_root)
    result = run_generate(repo_root, "--check")
    assert result.returncode == 0, result.stdout + result.stderr


@pytest.mark.trace("FR-002-AC-6")
def test_check_mode_is_red_when_an_emitted_byte_changes(repo_root, schemas_dir):
    require_toolchain(repo_root)
    target = next(p for p in schemas_dir.glob("*.json") if p.name != "toolchain.json")
    original = target.read_bytes()
    # The committed bytes are restored by `finally`, and a hard kill mid-test
    # would leave them mutated. That is the trade this row accepts: check mode
    # reads the committed tree by design, so proving it goes red means mutating
    # the tree it reads. `make schemas` restores it in one command.
    try:
        target.write_bytes(original.replace(b'"type"', b'"typ3"', 1))
        result = run_generate(repo_root, "--check")
        assert result.returncode != 0
        assert target.name in (result.stdout + result.stderr)
    finally:
        target.write_bytes(original)


@pytest.mark.trace("FR-002-AC-7")
def test_the_package_metadata_declares_no_engine_dependency(repo_root):
    pyproject = (repo_root / "pyproject.toml").read_text()
    section = pyproject.split("[tool.poetry.dependencies]", 1)[1].split("[", 1)[0]
    # Comments are not declarations. The section carries one explaining WHY the
    # engine is absent, and a naive substring search would read that explanation
    # as the thing it warns about.
    declarations = "\n".join(
        line for line in section.splitlines() if not line.strip().startswith("#")
    )
    assert "quire" not in declarations, (
        "the engine is declared as a dependency. No index this repository may "
        "depend on serves the required wheel, so declaring it makes "
        "`poetry install` fail everywhere."
    )


@pytest.mark.trace("FR-002-AC-8")
def test_the_repository_ships_no_npmrc(repo_root):
    for root, dirs, files in os.walk(repo_root):
        dirs[:] = [d for d in dirs if d not in ("node_modules", ".git")]
        assert ".npmrc" not in files, f"{root} carries an .npmrc"
