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

## 2. Add the catalog entry

In the `agent-ix/quoin` repository, `default-modules.yaml` lists the modules
Quoin reconciles into `~/.ix/filament/modules`. Add an entry pinned to the exact
version you published:

```yaml
- name: {{ cookiecutter.module_name }}
  source: npm:@{{ cookiecutter.org }}/{{ cookiecutter.repo_name }}
  version: "X.Y.Z"
```

Pin the exact version. A moving reference makes two machines resolve different
schemas under the same module name, and the digests in this module's manifest
then describe bytes one of them never read.

Open that change as its own pull request against `agent-ix/quoin`, and say in the
description which types the module contributes and which schema digests they
reference.

## 3. Verify the install resolves

From a clean environment:

```bash
quoin plugin install npm:@{{ cookiecutter.org }}/{{ cookiecutter.repo_name }}
quoin catalog list
```

Every type this module exports must appear, and `quoin catalog show <type>` must
report the schema path and digest this repository's manifest declares. If it
reports something else, the published tarball and this repository disagree — stop
and reconcile them rather than editing either to match.

## 4. Add it to the tracking project

Add the repository to the organization's module tracking project (GitHub
Project 18, "Quoin work") so its schema-completion and contract-migration work is
visible beside the rest of the fleet. Give it a `Track` and a board state; a
repository nobody can see on the board is a repository whose drift nobody
notices.

## 5. Record the decision

Add a line to `spec/log.md` here naming the published version, the catalog pull
request, and the date. The catalog entry and this repository are two halves of
one fact, and the log is what lets a later reader tell whether they still agree.
