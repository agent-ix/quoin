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

Unlike every other row in this matrix, these are traced by **tracking tag**
rather than by prose: each test carries its criterion's `row_id` in a `Trace:`
line and in its own name, so `gap-analysis` reconciles them by grep. 17 criteria
across FR-002, FR-005, FR-006, FR-008, FR-009, FR-010, FR-012, FR-013, FR-015,
FR-018 and FR-025 are covered this way, in addition to — not instead of — the
hand-written tests named in the tables above.

`tests/props/QUEUE.md` records the full run: what was emitted, the 31 criteria
that are single witnesses rather than properties, the 12 verified by another
method, and the 70 left to the second pass.

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
