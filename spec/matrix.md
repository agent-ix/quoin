---
id: TM-001
title: quoin Test Matrix
type: TestMatrix
---

# quoin Test Matrix

Tests live in `tests/` and run under vitest (`make test` → `vitest run`).
Coverage is mapped requirement → test as `file :: "test name"`:

- `cli.test.ts` / `cli-version-missing.test.ts` — `main()` dispatch, arg
  grammar, version, help, config root, `runCatalog`/`runPlugin`/`runWrite`.
- `catalog.test.ts` — `defaultModuleRoots`, `loadCatalog`, `findCatalogEntry`,
  `findDuplicates`, module-root location.
- `write.test.ts` — `parseTypeList`, `createAuthoringPack`,
  `formatAuthoringPack`, `shellQuote`.
- `plugins.test.ts` — `parseSourceArg`, install/list/remove, `readModuleName`.
- `modules` coverage via `index.test.ts` (default-set reconcile + manifest).
- `flows.test.ts` / `flows-notfound.test.ts` — workflow launchers and the real
  fake `ix-flow` binary (exit 0 / non-zero / signal / spawn-error).
- `scripts.test.ts` — built CLI help/version surface.
- `update.test.ts` — `runUpdate` delegation to ix-cli-core's `runSelfUpdate`
  (package coordinates, `--check`, `--registry`); module fully mocked.
- `index.test.ts` — the public library surface end to end.

> Coverage gate: `vite.config.ts` declares 100% v8 thresholds
> (branches/functions/lines/statements). `make test` (`vitest run`) passes all
> 102 tests and `pnpm run test:coverage` passes the gate at **100%**
> (branches/functions/lines/statements).

## Functional Requirements

