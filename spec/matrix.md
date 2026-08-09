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
| FR-001      | ✅ Covered | `cli.test.ts` :: "--config-root=<home> (eq form) and repeated/positional flags work"; :: "boolean --json flag (no value) is honored"; :: "subcommands are space-separated (topicSeparator)"                                                                                                                                                                                                                            |
| FR-002      | ✅ Covered | `cli.test.ts` :: "--version, -v, and the version command all print the package version"; :: "packageVersion returns a non-empty string"; `cli-version-missing.test.ts` :: "throws when package.json has no string version"; `version.test.ts` :: "returns the baked version verbatim when present"; :: "falls back to the package.json version when nothing is baked in"                                                               |
| FR-003      | ⚠️ Delegated | Help output is produced by the oclif runner, not by quoin code: the hand-rolled `helpFor`/USAGE strings were removed with the dispatcher, and per-command help now comes from each command class's `static summary`/`description`/`examples`. Those statics are asserted structurally by `cli.test.ts` :: "the runner discovers every migrated command (no legacy dispatcher)" and :: "the hand-rolled parseArgs dispatcher is gone from cli.ts"; the rendered help text itself is `@oclif/core`'s behaviour and is not re-tested here (see FR-026) |
| FR-004      | ✅ Covered | `cli.test.ts` :: "falls back to IX_HOME when --config-root is omitted, and prints 'unknown' for a versionless module"; :: "--no-project-config is accepted"                                                                                                                                                                                                                                              |
| FR-005      | ✅ Covered | `cli.test.ts` :: "an unknown command is rejected by the runner"; :: "an unknown subcommand is rejected by the runner"                                                                                                                                                                                                                                                                                                  |
| FR-006      | ✅ Covered | `catalog.test.ts` :: "discovers a manifest one level deep under a candidate dir"; :: "skips a candidate that is a file, not a directory"; :: "skips a non-manifest child while scanning one level deep, then finds the manifest sibling"; :: "skips candidates that do not resolve to a module root"                                                                                                                            |
| FR-007      | ✅ Covered | `catalog.test.ts` :: "includes QUOIN_MODULE_PATHS entries and installed module dirs"; :: "omits installed dirs when none have been installed"                                                                                                                                                                                                                                                                                   |
| FR-008      | ✅ Covered | `catalog.test.ts` :: "deduplicates repeated module roots and module names"                                                                                                                                                                                                                                                                                                                                                      |
| FR-009      | ✅ Covered | AC-1 `catalog.test.ts` :: "falls back to the directory basename when manifest has no name". AC-2 :: "handles modules with and without a version and artifacts with/without schemaRef". AC-3 :: "ignores non-array and malformed type entries". AC-4 `write.test.ts` :: "builds a pack with contracts and a non-quoted clean repo path"; :: "renders skeleton+schema, manifest-only, and object contracts"; `index.test.ts` :: "creates authoring packs for case-insensitive artifact and object types"; `catalog.test.ts` :: "resolves a skeleton named with a lowercase filename". AC-5 `catalog.test.ts` :: "resolves a skeleton named with the type's own casing"; :: "resolves no skeleton when only an unrelated casing is present"; :: "resolves no skeleton when the module ships no skeletons directory" |
| FR-010      | ✅ Covered | `index.test.ts` :: "creates authoring packs for case-insensitive artifact and object types" (asserts `["fr","DOMAIN"]` → `["FR","domain"]`)                                                                                                                                                                                                                                                                                     |
| FR-011      | ✅ Covered | `cli.test.ts` :: "list (explicit) prints one line per module"; :: "list --json prints the whole catalog as JSON"; :: "the catalog index command behaves like list"; :: "show prints a single entry (text)"; :: "show --json prints the entry as JSON"; :: "show of a missing type throws"; :: "show without a type throws"                                                                                                           |
| FR-012      | ✅ Covered | `catalog.test.ts` :: "reports a type declared by two modules"; :: "reports no duplicates when type names are unique"; `cli.test.ts` :: "validate reports duplicates on stderr and sets exit code 1"; :: "validate succeeds with no duplicates (text)"; :: "validate --json succeeds with no duplicates"                                                                                                                         |
| FR-013      | ✅ Covered | `write.test.ts` :: "throws when no type names are given"; :: "throws when repo_dir is not a directory"; :: "throws when repo_dir is a file (exists but not a directory)"; :: "throws with available type list when a type is not found"; `cli.test.ts` :: "missing repo_dir throws"; `write.test.ts` parseTypeList suite                                                                                                        |
| FR-014      | ✅ Covered | `write.test.ts` :: "renders an authoring pack as text"; :: "renders an authoring pack as JSON with --json"; :: "renders skeleton+schema, manifest-only, and object contracts"; :: "does not emit a manifest-only line when a contract has artifacts"                                                                                                                                                                            |
| FR-015      | ✅ Covered | `write.test.ts` :: "builds a pack with contracts and a non-quoted clean repo path"; :: "quotes a repo path containing a space (shellQuote quoting branch)"; `index.test.ts` :: validation.command asserts `quire validate --scope`                                                                                                                                                                                              |
| FR-016      | ✅ Covered | `index.test.ts` :: "ships the committed default module set" (asserts `default-modules.yaml` validates as a marketplace manifest with the expected entries)                                                                                                                                                                                                                                                                      |
| FR-017      | ✅ Covered | `index.test.ts` :: "lazily installs the default module set, then loads its artifacts and objects"; `cli.test.ts` :: "ensure-defaults runs the installer and reports the registry"; :: "ensure-defaults reports installed module names from a non-empty registry"                                                                                                                                                                |
| FR-018      | ✅ Covered | `plugins.test.ts` :: "path: prefix"; :: "github: with @ref"; :: "github: without ref leaves ref undefined"; :: "github: with //subdir maps to a git-subdir source (with ref)"; :: "github: with //subdir and no ref leaves ref undefined"; :: "package: npm with @version"; :: "package: npm without version"; :: "scoped package: npm keeps the scope @, splits on the last @"; :: "bare argument falls back to a path source" |
| FR-019      | ✅ Covered | `plugins.test.ts` :: "install adds a module from a path source"; :: "installs, lists, then removes a plugin and its target dir + registry entry"; readModuleName suite ("reads the name from a top-level manifest.yaml"…); `cli.test.ts` :: "install without a source throws"; :: "remove without a name throws"; :: "remove deletes a module and prints confirmation"                                                          |
| FR-020      | ✅ Covered | `flows.test.ts` :: "lists the bundled spec flows"; :: "throws for an unknown flow name"; `flows-notfound.test.ts` :: "throws when no candidate root contains the skill"                                                                                                                                                                                                                                                         |
| FR-021      | ✅ Covered | `flows.test.ts` :: "resolves when ix-flow exits 0; builds id/json/target args"; :: "matrix runs the flow and propagates a non-zero exit code"; :: "defaults exit code to 1 when ix-flow is killed by a signal (null code)"; :: "rejects when ix-flow cannot be spawned (PATH has no ix-flow)"; `cli.test.ts` :: "review with --target/--json/--id runs the flow (ix-flow exit 0)"                                               |
| FR-022      | ✅ Covered | `update.test.ts` :: "delegates to runSelfUpdate with quoin's package coordinates" (asserts packageName `@agent-ix/quoin`, currentVersion, header "quoin update", registry defaults to `https://registry.npmjs.org/`, check false); :: "passes --check through"; :: "passes a custom --registry through"; :: "defaults the registry to public npm when no --registry is given"                                                   |

