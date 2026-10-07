# Adding {{ cookiecutter.repo_name }} to the Quoin catalog

This module is not in the default catalog until somebody puts it there. These are
the steps, so the next person does not have to reconstruct them.

## 1. Publish the package

Tag `vX.Y.Z` and dispatch `.github/workflows/release-npm.yml`. It delegates to
the organization's shared reusable workflow and publishes
`@{{ cookiecutter.org }}/{{ cookiecutter.repo_name }}` to the public npm registry
through OIDC Trusted Publishing. No token is stored here.

**Dry-run first.** Run `make pack` locally and inspect the tarball: its root must
be the module root — `manifest.yaml` at the top, with `schemas/` and
`skeletons/` beside it — because that is how a Filament tool discovers a module.

## 2. Verify the install resolves

From a clean environment:

```bash
quoin plugin install npm:@{{ cookiecutter.org }}/{{ cookiecutter.repo_name }}
quoin catalog list
```

Every type this module exports must appear.

## 3. Add it to the tracking project

Add the repository to the organization's module tracking project (GitHub
Project 18, "Quoin work") so its schema-completion and contract-migration work is
visible beside the rest of the fleet. Give it a `Track` and a board state; a
repository nobody can see on the board is a repository whose drift nobody
notices.