| Requirement | Coverage   | Test (file :: name)                                                                                                                                                                                                                                                                                                                                                                                                             |
| ----------- | ---------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| FR-001      | ✅ Covered | `cli.test.ts` :: "long flag with = and following-value form both work"; :: "boolean long flag (no value) is honored"; :: "subcommand is only captured for catalog/plugin; others become positionals"                                                                                                                                                                                                                            |
| FR-002      | ✅ Covered | `cli.test.ts` :: "--version, -v, and the version command all print the package version"; :: "returns a non-empty version string"; `cli-version-missing.test.ts` :: "throws when package.json has no string version"; `version.test.ts` :: "returns the baked version verbatim when present"; :: "falls back to the package.json version when nothing is baked in"                                                               |
| FR-003      | ✅ Covered | `cli.test.ts` :: "no command prints the top-level usage"; :: "--help and -h print command help"; :: "--help on an unrecognized command falls back to the top-level usage"; :: "help for a spec-flow command prints the workflow usage"                                                                                                                                                                                          |
| FR-004      | ✅ Covered | `cli.test.ts` :: "falls back to IX_HOME when --config-root is omitted, and prints 'unknown' for a versionless module"; :: "--no-project-config short-circuits project config root"                                                                                                                                                                                                                                              |
| FR-005      | ✅ Covered | `cli.test.ts` :: "throws on an unknown command"; :: "unknown catalog subcommand throws"; :: "unknown plugin subcommand throws"                                                                                                                                                                                                                                                                                                  |
| FR-006      | ✅ Covered | `catalog.test.ts` :: "discovers a manifest one level deep under a candidate dir"; :: "skips a candidate that is a file, not a directory"; :: "skips a non-manifest child while scanning one level deep, then finds the manifest sibling"; :: "skips candidates that do not resolve to a module root"                                                                                                                            |
| FR-007      | ✅ Covered | `catalog.test.ts` :: "includes QUOIN_MODULE_PATHS entries and installed module dirs"; :: "omits installed dirs when none have been installed"                                                                                                                                                                                                                                                                                   |
| FR-008      | ✅ Covered | `catalog.test.ts` :: "deduplicates repeated module roots and module names"                                                                                                                                                                                                                                                                                                                                                      |
| FR-009      | ✅ Covered | AC-1 `catalog.test.ts` :: "falls back to the directory basename when manifest has no name". AC-2 :: "handles modules with and without a version and artifacts with/without schemaRef". AC-3 :: "ignores non-array and malformed type entries". AC-4 `write.test.ts` :: "builds a pack with contracts and a non-quoted clean repo path"; :: "renders skeleton+schema, manifest-only, and object contracts"; `index.test.ts` :: "creates authoring packs for case-insensitive artifact and object types"; `catalog.test.ts` :: "resolves a skeleton named with a lowercase filename". AC-5 `catalog.test.ts` :: "resolves a skeleton named with the type's own casing"; :: "resolves no skeleton when only an unrelated casing is present"; :: "resolves no skeleton when the module ships no skeletons directory" |
| FR-010      | ✅ Covered | `index.test.ts` :: "creates authoring packs for case-insensitive artifact and object types" (asserts `["fr","DOMAIN"]` → `["FR","domain"]`)                                                                                                                                                                                                                                                                                     |
| FR-011      | ✅ Covered | `cli.test.ts` :: "list (explicit) prints one line per module"; :: "list --json prints the whole catalog as JSON"; :: "undefined subcommand behaves like list"; :: "show prints a single entry (text)"; :: "show --json prints the entry as JSON"; :: "show of a missing type throws"; :: "show without a type throws"                                                                                                           |
| FR-012      | ✅ Covered | `catalog.test.ts` :: "reports a type declared by two modules"; :: "reports no duplicates when type names are unique"; `cli.test.ts` :: "validate reports duplicates on stderr and sets exit code 1"; :: "validate succeeds with no duplicates (text)"; :: "validate --json succeeds with no duplicates"                                                                                                                         |
| FR-013      | ✅ Covered | `write.test.ts` :: "throws when no type names are given"; :: "throws when repo_dir is not a directory"; :: "throws when repo_dir is a file (exists but not a directory)"; :: "throws with available type list when a type is not found"; `cli.test.ts` :: "missing repo_dir throws"; `write.test.ts` parseTypeList suite                                                                                                        |
| FR-014      | ✅ Covered | `write.test.ts` :: "renders an authoring pack as text"; :: "renders an authoring pack as JSON with --json"; :: "renders skeleton+schema, manifest-only, and object contracts"; :: "does not emit a manifest-only line when a contract has artifacts"                                                                                                                                                                            |
| FR-015      | ✅ Covered | `write.test.ts` :: "builds a pack with contracts and a non-quoted clean repo path"; :: "quotes a repo path containing a space (shellQuote quoting branch)"; `index.test.ts` :: validation.command asserts `quire validate --scope`                                                                                                                                                                                              |
| FR-016      | ✅ Covered | `index.test.ts` :: "ships the committed default module set" (asserts `default-modules.yaml` validates as a marketplace manifest with the expected entries)                                                                                                                                                                                                                                                                      |
| FR-017      | ✅ Covered | `index.test.ts` :: "lazily installs the default module set, then loads its artifacts and objects"; `cli.test.ts` :: "ensure-defaults runs the installer and reports the registry"; :: "ensure-defaults reports installed plugin names from a non-empty registry"                                                                                                                                                                |
| FR-018      | ✅ Covered | `plugins.test.ts` :: "path: prefix"; :: "github: with @ref"; :: "github: without ref leaves ref undefined"; :: "github: with //subdir maps to a git-subdir source (with ref)"; :: "github: with //subdir and no ref leaves ref undefined"; :: "package: npm with @version"; :: "package: npm without version"; :: "scoped package: npm keeps the scope @, splits on the last @"; :: "bare argument falls back to a path source" |
| FR-019      | ✅ Covered | `plugins.test.ts` :: "install adds a plugin from a path source"; :: "installs, lists, then removes a plugin and its target dir + registry entry"; readModuleName suite ("reads the name from a top-level manifest.yaml"…); `cli.test.ts` :: "install without a source throws"; :: "remove without a name throws"; :: "remove deletes a plugin and prints confirmation"                                                          |
| FR-020      | ✅ Covered | `flows.test.ts` :: "lists the bundled spec flows"; :: "throws for an unknown flow name"; `flows-notfound.test.ts` :: "throws when no candidate root contains the skill"                                                                                                                                                                                                                                                         |
| FR-021      | ✅ Covered | `flows.test.ts` :: "resolves when ix-flow exits 0; builds id/json/target args"; :: "matrix runs the flow and propagates a non-zero exit code"; :: "defaults exit code to 1 when ix-flow is killed by a signal (null code)"; :: "rejects when ix-flow cannot be spawned (PATH has no ix-flow)"; `cli.test.ts` :: "review with --target/--json/--id runs the flow (ix-flow exit 0)"                                               |
| FR-022      | ✅ Covered | `update.test.ts` :: "delegates to runSelfUpdate with quoin's package coordinates" (asserts packageName `@agent-ix/quoin`, currentVersion, header "quoin update", registry defaults to `https://registry.npmjs.org/`, check false); :: "passes --check through"; :: "passes a custom --registry through"; :: "defaults the registry to public npm when no --registry is given"                                                   |

