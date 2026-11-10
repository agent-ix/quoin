---
id: SR-20261103118157
title: code review of the shared semantic-module emit package
type: SpecReview
analysis: code-review
scope: agent-ix/quoin; packages/semantic-module-tools/; templates/semantic-module/; rust/crates/quoin-semantic/tests/tc_264_template_render.rs
review_set: subset
date: '2026-11-10'
---

# code review of the shared semantic-module emit package

## Summary

Ticket: PLAT-195 (agent-ix/quoin PR #793, head a97402df). Scope is `git diff origin/main...HEAD`.

The PR moves the template's `scripts/generate-schemas.mjs` into `@agent-ix/semantic-module-tools`, with a library entry point and a `semantic-module-emit` bin. The template now depends on the package at `0.1.0` and its poe `schemas` and `schemas-check` tasks call the bin. Includes a Rust lane for `tc_264_056`.

## Scope

- `packages/semantic-module-tools/src/emit.mjs` (examined): compared line by line with origin/main `templates/.../scripts/generate-schemas.mjs`
- `packages/semantic-module-tools/src/cli.mjs` (examined)
- `packages/semantic-module-tools/bin/semantic-module-emit.mjs` (examined)
- `packages/semantic-module-tools/package.json` (examined)
- `packages/semantic-module-tools/README.md` (examined)
- `templates/semantic-module/{{cookiecutter.repo_name}}/package.json` (examined)
- `templates/semantic-module/{{cookiecutter.repo_name}}/pyproject.toml` (examined)
- `templates/semantic-module/{{cookiecutter.repo_name}}/tests/test_schema_emission.py` (examined)
- `templates/semantic-module/conformance.yaml` (examined)
- `rust/crates/quoin-semantic/tests/tc_264_template_render.rs` tc_264_056 (examined)

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | The template pins `@agent-ix/semantic-module-tools` at 0.1.0, but no registry has that package (npmjs returns E404) and no quoin workflow publishes `packages/`. Once this merges, `make install` and `make bootstrap` fail in every newly rendered repository until 0.1.0 is published. | templates/semantic-module/{{cookiecutter.repo_name}}/package.json:40 |
| FND-002 | medium | The CLI now takes the repository root from the current directory, where the old driver took it from its own location. `readIdentity` also reads `package.json` without a guard. Running the bin from a subdirectory therefore crashes with an uncaught ENOENT stack trace (reproduced) instead of a GenerateError message. | packages/semantic-module-tools/src/cli.mjs:17-25 |
| FND-003 | low | tc_264_056 checks the literal string "0.1.0". That is version tracking, so every release bump has to edit the test. FR-076-CON-4 requires an exact version, so the test should check that the value is an exact semver (no range operator). | rust/crates/quoin-semantic/tests/tc_264_template_render.rs:1047 |

## Details

- **FND-001**: `npm view @agent-ix/semantic-module-tools --registry https://registry.npmjs.org` returns 404. `release.yml` publishes only the native quoin distribution. The fix is to publish 0.1.0 before the template change ships, or together with it. That publish needs a path, either a workflow or a manual `npm publish` from `packages/semantic-module-tools`. Users resolve `@agent-ix` through their own npm configuration, so 0.1.0 must also be reachable on whatever registry that configuration points to. The end-to-end build below meets the ticket's condition that a rendered variant is built against the package before it is published.
- **FND-002**: Repro: `cd spec_objects_example && npx --no-install semantic-module-emit --package-dir spec_objects_example --check` gives `Error: ENOENT ... spec_objects_example/package.json`, raised from `readIdentity`. The poe tasks run from the project root, so `make schemas` is not affected. Fix: wrap the read in a GenerateError that names the path and suggests `--repo-root`.
- **FND-003**: Delete the literal-version assertion and assert the exact-version form instead.

## Verdict

I checked the move by comparing the code and by running it end to end. Apart from parameterisation, the logic in `emit.mjs` matches the deleted driver:

- `moduleBase` now takes its identity as a parameter.
- `semanticCoreBase` gains an optional explicit version.
- `importedBases` takes the manifest text.
- The namespace filter is extracted as `filterModuleSchemas`.
- `check` and `write` become `checkOutput` and `writeOutput`, with the output directory passed in.

Defaults are correct. `--identity` defaults to the package.json name without the `@`, which equals the old `{{org}}/{{repo_name}}` substitution. `--package-dir` defaults to the substituted package name in pyproject. `--manifest` defaults to `<package-dir>/manifest.yaml` and `--source-dir` to `<repo-root>/typespec`.

End to end, using the toolchain from npmjs and the package installed from `npm pack`:

- I rendered the object, artifact and mixed variants (mixed with one import) from origin/main and from this PR. For every variant the old `node scripts/generate-schemas.mjs` and the new `npx --no-install semantic-module-emit --package-dir spec_objects_example` wrote byte-identical `schemas/` (diff -r clean).
- `--check` is green on clean output. It is red, exit 1, when a byte changes, naming the file, and also on a stale extra file.
- An identity mismatch fails with the same message as before. A missing `--package-dir` exits 2 with usage.
- The rendered `tests/test_schema_emission.py` passes (7 tests).
- The package's `node --test` passes (4 tests).

Rust lane: tc_264_056 is a plain integration test with a trace tag and no production Rust. I did not run it, because building quoin-semantic needs the private build-inputs. I checked its three assertions by hand against all three renders.

No ceremony was added. TC-1456 and its byte-identity row are deleted, along with the conformance entry. The exact pin is an ordinary package.json dependency. Nothing is vendored: the package lives in its owning repo and consumers reach it as an npm dependency.

There is also a defect on origin/main, outside this diff. The template ships no lockfile, and `@typespec/json-schema@1.15.0` takes `@typespec/asset-emitter@^0.79`, which now resolves to 0.79.3, which needs compiler ^1.17. A fresh `npm install` therefore yields a toolchain whose `tsp compile` fails with "does not provide an export named 'sanitizePathSegment'". Both the old and new renders hit this. I worked around it in scratch only, with an `overrides` pin of asset-emitter 0.79.1.

Not mergeable as is: fix FND-001 (publish 0.1.0) and FND-002 first.

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-004 | low | The fix round adds `assert_eq!(package["overrides"]["@typespec/asset-emitter"], "0.79.1")` to tc_264_056. That brings back the literal-version assertion FND-003 removed: any compatible bump of the override has to edit the test. It also sits under the FR-076-CON-4 trace, and CON-4 says nothing about overrides. Assert instead that the override exists and is an exact version (the same shape check used for the emit package pin), or drop the assertion. | rust/crates/quoin-semantic/tests/tc_264_template_render.rs:1056-1059 |

## Dispositions

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | deferred | Not a code fix. Publishing `@agent-ix/semantic-module-tools` 0.1.0 is a release step owned by the lead and Peter. At disposition pass 1 (head 855e68ba) npmjs still returns E404 for the package. It must be published before or together with the merge, or newly rendered repositories fail `make install`. |
| FND-002 | fixed | `readIdentity` wraps the read and parse in a GenerateError that names the path and suggests `--repo-root`/`--identity`. A new test covers it (exit 1, stderr names package.json and --repo-root). |
| FND-003 | fixed | The literal "0.1.0" check is replaced by an exact-version shape check: three non-empty, all-digit, dot-separated parts. |