| FR-023 | ✅ Covered | `cli.test.ts` :: "falls back to IX_HOME when --config-root is omitted, and prints 'unknown' for a versionless module"; `catalog.test.ts` :: "includes QUOIN_MODULE_PATHS entries and installed module dirs"; `flows.test.ts` :: "resolves when ix-flow exits 0; builds id/json/target args" (IX_SPEC_WORKFLOWS_ROOT); AC-4 (`--org`/`QUOIN_ORG`, neither defaulting) via `org.test.ts` :: "prefers --org over QUOIN_ORG and the git remote"; :: "prefers QUOIN_ORG over the git remote when no flag is given"; `cli.test.ts` :: "--org reaches the pack in the text rendering"; :: "reports an unresolved org with the --org remedy" |

| FR-024 | ✅ Covered | `index.test.ts` :: "exports the quoin CLI entrypoint"; :: "lazily installs the default module set, then loads its artifacts and objects"; :: "creates authoring packs for case-insensitive artifact and object types"; :: "installs, lists, and removes a plugin from a local path source"; :: "parseSourceArg maps CLI prefixes to typed sources" |

| FR-025 | ✅ Covered | AC-1 `org.test.ts` :: "prefers --org over QUOIN_ORG and the git remote"; :: "prefers QUOIN_ORG over the git remote when no flag is given"; :: "falls through to the git remote when neither flag nor env is set"; :: "ignores a blank flag and a blank QUOIN_ORG rather than resolving to empty". AC-2 :: "parses the org from an SSH remote url"; :: "parses urls with no .git suffix and a trailing slash"; :: "reads the origin remote, not another remote that precedes it". AC-3 :: "parses the org from an HTTPS remote url"; :: "parses a self-hosted https url with a port". AC-4 :: "resolves to none when the repo has no .git/config"; :: "resolves to none when the config declares no origin remote"; :: "resolves to none when the origin url has no org/repo tail". AC-5 :: "never substitutes a default organization"; `write.test.ts` :: "reports unresolved with the remedy and no substituted value". AC-6 `write.test.ts` :: "carries an explicit --org value and its source"; :: "carries QUOIN_ORG when no flag is given"; :: "serializes the org into the --json rendering"; `cli.test.ts` :: "--org reaches the pack in the text rendering"; :: "--org reaches the pack under --json"; :: "reports an unresolved org with the --org remedy". AC-7 `org-no-subprocess.test.ts` :: "resolves the org from a git remote without executing any subprocess"; :: "reports unresolved without executing any subprocess". AC-8 `org.test.ts` :: "resolves through a .git file pointing at a worktree gitdir"; :: "yields no org when the .git file points nowhere useful"; :: "yields no org when .git is a file with no gitdir pointer". AC-9 :: "yields no org for a host-based url with no owner segment"; :: "yields no org for an absolute local path"; :: "yields no org for a relative local path"; :: "yields no org for a file:// url"; :: "yields no org for a home-relative path"; :: "reports unresolved rather than a wrong org for a local-path remote". AC-10 :: "qualifies by the segment immediately preceding the repository". AC-11 :: "matches the section name case-insensitively but the remote name exactly" |

