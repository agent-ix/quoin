# {{ cookiecutter.repo_name }}

{{ cookiecutter.description }}

A Quire **semantic module**: it contributes typed declaration schemas, authoring
skeletons and extraction mappings to the Filament catalog. Quoin installs it,
Quire validates artifacts against it, and the shared compiler reads its emitted
JSON Schemas.

Licensed **{{ cookiecutter.license }}**. See [LICENSE](./LICENSE).

## What this module declares

| Type | `type:` | Schema |
| --- | --- | --- |
{%- if cookiecutter.module_kind in ("object", "mixed") %}
| Element | `element` | `{{ cookiecutter.package_name }}/schemas/Element.json` |
{%- endif %}
{%- if cookiecutter.module_kind in ("artifact", "mixed") %}
| Note | `note` | `{{ cookiecutter.package_name }}/schemas/Note.json` |
{%- endif %}

These are the template's worked examples, not this module's vocabulary. Replace
them: edit `typespec/main.tsp`, `{{ cookiecutter.package_name }}/manifest.yaml`
and the skeletons together, then run `make schemas`.

## The contract this module carries

`{{ cookiecutter.package_name }}/manifest.yaml` declares a `semantic` block at
contract version `1.0.0`: the exact `@agent-ix/semantic-core` version it imports,
the package identity it exports under, its exports and imports, its generated
targets, its mappings, `compatibility_posture: additive` and
`legacy_forms: warning`. Every exported type references its emitted schema by
path **and** digest — never an inline `{type: object}` placeholder, and never a
digest anybody typed.

### Generated targets

`semantic.targets` declares `{{ cookiecutter.generated_targets }}`.

**Only `json-schema` is emitted today.** Every other declared target is recorded
as declared and not emitted: the emitters for `rust`, `typescript`,
`python-pydantic-v2` and `python-dataclass` belong to
`agent-ix/filament-core-data#11` and do not exist yet. The Test Matrix in
`spec/matrix.md` carries a `🚧` row saying so. A declared target is not an
emitted one, and this file will not pretend otherwise.

## Getting started

```bash
make bootstrap    # install the pinned toolchain, emit schemas/ and the manifest digests
make dev-quire    # install the Quire wheel the semantic tests need
make gate         # spec validation, lint, schema drift check, tests
```

`make gate` is the green bar. It **fails** without the Quire wheel exposing
`extract_semantic` and names `make dev-quire` in the failure. It does not skip:
a skipped row is not coverage, and the environment where a skip fires is exactly
the clean runner where a regression would first show.
`agent-ix/quire-rs#392` tracks publishing that wheel to an index this repository
may depend on; until it closes, the engine is provisioned out of band and is
deliberately absent from `pyproject.toml`.

`toolchain.yaml` records every external command this repository invokes and the
version it must be at or above.

## Commands

| Command | Does |
| --- | --- |
| `make bootstrap` | Install dependencies, then emit schemas and digests |
| `make install` | Python and Node dependencies |
| `make dev-quire` | Install the Quire wheel the semantic tests need |
| `make gate` | Validate, lint, schema drift check, tests |
| `make validate` | `quire validate` over `spec/` |
| `make schemas` | Emit `{{ cookiecutter.package_name }}/schemas/` from `typespec/main.tsp` |
| `make schemas-check` | Fail on schema, toolchain or digest drift |
| `make test` | The verification suite |
| `make build` / `make pack` | Python distribution / npm tarball |

## How this module is consumed

Quoin installs it into the local module set:

```bash
quoin plugin install npm:@{{ cookiecutter.org }}/{{ cookiecutter.repo_name }}
```

Quire then validates artifacts against it:

```bash
quire validate --scope . "spec/**/*.md"
```

`docs/catalog-entry.md` names the steps to add this module to the Quoin default
catalog and to the tracking project.

## Registry configuration

This repository ships **no `.npmrc`**. `@agent-ix` resolves from the user-level
npm configuration, which is where a registry choice belongs — a per-repository
`.npmrc` is a credential surface and a second place for the answer to drift.

## Contributing

See [CONTRIBUTING.md](./CONTRIBUTING.md) and [SECURITY.md](./SECURITY.md).
Generated from the Quoin semantic-module template (`agent-ix/quoin#307`).
