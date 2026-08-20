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

| FR-032 | ✅ Covered | `tests/auditor.test.ts` — TC-137 (healthy produces nothing), TC-138 (suspect link, high), TC-139 (missing run high vs behind-HEAD medium), TC-140 (vacuity needs EVERY symbol skipped/absent), TC-141 (undischarged at medium, not high), TC-142 (method conformance via the catalog; skipped without one), TC-143 (multiplicity measured in suites, not symbols), TC-144 (ratchet + per-PR delta), TC-145 (every check folds over all of an obligation's bindings), TC-146 (kind-to-kind conformance; `unknown-method` reported), TC-147 (every kind is baselineable), TC-148 (the catalog follows `--module`). CON-1/CON-2/CON-3 → TC-142's no-catalog case plus inspection of `src/auditor/` (pure functions, no clock, no subprocess, no write). |
| FR-033 | ✅ Covered | `tests/evidence-adapters.test.ts` — TC-151..TC-164. Five criteria are stated over `quoin evidence record --adapter <x> --results <file>` rather than over the parse function, because the P1 review found three of four P0 gaps were a matrix reading ✅ over a capability nothing could reach. CON-1/CON-2 → inspection of `src/evidence/adapters/` (no fs, no subprocess, no threshold). CON-3 → TC-156. CON-4 → TC-164. |
| FR-034 | ✅ Covered | `tests/finding-record.test.ts` — TC-165..TC-176. The cargo-audit criteria run against output captured with `cargo audit --json` and checked in unedited (`tests/fixtures/evidence/`), because a fixture written to match the reader only proves the reader parses itself. CON-1/CON-3 → inspection of `src/evidence/adapters/sarif.ts` and `src/evidence/store.ts`. CON-2 → TC-171. CON-4 → `readScans` mirrors `readRuns` ordering. |
| FR-035 | ✅ Covered | `tests/combinatorial-coverage.test.ts` — TC-180..TC-191. TC-183 pins agreement with quire-rs TC-925 on the same space: if the two ever disagree, an obligation is measured against a target it does not state. CON-1/CON-2 → inspection of `src/auditor/combinatorial.ts` (nothing is generated, nothing is declared). CON-3 → TC-188. |
| FR-042 | ✅ Covered | `tests/agent-eval.test.ts` — TC-240..TC-244. `agent-eval-real.json` is a real `cli-agent-evals` report, unedited — the TC-EV-057 run of the spec-fuzz scenarios. The multi-scenario, failing and empty cases are **constructed and labelled as such**: no failing report survived to be captured, and a fabricated one claiming to be real would be worse than saying which is which. CON-1/CON-3 → inspection of `src/evidence/adapters/agent-eval.ts` (no subprocess; an unmatched trace id is reported, never assumed). CON-2 → TC-242. |
| FR-041 | ✅ Covered | `tests/sbom.test.ts` — TC-231..TC-236. Both fixtures are **real tool output, unedited**: `cyclonedx-real.json` from `@cyclonedx/cyclonedx-npm` 6.0.1 over a real `npm install`, and `spdx-real.json` from GitHub's dependency-graph SBOM for `sindresorhus/slugify`. A fixture written to match the reader only proves the reader parses itself. CON-1/CON-2/CON-3 → inspection of `src/evidence/adapters/sbom.ts` (no subprocess, no new record type, no verdict). CON-4 → TC-231. |
| FR-040 | ✅ Covered | `tests/assurance.test.ts` — TC-221..TC-230. TC-224 and TC-225 are the ones that matter: a claim nothing argues for is `open`, and a requirement no claim reaches gets reported. Both were written before the code and both failed it. TC-225 earned itself on the first real run — 15 requirements over this repository, 7 of them added during this program and fixed, 8 pre-existing (`agent-ix/quoin#136`). CON-1/CON-3/CON-4 → inspection of `src/assurance/` (no subprocess, no write, the auditor's verdict used as given). CON-2 → TC-224 and TC-225. |
| FR-039 | ✅ Covered | `tests/auditor.test.ts` — TC-219 (the seven policy cases) and TC-220 (the flag, including the two refusals). TC-220 states AC-8/AC-9 over `quoin evidence audit --mutation-floor`, not over the auditor function, because the flag is where a percentage typed as `80` would otherwise become a floor nothing can reach. CON-1 → inspection of `src/auditor/` (no subprocess). CON-2 → TC-219's unset-floor case. CON-3 → TC-219's skipped-symbol and unmeasured cases. |
| FR-038 | ✅ Covered | The `spec-fuzz` skill (`skills/spec-fuzz/`) is agent-facing, so it is verified at the eval layer rather than by vitest — TC-EV-054…TC-EV-057, implemented in `evals/scenarios/index.mjs` and run by `make evals-all`. AC-1/AC-2/AC-5 → TC-EV-054. AC-3/AC-4 → TC-EV-055, which carries **two** repos because absent tooling and an ungroundable entry point are different refusals. AC-6/AC-7 → TC-EV-056. AC-8/AC-9 → TC-EV-057. CON-1 → TC-EV-054's fixture renames a catalog method, so matching on the name `fuzzing` fails the eval. CON-2 → TC-EV-055 `absentFiles`. CON-3 → TC-EV-057. CON-4 → TC-EV-056 `absentFiles`. |
| FR-037 | ✅ Covered | `tests/completeness.test.ts` — TC-206..TC-218. Five criteria are stated over `quoin completeness` rather than over `assessVocabulary`, per the P1 review lens. TC-218 asserts quoin's unowned set equals the bundle read's for the same declaration; the same figure was checked against the engine by hand — `quire validate --okf --scope .` reports 7 unowned characteristics for this repository and so does the command. TC-217 exists because the first draft printed `PASS` over a bundle nothing had checked. CON-1 → TC-206. CON-2/CON-3 → inspection of `src/completeness/` (no corpus walk, no archetype resolution, no schema validation). CON-4 → TC-217. |
| FR-036 | ✅ Covered | `tests/arch-conformance.test.ts` — TC-197..TC-201, TC-204, TC-205; `tests/arch-boundaries.test.ts` — TC-202, TC-203. TC-197 runs against the real captured output of every `quire-rs/scripts/audits/*.sh`, checked in unedited (`tests/fixtures/evidence/audit-static-real.txt`), because a fixture written to match the reader only proves the reader parses itself. TC-202/TC-203 are quoin's own boundaries and were each observed to fail against an injected violation before being relied on (CON-4). CON-1 → TC-203, which is the invariant stated over quoin itself. CON-2 → inspection of `src/evidence/adapters/audit-script.ts` (a parser, no engine). CON-3 → TC-204. |
| FR-031 | ✅ Covered | `tests/advisor.test.ts` — TC-129 (merged catalog, first-wins, collision reported, undeclared is empty), TC-130 (attack surface → DAST/SAST/negative-abuse), TC-131 (temporal → runtime monitoring + model checking), TC-132 (reliability → fault injection), TC-133 (round-trip → property-based + metamorphic; ranked by matching-rule count), TC-134 (mismatch flagged; class match accepted), TC-135 (inconclusive reported as silence, not as a mismatch), TC-136 (unobservable axis skipped, not failed), TC-149 (every command is a build entry), TC-150 (the command exists and is wired to quire), TC-247 (`tests/catalog-fact-join.test.ts` — every declared characteristic is producible or exempt with a reason), TC-248 (no method is unreachable by every statement unless declared so), TC-249 (prose only: a link target mints nothing, `path-safety` is not `safety`), TC-250 (`high-criticality` read from the obligation's value, not a chosen threshold), TC-251 (no exemption survives the value it excused), TC-252 (the three spec-side triggers for concolic execution, with their two traps), TC-253 (the evidence-side pair, and the three cases that mint neither). CON-1 → TC-129 + inspection of `skills/spec-evidence-analysis/SKILL.md` (no method table remains). CON-2/CON-3 → inspection of `src/advisor/` (no spec is written; rule-matched and judged results are distinguishable). |
| FR-030 | ✅ Covered | `tests/evidence-store.test.ts` — TC-119 (store under `spec/`, run path shape), TC-120 (canonical, byte-identical writes), TC-121 (suite is atomic; last-write-wins), TC-122 (passing binds, failing/skipped does not, run still recorded), TC-123 (rewording makes suspect; re-running does not clear it), TC-124 (affirmation moves the hash and records who/where), TC-125 (unmatched trace id reported), TC-126 (gc keeps latest + referenced; --dry-run deletes nothing; TC-130 pins which run "latest" means), TC-127 (absent store reads empty), TC-128 (CON-2 no statement/document/method stored), TC-129 (a second suite appends; affirmation clears every suite), TC-130 (latest is newest by timestamp, not by filename), TC-131 (a corrupt store file is named), TC-132 (byte-exact, locale-independent ordering). CON-1/CON-3 → inspection of `src/evidence/` (no suite is executed, no database). |
| FR-029 | ✅ Covered | `tests/quire-contract.test.ts` — TC-110 (vendored schemas hash to their recorded provenance and carry the pinned `$id`), TC-111 (conformant payloads validate), TC-112 (missing key / added field / value outside a closed engine enum each rejected, failing path named), TC-113 (unreadable output is a named violation, not a throw), TC-114 (version premise names found/required/consequence; an unreadable version fails rather than passes), TC-115 (numeric comparison, so `0.21.0` > `0.9.0`), TC-116 (every optional key absent and present; malformed statement hash rejected), TC-117 (the eval harness floor equals the pinned minimum), TC-118 (a payload from the **installed** `quire` validates — the contract checked against the real emitter, not only fixtures). CON-1/CON-2 → inspection of `src/quire/` (no hand-written validator, no network read). CON-3 → TC-112, which rejects a closed enum and leaves `diagnostics[].reason` open. |
| FR-028 | ✅ Covered | The `spec-correctness` skill (`skills/spec-correctness/`) is agent-facing, so it is verified at the eval layer rather than by vitest — TC-EV-050…TC-EV-053, implemented in `evals/scenarios/index.mjs` and run by `make evals-all`. AC-1/AC-3/AC-4 → TC-EV-050. AC-2/AC-6/AC-7 → TC-EV-051. AC-5 → TC-EV-052. AC-8/AC-9/AC-11/AC-13 → TC-EV-053. AC-10/AC-12 → Inspection of `skills/spec-correctness/` (no framework name reaches any `spec/**` output; strategy selection is keyed on `property`, never on `shape`). CON-1 → TC-EV-053 `absentFiles`. The skill's own output on this repo is `tests/props/` + a `SpecReview` under `reviews/`. |

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
| NFR-010     | ⚠️ Spec-ahead-of-code | Module pins record `version` and `ref` only; no resolved commit SHA is stored, so a repointed tag resolves differently under the same pin and nothing notices. Stated ahead of the implementation — `agent-ix/quoin#132`. |
| NFR-011     | ⚠️ Spec-ahead-of-code | No performance measurement exists anywhere in the repository — no benchmark, no timing assertion, no threshold. The budget is stated so a regression fails a test rather than being absorbed as "CI got slower" — `agent-ix/quoin#133`. |
| NFR-012     | ⚠️ Partial | The quire half is covered: `quire-contract.test.ts` TC-114 asserts the version premise names found/required/consequence, TC-118 validates a payload from the **installed** quire. The module half is `scripts/release-drift.js pins`, which no test invokes. |

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
| TC-110 | The vendored quire schemas hash to the value recorded in `contract.ts` and each `$id` names the pinned contract version — an edit without a matching refresh fails loudly | Unit | P0 | FR-029-AC-1 | ✅ |
| TC-111 | A conformant coverage payload and a conformant properties payload each validate against the published schema | Unit | P0 | FR-029-AC-2 | ✅ |
| TC-112 | A missing required key, an added field, and a value outside a closed engine enum are each rejected, and the report names the failing path | Unit | P0 | FR-029-AC-3 | ✅ |
| TC-113 | Unreadable quire output is a named contract violation naming the likeliest cause, not a throw — "unreadable" and "wrong shape" are the same actionable fact to the caller | Unit | P0 | FR-029-AC-4 | ✅ |
| TC-114 | An older CLI fails the version premise with the found version, the required version and the consequence; an unreadable version is a failure rather than a pass | Unit | P0 | FR-029-AC-5 | ✅ |
| TC-115 | Version comparison is numeric, so `0.21.0` ranks above `0.9.0` — the bug a lexical comparison produces across exactly this range | Unit | P1 | FR-029-AC-6 | ✅ |
| TC-116 | A payload omitting every optional key validates, one carrying every optional key validates, and a malformed statement hash is rejected | Unit | P0 | FR-029-AC-7 | ✅ |
| TC-117 | The eval harness's `HARNESS_MIN_QUIRE` equals `QUIRE_CONTRACT.minimumCli`, so the two restatements cannot drift silently | Unit | P1 | FR-029-AC-8 | ✅ |
| TC-118 | A payload emitted by the **installed** `quire` binary validates against the vendored schema — the one thing a fixture cannot show | Integration | P0 | FR-029-AC-9 | ✅ |
| TC-119 | The store root is `spec/evidence/` and a run path is `runs/<SUITE-N>/<commit12>.json` — under `spec/` because CR-045 bounds the document walk there, measured | Unit | P0 | FR-030-AC-1 | ✅ |
| TC-120 | Writes are canonical — keys sorted at every level, trailing newline — and byte-identical across repeated serialization, so a PR diff of the store IS the per-PR delta | Unit | P0 | FR-030-AC-2 | ✅ |
| TC-121 | Two suites at one commit produce two files and a re-run at the same commit keeps one: the suite is the atomic unit, and a partial run must never masquerade as a full one | Unit | P0 | FR-030-AC-3 | ✅ |
| TC-122 | A passing symbol binds with the current hash; a failing or skipped symbol binds nothing while its run is still recorded — a red build must not be something the store agrees with | Unit | P0 | FR-030-AC-4 | ✅ |
| TC-123 | A reworded statement makes the binding suspect, and re-recording the same passing run does NOT overwrite the hash — otherwise the state clears itself on the next CI run and the detector never fires | Unit | P0 | FR-030-AC-5 | ✅ |
| TC-124 | Affirmation moves the hash forward and records who and at which commit; affirming an unknown obligation reports rather than inventing a binding | Unit | P0 | FR-030-AC-6 | ✅ |
| TC-125 | A trace id no obligation states is reported as unmatched — quire-rs#72 from the other direction, a test claiming to verify something the spec does not state | Unit | P0 | FR-030-AC-7 | ✅ |
| TC-126 | `gc` deletes only runs that are neither latest for their suite nor referenced by a binding, and `--dry-run` deletes nothing | Unit | P1 | FR-030-AC-8 | ✅ |
| TC-127 | An absent store reads as an empty binding graph, an empty run list and an empty collection — never an error | Unit | P1 | FR-030-AC-9 | ✅ |
| TC-128 | No obligation statement, document or method appears in the written store — only the id and hash. Anything re-derivable is re-derived, so the store cannot disagree with the spec | Unit | P0 | FR-030-AC-10 | ✅ |
| TC-129 | A second suite discharging the same obligation appends a binding rather than replacing the first; re-discharging the same suite merges into its own; affirmation clears every suite unless narrowed to one. `BindingsFile`'s own doc says the graph IS cross-suite, and keying on the obligation alone destroyed that on write | Unit | P0 | FR-030-AC-11 | ✅ |
| TC-130 | The latest run is the newest by `timestamp`, proven with fixtures whose newest run's commit prefix sorts FIRST — the arrangement where filename order and time order disagree. `gc` keeps that run, and a shared timestamp ties on commit | Unit | P0 | FR-030-AC-12 | ✅ |
| TC-131 | A merge-conflicted `bindings.json` raises a diagnostic naming the file and the cause, not a bare `SyntaxError: Unexpected token '<'`; one corrupt run file is skipped and reported rather than hiding every finding | Unit | P0 | FR-030-AC-13 | ✅ |
| TC-132 | The written binding graph is pinned byte-exactly, so a runtime's collation data cannot change the diff of a file whose diff is meant to BE the per-PR delta | Unit | P1 | FR-030-AC-14 | ✅ |
| TC-133 | A module whose `manifest.yaml` does not parse is skipped and reported on the merged catalog while the modules that parsed still merge — the crash was in the command an operator runs to diagnose module problems | Unit | P1 | FR-031-AC-9 | ✅ |
| TC-134 | The `quire` subprocess runs through one helper that captures stderr and surfaces it on a non-zero exit; `stdio: [..., "ignore"]` threw away exactly the sentence the operator needed | Static | P1 | FR-029-AC-10 | ✅ |
| TC-149 | Every command file under `src/commands/` with a `default export class` has a vite build entry. The enumeration is hand-maintained, so a command could exist in `src/`, pass `tsc`, pass every unit test, ship a `.d.ts` — and not exist at runtime; **two did** | Unit | P0 | FR-031-AC-10 | ✅ |
| TC-150 | The advisor has a reachable command: `quoin advise` turns quire payloads into `ObligationFacts`, advises an NFR metric row from its statement alone, and keeps the payload of a `properties` run that exited non-zero | Unit | P0 | FR-031-AC-10, FR-031-AC-11 | ✅ |
| TC-151 | The registry selects by `--adapter`, then by the suite's `--tool`, then falls back to the normalized `entries` shape | Unit | P0 | FR-033-AC-1 | ✅ |
| TC-152 | An unknown `--adapter` is an error naming the available adapters — falling back would complain about JSON shape and send the reader to their XML instead of their typo | Unit | P0 | FR-033-AC-2 | ✅ |
| TC-153 | Adapters are registered as data, so an external user can add one for their own tool | Unit | P1 | FR-033-AC-3 | ✅ |
| TC-154 | JUnit `classname` + `name` map to the qualified name the symbol extractor emits; a tool's test name is not a symbol identity | Unit | P0 | FR-033-AC-4 | ✅ |
| TC-155 | Every JUnit outcome class is read and declared trace ids are carried | Unit | P0 | FR-033-AC-5 | ✅ |
| TC-156 | The JUnit adapter names no evidence kind — unit, integration and e2e suites all emit JUnit, so the format does not say which | Unit | P0 | FR-033-AC-6, FR-033-CON-3 | ✅ |
| TC-157 | JUnit input carrying no `<testcase>` is rejected rather than recorded as an empty run | Unit | P0 | FR-033-AC-7 | ✅ |
| TC-158 | cargo-mutants produces a per-function score with unviable mutants in neither side of the ratio | Unit | P0 | FR-033-AC-8 | ✅ |
| TC-159 | Outcome is the tool's own classification, not a threshold | Unit | P0 | FR-033-AC-9, FR-033-CON-2 | ✅ |
| TC-160 | Malformed, empty and unattributable mutation reports are rejected; a timed-out mutant counts as survived, not as absent | Unit | P0 | FR-033-AC-10 | ✅ |
| TC-161 | `quoin evidence record --adapter junit --results <file>` records a JUnit file end to end | Integration | P0 | FR-033-AC-11 | ✅ |
| TC-162 | `quoin evidence record --adapter cargo-mutants` records a mutation report end to end, preserving score | Integration | P0 | FR-033-AC-12 | ✅ |
| TC-163 | `quoin evidence record` selects the adapter from `--tool` when none is named | Integration | P0 | FR-033-AC-13 | ✅ |
| TC-164 | `quoin evidence record` still accepts the normalized shape with no adapter, so the registry is never a gate | Integration | P0 | FR-033-AC-14, FR-033-CON-4 | ✅ |
| TC-165 | A SARIF run with an empty `results` array is a scan that HAPPENED — the distinction this record type exists to make | Unit | P0 | FR-034-AC-1 | ✅ |
| TC-166 | A SARIF log whose `runs` array is empty is rejected: that file proves no scan executed | Unit | P0 | FR-034-AC-2 | ✅ |
| TC-167 | The SARIF adapter reads rule id, level, message and first location | Unit | P0 | FR-034-AC-3 | ✅ |
| TC-168 | The nested `rule.id` form is accepted; a result naming no rule is skipped | Unit | P1 | FR-034-AC-4 | ✅ |
| TC-169 | Malformed SARIF and a log with no `runs` array are rejected | Unit | P0 | FR-034-AC-5 | ✅ |
| TC-170 | The cargo-audit adapter parses REAL captured `cargo audit --json` output, checked in unedited | Unit | P0 | FR-034-AC-6 | ✅ |
| TC-171 | Warning kinds (`unsound`, `unmaintained`, `yanked`) keep their own severity rather than being flattened | Unit | P0 | FR-034-AC-7, FR-034-CON-2 | ✅ |
| TC-172 | Malformed input and non-cargo-audit output are rejected; an advisory with no id is skipped | Unit | P0 | FR-034-AC-8 | ✅ |
| TC-173 | A clean scan discharges its binding — neither undischarged nor vacuous | Unit | P0 | FR-034-AC-9 | ✅ |
| TC-174 | A scan that evaluated no rules is reported vacuous at high: it found nothing because it looked for nothing | Unit | P0 | FR-034-AC-10 | ✅ |
| TC-175 | With no rule count reported the vacuity check stays silent rather than guessing | Unit | P0 | FR-034-AC-11 | ✅ |
| TC-176 | Each run-shaped binding pairs with its OWN suite's run when a scan is bound to the same obligation — silent until scans existed, wrong from the moment they did | Unit | P0 | FR-034-AC-12 | ✅ |
| TC-177 | `quoin evidence record --adapter sarif` writes a FindingRecord under `scans/` and nothing under `runs/` — the capability shipped unreachable and this is what reaches it | Integration | P0 | FR-034-AC-13 | ✅ |
| TC-178 | A clean scan recorded through the command keeps `findings: []` and its rule count | Integration | P0 | FR-034-AC-14 | ✅ |
| TC-179 | The command selects a finding adapter from `--tool` when none is named | Integration | P0 | FR-034-AC-15 | ✅ |
| TC-180 | The space — strength, dimensions, values — is parsed back out of the obligation statement | Unit | P0 | FR-035-AC-1 | ✅ |
| TC-181 | A statement that is not a configuration space yields null — which IS how a combinatorial obligation is told from every other kind, with no second flag | Unit | P0 | FR-035-AC-2 | ✅ |
| TC-182 | Exclusions are read without being mistaken for dimensions | Unit | P0 | FR-035-AC-3 | ✅ |
| TC-183 | The demanded-tuple count matches the number quire-rs computes for the same space (its TC-925) — disagreement would mean the obligation and its audit describe different spaces | Unit | P0 | FR-035-AC-4 | ✅ |
| TC-184 | A forbidden combination is not demanded | Unit | P0 | FR-035-AC-5 | ✅ |
| TC-185 | The result NAMES the combinations that never ran — a percentage says how much is missing, the list says what to run | Unit | P0 | FR-035-AC-6 | ✅ |
| TC-186 | A configuration exercising undeclared values covers nothing — counting it would let coverage rise by testing something else | Unit | P0 | FR-035-AC-7 | ✅ |
| TC-187 | Full coverage reports an empty gap list | Unit | P0 | FR-035-AC-8 | ✅ |
| TC-188 | `audit` reports `combinatorial-gap` naming the missing combinations | Unit | P0 | FR-035-AC-9, FR-035-CON-3 | ✅ |
| TC-189 | An obligation whose demanded combinations all ran is healthy | Unit | P0 | FR-035-AC-10 | ✅ |
| TC-190 | An obligation declaring no configuration space produces no combinatorial finding | Unit | P0 | FR-035-AC-11 | ✅ |
| TC-191 | A declared space advises the combinatorial method structurally; the prose regex would miss it and ordinary prose must not match | Unit | P0 | FR-035-AC-12 | ✅ |
| TC-192 | `--discharges` binds the obligations a scan was run to check — a clean scan is the strongest evidence a scanner produces and carries no finding to bind from | Integration | P0 | FR-034-AC-16 | ✅ |
| TC-193 | A scan that evaluated no rules binds nothing, whatever `--discharges` names | Integration | P0 | FR-034-AC-17 | ✅ |
| TC-194 | A suite that recorded only scans is enumerated — reading `runs/` alone made it invisible to every caller that enumerates, the auditor included | Integration | P0 | FR-034-AC-18 | ✅ |
| TC-195 | `gc` collects superseded scans, keeping the newest by timestamp, so a scan store does not grow without bound | Integration | P1 | FR-034-AC-19 | ✅ |
| TC-196 | A tool reporting ZERO rules is distinguishable from one reporting NO count — the distinction FR-034 turns on, erased by omitting the field when it was 0 | Unit | P0 | FR-034-AC-20 | ✅ |
| TC-197 | A real all-passing audit run reads as rules that ran and found nothing — the healthiest outcome must not be indistinguishable from a scan that never happened | Unit | P0 | FR-036-AC-1 | ✅ |
| TC-198 | A `FAIL` line becomes a finding carrying the criterion the script names — the rule-id ⇄ obligation join is stated by the tool, not maintained in a mapping | Unit | P0 | FR-036-AC-2 | ✅ |
| TC-199 | `OK` lines count toward rules evaluated, so a fully-passing suite is not reported as vacuous | Unit | P0 | FR-036-AC-3 | ✅ |
| TC-200 | Output with no recognised audit line is rejected — no audit ran, and recording it would manufacture evidence from a file that proves nothing | Unit | P0 | FR-036-AC-4 | ✅ |
| TC-201 | The adapter is selected by `--adapter`, and by `--tool` for the tools emitting this shape | Unit | P0 | FR-036-AC-5 | ✅ |
| TC-202 | No module outside `src/commands/` imports from `src/commands/`; commands are leaves and the library must load without oclif | Static | P0 | FR-036-AC-7 | ✅ |
| TC-203 | quoin executes exactly `git`, `quire` and `ix-flow` — ADR-0011 invariant 1 asserted as a whole set, so a new binary fails by default | Static | P0 | FR-036-AC-8 | ✅ |
| TC-204 | Both characteristics the catalog keys `architecture-conformance` on are minted, and they do not match the same phrase | Unit | P0 | FR-036-AC-6 | ✅ |
| TC-205 | An architectural statement is advised `architecture-conformance` through the real loader and advisor, with both halves of the rule matching | Unit | P0 | FR-036-AC-6 | ✅ |
| TC-206 | The vocabulary's values are read from module data — the ticket assumed 9 ISO 25010 characteristics and the module declares 12 | Unit | P0 | FR-037-AC-1 | ✅ |
| TC-207 | A declaration whose vocabulary cannot be resolved is reported; silently dropping it leaves engine findings quoin cannot explain | Unit | P0 | FR-037-AC-2 | ✅ |
| TC-208 | A value no document claims is an unowned gap at `medium` — an admitted gap a reader can see and decide about | Unit | P0 | FR-037-AC-3 | ✅ |
| TC-209 | An exclusion with no written reason is `high`, above the gap it replaces: a completeness claim with nothing behind it | Unit | P0 | FR-037-AC-4 | ✅ |
| TC-210 | An exclusion naming a value outside the vocabulary excuses nothing while reading as handled; the real value keeps reporting | Unit | P0 | FR-037-AC-5 | ✅ |
| TC-211 | A table row naming the value with a real reason accepts the exclusion | Unit | P0 | FR-037-AC-6 | ✅ |
| TC-212 | `-`, `TBD` and `n/a` are not reasons, and a mention of the value in prose is not a justification | Unit | P0 | FR-037-AC-7 | ✅ |
| TC-213 | A `high` finding fails whatever `--strict` says; a gap fails only under it | Unit | P0 | FR-037-AC-8 | ✅ |
| TC-214 | `quoin completeness` reports the gaps over a real bundle and exits 0 while advisory | Integration | P0 | FR-037-AC-9 | ✅ |
| TC-215 | `quoin completeness` exits non-zero on an unjustified exclusion without `--strict` | Integration | P0 | FR-037-AC-10 | ✅ |
| TC-216 | `quoin completeness --strict` exits non-zero on gaps alone — same bundle, both ways round, so `--strict` is a policy and not a second code path | Integration | P0 | FR-037-AC-11 | ✅ |
| TC-217 | A bundle whose modules declare no vocabulary reports `UNCHECKED`, never `PASS` — nothing checked is not nothing wrong | Integration | P0 | FR-037-AC-12 | ✅ |
| TC-218 | The unowned set quoin reports equals the one the bundle read yields for the same declaration | Unit | P0 | FR-037-AC-13 | ✅ |
| TC-219 | The mutation-score policy: silent until a floor is declared, judged on the weakest symbol not the mean, a skipped symbol's absent score is not zero, and a demanded-but-unmeasured floor is its own finding, and a latency is not a mutation score | Unit | P0 | FR-039-AC-1, FR-039-AC-2, FR-039-AC-3, FR-039-AC-4, FR-039-AC-5, FR-039-AC-6, FR-039-AC-7, FR-039-AC-10, FR-039-AC-11 | ✅ |
| TC-220 | `--mutation-floor` parses `<criticality>=<ratio>` and refuses a percentage or a malformed pair — an ignored floor reads as a passing gate | Unit | P0 | FR-039-AC-8, FR-039-AC-9 | ✅ |
| TC-221 | The case argues from a claim through its requirements down to their obligations | Unit | P0 | FR-040-AC-1 | ✅ |
| TC-222 | An obligation with a finding stays in the tree as an open node carrying the auditor's reason, rather than being dropped | Unit | P0 | FR-040-AC-2 | ✅ |
| TC-223 | Open propagates upward, so a claim is never reported supported over a broken branch | Unit | P0 | FR-040-AC-3 | ✅ |
| TC-224 | A claim with no sub-claim and no obligation is open — assuring it would rest on nobody having written anything | Unit | P0 | FR-040-AC-4 | ✅ |
| TC-225 | Requirements carrying obligations that no claim reaches are reported; a case over the reachable half reads as complete | Unit | P0 | FR-040-AC-5 | ✅ |
| TC-226 | The claim type is the caller's: a declared hazard argues as readily as an `StR`, because that vocabulary is module data | Unit | P0 | FR-040-AC-6 | ✅ |
| TC-227 | An obligation's owning requirement is derived from its id, not from an edge that would need keeping in agreement | Unit | P1 | FR-040-AC-7 | ✅ |
| TC-228 | Rendering unchanged inputs is byte-identical, so a diff means the evidence moved | Unit | P0 | FR-040-AC-8 | ✅ |
| TC-229 | Mermaid survives punctuation in a statement — each of `-`, `(`, `"` and `;` otherwise yields a diagram that fails to draw | Unit | P0 | FR-040-AC-9 | ✅ |
| TC-230 | A bundle declaring no claim says so rather than rendering an empty case; those are different problems | Unit | P0 | FR-040-AC-10 | ✅ |
| TC-231 | A real CycloneDX document yields one passing entry per component, identified by purl | Unit | P0 | FR-041-AC-1 | ✅ |
| TC-232 | A real SPDX document is read through its purl external refs — reading `name` would give the same component two identities across formats | Unit | P0 | FR-041-AC-2 | ✅ |
| TC-233 | An empty inventory yields no entries, so `vacuous-evidence` names it with no new machinery; a count in `score` would read as healthy | Unit | P0 | FR-041-AC-3 | ✅ |
| TC-234 | A document that is neither format is refused — zero entries is a real finding and an unreadable file must not masquerade as one | Unit | P0 | FR-041-AC-4 | ✅ |
| TC-235 | A component with neither purl nor name is dropped, not given an invented identity that binds to nothing and inflates the count | Unit | P0 | FR-041-AC-5 | ✅ |
| TC-236 | The adapter is selected by `--adapter sbom` and by the tools emitting these formats | Unit | P0 | FR-041-AC-6 | ✅ |
| TC-237 | A requirement refining two claims appears under both — one visited-set answered cycles and "rendered elsewhere" at once, and the second claim reported a statement that was false | Unit | P0 | FR-040-AC-11 | ✅ |
| TC-238 | A cycle terminates without dropping a legitimately shared child, so the FND-001 fix cannot regress into an infinite walk | Unit | P0 | FR-040-AC-12 | ✅ |
| TC-239 | With no catalog declaring a mutation method the check says nothing — reporting `unmeasured` fired on every obligation with a floor, including ones holding a real score | Unit | P0 | FR-039-AC-12 | ✅ |
| TC-240 | A real `cli-agent-evals` report yields one entry per scenario, keyed on the scenario id | Unit | P0 | FR-042-AC-1 | ✅ |
| TC-241 | The scenario id is its own trace id — the join other adapters must construct is already stated here | Unit | P0 | FR-042-AC-2 | ✅ |
| TC-242 | The outcome is the harness's `ok` over `repeats`, and `passRate` is kept because flaky and failing are different facts | Unit | P0 | FR-042-AC-3 | ✅ |
| TC-243 | A report with no scenario results is refused — it proves only that the harness started | Unit | P0 | FR-042-AC-4 | ✅ |
| TC-244 | The adapter is selected by `--adapter agent-eval` and by the harness name | Unit | P0 | FR-042-AC-5 | ✅ |
| TC-245 | A run's trace id binds through an obligation's declared test cases — several criteria per case, a direct id winning over the indirect route, and an id nobody states still reported unmatched | Unit | P0 | FR-030-AC-15 | ✅ |
| TC-129 | The merged catalog carries every declared method with its rules intact, merges first-wins, reports a colliding id rather than absorbing it, and treats an undeclared catalog as empty | Unit | P0 | FR-031-AC-1 | ✅ |
| TC-130 | An `attack_surface` object reaches DAST, SAST and negative/abuse testing — recommendations that were unreachable while the method table was skill-local prose | Unit | P0 | FR-031-AC-2 | ✅ |
| TC-131 | Temporal phrasing reaches runtime monitoring and model checking — the methods a single execution cannot discharge | Unit | P0 | FR-031-AC-3 | ✅ |
| TC-132 | Reliability phrasing reaches fault injection: inducing the failure the requirement claims to tolerate | Unit | P0 | FR-031-AC-4 | ✅ |
| TC-133 | A round-trip property shape reaches property-based and metamorphic testing, and recommendations are ordered by matching-rule count so the output is reproducible | Unit | P0 | FR-031-AC-5 | ✅ |
| TC-134 | An authored method no recommendation covers is flagged; one matching a recommended method's class is not, and `Test (TC-001)` normalizes to `Test` before comparison | Unit | P0 | FR-031-AC-6 | ✅ |
| TC-135 | An obligation matching no rule is inconclusive with no recommendations and **no** mismatch — silence is not disagreement, and defaulting to `Test` is the habit this replaces | Unit | P0 | FR-031-AC-7 | ✅ |
| TC-136 | A method whose rule names an axis the advisor cannot observe is still recommended on the axes it can, with only observable reasons reported — the engine leaves the axis set open on purpose | Unit | P1 | FR-031-AC-8 | ✅ |
| TC-247 | Every `characteristics` value the active catalog declares is producible by some fact source, or listed as exempt with a reason. Was 40 of 60 producible by nothing — the join the two sides were never checked against (CR-025) | Unit | P0 | FR-031-AC-12 | ✅ |
| TC-248 | No method keyed solely on `characteristics` is unreachable by every statement, unless declared unreachable with a reason. Was 7 of 33, including `integration-testing` and `mutation-testing` | Unit | P0 | FR-031-AC-13 | ✅ |
| TC-249 | A markdown link's target contributes no characteristic and its text does; `path-safety` does not mint `safety`. 12 of 13 `stakeholder` matches were the directory name in a link target, and 10 of 11 `safety` matches were `path-safety` | Unit | P0 | FR-031-AC-14 | ✅ |
| TC-250 | `high-criticality` is minted from the obligation's declared value (`P0`/`high`/`critical`), never from a threshold the engine chose — the CR-008 rule | Unit | P1 | FR-031-AC-15 | ✅ |
| TC-251 | The join check carries no exemption for a value the catalog no longer declares. Caught three the day it was written — a stale excuse reads as considered, which is worse than none (CR-026) | Unit | P0 | FR-031-AC-16 | ✅ |
| TC-252 | A checksum, constant-time phrasing and reference equivalence each mint their characteristic; `function signature` mints no magic-value comparison and a recorded-snapshot comparison mints no reference equivalence (CR-026) | Unit | P0 | FR-031-AC-17 | ✅ |
| TC-253 | Bound with no score mints `fault-detection-unmeasured`; weakest bound score below 1 mints `fault-detection-failed`; unbound and unconsulted mint neither — the latter two are `undischarged`, a different finding (CR-026) | Unit | P0 | FR-031-AC-18 | ✅ |
| TC-137 | An obligation whose hash matches, whose suite ran at HEAD, and whose symbols passed produces no finding and is reported healthy | Unit | P0 | FR-032-AC-1 | ✅ |
| TC-138 | A statement reworded after binding produces a **high**-severity suspect-link naming both hashes — the requirement moved and the evidence did not follow, while the matrix still read as covered | Unit | P0 | FR-032-AC-2 | ✅ |
| TC-139 | A binding naming a suite with no recorded run is high (it claims evidence not in the store); a run merely behind HEAD is medium | Unit | P0 | FR-032-AC-3 | ✅ |
| TC-140 | Vacuity fires when **every** bound symbol was skipped or absent, and not when one passed — one passing symbol is evidence even beside a skipped sibling | Unit | P0 | FR-032-AC-4 | ✅ |
| TC-141 | An obligation with no binding is undischarged at **medium**: an unwritten test is work in progress, and grading it like a lie about a written one teaches readers to skim both | Unit | P0 | FR-032-AC-5 | ✅ |
| TC-142 | A non-test-class method discharged by a test run is flagged, a test-class one is not, and with no catalog the question is not asked — an absent catalog means it cannot be asked, not that the answer is yes | Unit | P0 | FR-032-AC-6 | ✅ |
| TC-143 | Multiplicity is satisfied by two **suites**, not by two symbols in one: symbols in one suite share a harness and a failure mode, so counting them twice lets one broken assumption look like corroboration | Unit | P0 | FR-032-AC-7 | ✅ |
| TC-145 | Every check folds over ALL of an obligation's bindings: a suspect link fires when any binding predates the statement, vacuity needs every symbol in every suite skipped, and multiplicity counts the distinct suites actually bound — a count that could never exceed one before #102 | Unit | P0 | FR-032-AC-8 | ✅ |
| TC-146 | Method conformance compares kind to kind — `run.entries.length > 0` was read as "a test run" and is true of a transcribed inspection too — and a method no catalog carries is reported as `unknown-method` rather than skipped in silence (55 of 577 across the ecosystem) | Unit | P0 | FR-032-AC-9, FR-032-AC-10 | ✅ |
| TC-147 | Every finding kind can be accepted into the ratchet baseline, which is a flat `<kind>:<obligation>` set; five of the six kinds could never be baselined before, so `--ratchet` reported the whole backlog for them | Unit | P0 | FR-032-AC-11 | ✅ |
| TC-148 | `quoin evidence audit --module <dir>` loads the catalog from that module, not from the installed roots — otherwise obligations come from one catalog and conformance is checked against another | Static | P1 | FR-032-AC-12 | ✅ |
| TC-144 | `ratchet` reports only violations absent from the baseline — a gate that fails on the whole backlog gets disabled within a week — and `delta` names what a change added and resolved | Unit | P0 | FR-032-AC-8 | ✅ |

## Stakeholder Requirement Coverage

Every StR carries one validation criterion, verified by demonstration or
inspection rather than by a unit test — so these rows carry no tracking tag by
design (see "Tracking-tag coverage"). Added in response to SR-003 FND-002, which
found the stakeholder layer had no rows here at all.

| Stakeholder Req | Trace to US/FR         | Test/Validation                                                                                       | Coverage Status |
| --------------- | ---------------------- | ----------------------------------------------------------------------------------------------------- | --------------- |
| StR-001-VC-1    | US-009; FR-004, FR-023 | Demonstration — TC-EV-001…TC-EV-013 run the real CLI from an isolated `IX_HOME`; NFR-004 inspects the deps    | ✅ Covered      |
| StR-002-VC-1    | US-003; FR-018, FR-019 | Demonstration — TC-EV-003/TC-EV-009/TC-EV-010/TC-EV-020 install from local, GitHub and subdir sources              | ✅ Covered      |
| StR-003-VC-1    | US-004; FR-014, FR-015 | Demonstration — TC-EV-001/TC-EV-004/TC-EV-008 author to the skeleton and validate with a real `quire`           | ✅ Covered      |
| StR-004-VC-1    | US-005; FR-020, FR-021 | Inspection — TC-EV-005/TC-EV-013 start runs and inspect status; resume/advance/gate progression is untested  | ⚠️ Partial      |
| StR-005-VC-1    | US-003; FR-016, FR-017 | Inspection — the default set is version-pinned; NFR-001 covers idempotent offline reconcile            | ✅ Covered      |
| StR-006-VC-1    | US-009; FR-022         | Demonstration — `update.test.ts` covers delegation, `--check` and `--registry`                          | ✅ Covered      |

## Use Case Coverage

| Use Case | Coverage   | Test / Evidence                                                                                                                                                      |
| -------- | ---------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| US-001   | ✅ Covered | `index.test.ts` authoring-pack test; `write.test.ts` `formatAuthoringPack` suite; TC-EV-001/TC-EV-006/TC-EV-014 in `spec/evals.md`                                            |
| US-002   | ✅ Covered | `cli.test.ts` `runCatalog` show suite; `scripts.test.ts` catalog/write help; `index.test.ts` case-insensitive lookup; TC-EV-002/TC-EV-007                                  |
| US-003   | ⚠️ Partial | `plugins.test.ts` + `index.test.ts` install/list/remove (path source only). GitHub/subdir install is exercised only at the eval layer (TC-EV-003/TC-EV-009/TC-EV-010/TC-EV-020). |
| US-004   | ✅ Covered | `write.test.ts` validation-command assertions; `index.test.ts` `validation.command`; TC-EV-004/TC-EV-008/TC-EV-012                                                            |
| US-005   | ✅ Covered | `flows.test.ts` launchers end-to-end (real fake `ix-flow`); `cli.test.ts` `review`/`matrix` dispatch; TC-EV-005/TC-EV-013                                                  |
| US-006   | ✅ Covered | `catalog.test.ts` :: "reports a type declared by two modules"; `cli.test.ts` :: "validate reports duplicates on stderr and sets exit code 1"                         |
| US-010   | ✅ Covered | `org.test.ts` resolution suites (precedence, both url forms, worktrees, owner-less remotes); `write.test.ts` "authoring pack organization" suite; `cli.test.ts` `--org` text/JSON/unresolved trio; `org-no-subprocess.test.ts` no-subprocess proof — see FR-025 |
| US-011   | ✅ Covered | TC-EV-050…TC-EV-053 in `evals/scenarios/index.mjs` — the settled lane, the review artifact, the gap-analysis handoff, and the refusals; see FR-028. The skill's own run on this repo is `tests/props/` (17 criteria) + a `SpecReview` under `reviews/` |
| US-012   | ✅ Covered | TC-EV-054…TC-EV-057 in `evals/scenarios/index.mjs` — the generation lane, the two refusals, idempotent re-runs and harness selection, and the undischarged report; see FR-038. |

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

Criteria reconcilable by tag are counted by `quire coverage`, not by grep. A
previous figure here ("109 of 130") was grep-derived, and grep matches a tag
wherever it sits — including ~15 that sat above a `describe(` block and bound to
nothing (agent-ix/quoin#61). Re-derive from the tool rather than quoting a number
whose provenance is a shell command:

```
quire coverage --scope . --json
```

Some criteria are verified by a method that produces no test at all:
FR-003-AC-1/AC-3 (help rendering, delegated to `@oclif/core`), FR-028-AC-1…AC-13
(evals TC-EV-050…TC-EV-053 and inspection), NFR-007-AC-1 (an accepted limitation), and
the six StR validation criteria (demonstration).

## Eval Coverage

The agent-facing eval set (TC-EV-001…TC-EV-020) is defined in `spec/evals.md`
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
| IT-002           | ⚠️ Spec-ahead-of-code | `github://` plugin install. Unit tests cover path sources only (`plugins.test.ts`); live GitHub install is exercised at the eval layer (TC-EV-003/TC-EV-009/TC-EV-010/TC-EV-020).                                                    |

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