| FR-026 | ✅ Covered | AC-1 `cli.test.ts` :: "the runner discovers every migrated command (no legacy dispatcher)". AC-2 :: "subcommands are space-separated (topicSeparator)". AC-3 :: "the hand-rolled parseArgs dispatcher is gone from cli.ts". AC-4 :: "an unknown command is rejected by the runner"; :: "an unknown subcommand is rejected by the runner". AC-5 :: "--version, -v, and the version command all print the package version"; :: "a non-version argv is handed to the runner, whose error propagates". AC-6 :: "a non-version argv is handed to the runner, whose error propagates". AC-7 `it-005-sync-discovery.test.ts` :: "filament-plan-sync is discovered as a CORE plugin contributing `sync`"; :: "the `sync` command resolves with no runtime install step"; :: "an existing host command (catalog) still resolves unchanged" |

| FR-027 | ✅ Covered | AC-1 `org.test.ts` :: "prefers a stored org over the git remote". AC-2 :: "prefers an explicit --org over a stored value". AC-3 :: "lets QUOIN_ORG layer over the stored value, reported as env". AC-4 :: "falls through to the git remote when nothing is stored"; :: "resolves to none when nothing is stored and there is no remote". AC-5 :: "ignores a malformed config rather than failing the command". AC-6 `config-schema.test.ts` :: "rejects an unrecognized key"; :: "rejects an empty org"; :: "accepts a non-empty org"; :: "accepts an absent org rather than defaulting one". AC-7 :: "declares the plugin id, schema, and env binding". AC-8 `cli.test.ts` :: "set stores an org that write then resolves"; :: "get resolves for a stored key without erroring"; :: "set rejects an unrecognized key"; :: "doctor reports on a clean config without failing"; :: "edit opens the config file through the shared handler"; `config-schema.test.ts` :: "get calls runConfigGet from the shared package"; :: "set calls runConfigSet from the shared package"; :: "edit calls runConfigEdit from the shared package"; :: "doctor calls runConfigDoctor from the shared package"; :: "each command registers quoin's schema before delegating". AC-9 `org.test.ts` :: "prefers a project-local org over the user-level one (FR-027-AC-9)"; :: "ignores the project layer when the invocation disables it (FR-027-AC-9)" |

| FR-028 | ✅ Covered | The `spec-correctness` skill (`skills/spec-correctness/`) is agent-facing, so it is verified at the eval layer rather than by vitest — EV-050…EV-053, implemented in `evals/scenarios/index.mjs` and run by `make evals-all`. AC-1/AC-3/AC-4 → EV-050. AC-2/AC-6/AC-7 → EV-051. AC-5 → EV-052. AC-8/AC-9/AC-11 → EV-053. AC-10/AC-12 → Inspection of `skills/spec-correctness/` (no framework name reaches any `spec/**` output; strategy selection is keyed on `property`, never on `shape`). The skill's own output on this repo is `tests/props/` + `tests/props/QUEUE.md`. |

## Non-Functional Requirements

| Requirement | Coverage   | Test / Evidence                                                                                                                                                                                                                                                              |
| ----------- | ---------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| NFR-001     | ✅ Covered | `index.test.ts` :: "lazily installs the default module set…" runs `ensureDefaultModules` against path-source fixtures with `mode: "lazy"` and no network/git.                                                                                                                |
| NFR-002     | ✅ Covered | `catalog.test.ts` :: "deduplicates repeated module roots and module names"; :: "reports a type declared by two modules" (duplicate module lists are sorted, ordering is first-wins).                                                                                         |
| NFR-003     | ✅ Covered | `write.test.ts` :: "throws with available type list when a type is not found"; `cli.test.ts` :: "an unknown command is rejected by the runner"; `plugins.test.ts` :: "throws when no manifest.yaml exists"                                                                                   |
| NFR-004     | ⚠️ Review  | Standalone-dependency claim verified by inspecting `package.json` runtime deps (`ix-cli-core`, `ts-plugin-kit`, `yaml`); `ix-flow`/`quire` are spawned, not imported. No automated assertion.                                                                                |
| NFR-005     | ⚠️ Review  | Workflow launchers reference catalog modules via `flows.ts` and the bundled skills; no automated assertion. Verified by review.                                                                                                                                              |
| NFR-006     | ⚠️ Review  | The agent-pty harness (`evals/run.mjs`) records latency, tokens, tool calls, validation attempts, and context fetches from the Claude Code transcript; defined in `spec/evals.md`, implemented in `evals/`.                                                                  |
| NFR-007     | ✅ Covered | `flows.test.ts` :: "rejects when ix-flow cannot be spawned (PATH has no ix-flow)"; :: "sets process.exitCode when ix-flow exits non-zero"; `write.test.ts` validation-command tests confirm `quire` is emitted, not executed. (Version pinning is an accepted gap — Review.) |
| NFR-008     | ✅ Covered | `catalog.test.ts` :: "skips candidates that do not resolve to a module root" (missing-manifest skip); :: "aborts (strict) on a present but unparseable manifest.yaml" (strict-abort path).                                                                                   |

## Functional Requirement Coverage

Generated from the tracking tags in the suite, not hand-maintained: every row
below is a criterion whose id appears in a real test. The prose tables further
down carry the per-test detail (`file :: "test name"`) this fixed-column form
cannot hold, and are kept for that reason.

