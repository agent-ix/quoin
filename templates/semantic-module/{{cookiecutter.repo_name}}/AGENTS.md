# Agent guide — {{ cookiecutter.repo_name }}

A Quire semantic module. It is **data**: declaration schemas, authoring
skeletons, extraction mappings. There is no runtime here.

## Always use the Makefile

`make help` lists everything. The ones that matter:

- `make bootstrap` — install the toolchain and emit `schemas/` plus the manifest digests.
- `make gate` — the green bar: spec validation, lint, schema drift check, tests.
- `make dev-quire` — install the Quire wheel the semantic tests need.

## Rules a change here must not break

- **Never hand-edit `{{ cookiecutter.package_name }}/schemas/`.** It is emitted from `typespec/main.tsp`. Fix the `.tsp` and run `make schemas`.
- **Never type a digest.** `make schemas` writes every `data_schema.digest`.
- **Never make a test skip.** If a tool is missing, the suite fails naming the install command. A skipped row is not coverage. Do not reach for `pytest.importorskip`.
- **Never add an `.npmrc`.** `@agent-ix` resolves from the user-level npm configuration.
- **Bump `manifest.yaml` `version` and the `@jsonSchema` base in `main.tsp` together.** `make schemas` fails when they disagree.
- **`⚠️` is not a Test Matrix `Status` marker.** The vocabulary is `✅ ❌ 🚧 ⛔`; a partially-covered row is `🚧` with the reason after it.

## Where things live

| Path | What |
| --- | --- |
| `typespec/main.tsp` | The structural source. Everything under `schemas/` derives from it. |
| `{{ cookiecutter.package_name }}/manifest.yaml` | The module manifest and its `semantic` block. |
| `{{ cookiecutter.package_name }}/skeletons/` | One authoring skeleton per type, plus its `sysml` alternate. |
| `tests/fixtures/negative/` | One fixture per failure mode, each with a distinct `expect`. |
| `tests/fixtures/legacy/` | The pre-contract authoring form, accepted at `warning`. |
| `toolchain.yaml` | Every external command and its minimum version. |
| `spec/` | This repository's own requirements and Test Matrix. |