| FR-023 | ✅ Covered | `cli.test.ts` :: "falls back to IX_HOME when --config-root is omitted, and prints 'unknown' for a versionless module"; `catalog.test.ts` :: "includes QUOIN_MODULE_PATHS entries and installed module dirs"; `flows.test.ts` :: "resolves when ix-flow exits 0; builds id/json/target args" (IX_SPEC_WORKFLOWS_ROOT) |

## Non-Functional Requirements

| Requirement | Coverage   | Test / Evidence                                                                                                                                                                                                                                                              |
| ----------- | ---------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| NFR-001     | ✅ Covered | `index.test.ts` :: "lazily installs the default module set…" runs `ensureDefaultModules` against path-source fixtures with `mode: "lazy"` and no network/git.                                                                                                                |
| NFR-002     | ✅ Covered | `catalog.test.ts` :: "deduplicates repeated module roots and module names"; :: "reports a type declared by two modules" (duplicate module lists are sorted, ordering is first-wins).                                                                                         |
| NFR-003     | ✅ Covered | `write.test.ts` :: "throws with available type list when a type is not found"; `cli.test.ts` :: "throws on an unknown command"; `plugins.test.ts` :: "throws when no manifest.yaml exists"                                                                                   |
| NFR-004     | ⚠️ Review  | Standalone-dependency claim verified by inspecting `package.json` runtime deps (`ix-cli-core`, `ts-plugin-kit`, `yaml`); `ix-flow`/`quire` are spawned, not imported. No automated assertion.                                                                                |
| NFR-005     | ⚠️ Review  | Workflow launchers reference catalog modules via `flows.ts` and the bundled skills; no automated assertion. Verified by review.                                                                                                                                              |
| NFR-006     | ⚠️ Review  | The agent-pty harness (`evals/run.mjs`) records latency, tokens, tool calls, validation attempts, and context fetches from the Claude Code transcript; defined in `spec/evals.md`, implemented in `evals/`.                                                                  |
| NFR-007     | ✅ Covered | `flows.test.ts` :: "rejects when ix-flow cannot be spawned (PATH has no ix-flow)"; :: "sets process.exitCode when ix-flow exits non-zero"; `write.test.ts` validation-command tests confirm `quire` is emitted, not executed. (Version pinning is an accepted gap — Review.) |
| NFR-008     | ✅ Covered | `catalog.test.ts` :: "skips candidates that do not resolve to a module root" (missing-manifest skip); :: "aborts (strict) on a present but unparseable manifest.yaml" (strict-abort path).                                                                                   |

## Use Case Coverage