Criteria absent here are verified by a method that produces no test — see
"Tracking-tag coverage".

| Functional Req | Acceptance Criteria | Test Cases | Coverage Status |
| --- | --- | --- | --- |
| FR-001 | FR-001-AC-1, FR-001-AC-2, FR-001-AC-3, FR-001-AC-4 | TC-001, TC-002, TC-003, TC-004 | ✅ Covered |
| FR-002 | FR-002-AC-1, FR-002-AC-2, FR-002-AC-3, FR-002-AC-4 | TC-005, TC-006, TC-007, TC-008 | ✅ Covered |
| FR-003 | FR-003-AC-2 | TC-009 | ✅ Covered |
| FR-004 | FR-004-AC-1, FR-004-AC-2, FR-004-AC-3 | TC-010, TC-011, TC-012 | ✅ Covered |
| FR-005 | FR-005-AC-1, FR-005-AC-2, FR-005-AC-3 | TC-013, TC-014, TC-015 | ✅ Covered |
| FR-006 | FR-006-AC-1, FR-006-AC-2, FR-006-AC-3, FR-006-AC-4 | TC-016, TC-017, TC-018, TC-019 | ✅ Covered |
| FR-007 | FR-007-AC-1, FR-007-AC-2, FR-007-AC-3 | TC-020, TC-021, TC-022 | ✅ Covered |
| FR-008 | FR-008-AC-1, FR-008-AC-2 | TC-023, TC-024 | ✅ Covered |
| FR-009 | FR-009-AC-1, FR-009-AC-2, FR-009-AC-3, FR-009-AC-4, FR-009-AC-5 | TC-025, TC-026, TC-027, TC-028, TC-029 | ✅ Covered |
| FR-010 | FR-010-AC-1, FR-010-AC-2 | TC-030, TC-031 | ✅ Covered |
| FR-011 | FR-011-AC-1, FR-011-AC-2, FR-011-AC-3, FR-011-AC-4 | TC-032, TC-033, TC-034, TC-035 | ✅ Covered |
| FR-012 | FR-012-AC-1, FR-012-AC-2, FR-012-AC-3, FR-012-AC-4 | TC-036, TC-037, TC-038, TC-039 | ✅ Covered |
| FR-013 | FR-013-AC-1, FR-013-AC-2, FR-013-AC-3, FR-013-AC-4 | TC-040, TC-041, TC-042, TC-043 | ✅ Covered |
| FR-014 | FR-014-AC-1, FR-014-AC-2, FR-014-AC-3, FR-014-AC-4 | TC-044, TC-045, TC-046, TC-047 | ✅ Covered |
| FR-015 | FR-015-AC-1, FR-015-AC-2, FR-015-AC-3 | TC-048, TC-049, TC-050 | ✅ Covered |
| FR-016 | FR-016-AC-1, FR-016-AC-2 | TC-051, TC-052 | ✅ Covered |
| FR-017 | FR-017-AC-1, FR-017-AC-2, FR-017-AC-3 | TC-053, TC-054, TC-055 | ✅ Covered |
| FR-018 | FR-018-AC-1, FR-018-AC-2, FR-018-AC-3, FR-018-AC-4, FR-018-AC-5 | TC-056, TC-057, TC-058, TC-059, TC-060 | ✅ Covered |
| FR-019 | FR-019-AC-1, FR-019-AC-2, FR-019-AC-3, FR-019-AC-4 | TC-061, TC-062, TC-063, TC-064 | ✅ Covered |
| FR-020 | FR-020-AC-1, FR-020-AC-2, FR-020-AC-3 | TC-065, TC-066, TC-067 | ✅ Covered |
| FR-021 | FR-021-AC-1, FR-021-AC-2, FR-021-AC-3, FR-021-AC-4 | TC-068, TC-069, TC-070, TC-071 | ✅ Covered |
| FR-022 | FR-022-AC-1, FR-022-AC-2, FR-022-AC-3 | TC-072, TC-073, TC-074 | ✅ Covered |
| FR-023 | FR-023-AC-1, FR-023-AC-2, FR-023-AC-3, FR-023-AC-4, FR-023-AC-5 | TC-075, TC-076, TC-077, TC-078, TC-079 | ✅ Covered |
| FR-024 | FR-024-AC-1, FR-024-AC-2, FR-024-AC-3 | TC-080, TC-081, TC-082 | ✅ Covered |
| FR-025 | FR-025-AC-1, FR-025-AC-2, FR-025-AC-3, FR-025-AC-4, FR-025-AC-5, FR-025-AC-6, FR-025-AC-7, FR-025-AC-8, FR-025-AC-9, FR-025-AC-10, FR-025-AC-11 | TC-083, TC-084, TC-085, TC-086, TC-087, TC-088, TC-089, TC-090, TC-091, TC-092, TC-093 | ✅ Covered |
| FR-026 | FR-026-AC-1, FR-026-AC-2, FR-026-AC-3, FR-026-AC-4, FR-026-AC-5, FR-026-AC-6, FR-026-AC-7 | TC-094, TC-095, TC-096, TC-097, TC-098, TC-099, TC-100 | ✅ Covered |
| FR-027 | FR-027-AC-1, FR-027-AC-2, FR-027-AC-3, FR-027-AC-4, FR-027-AC-5, FR-027-AC-6, FR-027-AC-7, FR-027-AC-8, FR-027-AC-9 | TC-101, TC-102, TC-103, TC-104, TC-105, TC-106, TC-107, TC-108, TC-109 | ✅ Covered |

