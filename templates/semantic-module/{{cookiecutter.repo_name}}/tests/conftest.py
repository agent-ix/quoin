"""Shared fixtures for spec-mixed-example's verification suite.

Two policies live here and nowhere else.

**The engine is a hard dependency of the semantic rows.** ``quire`` is not
declared in ``pyproject.toml`` because no index this repository may commit
against carries the wheel exposing ``extract_semantic``
(``agent-ix/quire-rs#392`` tracks publishing it). The wheel is provisioned by
``make dev-quire``. When it is absent, or too old, or missing the capability,
the semantic tests **fail** and say how to fix it. They never skip, because a
skipped row is not coverage — and a clean runner, which is exactly where a
regression would first show, is exactly where a skip would fire.

**The grammar resolves from the committed tree.** The emitted schemas are read
from ``{{ cookiecutter.package_name }}/schemas/`` and every ``$ref`` to
semantic-core resolves against the package the pinned toolchain installs, so a
record test validates against the real bytes rather than against a stub.
"""

from __future__ import annotations

import hashlib
import json
import pathlib
import re
from typing import Any

import pytest
import yaml

REPO_ROOT = pathlib.Path(__file__).resolve().parent.parent
PACKAGE_ROOT = REPO_ROOT / "{{ cookiecutter.package_name }}"
MANIFEST_PATH = PACKAGE_ROOT / "manifest.yaml"
SCHEMAS_DIR = PACKAGE_ROOT / "schemas"
SKELETONS_DIR = PACKAGE_ROOT / "skeletons"
NEGATIVE_DIR = REPO_ROOT / "tests" / "fixtures" / "negative"
LEGACY_DIR = REPO_ROOT / "tests" / "fixtures" / "legacy"

SEMANTIC_CORE_VERSION = "{{ cookiecutter.semantic_core_version }}"
SEMANTIC_CORE_DIR = (
    REPO_ROOT
    / "node_modules"
    / "@agent-ix"
    / "semantic-core"
    / "generated"
    / "json-schema"
)
SEMANTIC_CORE_BASE = (
    f"https://schemas.agent-ix.org/semantic-core/{SEMANTIC_CORE_VERSION}/"
)

ENGINE_FLOOR = "{{ cookiecutter.quire_engine_floor }}"

QUIRE_MISSING = (
    "the Quire wheel exposing `extract_semantic` is not installed in this "
    "environment. Run `make dev-quire` (agent-ix/quire-rs#392 tracks publishing "
    f"{ENGINE_FLOOR} to an index this repository may depend on). The semantic "
    "tests fail rather than skip, because a skipped row is not coverage."
)

SEMANTIC_CORE_MISSING = (
    "@agent-ix/semantic-core is not installed, so `$ref`s to the grammar cannot "
    "resolve and a record test would validate against nothing. Run "
    "`make install`. `@agent-ix` resolves from the user-level npm "
    "configuration; this repository ships no .npmrc."
)

{% set exports = [] -%}
{% if cookiecutter.module_kind in ("object", "mixed") %}{% set _ = exports.append('"element"') %}{% endif -%}
{% if cookiecutter.module_kind in ("artifact", "mixed") %}{% set _ = exports.append('"note"') %}{% endif -%}
EXPORTS = ({{ exports | join(", ") }}{% if exports | length == 1 %},{% endif %})

MODEL_OF = {
{%- if cookiecutter.module_kind in ("object", "mixed") %}
    "element": "Element",
{%- endif %}
{%- if cookiecutter.module_kind in ("artifact", "mixed") %}
    "note": "Note",
{%- endif %}
}

SEMANTIC_KEYS = (
    "contract_version",
    "semantic_core",
    "package",
    "exports",
    "imports",
    "targets",
    "mappings",
    "compatibility_posture",
    "legacy_forms",
)


def load_manifest() -> dict[str, Any]:
    return yaml.safe_load(MANIFEST_PATH.read_text())


def declared_types(manifest: dict[str, Any]) -> list[dict[str, Any]]:
    """Every declared type, whichever section declares it."""
    return list(manifest.get("artifact_types") or []) + list(
        manifest.get("object_types") or []
    )


def frontmatter(markdown: str) -> dict[str, Any]:
    match = re.match(r"---\n(.*?)\n---\n", markdown, re.DOTALL)
    assert match, "document has no frontmatter"
    return yaml.safe_load(match.group(1))


def sha256_of(path: pathlib.Path) -> str:
    return f"sha256:{hashlib.sha256(path.read_bytes()).hexdigest()}"


def _version_tuple(value: str) -> tuple[int, ...]:
    return tuple(int(part) for part in re.split(r"[.+-]", value)[:3] if part.isdigit())


def _installed_engine_version() -> str | None:
    """The installed engine's version. `quire` exposes no `__version__`, so the
    distribution metadata is the only place that carries it."""
    import importlib.metadata

    try:
        return importlib.metadata.version("quire")
    except importlib.metadata.PackageNotFoundError:
        return None


