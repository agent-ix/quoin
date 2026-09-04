"""Reject a rendering whose inputs cannot produce a conforming module (FR-076).

Every check here aborts BEFORE any file is written and names the offending
value, because a half-rendered repository is worse than no repository: the
maintainer has to tell the template's mistakes from their own.

Cookiecutter renders this file as a template first, so the placeholders below are
substituted before Python ever sees them.
"""

import re
import sys

ORG = "{{ cookiecutter.org }}"
REPO_NAME = "{{ cookiecutter.repo_name }}"
PACKAGE_NAME = "{{ cookiecutter.package_name }}"
MODULE_KIND = "{{ cookiecutter.module_kind }}"
LICENSE = "{{ cookiecutter.license }}"
VERSION = "{{ cookiecutter.version }}"
SEMANTIC_CORE = "{{ cookiecutter.semantic_core_version }}"
GENERATED_TARGETS = "{{ cookiecutter.generated_targets }}"
IMPORTED_MODULES = "{{ cookiecutter.imported_modules }}"
TYPESPEC_VERSION = "{{ cookiecutter.typespec_version }}"
PYTHON_VERSION = "{{ cookiecutter.python_version }}"
QUIRE_ENGINE_FLOOR = "{{ cookiecutter.quire_engine_floor }}"
NAV_CATEGORY_ORDER = "{{ cookiecutter.nav_category_order }}"

# filament-core-data `common.schema.json`: `target` plus `representationFormat`,
# which `manifestTarget` admits as one union. Vendored here because a template
# cannot reach the schema at render time; the drift between this list and the
# vendored copy under `src/semantic/schemas/filament-core-data/` is asserted by
# quoin's own suite rather than trusted.
TARGET_REGISTRY = {
    "json-schema",
    "rust",
    "typescript",
    "python-pydantic-v2",
    "python-dataclass",
    "markdown",
    "json",
    "postgresql",
    "protobuf",
    "avro",
    "arrow",
    "parquet",
    "csv",
    "tsv",
}

# The only target with a working emitter today. Everything else is declared and
# recorded as declared-not-emitted (FR-076-AC-10) rather than presented as
# emitted, because a target nothing emits is a claim nobody checked.
EMITTED_TARGETS = {"json-schema"}

MODULE_KINDS = ("artifact", "object", "mixed")

# AGPL-3.0-or-later and its accepted spellings. FR-076 forbids a silent fallback
# to another licence: a rendered repository is public, and two licence strings
# make the grant a question rather than a grant.
AGPL_IDENTIFIERS = {"AGPL-3.0-or-later", "AGPL-3.0+"}

SEMVER = re.compile(r"^\d+\.\d+\.\d+(?:[-+][0-9A-Za-z.-]+)?$")
SLUG = re.compile(r"^[a-z][a-z0-9-]*[a-z0-9]$")
PY_IDENT = re.compile(r"^[a-z][a-z0-9_]*$")
IMPORT_REF = re.compile(r"^[a-z0-9][a-z0-9-]*/[a-z0-9][a-z0-9-]*@\d+\.\d+\.\d+(?:[-+][0-9A-Za-z.-]+)?$")


def fail(message):
    sys.stderr.write("cookiecutter(semantic-module): %s\n" % message)
    sys.exit(1)


def split_list(raw):
    return [item.strip() for item in raw.split(",") if item.strip()]


def main():
    if MODULE_KIND not in MODULE_KINDS:
        fail(
            "module_kind %r is not one of %s. A variant is a rendering decision, "
            "and there is no fourth shape." % (MODULE_KIND, ", ".join(MODULE_KINDS))
        )

    if LICENSE not in AGPL_IDENTIFIERS:
        fail(
            "license %r is refused. A rendered semantic module is public and "
            "AGPL-3.0-or-later; accepted spellings are %s. The template does not "
            "fall back to another licence, because a repository whose "
            "declarations disagree has no usable grant."
            % (LICENSE, ", ".join(sorted(AGPL_IDENTIFIERS)))
        )

    if not SLUG.match(ORG):
        fail("org %r is not a lowercase hyphenated slug." % ORG)

    if not SLUG.match(REPO_NAME):
        fail("repo_name %r is not a lowercase hyphenated slug." % REPO_NAME)

    if not PY_IDENT.match(PACKAGE_NAME):
        fail(
            "package_name %r is not a lowercase Python identifier; it names the "
            "importable package directory." % PACKAGE_NAME
        )

    # Every version this template writes into a rendered file is checked here.
    # NFR-020 makes each external command's floor a declared value, and a floor
    # nobody validated is a floor a typo lowers to nothing.
    for name, value in (
        ("version", VERSION),
        ("semantic_core_version", SEMANTIC_CORE),
        ("typespec_version", TYPESPEC_VERSION),
        ("quire_engine_floor", QUIRE_ENGINE_FLOOR),
    ):
        if not SEMVER.match(value):
            fail("%s %r is not an exact semantic version." % (name, value))

    if not re.match(r"^3\.\d+$", PYTHON_VERSION):
        fail(
            "python_version %r is not a <major>.<minor> Python version; it is "
            "written into the rendered dependency bound and the ruff target."
            % PYTHON_VERSION
        )

    if not NAV_CATEGORY_ORDER.isdigit():
        fail(
            "nav_category_order %r is not a whole number; it orders this "
            "module's category in the catalog navigation." % NAV_CATEGORY_ORDER
        )

    targets = split_list(GENERATED_TARGETS)
    if not targets:
        fail("generated_targets is empty; a module declares at least one target.")
    for target in targets:
        if target not in TARGET_REGISTRY:
            fail(
                "generated_targets entry %r is outside the filament-core-data "
                "target registry (%s)." % (target, ", ".join(sorted(TARGET_REGISTRY)))
            )
    if "json-schema" not in targets:
        fail(
            "generated_targets must include 'json-schema': it is the only target "
            "with an emitter today, and the module manifest references the "
            "emitted schemas by digest."
        )

    imported = split_list(IMPORTED_MODULES)
    seen = set()
    for entry in imported:
        if not IMPORT_REF.match(entry):
            fail(
                "imported_modules entry %r is not of the form "
                "<org>/<repo>@<exact-version>. An import without an exact version "
                "resolves differently on two machines." % entry
            )
        identity = entry.split("@", 1)[0]
        if identity in seen:
            fail(
                "imported_modules names %s twice. Two entries for one package "
                "identity collapse to whichever came last, which is a version "
                "choice nobody made." % identity
            )
        seen.add(identity)

    if MODULE_KIND == "mixed" and not imported:
        fail(
            "module_kind is 'mixed' but imported_modules is empty. A mixed module "
            "declares both artifact and object types AND at least one import; "
            "without an import it is an object-and-artifact module with nothing "
            "mixed, and the object or artifact variant is the honest shape."
        )

    declared_only = sorted(set(targets) - EMITTED_TARGETS)
    if declared_only:
        sys.stderr.write(
            "cookiecutter(semantic-module): declared-not-emitted targets: %s. "
            "The rendered README and Test Matrix record them as declared; no "
            "emitter produces them today.\n" % ", ".join(declared_only)
        )


main()