## Test Case Summary

One test case per criterion carrying a tracking tag. `Type` is `Property` for the
generated tests under `tests/props/` and `Unit` for the rest.

| Test ID | Title | Type | Priority | Traces To | Status |
| --- | --- | --- | --- | --- | --- |
| TC-001 | FR-001-AC-1 covered by `cli.test.ts` | Unit | P1 | FR-001-AC-1 | ✅ |
| TC-002 | FR-001-AC-2 covered by `cli.test.ts` | Unit | P1 | FR-001-AC-2 | ✅ |
| TC-003 | FR-001-AC-3 covered by `cli.test.ts` | Unit | P1 | FR-001-AC-3 | ✅ |
| TC-004 | FR-001-AC-4 covered by `cli.test.ts` | Unit | P1 | FR-001-AC-4 | ✅ |
| TC-005 | FR-002-AC-1 covered by `cli.test.ts` | Unit | P1 | FR-002-AC-1 | ✅ |
| TC-006 | FR-002-AC-2 covered by `cli.test.ts` | Unit | P1 | FR-002-AC-2 | ✅ |
| TC-007 | FR-002-AC-3 covered by `cli-version-missing.test.ts` | Unit | P1 | FR-002-AC-3 | ✅ |
| TC-008 | FR-002-AC-4 covered by `props/fr-002.prop.test.ts` | Property | P1 | FR-002-AC-4 | ✅ |
| TC-009 | FR-003-AC-2 covered by `scripts.test.ts` | Unit | P1 | FR-003-AC-2 | ✅ |
| TC-010 | FR-004-AC-1 covered by `cli.test.ts` | Unit | P1 | FR-004-AC-1 | ✅ |
| TC-011 | FR-004-AC-2 covered by `cli.test.ts` | Unit | P1 | FR-004-AC-2 | ✅ |
| TC-012 | FR-004-AC-3 covered by `cli.test.ts` | Unit | P1 | FR-004-AC-3 | ✅ |
| TC-013 | FR-005-AC-1 covered by `cli-usage.test.ts`, `props/fr-005.prop.test.ts` | Property | P1 | FR-005-AC-1 | ✅ |
| TC-014 | FR-005-AC-2 covered by `props/fr-005.prop.test.ts` | Property | P1 | FR-005-AC-2 | ✅ |
| TC-015 | FR-005-AC-3 covered by `props/fr-005.prop.test.ts` | Property | P1 | FR-005-AC-3 | ✅ |
| TC-016 | FR-006-AC-1 covered by `catalog.test.ts` | Unit | P1 | FR-006-AC-1 | ✅ |
| TC-017 | FR-006-AC-2 covered by `catalog.test.ts` | Unit | P1 | FR-006-AC-2 | ✅ |
| TC-018 | FR-006-AC-3 covered by `catalog.test.ts` | Unit | P1 | FR-006-AC-3 | ✅ |
| TC-019 | FR-006-AC-4 covered by `props/fr-catalog.prop.test.ts` | Property | P1 | FR-006-AC-4 | ✅ |
| TC-020 | FR-007-AC-1 covered by `catalog.test.ts` | Unit | P1 | FR-007-AC-1 | ✅ |
| TC-021 | FR-007-AC-2 covered by `props/second-pass.prop.test.ts` | Property | P1 | FR-007-AC-2 | ✅ |
| TC-022 | FR-007-AC-3 covered by `catalog.test.ts` | Unit | P1 | FR-007-AC-3 | ✅ |
| TC-023 | FR-008-AC-1 covered by `props/fr-catalog.prop.test.ts` | Property | P1 | FR-008-AC-1 | ✅ |
| TC-024 | FR-008-AC-2 covered by `props/fr-catalog.prop.test.ts` | Property | P1 | FR-008-AC-2 | ✅ |
| TC-025 | FR-009-AC-1 covered by `catalog.test.ts` | Unit | P1 | FR-009-AC-1 | ✅ |
| TC-026 | FR-009-AC-2 covered by `catalog.test.ts` | Unit | P1 | FR-009-AC-2 | ✅ |
| TC-027 | FR-009-AC-3 covered by `catalog.test.ts` | Unit | P1 | FR-009-AC-3 | ✅ |
| TC-028 | FR-009-AC-4 covered by `catalog.test.ts`, `index.test.ts`, `write.test.ts` | Unit | P1 | FR-009-AC-4 | ✅ |
| TC-029 | FR-009-AC-5 covered by `catalog.test.ts`, `props/fr-catalog.prop.test.ts` | Property | P1 | FR-009-AC-5 | ✅ |
| TC-030 | FR-010-AC-1 covered by `index.test.ts` | Unit | P1 | FR-010-AC-1 | ✅ |
| TC-031 | FR-010-AC-2 covered by `props/fr-catalog.prop.test.ts` | Property | P1 | FR-010-AC-2 | ✅ |
| TC-032 | FR-011-AC-1 covered by `cli.test.ts` | Unit | P1 | FR-011-AC-1 | ✅ |
| TC-033 | FR-011-AC-2 covered by `cli.test.ts` | Unit | P1 | FR-011-AC-2 | ✅ |
| TC-034 | FR-011-AC-3 covered by `cli.test.ts` | Unit | P1 | FR-011-AC-3 | ✅ |
| TC-035 | FR-011-AC-4 covered by `cli.test.ts` | Unit | P1 | FR-011-AC-4 | ✅ |
| TC-036 | FR-012-AC-1 covered by `props/fr-catalog.prop.test.ts` | Property | P1 | FR-012-AC-1 | ✅ |
| TC-037 | FR-012-AC-2 covered by `props/fr-catalog.prop.test.ts` | Property | P1 | FR-012-AC-2 | ✅ |
| TC-038 | FR-012-AC-3 covered by `cli.test.ts` | Unit | P1 | FR-012-AC-3 | ✅ |
| TC-039 | FR-012-AC-4 covered by `cli.test.ts` | Unit | P1 | FR-012-AC-4 | ✅ |
| TC-040 | FR-013-AC-1 covered by `write.test.ts` | Unit | P1 | FR-013-AC-1 | ✅ |
| TC-041 | FR-013-AC-2 covered by `props/second-pass.prop.test.ts` | Property | P1 | FR-013-AC-2 | ✅ |
| TC-042 | FR-013-AC-3 covered by `props/second-pass.prop.test.ts` | Property | P1 | FR-013-AC-3 | ✅ |
| TC-043 | FR-013-AC-4 covered by `props/fr-write.prop.test.ts` | Property | P1 | FR-013-AC-4 | ✅ |
| TC-044 | FR-014-AC-1 covered by `write.test.ts` | Unit | P1 | FR-014-AC-1 | ✅ |
| TC-045 | FR-014-AC-2 covered by `write.test.ts` | Unit | P1 | FR-014-AC-2 | ✅ |
| TC-046 | FR-014-AC-3 covered by `write.test.ts` | Unit | P1 | FR-014-AC-3 | ✅ |
| TC-047 | FR-014-AC-4 covered by `write.test.ts` | Unit | P1 | FR-014-AC-4 | ✅ |
| TC-048 | FR-015-AC-1 covered by `write.test.ts` | Unit | P1 | FR-015-AC-1 | ✅ |
| TC-049 | FR-015-AC-2 covered by `props/fr-write.prop.test.ts` | Property | P1 | FR-015-AC-2 | ✅ |
| TC-050 | FR-015-AC-3 covered by `props/fr-write.prop.test.ts` | Property | P1 | FR-015-AC-3 | ✅ |
| TC-051 | FR-016-AC-1 covered by `index.test.ts` | Unit | P1 | FR-016-AC-1 | ✅ |
| TC-052 | FR-016-AC-2 covered by `index.test.ts` | Unit | P1 | FR-016-AC-2 | ✅ |
| TC-053 | FR-017-AC-1 covered by `index.test.ts` | Unit | P1 | FR-017-AC-1 | ✅ |
| TC-054 | FR-017-AC-2 covered by `cli.test.ts` | Unit | P1 | FR-017-AC-2 | ✅ |
| TC-055 | FR-017-AC-3 covered by `cli.test.ts` | Unit | P1 | FR-017-AC-3 | ✅ |
| TC-056 | FR-018-AC-1 covered by `props/second-pass.prop.test.ts` | Property | P1 | FR-018-AC-1 | ✅ |
| TC-057 | FR-018-AC-2 covered by `props/second-pass.prop.test.ts` | Property | P1 | FR-018-AC-2 | ✅ |
| TC-058 | FR-018-AC-3 covered by `props/second-pass.prop.test.ts` | Property | P1 | FR-018-AC-3 | ✅ |
| TC-059 | FR-018-AC-4 covered by `props/second-pass.prop.test.ts` | Property | P1 | FR-018-AC-4 | ✅ |
| TC-060 | FR-018-AC-5 covered by `props/fr-018.prop.test.ts` | Property | P1 | FR-018-AC-5 | ✅ |
| TC-061 | FR-019-AC-1 covered by `cli.test.ts` | Unit | P1 | FR-019-AC-1 | ✅ |
| TC-062 | FR-019-AC-2 covered by `plugins.test.ts` | Unit | P1 | FR-019-AC-2 | ✅ |
| TC-063 | FR-019-AC-3 covered by `cli.test.ts` | Unit | P1 | FR-019-AC-3 | ✅ |
| TC-064 | FR-019-AC-4 covered by `cli.test.ts` | Unit | P1 | FR-019-AC-4 | ✅ |
| TC-065 | FR-020-AC-1 covered by `flows.test.ts` | Unit | P1 | FR-020-AC-1 | ✅ |
| TC-066 | FR-020-AC-2 covered by `flows.test.ts` | Unit | P1 | FR-020-AC-2 | ✅ |
| TC-067 | FR-020-AC-3 covered by `flows-notfound.test.ts` | Unit | P1 | FR-020-AC-3 | ✅ |
| TC-068 | FR-021-AC-1 covered by `flows.test.ts` | Unit | P1 | FR-021-AC-1 | ✅ |
| TC-069 | FR-021-AC-2 covered by `flows.test.ts` | Unit | P1 | FR-021-AC-2 | ✅ |
| TC-070 | FR-021-AC-3 covered by `flows.test.ts` | Unit | P1 | FR-021-AC-3 | ✅ |
| TC-071 | FR-021-AC-4 covered by `flows.test.ts` | Unit | P1 | FR-021-AC-4 | ✅ |
| TC-072 | FR-022-AC-1 covered by `update.test.ts` | Unit | P1 | FR-022-AC-1 | ✅ |
| TC-073 | FR-022-AC-2 covered by `update.test.ts` | Unit | P1 | FR-022-AC-2 | ✅ |
| TC-074 | FR-022-AC-3 covered by `update.test.ts` | Unit | P1 | FR-022-AC-3 | ✅ |
| TC-075 | FR-023-AC-1 covered by `cli.test.ts` | Unit | P1 | FR-023-AC-1 | ✅ |
| TC-076 | FR-023-AC-2 covered by `catalog.test.ts` | Unit | P1 | FR-023-AC-2 | ✅ |
| TC-077 | FR-023-AC-3 covered by `flows.test.ts` | Unit | P1 | FR-023-AC-3 | ✅ |
| TC-078 | FR-023-AC-4 covered by `cli.test.ts`, `org.test.ts` | Unit | P1 | FR-023-AC-4 | ✅ |
| TC-079 | FR-023-AC-5 covered by `cli.test.ts` | Unit | P1 | FR-023-AC-5 | ✅ |
| TC-080 | FR-024-AC-1 covered by `index.test.ts` | Unit | P1 | FR-024-AC-1 | ✅ |
| TC-081 | FR-024-AC-2 covered by `index.test.ts` | Unit | P1 | FR-024-AC-2 | ✅ |
| TC-082 | FR-024-AC-3 covered by `index.test.ts` | Unit | P1 | FR-024-AC-3 | ✅ |
| TC-083 | FR-025-AC-1 covered by `org.test.ts` | Unit | P1 | FR-025-AC-1 | ✅ |
| TC-084 | FR-025-AC-2 covered by `org.test.ts`, `props/second-pass.prop.test.ts` | Property | P1 | FR-025-AC-2 | ✅ |
| TC-085 | FR-025-AC-3 covered by `org.test.ts`, `props/second-pass.prop.test.ts` | Property | P1 | FR-025-AC-3 | ✅ |
| TC-086 | FR-025-AC-4 covered by `org.test.ts` | Unit | P1 | FR-025-AC-4 | ✅ |
| TC-087 | FR-025-AC-5 covered by `org.test.ts`, `write.test.ts` | Unit | P1 | FR-025-AC-5 | ✅ |
| TC-088 | FR-025-AC-6 covered by `cli.test.ts`, `write.test.ts` | Unit | P1 | FR-025-AC-6 | ✅ |
| TC-089 | FR-025-AC-7 covered by `org-no-subprocess.test.ts` | Unit | P1 | FR-025-AC-7 | ✅ |
| TC-090 | FR-025-AC-8 covered by `org.test.ts` | Unit | P1 | FR-025-AC-8 | ✅ |
| TC-091 | FR-025-AC-9 covered by `org.test.ts`, `props/fr-025.prop.test.ts` | Property | P1 | FR-025-AC-9 | ✅ |
| TC-092 | FR-025-AC-10 covered by `org.test.ts`, `props/fr-025.prop.test.ts` | Property | P1 | FR-025-AC-10 | ✅ |
| TC-093 | FR-025-AC-11 covered by `org.test.ts`, `props/second-pass.prop.test.ts` | Property | P1 | FR-025-AC-11 | ✅ |
| TC-094 | FR-026-AC-1 covered by `cli.test.ts` | Unit | P1 | FR-026-AC-1 | ✅ |
| TC-095 | FR-026-AC-2 covered by `cli.test.ts` | Unit | P1 | FR-026-AC-2 | ✅ |
| TC-096 | FR-026-AC-3 covered by `cli.test.ts` | Unit | P1 | FR-026-AC-3 | ✅ |
| TC-097 | FR-026-AC-4 covered by `cli.test.ts` | Unit | P1 | FR-026-AC-4 | ✅ |
| TC-098 | FR-026-AC-5 covered by `cli.test.ts` | Unit | P1 | FR-026-AC-5 | ✅ |
| TC-099 | FR-026-AC-6 covered by `cli.test.ts` | Unit | P1 | FR-026-AC-6 | ✅ |
| TC-100 | FR-026-AC-7 covered by `it-005-sync-discovery.test.ts` | Unit | P1 | FR-026-AC-7 | ✅ |
| TC-101 | FR-027-AC-1 covered by `org.test.ts` | Unit | P1 | FR-027-AC-1 | ✅ |
| TC-102 | FR-027-AC-2 covered by `org.test.ts` | Unit | P1 | FR-027-AC-2 | ✅ |
| TC-103 | FR-027-AC-3 covered by `org.test.ts` | Unit | P1 | FR-027-AC-3 | ✅ |
| TC-104 | FR-027-AC-4 covered by `org.test.ts` | Unit | P1 | FR-027-AC-4 | ✅ |
| TC-105 | FR-027-AC-5 covered by `org.test.ts` | Unit | P1 | FR-027-AC-5 | ✅ |
| TC-106 | FR-027-AC-6 covered by `cli.test.ts`, `config-schema.test.ts`, `props/second-pass.prop.test.ts` | Property | P1 | FR-027-AC-6 | ✅ |
| TC-107 | FR-027-AC-7 covered by `config-schema.test.ts` | Unit | P1 | FR-027-AC-7 | ✅ |
| TC-108 | FR-027-AC-8 covered by `cli.test.ts`, `config-schema.test.ts` | Unit | P1 | FR-027-AC-8 | ✅ |
| TC-109 | FR-027-AC-9 covered by `org.test.ts` | Unit | P1 | FR-027-AC-9 | ✅ |

