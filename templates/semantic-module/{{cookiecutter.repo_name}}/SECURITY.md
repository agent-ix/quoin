# Security policy

## Reporting a vulnerability

Report suspected vulnerabilities privately through this repository's GitHub
Security Advisories ("Report a vulnerability" under the Security tab). If that
is unavailable, email {{ cookiecutter.email }} with the details and a way to
reach you.

Please do not open a public issue for a suspected vulnerability before it has
been triaged.

## What this repository is

A Filament module: declaration schemas, authoring skeletons and extraction
mappings. It carries **no runtime, no network client, and no credential**. The
realistic security surface is therefore supply-chain rather than execution:

- The npm package is published to the public registry through OIDC Trusted
  Publishing. No token is stored in this repository or in its workflows.
- Every emitted schema is reproducible from `typespec/main.tsp` by
  `make schemas`, and `make lint` fails when the committed bytes differ from what
  the source produces.
- Every exported type's `data_schema` carries a SHA-256 digest of the schema it
  names, so a consumer can detect a substituted schema.
- `toolchain.yaml` records every external command and its minimum version.

If you find a way for a published artifact to differ from what this repository's
source produces, that is a vulnerability in the sense that matters here.

## Supported versions

The most recent published version is supported. Older versions are not patched;
their schema URLs remain valid and immutable by design.
