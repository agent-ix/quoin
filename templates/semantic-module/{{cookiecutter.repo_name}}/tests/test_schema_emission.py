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
def test_the_package_metadata_declares_the_engine_only_as_a_dev_dependency(repo_root):
    pyproject = (repo_root / "pyproject.toml").read_text()
    runtime_section = pyproject.split("[tool.poetry.dependencies]", 1)[1].split(
        "[", 1
    )[0]
    dev_section = pyproject.split("[tool.poetry.group.dev.dependencies]", 1)[1].split(
        "[", 1
    )[0]
    assert "quire" not in runtime_section, (
        "the engine is declared as a runtime dependency. A consumer installing "
        "this module should not also pull in the toolchain that generates it — "
        "quire belongs in the dev dependency group, never here."
    )
    quire_lines = [
        line for line in dev_section.splitlines() if line.strip().startswith("quire ")
    ]
    assert len(quire_lines) == 1, (
        "the engine is not declared as a dev dependency. It should be resolved "
        "from the `internal-pypi` Poetry source (see pyproject.toml's "
        "[[tool.poetry.source]])."
    )
    assert 'source = "internal-pypi"' in quire_lines[0], (
        "the quire dev dependency names no `internal-pypi` source, so Poetry "
        "could resolve the unrelated `quire` package on public PyPI."
    )


@pytest.mark.trace("FR-002-AC-8")
def test_the_repository_ships_no_npmrc(repo_root):
    for root, dirs, files in os.walk(repo_root):
        dirs[:] = [d for d in dirs if d not in ("node_modules", ".git")]
        assert ".npmrc" not in files, f"{root} carries an .npmrc"