## Stakeholder Requirement Coverage

Every StR carries one validation criterion, verified by demonstration or
inspection rather than by a unit test — so these rows carry no tracking tag by
design (see "Tracking-tag coverage"). Added in response to SR-003 FND-002, which
found the stakeholder layer had no rows here at all.

| Stakeholder Req | Trace to US/FR         | Test/Validation                                                                                       | Coverage Status |
| --------------- | ---------------------- | ----------------------------------------------------------------------------------------------------- | --------------- |
| StR-001-VC-1    | US-009; FR-004, FR-023 | Demonstration — EV-001…EV-013 run the real CLI from an isolated `IX_HOME`; NFR-004 inspects the deps    | ✅ Covered      |
| StR-002-VC-1    | US-003; FR-018, FR-019 | Demonstration — EV-003/EV-009/EV-010/EV-020 install from local, GitHub and subdir sources              | ✅ Covered      |
| StR-003-VC-1    | US-004; FR-014, FR-015 | Demonstration — EV-001/EV-004/EV-008 author to the skeleton and validate with a real `quire`           | ✅ Covered      |
| StR-004-VC-1    | US-005; FR-020, FR-021 | Inspection — EV-005/EV-013 start runs and inspect status; resume/advance/gate progression is untested  | ⚠️ Partial      |
| StR-005-VC-1    | US-003; FR-016, FR-017 | Inspection — the default set is version-pinned; NFR-001 covers idempotent offline reconcile            | ✅ Covered      |
| StR-006-VC-1    | US-009; FR-022         | Demonstration — `update.test.ts` covers delegation, `--check` and `--registry`                          | ✅ Covered      |