def require_quire():
    """Import the engine, or FAIL naming the provisioning path. Never skip."""
    try:
        import quire
    except ImportError as error:
        pytest.fail(f"{QUIRE_MISSING} (import error: {error})")
    if not hasattr(quire, "extract_semantic"):
        pytest.fail(
            f"`extract_semantic` is missing from the installed quire: {QUIRE_MISSING}"
        )
    installed = _installed_engine_version()
    if installed and _version_tuple(installed) < _version_tuple(ENGINE_FLOOR):
        pytest.fail(
            f"the installed quire is {installed}, older than this module's declared "
            f"floor {ENGINE_FLOOR}. A capability gap in an old engine reads as a "
            "module defect, so this is a failure rather than a warning. Run "
            "`make dev-quire`."
        )
    return quire


# --------------------------------------------------------------------------
# Fixtures
#
# Everything the test modules need arrives through a fixture. A test module that
# imported `tests.conftest` directly would resolve `tests` against whatever
# `tests` package happens to be first on `sys.path` — which, on a machine with
# several repositories checked out, is regularly somebody else's. The failure is
# an ImportError at collection time, which reads as a defect in this module and
# is not one.
# --------------------------------------------------------------------------


@pytest.fixture(scope="session")
def quire_engine():
    return require_quire()


@pytest.fixture(scope="session")
def repo_root() -> pathlib.Path:
    return REPO_ROOT


@pytest.fixture(scope="session")
def package_root() -> pathlib.Path:
    return PACKAGE_ROOT


@pytest.fixture(scope="session")
def schemas_dir() -> pathlib.Path:
    return SCHEMAS_DIR


@pytest.fixture(scope="session")
def skeletons_dir() -> pathlib.Path:
    return SKELETONS_DIR


@pytest.fixture(scope="session")
def exports() -> tuple[str, ...]:
    return EXPORTS


@pytest.fixture(scope="session")
def model_of() -> dict[str, str]:
    return MODEL_OF


@pytest.fixture(scope="session")
def semantic_keys() -> tuple[str, ...]:
    return SEMANTIC_KEYS


@pytest.fixture(scope="session")
def helpers():
    """The small pure helpers the test modules share."""

    class Helpers:
        declared_types = staticmethod(declared_types)
        frontmatter = staticmethod(frontmatter)
        sha256_of = staticmethod(sha256_of)

    return Helpers


@pytest.fixture(scope="session")
def manifest() -> dict[str, Any]:
    return load_manifest()


@pytest.fixture(scope="session")
def semantic_block(manifest: dict[str, Any]) -> dict[str, Any]:
    return manifest["semantic"]


@pytest.fixture(scope="session")
def semantic_module(semantic_block: dict[str, Any]) -> dict[str, Any]:
    """The `module` block `extract_semantic` takes, derived from the manifest."""
    return {
        "contractVersion": semantic_block["contract_version"],
        "semanticCore": semantic_block["semantic_core"],
        "package": semantic_block["package"],
        "exports": semantic_block["exports"],
        "imports": semantic_block["imports"],
        "compatibilityPosture": semantic_block["compatibility_posture"],
        "legacyForms": semantic_block["legacy_forms"],
    }


@pytest.fixture(scope="session")
def skeletons() -> list[pathlib.Path]:
    return sorted(SKELETONS_DIR.glob("*.md"))


@pytest.fixture(scope="session")
def negative_fixtures() -> list[pathlib.Path]:
    return sorted(NEGATIVE_DIR.glob("*.md"))


@pytest.fixture(scope="session")
def legacy_fixtures() -> list[pathlib.Path]:
    return sorted(LEGACY_DIR.glob("*.md"))


@pytest.fixture(scope="session")
def schema_registry():
    """A 2020-12 validator factory over the emitted schemas plus semantic-core.

    Every `$ref` resolves locally: this module's models from the committed
    `schemas/` directory, grammar models from the semantic-core package the
    pinned toolchain installs.
    """
    from referencing import Registry, Resource

    if not SCHEMAS_DIR.is_dir():
        pytest.fail(
            "the emitted schemas are missing. Run `make schemas` — they are "
            "produced from typespec/main.tsp and are not authored by hand."
        )
    if not SEMANTIC_CORE_DIR.is_dir():
        pytest.fail(SEMANTIC_CORE_MISSING)

    resources = []
    for path in sorted(SCHEMAS_DIR.glob("*.json")):
        if path.name == "toolchain.json":
            continue
        schema = json.loads(path.read_text())
        resources.append((schema["$id"], Resource.from_contents(schema)))
    for path in sorted(SEMANTIC_CORE_DIR.glob("*.json")):
        schema = json.loads(path.read_text())
        uri = schema.get("$id") or f"{SEMANTIC_CORE_BASE}{path.name}"
        resources.append((uri, Resource.from_contents(schema)))
    registry = Registry().with_resources(resources)

    def validator_for(model: str):
        from jsonschema import Draft202012Validator

        schema = json.loads((SCHEMAS_DIR / f"{model}.json").read_text())
        return Draft202012Validator(schema, registry=registry)

    return validator_for


MAPPINGS_PATH = PACKAGE_ROOT / "mappings.yaml"
EXAMPLES_DIR = PACKAGE_ROOT / "examples"


@pytest.fixture(scope="session")
def mappings() -> dict[str, Any] | None:
    """The Markdown mapping declarations, or None for a module that declares no
    artifact types. An object-only module maps object declarations through the
    manifest's locators alone and ships no mapping file."""
    if not MAPPINGS_PATH.is_file():
        return None
    return yaml.safe_load(MAPPINGS_PATH.read_text())


@pytest.fixture(scope="session")
def examples_dir() -> pathlib.Path:
    return EXAMPLES_DIR
