"""Prune the surfaces the chosen variant does not carry, and say what to run next.

Cookiecutter cannot decide whether to CREATE a file, only what goes in it, so
the artifact-only surfaces are always rendered and removed here when the variant
does not carry them (FR-076). Removal is the last write this hook makes: if it
fails, the rendered tree still carries every file, and the conformance check
reports the surplus rather than a repository silently half-shaped.

This hook writes no emitted schema. `make bootstrap` in the rendered repository
installs the pinned toolchain and runs the emit pipeline, because emitting needs
`@agent-ix/semantic-core` from a registry and a template that reached the
network at render time would render differently offline (NFR-019).
"""

import os
import shutil
import sys

MODULE_KIND = "{{ cookiecutter.module_kind }}"
PACKAGE_NAME = "{{ cookiecutter.package_name }}"
REPO_NAME = "{{ cookiecutter.repo_name }}"

ARTIFACT_ONLY = (
    os.path.join(PACKAGE_NAME, "mappings.yaml"),
    os.path.join(PACKAGE_NAME, "examples"),
)

# Golden records follow their type: an artifact-only module has no Element. An
# object-only module keeps no examples directory at all, so it needs no entry
# here — ARTIFACT_ONLY removes the whole directory.
OBJECT_ONLY_GOLDENS = (
    os.path.join(PACKAGE_NAME, "examples", "Element.record.json"),
)

OBJECT_ONLY_SKELETONS = (
    os.path.join(PACKAGE_NAME, "skeletons", "element.md"),
    os.path.join(PACKAGE_NAME, "skeletons", "element.sysml.md"),
)

ARTIFACT_ONLY_SKELETONS = (
    os.path.join(PACKAGE_NAME, "skeletons", "note.md"),
    os.path.join(PACKAGE_NAME, "skeletons", "note.sysml.md"),
)

OBJECT_ONLY_FIXTURES = (
    os.path.join("tests", "fixtures", "negative", "element-no-identity-row.md"),
    os.path.join("tests", "fixtures", "negative", "element-both-property-forms.md"),
    os.path.join("tests", "fixtures", "legacy", "element-free-text-properties.md"),
)

ARTIFACT_ONLY_FIXTURES = (
    os.path.join("tests", "fixtures", "negative", "note-missing-properties-section.md"),
    os.path.join("tests", "fixtures", "negative", "note-untyped-property-row.md"),
    os.path.join("tests", "fixtures", "negative", "note-both-property-forms.md"),
    os.path.join("tests", "fixtures", "legacy", "note-free-text-properties.md"),
)

# The both-Properties-forms refusal is one rule, not one rule per type. A mixed
# module therefore keeps the object variant's fixture and drops the artifact
# one: two fixtures declaring the same `expect` would break the distinctness the
# suite asserts, and the second would prove nothing the first does not.
MIXED_DUPLICATE_FIXTURES = (
    os.path.join("tests", "fixtures", "negative", "note-both-property-forms.md"),
)


def remove(path):
    if os.path.isdir(path):
        shutil.rmtree(path)
    elif os.path.exists(path):
        os.remove(path)


def main():
    drop = []
    if MODULE_KIND == "object":
        drop.extend(ARTIFACT_ONLY)
        drop.extend(ARTIFACT_ONLY_SKELETONS)
        drop.extend(ARTIFACT_ONLY_FIXTURES)
    elif MODULE_KIND == "artifact":
        drop.extend(OBJECT_ONLY_SKELETONS)
        drop.extend(OBJECT_ONLY_FIXTURES)
        drop.extend(OBJECT_ONLY_GOLDENS)
    else:
        drop.extend(MIXED_DUPLICATE_FIXTURES)

    for path in drop:
        remove(path)

    sys.stdout.write(
        "\n"
        "Rendered %s (%s module).\n"
        "\n"
        "Next, in that directory:\n"
        "  make bootstrap   # install the pinned toolchain and emit schemas/ + digests\n"
        "  make gate        # spec validation, lint, schema drift check, tests\n"
        "\n"
        "`make gate` fails without the Quire wheel exposing `extract_semantic`;\n"
        "`make dev-quire` provisions it. It fails rather than skipping, because a\n"
        "skipped row is not coverage.\n"
        "\n"
        "docs/catalog-entry.md names the steps to add this module to the Quoin\n"
        "default catalog and to the tracking project.\n"
        "\n" % (REPO_NAME, MODULE_KIND)
    )


main()