## Use Case Coverage

| Use Case | Coverage   | Test / Evidence                                                                                                                                                      |
| -------- | ---------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| US-001   | ✅ Covered | `index.test.ts` authoring-pack test; `write.test.ts` `formatAuthoringPack` suite; EV-001/EV-006/EV-014 in `spec/evals.md`                                            |
| US-002   | ✅ Covered | `cli.test.ts` `runCatalog` show suite; `scripts.test.ts` catalog/write help; `index.test.ts` case-insensitive lookup; EV-002/EV-007                                  |
| US-003   | ⚠️ Partial | `plugins.test.ts` + `index.test.ts` install/list/remove (path source only). GitHub/subdir install is exercised only at the eval layer (EV-003/EV-009/EV-010/EV-020). |
| US-004   | ✅ Covered | `write.test.ts` validation-command assertions; `index.test.ts` `validation.command`; EV-004/EV-008/EV-012                                                            |
| US-005   | ✅ Covered | `flows.test.ts` launchers end-to-end (real fake `ix-flow`); `cli.test.ts` `review`/`matrix` dispatch; EV-005/EV-013                                                  |
| US-006   | ✅ Covered | `catalog.test.ts` :: "reports a type declared by two modules"; `cli.test.ts` :: "validate reports duplicates on stderr and sets exit code 1"                         |
| US-010   | ✅ Covered | `org.test.ts` resolution suites (precedence, both url forms, worktrees, owner-less remotes); `write.test.ts` "authoring pack organization" suite; `cli.test.ts` `--org` text/JSON/unresolved trio; `org-no-subprocess.test.ts` no-subprocess proof — see FR-025 |
| US-011   | ✅ Covered | EV-050…EV-053 in `evals/scenarios/index.mjs` — the unattended lane, the review gate, the gap-analysis handoff, and the refusals; see FR-028. The skill's own run on this repo is `tests/props/` (17 criteria) + `tests/props/QUEUE.md` |

