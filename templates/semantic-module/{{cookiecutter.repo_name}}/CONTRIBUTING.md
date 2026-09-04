# Contributing to {{ cookiecutter.repo_name }}

## The rules that are not negotiable

**Emitted schemas are never hand-edited.** `{{ cookiecutter.package_name }}/schemas/`
is produced from `typespec/main.tsp` by `make schemas`. A wrong schema is a wrong
`.tsp`; fix it there and regenerate. `make lint` fails on any drift between the
source and the committed bytes.

**Digests are never typed.** Every `data_schema.digest` in `manifest.yaml` is
written by `make schemas` from the emitted bytes. A hand-written digest is a
claim nobody checked.

**Tests fail; they do not skip.** If a tool the suite needs is absent, the suite
fails and names the command that installs it. A skipped row reports green for a
check that did not run, which is the one failure mode this module's verification
exists to prevent. Do not add `pytest.importorskip`.

**One artifact carries one Properties form.** The typed table is the default and
the `sysml` fence is the alternate; they are separate files declaring the same
fields, never two blocks in one document.

## Adding or changing a type

1. Edit `typespec/main.tsp`.
2. Add or edit the type's entry in `{{ cookiecutter.package_name }}/manifest.yaml`
   — `data_schema` with `schema:` and a `digest:` placeholder, plus the
   `body_extraction` locators — and add it to `semantic.exports`.
3. Add the skeleton pair under `{{ cookiecutter.package_name }}/skeletons/`.
4. Add one negative fixture per failure mode the schema refuses, each with a
   distinct `expect` and a `because`.
5. Run `make schemas`, then `make gate`.
6. Bump `manifest.yaml`'s `version` and the `@jsonSchema` base in `main.tsp`
   **in the same commit** — `make schemas` fails when they disagree.

## Before opening a pull request

```bash
make gate
```

`quire validate --scope . "spec/**/*.md"` must be structurally clean, and the
Test Matrix in `spec/matrix.md` must be honest: a row that is not covered is
`🚧` with the reason, never `✅`. `⚠️` is not a valid `Status` marker.

## Releasing

Releases are manual. Tag `vX.Y.Z`, then dispatch the release workflow. Nothing
publishes automatically and no credential lives in this repository.
