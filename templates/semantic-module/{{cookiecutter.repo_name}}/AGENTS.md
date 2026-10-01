# Agent guide — {{ cookiecutter.repo_name }}

A Quire semantic module. It is **data**: declaration schemas, authoring
skeletons, extraction mappings. There is no runtime here.

## Always use the Makefile

`make help` lists everything. The ones that matter:

- `make bootstrap` — install the toolchain and emit `schemas/`.
- `make gate` — the green bar: spec validation, lint, schema drift check, tests.
- `make semantic-install` — npm ci for the pinned TypeSpec toolchain and semantic-core.

## Rules a change here must not break

- **Never hand-edit `{{ cookiecutter.package_name }}/schemas/`.** It is emitted from `typespec/main.tsp`. Fix the `.tsp` and run `make schemas`.
- **Never make a test skip.** If a tool is missing, the suite fails naming the install command. A skipped row is not coverage. Do not reach for `pytest.importorskip`.
- **Never add an `.npmrc`.** `@agent-ix` resolves from the user-level npm configuration.
- **The `@jsonSchema` base in `main.tsp` carries no version.** A release bumps `manifest.yaml` `version` only.
- **Never hand-write a Test Matrix.** Tag each test with the acceptance-criterion ids it asserts (`@pytest.mark.trace("FR-001-AC-1")`); `quire matrix` computes the matrix from those tags.

## Where things live

| Path | What |
| --- | --- |
| `typespec/main.tsp` | The structural source. Everything under `schemas/` derives from it. |
| `{{ cookiecutter.package_name }}/manifest.yaml` | The module manifest and its `semantic` block. |
| `{{ cookiecutter.package_name }}/skeletons/` | One authoring skeleton per type, plus its `sysml` alternate. |
| `tests/fixtures/negative/` | One fixture per failure mode, each with a distinct `expect`. |
| `tests/fixtures/legacy/` | The pre-contract authoring form, accepted at `warning`. |
| `spec/` | This repository's own requirements and acceptance criteria. |