## Property Test Layer

`tests/props/` holds property tests generated by the `spec-correctness` skill
([FR-028](./functional/FR-028-generate-property-tests-from-criteria.md)) from
`quire properties --scope . --json 'spec/**/*.md'`. They run under vitest with
fast-check as part of `make test` and count toward the same coverage gate.

**28 criteria** are covered by generated property tests: 17 emitted unattended,
and 11 recovered by the review-gated second pass over criteria the deterministic
classifier read as concrete examples (`tests/props/second-pass.prop.test.ts`).
These are in addition to — not instead of — the hand-written tests named in the
tables above.

## Tracking-tag coverage

The tables above trace requirements to tests as prose. That form is readable but
not greppable, so `gap-analysis` could not reconcile a single row against the
suite. Every criterion whose coverage is a test now also carries a **tracking
tag** — a `// Trace: FR-XXX-AC-N` comment, 115 of them across 15 test files.

**109 of 130 criteria are reconcilable by tag.** The remaining 21 are verified by
a method that produces no test: FR-003-AC-1/AC-3 (help rendering, delegated to
`@oclif/core`), FR-028-AC-1…AC-12 (evals EV-050…EV-053 and inspection),
NFR-007-AC-1 (an accepted limitation), and the six StR validation criteria
(demonstration).

`tests/props/QUEUE.md` records the full run and the per-criterion disposition.

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