| Use Case | Coverage   | Test / Evidence                                                                                                                                                      |
| -------- | ---------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| US-001   | ✅ Covered | `index.test.ts` authoring-pack test; `write.test.ts` `formatAuthoringPack` suite; EV-001/EV-006/EV-014 in `spec/evals.md`                                            |
| US-002   | ✅ Covered | `cli.test.ts` `runCatalog` show suite; `scripts.test.ts` catalog/write help; `index.test.ts` case-insensitive lookup; EV-002/EV-007                                  |
| US-003   | ⚠️ Partial | `plugins.test.ts` + `index.test.ts` install/list/remove (path source only). GitHub/subdir install is exercised only at the eval layer (EV-003/EV-009/EV-010/EV-020). |
| US-004   | ✅ Covered | `write.test.ts` validation-command assertions; `index.test.ts` `validation.command`; EV-004/EV-008/EV-012                                                            |
| US-005   | ✅ Covered | `flows.test.ts` launchers end-to-end (real fake `ix-flow`); `cli.test.ts` `review`/`matrix` dispatch; EV-005/EV-013                                                  |
| US-006   | ✅ Covered | `catalog.test.ts` :: "reports a type declared by two modules"; `cli.test.ts` :: "validate reports duplicates on stderr and sets exit code 1"                         |

## Eval Coverage

The agent-facing eval set (EV-001…EV-020) is defined in `spec/evals.md`
(Matrix-002) and implemented by the agent-pty-driven harness in `evals/`
(`make evals` for canaries, `make evals-all` for the full set). It drives the
real agent through this CLI + the `/spec-*` skills and records real metrics from
the Claude Code transcript. This unit-test matrix does not duplicate that table.

## Integration Tests

The integration tests cover the two live-git boundaries `quoin` owns. Both are
REAL (live git) and currently **spec-ahead-of-code**: the deterministic
network-git tests they describe do not yet exist in `tests/`, and the same
boundaries are exercised non-deterministically by the agent-pty evals.

| Integration Test | Coverage              | Evidence / Related evals                                                                                                                                                                                                 |
| ---------------- | --------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| IT-001           | ⚠️ Spec-ahead-of-code | Default-module reconcile from pinned git tags. Unit tests use path-source fixtures (`index.test.ts`) and the evals seed modules into `evals/.seed-cache/`; no deterministic live first-run-then-offline test exists yet. |
| IT-002           | ⚠️ Spec-ahead-of-code | `github://` plugin install. Unit tests cover path sources only (`plugins.test.ts`); live GitHub install is exercised at the eval layer (EV-003/EV-009/EV-010/EV-020).                                                    |

## Backsync Notes

- **Spec overhaul (2026-06-21):** the functional and non-functional requirements
  are now discrete artifact files under `spec/functional/` and
  `spec/non-functional/`, replacing the requirement tables formerly embedded in
  `spec.md`. FR-001…FR-022 and NFR-001…NFR-008 keep their IDs and semantics
  (unchanged test mappings above); FR-023 (runtime configuration surface), a
  stakeholder layer (StR-001…StR-006), and an integration-test layer
  (IT-001/IT-002) were added. `spec.md` is now the master-requirements index.
- Requirements were renumbered and expanded in the spec overhaul to be a
  faithful, atomic backport of `src/` (`cli`, `catalog`, `write`, `plugins`,
  `modules`, `flows`). The prior coarse FR-001…FR-011 set is superseded by the
  capability-grouped FR-001…FR-021 / NFR-001…NFR-006 set in `spec.md`.
- The previous meta-requirement "the spec SHALL define eval scenarios" was
  dropped from the functional set; eval metric capture is now NFR-006 and the
  scenarios live in `spec/evals.md`.
- US trace targets were re-pointed to the new FR IDs and US-006 (duplicate
  detection) was added to cover `catalog validate`.
- **Strict-abort (NFR-008) — RESOLVED:** `catalog.test.ts` :: "aborts (strict)
  on a present but unparseable manifest.yaml" now covers the corrupt-manifest
  path alongside the missing-manifest skip.
- **Coverage gate (FR-017) — RESOLVED:** `cli.test.ts` :: "ensure-defaults
  reports installed plugin names from a non-empty registry" exercises the
  `(p) => p.name` mapping; `pnpm run test:coverage` now passes at 100%.
