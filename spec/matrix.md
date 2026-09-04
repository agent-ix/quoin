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

| FR-032 | ✅ Covered | `tests/auditor.test.ts` — TC-137 (healthy produces nothing), TC-138 (suspect link, high), TC-139 (missing run high vs behind-HEAD medium), TC-140 (vacuity needs EVERY symbol skipped/absent), TC-141 (undischarged at medium, not high), TC-142 (method conformance via the catalog; skipped without one), TC-143 (multiplicity measured in suites, not symbols), TC-144 (ratchet + per-PR delta), TC-145 (every check folds over all of an obligation's bindings), TC-146 (kind-to-kind conformance; `unknown-method` reported), TC-147 (every kind is baselineable), TC-148 (the catalog follows `--module`). `tests/evidence-audit-command.test.ts` — TC-258..TC-260 (the ratchet label and JSON `ratchet` field key on a baseline actually found, and a missing one is named). TC-264 (`unknown-method` before the binding guard: an unbound obligation with an uncatalogued method is both `undischarged` and `unknown-method`), TC-265 (the command reports uncatalogued methods against a repository with no evidence store at all). CON-1/CON-2/CON-3 → TC-142's no-catalog case plus inspection of `src/auditor/` (pure functions, no clock, no subprocess, no write). |
| FR-033 | ✅ Covered | `tests/evidence-adapters.test.ts` — TC-151..TC-164. Five criteria are stated over `quoin evidence record --adapter <x> --results <file>` rather than over the parse function, because the P1 review found three of four P0 gaps were a matrix reading ✅ over a capability nothing could reach. CON-1/CON-2 → inspection of `src/evidence/adapters/` (no fs, no subprocess, no threshold). CON-3 → TC-156. CON-4 → TC-164. |
| FR-034 | ✅ Covered | `tests/finding-record.test.ts` — TC-165..TC-176. The cargo-audit criteria run against output captured with `cargo audit --json` and checked in unedited (`tests/fixtures/evidence/`), because a fixture written to match the reader only proves the reader parses itself. CON-1/CON-3 → inspection of `src/evidence/adapters/sarif.ts` and `src/evidence/store.ts`. CON-2 → TC-171. CON-4 → `readScans` mirrors `readRuns` ordering. |
| FR-035 | ✅ Covered | `tests/combinatorial-coverage.test.ts` — TC-180..TC-191. TC-183 pins agreement with quire-rs TC-925 on the same space: if the two ever disagree, an obligation is measured against a target it does not state. CON-1/CON-2 → inspection of `src/auditor/combinatorial.ts` (nothing is generated, nothing is declared). CON-3 → TC-188. |
| FR-042 | ✅ Covered | `tests/agent-eval.test.ts` — TC-240..TC-244. `agent-eval-real.json` is a real `cli-agent-evals` report, unedited — the TC-EV-057 run of the spec-fuzz scenarios. The multi-scenario, failing and empty cases are **constructed and labelled as such**: no failing report survived to be captured, and a fabricated one claiming to be real would be worse than saying which is which. CON-1/CON-3 → inspection of `src/evidence/adapters/agent-eval.ts` (no subprocess; an unmatched trace id is reported, never assumed). CON-2 → TC-242. |
| FR-044 | ✅ Covered | `tests/measurement.test.ts` — TC-1003 (unplanned records refused), TC-1004 (atomic/idempotent collection), TC-1005 (definition/config refusal names both), TC-1006 (population movement beside delta; no severity), TC-1007 (missing metric is `not_computed`), TC-1008 (byte-identical report retains plans without records). Tier-1 envelope provenance/raw attachment and explicit legacy history remain TC-997/TC-998/TC-1000. |
| FR-063 | ✅ Covered | `tests/change-assurance.test.ts` TC-1261..TC-1271 verify the closed record contract, complete collections, proof premises, visible unknowns, RFC 8785/BLAKE3 sealing, mutation and malformed-input refusal, immutable lineage, exact ix-flow decisions, and non-identity boundary. |
| FR-064 | ✅ Covered | `tests/change-assurance.test.ts` TC-1272..TC-1280 verify closed attestation bindings/states, exact output integrity, canonical sealing, missing-field refusal, crash-atomic/idempotent/collision-safe intake, unavailable diagnostics, FR-030 compatibility, and no-execution/non-verdict boundaries. |
| FR-065 | ✅ Covered | `tests/change-assurance.test.ts` TC-1281..TC-1292 verify receipt shape and precedence, explicit selections, exact binding, all unhealthy/missing evidence states, ix-flow decisions, incomplete premises, unchanged FR-032 findings, determinism, compatibility, and non-identity terminology. |
| FR-045 | ✅ Covered | `tests/portfolio-report.test.ts` — TC-1011 (mixed definitions, missing/unreadable stores, relative staleness), TC-1012 (plan/collection links, same-object deterministic human/JSON views, no aggregate), TC-1013 (one command accepts repeated repository paths and no values). |
| FR-046 | ✅ Covered | `tests/semantic-module-architecture.test.ts` — TC-1125..TC-1128 define the four planes, definition/occurrence/presentation distinctions, structural-role independence, and indexed decision status. |
| FR-047 | ✅ Covered | `tests/semantic-module-architecture.test.ts` — TC-1129..TC-1133 assert positive and negative Quire, Quoin, compiler, module, and consumer ownership plus ADR-0011 levels and roles. |
| FR-048 | ✅ Covered | `tests/semantic-module-architecture.test.ts` — TC-1134..TC-1139 assert concern-specific authority, schema-source status, stores, projections, provenance, and conflict stopping. |
| FR-049 | ✅ Covered | `tests/semantic-module-architecture.test.ts` — TC-1140..TC-1144 assert open dynamic values, finite native exports, unknown-extension policy, elective regeneration, and declaration separation. |
| FR-050 | ✅ Covered | `tests/semantic-module-architecture.test.ts` — TC-1145..TC-1150 reconcile Quire ADR-0002/0003/0004/0011 and require identity-complete external decisions. |
| FR-051 | ✅ Covered | `tests/semantic-module-type-fit.test.ts` TC-1156..TC-1161 verify immutable audit identity, module provenance, external evidence revisions, drift visibility, and deterministic identity serialization. |
| FR-052 | ✅ Covered | `tests/semantic-module-type-fit.test.ts` TC-1162..TC-1168 close the module, declaration, contract-surface, Markdown-path, parse-state, instance, and reconciliation denominators. |
| FR-053 | ✅ Covered | `tests/semantic-module-type-fit.test.ts` TC-1169..TC-1176 verify every scoring axis, closed vocabularies, placeholder schemas, duplicate archetypes, blob fields, plane confusion, and missing concepts. |
| FR-054 | ✅ Covered | `tests/semantic-module-type-fit.test.ts` TC-1177..TC-1182 verify the canonical artifact set, ledger and impact records, generated projections, digests/counts, and equal-input determinism. |
| FR-055 | ✅ Covered | `tests/semantic-module-type-fit.test.ts` TC-1183..TC-1187 verify architecture, core-data, and Quire reconciliation plus follow-up boundaries and fresh-census staleness. |
| FR-056 | ✅ Covered | `tests/intervention.test.ts` — TC-1195..TC-1203 cover the versioned record envelope, unchanged producer tuple, design/arm/effect boundaries, qualifiers, causal-safety rules, governance, and exact raw evidence. |
| FR-057 | ✅ Covered | `tests/intervention.test.ts` — TC-1204..TC-1216 cover atomic governed intake, stable refusals, idempotence/collision handling, negative outcomes, deterministic claim-centered rendering, no execution path, and legacy-store compatibility. |
| FR-058 | ✅ Covered | `tests/intervention.test.ts` — TC-1217..TC-1221 consume the two byte-exact real Codex reports under `spec/evidence/agent-evals/`, derive the observed 0.5 → 0.5 effect, retain both SHA-256 digests, refuse invalid report families, force `cause_not_established`, and prove the adapter does not execute the experiment. |
| FR-059 | ✅ Covered | `tests/operational.test.ts` — TC-1223..TC-1231 cover the versioned operational envelope, full control vocabulary, standing/exercise distinction, clock derivation, typed pins, adverse outcomes, governance, and exact raw evidence. |
| FR-060 | ✅ Covered | `tests/operational.test.ts` — TC-1232..TC-1243 cover atomic governed intake, stable refusals, idempotence/collision handling, clocked discharge, deterministic claim-centered reporting, no control-execution path, and legacy-store compatibility. |
| FR-061 | ✅ Covered | `tests/operational.test.ts` — TC-1244..TC-1248 consume the byte-exact v0.22.5 workflow/run/jobs artifacts under `spec/evidence/github-actions/`, derive and atomically persist the linked operational pair, refuse malformed or mismatched inputs, preserve adverse outcomes, and prove the adapter remains offline. |
| FR-062 | ✅ Covered | TC-1249..TC-1260 verify distinct suite fan-out, unresolved bindings, deterministic reverse dependency closure, unchanged auditor verdicts, deduplicated reaffirmation events, availability states, source premises, rendering determinism, read-only boundaries, and non-scoring in `graph-analysis.test.ts` and `graph-command.test.ts`. |
| FR-066 | ✅ Covered | `tests/graph-adapters.test.ts` — TC-1293..TC-1304 cover the exact adapter registry, fail-closed Quire intake, lossless graph handoff, closed graph-quality schema and identity, exact raw attachment, invocation attestation, plan gating, bijective normalization, non-measured states, atomic intake, and no-execution boundaries. |
| FR-067 | ✅ Covered | `tests/graph-portfolio.test.ts` and `tests/graph-portfolio-command.test.ts` — TC-1305..TC-1315 cover current/history evidence, partitions and provenance, distinct availability, compatibility-gated comparison, raw identities, exact FR-062 embedding, local failures, determinism, backward compatibility, and read-only/non-scoring boundaries. |
| FR-069 | ✅ Covered | `tests/campaign-adapters.test.ts` — TC-1328..TC-1335 cover the two campaign-native formats against real pinned samples, the unsupported state that no run-entry outcome carries, fail-closed refusals, registry selection, the record command's reporting of unrepresented results, the published inventory, and the no-execution and no-scraping boundaries. |
| FR-070 | ✅ Covered | `tests/semantic-manifest.test.ts`, `tests/semantic-contract.test.ts` — TC-1336..TC-1343: manifest `semantic` block load, unknown-key, export, version, and duplicate-package rejection. |
| FR-071 | ✅ Covered | `tests/semantic-mapping.test.ts` (golden fixtures under `tests/fixtures/semantic-module/mapping/`) — TC-1344..TC-1352: typed table and `sysml` fence to identical `FieldDecl[]`, type/multiplicity/constraint cells, both-forms rejection, subset-only fence lines, golden fixtures. |
| FR-072 | ✅ Covered | `tests/semantic-mapping.test.ts` — TC-1353..TC-1359: `ClauseRef`/`OperationDecl` extraction, language and duplicate rejection, dangling pre/post, inline-vs-external duplicate. |
| FR-073 | ✅ Covered | `tests/semantic-manifest.test.ts`, `tests/semantic-contract.test.ts` — TC-1360..TC-1366: `data_schema` by path + digest, mismatch, semantic-core version drift, legacy inline warning, path escape, offline. |
| FR-074 | ✅ Covered | `tests/semantic-mapping.test.ts`, `tests/semantic-sweep-command.test.ts` — TC-1367..TC-1371: legacy form warnings on unmodified FR-006 and bullet lists, promotion guard, pack migration example. |
| FR-075 | ✅ Covered | `tests/semantic-package-manifest.test.ts` — TC-1372..TC-1378: derived package manifest, lock digests, missing import, dynamic/static identity parity, URL rejection. |
| FR-068 | ✅ Covered | `tests/change-assurance-command.test.ts` — TC-1317..TC-1327 cover record and attestation sealing from explicit inputs, atomic exact-byte intake and idempotence, receipt assembly from named selections only, preserved unavailable/not-computed/missing states, the 0/1/2 exit grammar, receipt re-verification, packaged schema emission, staging recovery, canonical goldens, and the no-execution and no-identity-claim boundaries. |
| StR-007 | ✅ Covered | `tests/graph-portfolio.test.ts` TC-1316 retains producer identity, partitions, availability, and comparison premises through the adapter-to-portfolio flow without execution or an aggregate verdict. |
| FR-041 | ✅ Covered | `tests/sbom.test.ts` — TC-231..TC-236. Both fixtures are **real tool output, unedited**: `cyclonedx-real.json` from `@cyclonedx/cyclonedx-npm` 6.0.1 over a real `npm install`, and `spdx-real.json` from GitHub's dependency-graph SBOM for `sindresorhus/slugify`. A fixture written to match the reader only proves the reader parses itself. CON-1/CON-2/CON-3 → inspection of `src/evidence/adapters/sbom.ts` (no subprocess, no new record type, no verdict). CON-4 → TC-231. |
| FR-040 | ✅ Covered | `tests/assurance.test.ts` — TC-221..TC-230, TC-261 (empty case carries a machine-readable `reason`), TC-262 (`--claim-type` matched case-insensitively). TC-224 and TC-225 are the ones that matter: a claim nothing argues for is `open`, and a requirement no claim reaches gets reported. Both were written before the code and both failed it. TC-225 earned itself on the first real run — 15 requirements over this repository, 7 of them added during this program and fixed, 8 pre-existing (`agent-ix/quoin#136`). CON-1/CON-3/CON-4 → inspection of `src/assurance/` (no subprocess, no write, the auditor's verdict used as given). CON-2 → TC-224 and TC-225. |
| FR-039 | ✅ Covered | `tests/auditor.test.ts` — TC-219 (the seven policy cases plus the mislabelled-score case), TC-269 (the `metric` discriminator: a labelled score from an uncatalogued tool is judged, no catalog is read, and an unlabelled score is not a mutation score) and TC-220 (the flag, including the two refusals). TC-220 states AC-8/AC-9 over `quoin evidence audit --mutation-floor`, not over the auditor function, because the flag is where a percentage typed as `80` would otherwise become a floor nothing can reach. CON-1 → inspection of `src/auditor/` (no subprocess). CON-2 → TC-219's unset-floor case. CON-3 → TC-219's skipped-symbol and unmeasured cases. |
| FR-038 | ✅ Covered | The `spec-fuzz` skill (`skills/spec-fuzz/`) is agent-facing, so it is verified at the eval layer rather than by vitest — TC-EV-054…TC-EV-057, implemented in `evals/scenarios/index.mjs` and run by `make evals-all`. AC-1/AC-2/AC-5 → TC-EV-054. AC-3/AC-4 → TC-EV-055, which carries **two** repos because absent tooling and an ungroundable entry point are different refusals. AC-6/AC-7 → TC-EV-056. AC-8/AC-9 → TC-EV-057. CON-1 → TC-EV-054's fixture renames a catalog method, so matching on the name `fuzzing` fails the eval. CON-2 → TC-EV-055 `absentFiles`. CON-3 → TC-EV-057. CON-4 → TC-EV-056 `absentFiles`. AC-10 → `tests/eval-assert.test.ts` TC-270, the falsifiability of those `absentFiles` gates: a brace glob the harness cannot express dies at scenario load instead of passing vacuously (#135). |
| FR-037 | ✅ Covered | `tests/completeness.test.ts` — TC-206..TC-218. Five criteria are stated over `quoin completeness` rather than over `assessVocabulary`, per the P1 review lens. TC-218 asserts quoin's unowned set equals the bundle read's for the same declaration; the same figure was checked against the engine by hand — `quire validate --okf --scope .` reports 7 unowned characteristics for this repository and so does the command. TC-217 exists because the first draft printed `PASS` over a bundle nothing had checked. CON-1 → TC-206. CON-2/CON-3 → inspection of `src/completeness/` (no corpus walk, no archetype resolution, no schema validation). CON-4 → TC-217. |
| FR-036 | ✅ Covered | `tests/arch-conformance.test.ts` — TC-197..TC-201, TC-204, TC-205; `tests/arch-boundaries.test.ts` — TC-202, TC-203. TC-197 runs against the real captured output of every `quire-rs/scripts/audits/*.sh`, checked in unedited (`tests/fixtures/evidence/audit-static-real.txt`), because a fixture written to match the reader only proves the reader parses itself. TC-202/TC-203 are quoin's own boundaries and were each observed to fail against an injected violation before being relied on (CON-4). CON-1 → TC-203, which is the invariant stated over quoin itself. CON-2 → inspection of `src/evidence/adapters/audit-script.ts` (a parser, no engine). CON-3 → TC-204. |
| FR-031 | ✅ Covered | `tests/advisor.test.ts` — TC-129 (merged catalog, first-wins, collision reported, undeclared is empty), TC-130 (attack surface → DAST/SAST/negative-abuse), TC-131 (temporal → runtime monitoring + model checking), TC-132 (reliability → fault injection), TC-133 (round-trip → property-based + metamorphic; ranked by matching-rule count), TC-134 (mismatch flagged; class match accepted), TC-135 (inconclusive reported as silence, not as a mismatch), TC-136 (unobservable axis skipped, not failed), TC-149 (every command is a build entry), TC-150 (the command exists and is wired to quire), TC-247 (`tests/catalog-fact-join.test.ts` — every declared characteristic is producible or exempt with a reason), TC-248 (no method is unreachable by every statement unless declared so), TC-249 (prose only: a link target mints nothing, `path-safety` is not `safety`), TC-250 (`high-criticality` read from the obligation's value, not a chosen threshold), TC-251 (no exemption survives the value it excused), TC-252 (the three spec-side triggers for concolic execution, with their two traps), TC-253 (the evidence-side pair, and the three cases that mint neither), TC-266 (structured `parameters` mint `quantified-threshold` regardless of prose), TC-267 (compound tokens are single tokens: `unsafe-audit` mints no memory-safety, named compounds still match), TC-268 (architectural statements in the corpus's own wording reach `architecture-conformance`), TC-273 (`tests/advisor.test.ts` — three states: uncatalogued is never mismatch, declared disagreements stay mismatches, uncatalogued survives inconclusive, absent classification degrades to two states), TC-274 (`tests/advise-command.test.ts` — the five battle-test strings classify uncatalogued end to end through the real command, and the CR-091 payload validates against the vendored contract), TC-275 (an engine predating CR-091 degrades with an explicit note, never a misclassification), TC-276 (`--*-only` filters union; the footer tallies the full population with the shown count separate). CON-1 → TC-129 + inspection of `skills/spec-evidence-analysis/SKILL.md` (no method table remains). CON-2/CON-3 → inspection of `src/advisor/` (no spec is written; rule-matched and judged results are distinguishable). |
| FR-030 | ✅ Covered | `tests/evidence-store.test.ts` — TC-119 (store under `spec/`, run path shape), TC-120 (canonical, byte-identical writes), TC-121 (suite is atomic; last-write-wins), TC-122 (passing binds, failing/skipped does not, run still recorded), TC-123 (rewording makes suspect; re-running does not clear it), TC-124 (affirmation moves the hash and records who/where), TC-125 (unmatched trace id reported), TC-126 (gc keeps latest + referenced; --dry-run deletes nothing; TC-130 pins which run "latest" means), TC-127 (absent store reads empty), TC-128 (CON-2 no statement/document/method stored), TC-129 (a second suite appends; affirmation clears every suite), TC-130 (latest is newest by timestamp, not by filename), TC-131 (a corrupt store file is named), TC-132 (byte-exact, locale-independent ordering). `tests/evidence-gc-command.test.ts` — TC-263 (the gc command surface, including the deliberate absence of `--module`). CON-1/CON-3 → inspection of `src/evidence/` (no suite is executed, no database). |
| FR-029 | ✅ Covered | `tests/quire-contract.test.ts` — TC-110 (vendored schemas hash to their recorded provenance and carry the pinned `$id`), TC-111 (conformant payloads validate, including every self-named binding-census field added by the locked engine), TC-112 (missing key / added field / value outside a closed engine enum each rejected, failing path named), TC-113 (unreadable output is a named violation, not a throw), TC-114 (version premise names found/required/consequence; an unreadable version fails rather than passes), TC-115 (numeric comparison, so `0.21.0` > `0.9.0`), TC-116 (every optional key absent and present, "every" read off the schema's own optional-key list; the v0.41.0 keys accepted; a malformed `undeclared_statuses` entry and a malformed statement hash rejected), TC-117 (the eval harness floor equals the pinned minimum), TC-118 (payloads from the **installed** `quire` validate — both `properties` and `coverage`, the contract checked against the real emitter rather than only fixtures). `tests/quire-types-conformance.test.ts` — TC-272 (the interfaces conform to the vendored coverage schema: `Required<Interface>` samples per `$defs` entry, key equality both directions, schema validation, and a spawned `tsc` enforcing the compile half). `tests/quire-exec.test.ts` — TC-254..TC-257 (the subprocess failure contract: explicit `maxBuffer`, kill-path deaths reported by cause with no stderr appended, a non-zero exit still surfacing the child's own diagnostic). CON-1/CON-2 → inspection of `src/quire/` (no hand-written validator, no network read). CON-3 → TC-112, which rejects a closed enum and leaves `diagnostics[].reason` open. |
| FR-028 | ✅ Covered | The `spec-correctness` skill (`skills/spec-correctness/`) is agent-facing, so it is verified at the eval layer rather than by vitest — TC-EV-050…TC-EV-053, implemented in `evals/scenarios/index.mjs` and run by `make evals-all`. AC-1/AC-3/AC-4 → TC-EV-050. AC-2/AC-6/AC-7 → TC-EV-051. AC-5 → TC-EV-052. AC-8/AC-9/AC-11/AC-13 → TC-EV-053. AC-10/AC-12 → Inspection of `skills/spec-correctness/` (no framework name reaches any `spec/**` output; strategy selection is keyed on `property`, never on `shape`). CON-1 → TC-EV-053 `absentFiles`. The skill's own output on this repo is `tests/props/` + a `SpecReview` under `reviews/`. |

## Non-Functional Requirements

| Requirement | Coverage   | Test / Evidence                                                                                                                                                                                                                                                              |
| ----------- | ---------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| NFR-001     | ✅ Covered | `index.test.ts` :: "lazily installs the default module set…" runs `ensureDefaultModules` against path-source fixtures with `mode: "lazy"` and no network/git.                                                                                                                |
| NFR-002     | ✅ Covered | `catalog.test.ts` :: "deduplicates repeated module roots and module names"; :: "reports a type declared by two modules" (duplicate module lists are sorted, ordering is first-wins).                                                                                         |
| NFR-003     | ✅ Covered | `write.test.ts` :: "throws with available type list when a type is not found"; `cli.test.ts` :: "an unknown command is rejected by the runner"; `plugins.test.ts` :: "throws when no manifest.yaml exists"                                                                                   |
| NFR-004     | ⚠️ Review  | Standalone-dependency claim verified by inspecting `package.json` runtime deps (`ix-cli-core`, `ts-plugin-kit`, `yaml`); `ix-flow`/`quire` are spawned, not imported. No automated assertion.                                                                                |
| NFR-005     | ⚠️ Partial | The Status-vocabulary half is now enforced: `tests/skills-vocab-drift.test.ts` TC-271 couples `skills/spec-matrix/SKILL.md` + both templates to the installed `spec-artifacts-process` manifest's classed/admitted sets (#177). The doc/object-type half — workflow launchers referencing catalog modules via `flows.ts` — remains verified by review only.                                            |
| NFR-006     | ⚠️ Review  | The agent-pty harness (`evals/run.mjs`) records latency, tokens, tool calls, validation attempts, and context fetches from the Claude Code transcript; defined in `spec/evals.md`, implemented in `evals/`.                                                                  |
| NFR-007     | ✅ Covered | `flows.test.ts` :: "rejects when ix-flow cannot be spawned (PATH has no ix-flow)"; :: "sets process.exitCode when ix-flow exits non-zero"; `write.test.ts` validation-command tests confirm `quire` is emitted, not executed. (Version pinning is an accepted gap — Review.) |
| NFR-008     | ✅ Covered | `catalog.test.ts` :: "skips candidates that do not resolve to a module root" (missing-manifest skip); :: "aborts (strict) on a present but unparseable manifest.yaml" (strict-abort path).                                                                                   |
| NFR-010     | ⚠️ Spec-ahead-of-code | Module pins record `version` and `ref` only; no resolved commit SHA is stored, so a repointed tag resolves differently under the same pin and nothing notices. Stated ahead of the implementation — `agent-ix/quoin#132`. |
| NFR-011     | ⚠️ Spec-ahead-of-code | No performance measurement exists anywhere in the repository — no benchmark, no timing assertion, no threshold. The budget is stated so a regression fails a test rather than being absorbed as "CI got slower" — `agent-ix/quoin#133`. |
| NFR-012     | ⚠️ Partial | The quire half is covered: `quire-contract.test.ts` TC-114 asserts the version premise names found/required/consequence, TC-118 validates a payload from the **installed** quire. The module half is `scripts/release-drift.js pins`, which no test invokes. |
| NFR-013     | ✅ Covered | `tests/semantic-module-architecture.test.ts` resolves index links, checks every external-decision identity field, and rejects provisional claims presented as normative (TC-1151..TC-1153). |
| NFR-014     | ✅ Covered | TC-1154 proves the branch changes no production, manifest, schema, generated-package, migration, or runtime behavior. TC-1155 records named active maintainer `kreneskyp`'s review and admin merge of PR #311 as merge commit `4a82644ad3cf75770cc53ef3812e3b13e80b516d`; SR-058 preserves the promotion evidence. |
| NFR-015     | ✅ Covered | TC-1188..TC-1191 prove complete module, type-axis, and Markdown parse-state denominators plus byte-identical equal-input output. The retained census closes 10 modules, 90 declarations, 450 contract-surface states, and 299 Markdown paths. |
| NFR-016     | ✅ Covered | TC-1192..TC-1193 prove read-only execution and changed-path isolation. TC-1194 records the human decision to admin-merge PR #316 after the architecture gate closed; SR-058 limits that decision to this read-only audit and leaves every downstream compiler, schema, migration, publication, enforcement, and retirement boundary gated. |
| NFR-017     | ✅ Covered | TC-1379..TC-1382: default module load, warning-only sweep, corpus changed-path gate, unchanged manifest `required` arrays. |

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
| FR-046 | FR-046-AC-1, FR-046-AC-2, FR-046-AC-3, FR-046-AC-4 | TC-1125, TC-1126, TC-1127, TC-1128 | ✅ Covered |
| FR-047 | FR-047-AC-1, FR-047-AC-2, FR-047-AC-3, FR-047-AC-4, FR-047-AC-5 | TC-1129, TC-1130, TC-1131, TC-1132, TC-1133 | ✅ Covered |
| FR-048 | FR-048-AC-1, FR-048-AC-2, FR-048-AC-3, FR-048-AC-4, FR-048-AC-5, FR-048-AC-6 | TC-1134, TC-1135, TC-1136, TC-1137, TC-1138, TC-1139 | ✅ Covered |
| FR-049 | FR-049-AC-1, FR-049-AC-2, FR-049-AC-3, FR-049-AC-4, FR-049-AC-5 | TC-1140, TC-1141, TC-1142, TC-1143, TC-1144 | ✅ Covered |
| FR-050 | FR-050-AC-1, FR-050-AC-2, FR-050-AC-3, FR-050-AC-4, FR-050-AC-5, FR-050-AC-6 | TC-1145, TC-1146, TC-1147, TC-1148, TC-1149, TC-1150 | ✅ Covered |
| FR-051 | FR-051-AC-1, FR-051-AC-2, FR-051-AC-3, FR-051-AC-4, FR-051-AC-5, FR-051-AC-6 | TC-1156, TC-1157, TC-1158, TC-1159, TC-1160, TC-1161 | ✅ Covered |
| FR-052 | FR-052-AC-1, FR-052-AC-2, FR-052-AC-3, FR-052-AC-4, FR-052-AC-5, FR-052-AC-6, FR-052-AC-7 | TC-1162, TC-1163, TC-1164, TC-1165, TC-1166, TC-1167, TC-1168 | ✅ Covered |
| FR-053 | FR-053-AC-1, FR-053-AC-2, FR-053-AC-3, FR-053-AC-4, FR-053-AC-5, FR-053-AC-6, FR-053-AC-7, FR-053-AC-8 | TC-1169, TC-1170, TC-1171, TC-1172, TC-1173, TC-1174, TC-1175, TC-1176 | ✅ Covered |
| FR-054 | FR-054-AC-1, FR-054-AC-2, FR-054-AC-3, FR-054-AC-4, FR-054-AC-5, FR-054-AC-6 | TC-1177, TC-1178, TC-1179, TC-1180, TC-1181, TC-1182 | ✅ Covered |
| FR-055 | FR-055-AC-1, FR-055-AC-2, FR-055-AC-3, FR-055-AC-4, FR-055-AC-5 | TC-1183, TC-1184, TC-1185, TC-1186, TC-1187 | ✅ Covered |
| FR-063 | FR-063-AC-1, FR-063-AC-2, FR-063-AC-3, FR-063-AC-4, FR-063-AC-5, FR-063-AC-6, FR-063-AC-7, FR-063-AC-8, FR-063-AC-9, FR-063-AC-10, FR-063-AC-11 | TC-1261, TC-1262, TC-1263, TC-1264, TC-1265, TC-1266, TC-1267, TC-1268, TC-1269, TC-1270, TC-1271 | ✅ Covered |
| FR-064 | FR-064-AC-1, FR-064-AC-2, FR-064-AC-3, FR-064-AC-4, FR-064-AC-5, FR-064-AC-6, FR-064-AC-7, FR-064-AC-8, FR-064-AC-9 | TC-1272, TC-1273, TC-1274, TC-1275, TC-1276, TC-1277, TC-1278, TC-1279, TC-1280 | ✅ Covered |
| FR-065 | FR-065-AC-1, FR-065-AC-2, FR-065-AC-3, FR-065-AC-4, FR-065-AC-5, FR-065-AC-6, FR-065-AC-7, FR-065-AC-8, FR-065-AC-9, FR-065-AC-10, FR-065-AC-11, FR-065-AC-12 | TC-1281, TC-1282, TC-1283, TC-1284, TC-1285, TC-1286, TC-1287, TC-1288, TC-1289, TC-1290, TC-1291, TC-1292 | ✅ Covered |
| FR-066 | FR-066-AC-1, FR-066-AC-2, FR-066-AC-3, FR-066-AC-4, FR-066-AC-5, FR-066-AC-6, FR-066-AC-7, FR-066-AC-8, FR-066-AC-9, FR-066-AC-10, FR-066-AC-11, FR-066-AC-12 | TC-1293, TC-1294, TC-1295, TC-1296, TC-1297, TC-1298, TC-1299, TC-1300, TC-1301, TC-1302, TC-1303, TC-1304 | ✅ Covered |
| FR-067 | FR-067-AC-1, FR-067-AC-2, FR-067-AC-3, FR-067-AC-4, FR-067-AC-5, FR-067-AC-6, FR-067-AC-7, FR-067-AC-8, FR-067-AC-9, FR-067-AC-10, FR-067-AC-11 | TC-1305, TC-1306, TC-1307, TC-1308, TC-1309, TC-1310, TC-1311, TC-1312, TC-1313, TC-1314, TC-1315 | ✅ Covered |
| StR-007 | StR-007-VC-1 | TC-1316 | ✅ Covered |
| FR-068 | FR-068-AC-1, FR-068-AC-2, FR-068-AC-3, FR-068-AC-4, FR-068-AC-5, FR-068-AC-6, FR-068-AC-7, FR-068-AC-8, FR-068-AC-9, FR-068-AC-10, FR-068-AC-11 | TC-1317, TC-1318, TC-1319, TC-1320, TC-1321, TC-1322, TC-1323, TC-1324, TC-1325, TC-1326, TC-1327 | ✅ Covered |
| FR-069 | FR-069-AC-1, FR-069-AC-2, FR-069-AC-3, FR-069-AC-4, FR-069-AC-5, FR-069-AC-6, FR-069-AC-7, FR-069-AC-8 | TC-1328, TC-1329, TC-1330, TC-1331, TC-1332, TC-1333, TC-1334, TC-1335 | ✅ Covered |
| FR-070 | FR-070-AC-1, FR-070-AC-2, FR-070-AC-3, FR-070-AC-4, FR-070-AC-5, FR-070-AC-6, FR-070-AC-7, FR-070-CON-1, FR-070-CON-2 | TC-1336, TC-1337, TC-1338, TC-1339, TC-1340, TC-1341, TC-1342, TC-1343, TC-1383 | ✅ Complete |
| FR-071 | FR-071-AC-1, FR-071-AC-2, FR-071-AC-3, FR-071-AC-4, FR-071-AC-5, FR-071-AC-6, FR-071-AC-7, FR-071-AC-8, FR-071-CON-1, FR-071-CON-2 | TC-1344, TC-1345, TC-1346, TC-1347, TC-1348, TC-1349, TC-1350, TC-1351, TC-1352, TC-1384 | ✅ Complete |
| FR-072 | FR-072-AC-1, FR-072-AC-2, FR-072-AC-3, FR-072-AC-4, FR-072-AC-5, FR-072-AC-6, FR-072-CON-1 | TC-1353, TC-1354, TC-1355, TC-1356, TC-1357, TC-1358, TC-1359 | ✅ Complete |
| FR-073 | FR-073-AC-1, FR-073-AC-2, FR-073-AC-3, FR-073-AC-4, FR-073-AC-5, FR-073-AC-6, FR-073-CON-1, FR-073-CON-2 | TC-1360, TC-1361, TC-1362, TC-1363, TC-1364, TC-1365, TC-1366, TC-1385 | ✅ Complete |
| FR-074 | FR-074-AC-1, FR-074-AC-2, FR-074-AC-3, FR-074-AC-4, FR-074-AC-5, FR-074-CON-1 | TC-1367, TC-1368, TC-1369, TC-1370, TC-1371, TC-1386 | ✅ Complete |
| FR-075 | FR-075-AC-1, FR-075-AC-2, FR-075-AC-3, FR-075-AC-4, FR-075-AC-5, FR-075-CON-1, FR-075-CON-2 | TC-1372, TC-1373, TC-1374, TC-1375, TC-1376, TC-1377, TC-1378 | ✅ Complete |

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
| TC-116 | A payload omitting every optional key validates; one carrying every optional key validates, where "every" is read off the schema's own optional-key list so a refresh fails until the fixture carries each addition; malformed status, statement-hash, minted-target and unmatched-tag records are rejected | Unit | P0 | FR-029-AC-7 | ✅ |
| TC-117 | The eval harness's `HARNESS_MIN_QUIRE` equals `QUIRE_CONTRACT.minimumCli`, so the two restatements cannot drift silently | Unit | P1 | FR-029-AC-8 | ✅ |
| TC-118 | Payloads emitted by the explicitly selected `QUIRE` binary validate against both vendored schemas. The live coverage bundle must emit an `implements` edge, a minted-target record and an unmatched-tag record, so a narrower stale contract or the wrong engine cannot pass through an unexercised optional field | Integration | P0 | FR-029-AC-9 | ✅ |
| TC-119 | The store root is `spec/evidence/` and a run path is `runs/<SUITE-N>/<commit12>.json` — under `spec/` because CR-045 bounds the document walk there, measured | Unit | P0 | FR-030-AC-1 | ✅ |
| TC-120 | Writes are canonical — keys sorted at every level, trailing newline — and byte-identical across repeated serialization, so a PR diff of the store IS the per-PR delta | Unit | P0 | FR-030-AC-2 | ✅ |
| TC-121 | Two suites at one commit produce two files and a re-run at the same commit keeps one: the suite is the atomic unit, and a partial run must never masquerade as a full one | Unit | P0 | FR-030-AC-3 | ✅ |
| TC-122 | A passing symbol binds with the current hash; a failing or skipped symbol binds nothing while its run is still recorded — a red build must not be something the store agrees with | Unit | P0 | FR-030-AC-4 | ✅ |
| TC-123 | A reworded statement makes the binding suspect, and re-recording the same passing run does NOT overwrite the hash — otherwise the state clears itself on the next CI run and the detector never fires | Unit | P0 | FR-030-AC-5 | ✅ |
| TC-124 | Affirmation moves the hash forward and records who and at which commit; affirming an unknown obligation reports rather than inventing a binding | Unit | P0 | FR-030-AC-6 | ✅ |
| TC-125 | A trace id no obligation states is reported as unmatched — quire-rs#72 from the other direction, a test claiming to verify something the spec does not state | Unit | P0 | FR-030-AC-7 | ✅ |
| TC-126 | `gc` deletes only runs that are neither latest for their suite nor referenced by a binding, and `--dry-run` deletes nothing | Unit | P1 | FR-030-AC-8 | ✅ |
| TC-127 | An absent store reads as an empty binding graph, an empty run list and an empty collection — never an error | Unit | P1 | FR-030-AC-9 | ✅ |
| TC-263 | `evidence gc` through the command: dry-run JSON lists without deleting, a real run deletes and reports each path, an absent store collects nothing — and the flag set is exactly `repo`/`dry-run`/`json`, pinning the deliberate absence of `--module` (#171: gc never invokes quire, so the flag would be a no-op that reads as doing something) | Unit | P1 | FR-030-AC-8 | ✅ |
| TC-128 | No obligation statement, document or method appears in the written store — only the id and hash. Anything re-derivable is re-derived, so the store cannot disagree with the spec | Unit | P0 | FR-030-AC-10 | ✅ |
| TC-129 | A second suite discharging the same obligation appends a binding rather than replacing the first; re-discharging the same suite merges into its own; affirmation clears every suite unless narrowed to one. `BindingsFile`'s own doc says the graph IS cross-suite, and keying on the obligation alone destroyed that on write | Unit | P0 | FR-030-AC-11 | ✅ |
| TC-130 | The latest run is the newest by `timestamp`, proven with fixtures whose newest run's commit prefix sorts FIRST — the arrangement where filename order and time order disagree. `gc` keeps that run, and a shared timestamp ties on commit | Unit | P0 | FR-030-AC-12 | ✅ |
| TC-131 | A merge-conflicted `bindings.json` raises a diagnostic naming the file and the cause, not a bare `SyntaxError: Unexpected token '<'`; one corrupt run file is skipped and reported rather than hiding every finding | Unit | P0 | FR-030-AC-13 | ✅ |
| TC-132 | The written binding graph is pinned byte-exactly, so a runtime's collation data cannot change the diff of a file whose diff is meant to BE the per-PR delta | Unit | P1 | FR-030-AC-14 | ✅ |
| TC-133 | A module whose `manifest.yaml` does not parse is skipped and reported on the merged catalog while the modules that parsed still merge — the crash was in the command an operator runs to diagnose module problems | Unit | P1 | FR-031-AC-9 | ✅ |
| TC-134 | The `quire` subprocess runs through one helper that captures stderr and surfaces it on a non-zero exit; `stdio: [..., "ignore"]` threw away exactly the sentence the operator needed | Static | P1 | FR-029-AC-10 | ✅ |
| TC-254 | An ENOBUFS death names the buffer overrun and the byte limit, reports no exit status, and appends no child stderr — the default 1 MiB cap killed six commands on a 1.09 MB corpus payload while the message blamed quire's harmless `DuplicateArchetype` warnings (#164) | Unit | P0 | FR-029-AC-11, FR-029-AC-12 | ✅ |
| TC-255 | A signal death names the signal, reports no exit status, and appends no child stderr | Unit | P0 | FR-029-AC-12 | ✅ |
| TC-256 | A child that itself exits non-zero still surfaces its own stderr — that is the case where the child's message IS the diagnosis, and the only one | Unit | P0 | FR-029-AC-10 | ✅ |
| TC-257 | A binary that could not be run at all is reported by its spawn error code (`ENOENT`), not as an exit status | Unit | P1 | FR-029-AC-12 | ✅ |
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
| TC-219 | The mutation-score policy: silent until a floor is declared, judged on the weakest symbol not the mean, a skipped symbol's absent score is not zero, a demanded-but-unmeasured floor is its own finding, and an entry declaring another metric is not a mutation score | Unit | P0 | FR-039-AC-1, FR-039-AC-2, FR-039-AC-3, FR-039-AC-4, FR-039-AC-5, FR-039-AC-6, FR-039-AC-7, FR-039-AC-10 | ✅ |
| TC-269 | The `metric` discriminator (#138): a `metric: mutation-score` entry from a tool no catalog lists is judged against the floor; the check reads no catalog at all; an entry without a `metric` is not a mutation score even when its tool string says `cargo-mutants` — migrate by re-recording through the adapter, which labels every entry | Unit | P0 | FR-039-AC-11, FR-039-AC-12 | ✅ |
| TC-270 | A brace glob is rejected when the harness compiles it (#135): `globToRegExp` supports `**`/`*`/`?` only, and escaping `{`/`}` compiled `*.{js,ts,mjs}` to a literal-suffix match no file can hit — loud in `fileContains`, a vacuous pass in `absentFiles`, where the assertion exists to prove a file was NOT created. Pre-fix `ok: true` with the forbidden file present is on record; the same scenario now dies at load naming the glob, and a supported glob still fails the gate honestly | Unit | P0 | FR-038-AC-10 | ✅ |
| TC-271 | The spec-matrix Status vocabulary is coupled to the installed module manifest (#177): SKILL.md's declared set and the `## Markers` list equal the manifest's classed set (`traceability.status`) exactly, the template's Status cells use exactly that set, the example stays within it, every taught marker is admitted by `column_patterns.Status`, and no Status cell reintroduces the retired concept as a note word — the seam is the installed manifest global-setup reconciles from the pins, so a pin bump re-tests the skills in the same run | Unit | P0 | NFR-005-AC-1 | ✅ |
| TC-272 | The TypeScript interfaces and the vendored coverage schema agree (#179): one sample per `$defs` entry typed `Required<Interface>` — so it must carry every interface field and can carry nothing else — has exactly its schema entry's keys in both directions, the assembled report's top-level keys equal the schema's, the whole typed payload validates, and a spawned `tsc --noEmit` over the gate file enforces the compile half (vitest strips types and the build's dts diagnostics are non-fatal, so without it an interface-side deletion would fail nothing). Red-verified both ways: deleting `form` from the interface fails tsc with TS2353 on the sample; deleting it from the schema fails the key comparison | Unit | P0 | FR-029-AC-13 | ✅ |
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
| TC-261 | A case with no claims carries a machine-readable `reason` naming the searched claim types, present exactly when `claims` is empty — `claims: []` alone cannot distinguish a clean case from one where nothing was argued (#170) | Unit | P0 | FR-040-AC-13 | ✅ |
| TC-262 | `--claim-type` matches case-insensitively — `str`, `STR` and `Hazard` all matched nothing under `===` and exited 0 with an empty case | Unit | P0 | FR-040-AC-14 | ✅ |
| TC-239 | Retired by #138 (CR-035): the no-catalog-silence rule it pinned is gone — the `metric` discriminator lives on the entry, so the check reads no catalog and TC-269 states the replacement behaviour | Unit | P0 | FR-039-AC-12 | ⛔ Retired |
| TC-941 | Finding precision/recall is reported PER FAMILY: a tool finding every marker mismatch and no vacuous suite shows recall 1 and 0, not a respectable average (#201) | Unit | P0 | FR-043-AC-2 | ✅ |
| TC-942 | A finding matching no label is a false positive; an unfindable label is excluded from the denominator AND reported, so a scored miss stays distinguishable from a defect nobody claimed was findable (#201) | Unit | P0 | FR-043-AC-2 | ✅ |
| TC-943 | A family with no denominator reports `null`, not 0 — 0/0 is not 0%, and a precision of 0 claims the run was wrong (#201) | Unit | P0 | FR-043-AC-2 | ✅ |
| TC-944 | Actionability counts findings naming a row id or a line — the property measured at 15 of 496 in pass 2 (#201) | Unit | P0 | FR-043-AC-4 | ✅ |
| TC-945 | Cost per confirmed insight reports tokens AND tool calls, divides by CONFIRMED findings rather than emitted ones, and reports `null` per-insight when nothing was confirmed (#201) | Unit | P0 | FR-043-AC-5 | ✅ |
| TC-946 | The score pairs a finding to a label by LOCATION where both name one: two right-family findings at the same place, against two labels seeded at different places, score precision 0.50 not 1.00; both in the right places score 1.00; a label naming a file with no line matches a finding in that file (#201) | Unit | P0 | FR-043-AC-2, FR-043-AC-7 | ✅ |
| TC-947 | An answer-key entry declaring `expect_metric` with no usable `expect_value` is malformed and fails, never scoring as a permanent miss — AK-003 shipped in that state (#200) | Unit | P0 | FR-043-AC-11 | ✅ |
| TC-948 | Every family the metric dictionary declares has a seeded corpus and every corpus family is declared — the cross-check that was absent while 8 families were declared and 4 seeded, so precision and recall were unmeasurable for half the dictionary and nothing said so (#199) | Unit | P0 | FR-043-AC-2, FR-043-AC-7 | ✅ |
| TC-949 | The family cross-check fails in BOTH directions, each mutated separately: a declared family nothing seeds, and a corpus family no metric governs (#199) | Unit | P0 | FR-043-AC-2 | ✅ |
| TC-950 | Declared collateral names a family, a reason and a note, and consumes ONE finding per declaration — so a consequence of the seeded defect is not scored as a false positive, and a duplicate cannot hide behind the same declaration (#199) | Unit | P0 | FR-043-AC-2, FR-043-AC-7 | ✅ |
| TC-951 | A label whose family has no working detector records which of the three reasons applies and names its ticket, so a recall of 0 says whether the tool looked and missed or never looked (#199) | Unit | P0 | FR-043-AC-7 | ✅ |
| TC-952 | Declared collateral is set aside from scoring, reported by name, and SPENT once per declaration — a second identical consequence is a false positive, and a declaration whose reason does not match absorbs nothing (#199) | Unit | P0 | FR-043-AC-2 | ✅ |
| TC-953 | `labels.json`'s per-corpus wrapper is flattened into the array `scoreFindings` consumes, in one place, carrying the corpus name — the shape mismatch that kept `buildBenchCorpora` and the scorer from ever meeting (#199) | Unit | P0 | FR-043-AC-7 | ✅ |
| TC-954 | `finding_localisation_rate` is positional pairings over true positives, and `null` — never 0 — when nothing was confirmed (#199) | Unit | P0 | FR-043-AC-4 | ✅ |
| TC-955 | The ratchet is one-way: a regression keeps the OLD baseline in both directions, so a bad run can never lower the bar (#199) | Unit | P0 | FR-043-AC-10 | ✅ |
| TC-956 | A missing baseline scores `new` — neither a pass by default nor a failure — and the observed value is proposed, not written (#199) | Unit | P0 | FR-043-AC-10 | ✅ |
| TC-957 | `gate-zero` carries no baseline and no tolerance, and its proposed baseline is forced to 0 so `--update` cannot launder a non-zero value (#199) | Unit | P0 | FR-043-AC-6 | ✅ |
| TC-958 | Every family the dictionary declares is mapped to a real payload source and key, and no mapping names a family nothing declares (#199) | Unit | P0 | FR-043-AC-2 | ✅ |
| TC-959 | The completed corpus plan leaves no family on the supported `source: none` escape hatch; every declared family now names a production source (#199, #224) | Unit | P0 | FR-043-AC-7 | ✅ |
| TC-960 | A family the baseline scored and a run does not report AT ALL is a regression with the baseline kept — deleting a corpus must not read as a clean run (#199) | Unit | P0 | FR-043-AC-10 | ✅ |
| TC-964 | A case whose `expect.yaml` names a diagnostic reason no family claims FAILS the run rather than deriving zero defects in silence, and the message names `source: none` as the way to declare a hole (#236) | Unit | P0 | FR-043-AC-2 | ✅ |
| TC-965 | A recognised reason still loads and carries its family, its `expect_reason` and the language the case declares, so the guard refuses only the hole (#236) | Unit | P0 | FR-043-AC-2 | ✅ |
| TC-966 | The score is cut by the language each case declares, partitioning the same findings and labels the headline used — a `held` verdict over a single-language corpus is not "verified in every language" (#236) | Unit | P0 | FR-043-AC-9 | ✅ |
| TC-967 | A collateral declaration on one case cannot absorb a finding from another: pairing is scoped to the declaring case, so one case's consequence does not consume a second case's seeded true positive (#238) | Unit | P0 | FR-043-AC-7 | ✅ |
| TC-968 | The report records WHICH declaration it scored against — a content digest over the module tree, a digest per bound module, and the upstream SHA the corpus vendored — and the digest moves on one byte of one manifest and holds on identical bytes (#240) | Unit | P0 | FR-043-AC-12 | ✅ |
| TC-969 | A `VENDORED.md` present and unparseable FAILS the run rather than reporting no source, and a declaration root carrying none records `sources: null` — a provenance file that has silently stopped parsing is worse than none (#240) | Unit | P0 | FR-043-AC-12 | ✅ |
| TC-970 | Cases resolve their module under an overridden declaration root, so the same corpus can be scored with the engine held fixed and the declaration moved; a module id resolving to no manifest fails the run rather than reading as a detection collapse (#240) | Unit | P0 | FR-043-AC-12 | ✅ |
| TC-971 | The ENGINE may move and still be compared; the corpus revision, the declaration digest and the scored population — count and per-language mix — may not (#240, #231) | Unit | P0 | FR-043-AC-13 | ✅ |
| TC-972 | An incomparable run withdraws the CLAIM and keeps both numbers: neither `improved` nor `regressed`, because a change was not observed — and the same run without the guard reads `regressed` (#240, #231) | Unit | P0 | FR-043-AC-13 | ✅ |
| TC-973 | A baseline recording nothing for one of those fields is reported as UNKNOWN rather than assumed to match, so a legacy baseline is neither refused nor silently trusted (#240) | Unit | P0 | FR-043-AC-13 | ✅ |
| TC-981 | An unreadable SHA on ONE row of a provenance table fails the run rather than being dropped once another row parses, and an annotated SHA is read from its first token (#240 reopened, found by #242) | Unit | P0 | FR-043-AC-12 | ✅ |
| TC-974 | Canonically resolved language-set entries become one scorable case per language with distinct ids and input trees (#246) | Unit | P0 | FR-043-AC-14 | ✅ |
| TC-975 | The real runner population and bounds equal qa-corpus's canonical envelope, with no unknown language produced by a third metadata interpretation (#246) | Integration | P0 | FR-043-AC-14 | ✅ |
| TC-976 | A canonical envelope missing its case list, numeric bounds or required case fields fails instead of becoming an empty population (#246) | Unit | P0 | FR-043-AC-14 | ✅ |
| TC-977 | The runner can consume a canonical case without reading `case.yaml`, proving it does not remain a third metadata reader (#246) | Unit | P0 | FR-043-AC-14 | ✅ |
| TC-978 | A canonical entry whose input or expectation path does not resolve fails by case id (#246) | Unit | P0 | FR-043-AC-14 | ✅ |
| TC-979 | A pending case's expiry signal comes from `expect-pending.yaml`, never from the live block where stating it would be a false claim about today, and reaches the check without a scoring family (#242, #236) | Unit | P0 | FR-043-AC-15 | ✅ |
| TC-980 | A pending case with NO forward block is distinguishable from one whose forward block this runner cannot evaluate — the first fails, the second is deferred to the corpus's graders and named (#242) | Unit | P0 | FR-043-AC-15 | ✅ |
| TC-982 | A ruling SCOPED to one declaration governs only findings that declaration raised: a control ruling out `test-case/archetype-matches-nothing` does not make the `suite` and `inspection` firings on the same tree false positives — the reading that published a precision of 0.556 (#245) | Unit | P0 | FR-043-AC-16 | ✅ |
| TC-983 | An UNSCOPED ruling still governs every firing of its family on that case, so scoping the match by declaration does not quietly turn a bare-token ruling into a ruling on nothing (#245) | Unit | P0 | FR-043-AC-16 | ✅ |
| TC-984 | A family the baseline measured that now reports a null precision is `regressed`, naming it — reclassifying to `advisory` may no longer delete a number in silence, which is how 0.167 and 0.01 both vanished (#234, #245) | Unit | P0 | FR-043-AC-16 | ✅ |
| TC-985 | A family that NEVER had a precision is still skipped, so a detector that does not exist cannot hold the gate permanently red and therefore permanently ignored (#245) | Unit | P0 | FR-043-AC-16 | ✅ |
| TC-986 | An advisory's UNADJUDICATED count is ratcheted `lower-is-better` — the rate cannot fall independently of the corpus's own differential gate, so the count is what carries the evidence (#245) | Unit | P0 | FR-043-AC-16 | ✅ |
| TC-987 | A firing nobody ruled on is counted and published beside the null, never folded into it: not-measured and not-asked are different claims, and 316 of them read as "nothing to see" (#245) | Unit | P0 | FR-043-AC-16 | ✅ |
| TC-988 | A ratio-shaped metric that walked a non-zero population, matched none and carries no diagnostic naming it is a gate violation — the shape of `555/2389 (23%)`, declared since the dictionary was written and computed by nothing (#243) | Unit | P0 | FR-043-AC-6 | ✅ |
| TC-989 | A COUNT-shaped metric reading zero is exempt: `matched` and the value are the same fact, so a zero reports that none was found, not that none was read (CR-098, #243) | Unit | P0 | FR-043-AC-6 | ✅ |
| TC-990 | Accompanied means a diagnostic NAMES the metric, in `value` or in the message — "the payload carried a diagnostic" is true of every case in this corpus and would excuse every hollow ratio in it (#243) | Unit | P0 | FR-043-AC-6 | ✅ |
| TC-991 | A metric the engine reports as not `measured`, and a ratio over a zero population, are absences rather than silent zeros — failing on either would fail the gate for the engine correctly saying it did not know (#243) | Unit | P0 | FR-043-AC-6 | ✅ |
| TC-992 | The gate reads against 0 and never consults the baseline, so `--update` cannot launder a run that violated it (#243) | Unit | P0 | FR-043-AC-6 | ✅ |
| TC-993 | Walking NONE of a population is reported and not gated: it is authoring absence, not instrument failure, and the first draft of this gate fired on three fixtures that seed exactly that on purpose (#243, quire-rs#271) | Unit | P0 | FR-043-AC-6 | ✅ |
| TC-997 | A record's tool version and configuration digest are read from the payload envelope, never from an operator string — the two fields that would have caught a CLI 0.29.0 pinning an engine sixteen releases behind (#228) | Unit | P0 | FR-043-AC-18 | ✅ |
| TC-998 | The raw report is ATTACHED to the record rather than transcribed into it, so a later reader re-derives instead of re-typing — three published SpecReviews cited hand-typed figures from a wrong-versioned binary (#228) | Unit | P0 | FR-043-AC-18 | ✅ |
| TC-1000 | Every record in the committed series carries the four join fields — engine, declaration, corpus and scorer — because a delta is only meaningful when all four are known (#229) | Unit | P0 | FR-043-AC-18 | ✅ |
| TC-1003 | A metric with no authored active MeasurementPlan is refused by name rather than stored untyped | Unit | P0 | FR-044-AC-1 | ✅ |
| TC-1004 | One complete producer collection is written atomically and an identical id/content write is byte-idempotent | Unit | P0 | FR-044-AC-2 | ✅ |
| TC-1005 | Changed definition and configuration refuse a delta and name both incompatible fields | Unit | P0 | FR-044-AC-3 | ✅ |
| TC-1006 | A population change stays beside a numeric delta and the comparison carries no verdict or severity | Unit | P0 | FR-044-AC-3 | ✅ |
| TC-1007 | A metric present in only one collection is `not_computed`, never zero or unchanged | Unit | P0 | FR-044-AC-3 | ✅ |
| TC-1008 | Re-rendering unchanged plans/store is byte-identical and an authored plan with no record remains visible as `not_computed` | Unit | P0 | FR-044-AC-4 | ✅ |
| TC-1009 | The Tier-1 execution boundary preserves structured coverage and validation locations and counts exactly the subprocesses it invokes (#239, #246) | Unit | P0 | FR-043-AC-19 | ✅ |
| TC-1010 | Tier-1 loading, execution, scoring, comparison, persistence and rendering modules are acyclic; direct module tests prove deterministic rendering from the same report (#246) | Unit | P0 | FR-043-AC-19 | ✅ |
| TC-1011 | A five-repository fixture preserves a changed definition as incomparable, names a missing store and unreadable collection, and marks exactly the repository more than 30 days behind the newest collection stale (#248) | Unit | P0 | FR-045-AC-2, FR-045-AC-3 | ✅ |
| TC-1012 | Human and canonical JSON portfolio views derive from one report; every observation links its plan and collection; no cross-repository aggregate is emitted (#248) | Unit | P0 | FR-045-AC-3, FR-045-AC-4 | ✅ |
| TC-1013 | One `quoin report` invocation accepts repeated `--portfolio` repository locations and exposes no typed observation-value flag (#248) | Integration | P0 | FR-045-AC-1 | ✅ |
| TC-995 | The committed baseline compared against ITSELF holds on every verdict and produces no `new` — so a hand-edit into a shape the scorer cannot read is contradicted by `make test`, not by the next person to type the bench command (#244) | Unit | P0 | FR-043-AC-17 | ✅ |
| TC-996 | Every family the committed baseline scores is one `bench/tier1-mapping.json` still claims: a row surviving a dropped mapping entry is a score against a contract nobody holds (#244) | Unit | P0 | FR-043-AC-17 | ✅ |
| TC-994 | An absent cost is `null` and never 0 — tier 1 calls no model, so its token cost is unmeasured rather than free — and the half it does know, its exact subprocess count, is still reported (#243) | Unit | P0 | FR-043-AC-5 | ✅ |
| TC-936 | An obligation discharged only by a mocked stand-in for its own subject is a `medium` `mocked-confirmation` finding naming the injected identifier (#204) | Unit | P0 | FR-032-AC-15 | ✅ |
| TC-937 | A mock unrelated to the statement's subject (`FakeClock`) is not reported — tests legitimately mock clocks, filesystems and networks (#204) | Unit | P0 | FR-032-AC-15 | ✅ |
| TC-961 | Every answer-key finding records HOW STRONGLY it is detected — located, aggregate or none — and the strength agrees with `now_detectable`, so "a number moved" is not counted as "here is the file" (#200) | Unit | P0 | FR-043-AC-8 | ✅ |
| TC-962 | A finding recorded as not detectable claims no `detectable_since` and no `fixed_by`: a capability nothing can reach has no date it arrived and no PR that delivered it (#200) | Unit | P0 | FR-043-AC-8 | ✅ |
| TC-963 | Each undetectable finding names its OWN tracking ticket, so closing one family's work cannot silently close another's (#200) | Unit | P0 | FR-043-AC-8 | ✅ |
| TC-938 | One real suite alongside a mocked one is not reported: standing in a dependency while another suite exercises the real path is ordinary test design (#204) | Unit | P0 | FR-032-AC-15 | ✅ |
| TC-939 | Absent injection data yields silence, never a clean bill — "nobody looked" is not "nothing was mocked" (#204) | Unit | P0 | FR-032-AC-15 | ✅ |
| TC-940 | The finding ratchets through the existing `<kind>:<obligation>` key, so it is acceptable in a baseline like every other kind (#204) | Unit | P1 | FR-032-AC-15 | ✅ |
| TC-1062 | The mock-inspection producer locates the same explicit permissive stand-in in Rust, Python and TypeScript test source (#204) | Unit | P0 | FR-032-AC-16 | ✅ |
| TC-1063 | The producer ignores production-only calls and ordinary constructors while retaining unrelated explicit mocks for the auditor's independent statement-overlap decision (#204) | Unit | P0 | FR-032-AC-16 | ✅ |
| TC-1064 | A completed empty mock inspection is stored as evidence that the suite was examined, and only the exact full commit is consumed (#204) | Unit | P0 | FR-030-AC-16, FR-032-AC-16 | ✅ |
| TC-1065 | Through real commands, `evidence inspect-mocks` records a three-language-capable source observation and `evidence audit` reports the seeded obligation with path, line, symbol and injected identifier (#204) | Integration | P0 | FR-030-AC-16, FR-032-AC-15, FR-032-AC-16 | ✅ |
| TC-1066 | Tier 1 materializes the controlled premise and executes the real record → inspect → audit command path for `audit.findings`, preserving the reported path and line in scoring (#204) | Unit | P0 | FR-032-AC-16, FR-043-AC-19 | ✅ |
| TC-1067 | A gate finding requires and joins an explicit negative obligation claim, real build/CI wiring and an unasserted forbidden-pattern count, and names the exact repair locus (#224) | Unit | P0 | FR-043-AC-20 | ✅ |
| TC-1068 | An unwired report with identical shell text remains silent because script text alone does not give it a gate role (#224) | Unit | P0 | FR-043-AC-20 | ✅ |
| TC-1069 | A wired gate with an explicit non-zero failure path remains silent (#224) | Unit | P0 | FR-043-AC-20 | ✅ |
| TC-1070 | A wired counting script with no explicit requirement claim remains silent (#224) | Unit | P0 | FR-043-AC-20 | ✅ |
| TC-1071 | The shipped `quoin validate --json` command emits the same located finding as the validator and remains advisory unless `--strict` is selected (#224) | Integration | P0 | FR-043-AC-20 | ✅ |
| TC-1072 | Every Tier-2 answer-key entry names one production source and signal; an evaluated source returning no match scores as a miss (#203) | Unit | P0 | FR-043-AC-21 | ✅ |
| TC-1073 | An unavailable Tier-2 source is reported by entry, source and missing premise; it is neither a miss nor a clean result (#203) | Unit | P0 | FR-043-AC-21 | ✅ |
| TC-1074 | The Tier-2 registry executes Quire coverage and Quoin validate, and names evidence audit as not evaluated when the pinned store has no suite-to-obligation join (#203) | Integration | P0 | FR-043-AC-21 | ✅ |
| TC-1075 | A stand-in in an unrelated test symbol within the same workspace suite does not accuse the obligation; suite-only joining is forbidden (#204) | Unit | P0 | FR-032-AC-15, FR-032-AC-16 | ✅ |
| TC-1076 | An inspector's unqualified source symbol joins an exact or terminally module-qualified result symbol, preserving the common Cargo adapter shape without substring matching (#204) | Unit | P0 | FR-032-AC-15, FR-032-AC-16 | ✅ |
| TC-1123 | Evidence intake accepts an exact semantic version, source SHA, or executable digest and rejects bare or mutable tool identities (#274) | Unit | P0 | FR-030-AC-17 | ✅ |
| TC-1124 | The first-party mock-inspection command records the exact Quoin package version in its stored tool identity (#274) | Integration | P0 | FR-030-AC-17, FR-032-AC-16 | ✅ |
| TC-1125 | Architecture record defines the meta, definition, execution/observation, and presentation planes | Static | P0 | FR-046-AC-1 | ✅ |
| TC-1126 | Definition, occurrence, and presentation examples remain distinct | Static | P0 | FR-046-AC-2 | ✅ |
| TC-1127 | Structural kind is independent of semantic role and no universal runtime envelope is introduced | Static | P0 | FR-046-AC-3 | ✅ |
| TC-1128 | Architecture index links all records and exposes provisional gates | Static | P0 | FR-046-AC-4 | ✅ |
| TC-1129 | Quire ownership and exclusions remain explicit | Static | P0 | FR-047-AC-1 | ✅ |
| TC-1130 | Quoin ownership and exclusions remain explicit | Static | P0 | FR-047-AC-2 | ✅ |
| TC-1131 | Compiler and module-repository ownership remain separate | Static | P0 | FR-047-AC-3 | ✅ |
| TC-1132 | Consumer adapter, persistence, migration, runtime, and presentation ownership remains explicit | Static | P0 | FR-047-AC-4 | ✅ |
| TC-1133 | Quire ADR-0011 validation levels, roles, and consumer-CI execution remain governing | Static | P0 | FR-047-AC-5 | ✅ |
| TC-1134 | Typed Markdown remains authoritative for authored durable knowledge | Static | P0 | FR-048-AC-1 | ✅ |
| TC-1135 | Schema/package sources remain authoritative over generated language packages | Static | P0 | FR-048-AC-2 | ✅ |
| TC-1136 | Record cites filament-core-data ADR-0005 (TypeSpec source) and describes no fallback | Static | P0 | FR-048-AC-3 | ✅ |
| TC-1137 | Transactional and operational stores remain authoritative over Markdown reports | Static | P0 | FR-048-AC-4 | ✅ |
| TC-1138 | Wire, analytical, and export representations remain selectable projections | Static | P0 | FR-048-AC-5 | ✅ |
| TC-1139 | Competing authorities stop promotion and last-writer-wins is rejected | Static | P0 | FR-048-AC-6 | ✅ |
| TC-1140 | Dynamic consumers preserve unknown namespaced module data | Static | P0 | FR-049-AC-1 | ✅ |
| TC-1141 | Static consumers use finite versioned native exports | Static | P0 | FR-049-AC-2 | ✅ |
| TC-1142 | Unknown extensions follow an explicit preserve, reject, or surface profile | Static | P0 | FR-049-AC-3 | ✅ |
| TC-1143 | Dynamic installation does not force static regeneration | Static | P0 | FR-049-AC-4 | ✅ |
| TC-1144 | Distribution manifests remain distinct from exports, targets, mappings, and profiles | Static | P0 | FR-049-AC-5 | ✅ |
| TC-1145 | Unified archetype shape remains structural rather than a universal semantic base class | Static | P0 | FR-050-AC-1 | ✅ |
| TC-1146 | Direct typed Markdown and document-boundary canonical Markdown remain preserved | Static | P0 | FR-050-AC-2 | ✅ |
| TC-1147 | Draft rendering ownership is historical while byte-splicing remains preserved | Static | P0 | FR-050-AC-3 | ✅ |
| TC-1148 | Accepted Quire ADR-0011 remains governing | Static | P0 | FR-050-AC-4 | ✅ |
| TC-1149 | Quire, Quoin, and compiler exclusions prevent boundary regression | Static | P0 | FR-050-AC-5 | ✅ |
| TC-1150 | External decisions retain repository, path, status, and revision or date | Static | P0 | FR-050-AC-6 | ✅ |
| TC-1151 | Every external decision entry carries complete identity metadata | Static | P0 | NFR-013 | ✅ |
| TC-1152 | Every local architecture-index link resolves | Static | P0 | NFR-013 | ✅ |
| TC-1153 | Provisional and unresolved claims cannot appear as normative | Static | P0 | NFR-013 | ✅ |
| TC-1154 | Changed-path guard rejects behavior, manifest, schema, generated-package, and migration changes; the unchanged 763-test suite passes | Static | P0 | NFR-014 | ✅ |
| TC-1155 | Merge requires named Quoin/Quire maintainer review | Inspection | P0 | NFR-014 | ✅ |
| TC-1156 | Snapshot records Quoin identity, cleanliness, manifest digest, version, and run timestamp | Static | P0 | FR-051-AC-1 | ✅ |
| TC-1157 | Every default-module declaration retains canonical source, request, and resolved full SHA | Static | P0 | FR-051-AC-2 | ✅ |
| TC-1158 | Every inspected module retains content, manifest, path, commit, and cleanliness identity | Static | P0 | FR-051-AC-3 | ✅ |
| TC-1159 | Snapshot pins Quire CLI/engine, quire-rs corpus, and core-data census evidence | Static | P0 | FR-051-AC-4 | ✅ |
| TC-1160 | Provenance disagreement remains typed and blocks a clean verdict | Unit | P0 | FR-051-AC-5 | ✅ |
| TC-1161 | Declaration ordering and canonical path/digest encoding make equal inputs byte-identical | Property | P0 | FR-051-AC-6 | ✅ |
| TC-1162 | Module denominator equals manifest entries without collapsing duplicates or failures | Unit | P0 | FR-052-AC-1 | ✅ |
| TC-1163 | Every artifact and object declaration appears exactly once per declaration | Unit | P0 | FR-052-AC-2 | ✅ |
| TC-1164 | Every type records each contract surface as present or explicitly absent | Unit | P0 | FR-052-AC-3 | ✅ |
| TC-1165 | Every Markdown path receives exactly one parse state and every failure has a reason | Unit | P0 | FR-052-AC-4 | ✅ |
| TC-1166 | Parsed documents retain type, identity, and definition/occurrence signals | Unit | P0 | FR-052-AC-5 | ✅ |
| TC-1167 | Types without a representative document retain an explicit no-instance observation | Unit | P0 | FR-052-AC-6 | ✅ |
| TC-1168 | Reconciliation proves every source and inventory denominator equal | Property | P0 | FR-052-AC-7 | ✅ |
| TC-1169 | Every declaration receives every semantic-fit axis | Unit | P0 | FR-053-AC-1 | ✅ |
| TC-1170 | Axis status, confidence, and evidence use closed complete vocabularies | Unit | P0 | FR-053-AC-2 | ✅ |
| TC-1171 | Every declaration receives exactly one derived disposition | Unit | P0 | FR-053-AC-3 | ✅ |
| TC-1172 | Placeholder schemas cannot masquerade as generated-code or round-trip support | Unit | P0 | FR-053-AC-4 | ✅ |
| TC-1173 | Duplicate archetypes retain qualified identities and conflicts | Unit | P0 | FR-053-AC-5 | ✅ |
| TC-1174 | JSON/string blobs and free-form Markdown are evaluated for structural loss | Unit | P0 | FR-053-AC-6 | ✅ |
| TC-1175 | Occurrence signals in definition-shaped types produce plane-confusion evidence | Unit | P0 | FR-053-AC-7 | ✅ |
| TC-1176 | Run, result, evidence, report, relationship, identity, version, provenance, and lifecycle concepts are explicitly evaluated | Static | P0 | FR-053-AC-8 | ✅ |
| TC-1177 | Canonical output contains the complete versioned artifact set | Unit | P0 | FR-054-AC-1 | ✅ |
| TC-1178 | Every ledger row carries stable identity, impact, evidence, rationale, and next boundary | Unit | P0 | FR-054-AC-2 | ✅ |
| TC-1179 | Repository impact closes every named boundary with effort, risk, wave, and confidence | Unit | P0 | FR-054-AC-3 | ✅ |
| TC-1180 | SpecReview and report are generated projections with no independent findings | Snapshot | P0 | FR-054-AC-4 | ✅ |
| TC-1181 | Summary digests and counts reject missing, stale, unreferenced, or disagreeing artifacts | Unit | P0 | FR-054-AC-5 | ✅ |
| TC-1182 | Equal-input runs produce byte-identical canonical content apart from isolated run time | Property | P0 | FR-054-AC-6 | ✅ |
| TC-1183 | Findings cite data plane, authority, owner, and decision | Static | P0 | FR-055-AC-1 | ✅ |
| TC-1184 | Core-data census overlap is reconciled without shadow contracts | Static | P0 | FR-055-AC-2 | ✅ |
| TC-1185 | Quire implications preserve and cite its pinned boundary evidence | Static | P0 | FR-055-AC-3 | ✅ |
| TC-1186 | Follow-up boundaries and major-interference gates are explicit | Static | P0 | FR-055-AC-4 | ✅ |
| TC-1187 | Fresh-census drift makes the review stale and blocks signoff | Unit | P0 | FR-055-AC-5 | ✅ |
| TC-1188 | Inventoried module count equals declared default-module count | Property | P0 | NFR-015 | ✅ |
| TC-1189 | Every inventoried type declaration has every required axis assessment | Property | P0 | NFR-015 | ✅ |
| TC-1190 | Every discovered Markdown path has exactly one retained parse state | Property | P0 | NFR-015 | ✅ |
| TC-1191 | Equal-input canonical artifact bytes never differ | Property | P0 | NFR-015 | ✅ |
| TC-1192 | Read-only fixture inputs and all paths outside the output root are unchanged | Integration | P0 | NFR-016 | ✅ |
| TC-1193 | Changed-path guard permits audit/spec/review/plan files and rejects production, module, schema, skeleton, registry, generated, migration, and consumer changes | Static | P0 | NFR-016 | ✅ |
| TC-1194 | Major-interference recommendations stop at their named human gate | Inspection | P0 | NFR-016 | ✅ |
| TC-1195 | The v1 schema requires record and subject identities, an observation timestamp, and every unchanged FR-044 producer-tuple field | Unit | P0 | FR-056-AC-1 | ✅ |
| TC-1196 | Each missing producer field, mutable tool version, empty environment, and malformed configuration digest is rejected independently | Unit | P0 | FR-056-AC-2 | ✅ |
| TC-1197 | Repeated, randomized, and factorial designs accept their valid assignment combinations; randomized assignment without a seed, zero repetitions, and empty sampling conditions fail at their boundaries | Unit | P0 | FR-056-AC-3 | ✅ |
| TC-1198 | Unique baseline and treatment ids, treatment-linked changed variables, and an explicit held-constant list validate; duplicate or dangling treatment references fail | Property | P0 | FR-056-AC-4 | ✅ |
| TC-1199 | Treatment-linked numeric, textual, boolean, and unavailable effect values round-trip without coercing unavailable to zero; duplicate treatment/metric keys fail | Property | P0 | FR-056-AC-5 | ✅ |
| TC-1200 | Interaction and confounder entries preserve all four dispositions and never move between the two collections | Property | P0 | FR-056-AC-6 | ✅ |
| TC-1201 | Completed, failed, and inconclusive records validate, while failed/inconclusive records with any conclusion other than `cause_not_established` fail | Unit | P0 | FR-056-AC-7 | ✅ |
| TC-1202 | Causal and no-effect conclusions require positive samples and observed comparisons; causal conclusions also require non-null effects, non-none confidence, and no uncontrolled/unknown qualifier | Unit | P0 | FR-056-AC-8 | ✅ |
| TC-1203 | Gaps, owner, actions, and safe content-digested raw evidence with media type and byte size are required, and every undeclared field is refused recursively | Property | P0 | FR-056-AC-9 | ✅ |
| TC-1204 | Valid intake writes exactly one complete canonical record by one atomic same-directory no-replace publication, and an existing concurrent destination is never overwritten | Integration | P0 | FR-057-AC-1 | ✅ |
| TC-1205 | Every invalid schema or cross-record integrity path is reported with `invalid_record` and no store entry is written | Unit | P0 | FR-057-AC-2 | ✅ |
| TC-1206 | An absent or mismatched governing definition returns its stable reason code, names requested, expected, and observed versions, and leaves the store unchanged | Unit | P0 | FR-057-AC-3 | ✅ |
| TC-1207 | Repeating identical canonical bytes for one id is byte-idempotent | Property | P0 | FR-057-AC-4 | ✅ |
| TC-1208 | A same-id/different-bytes collision returns `record_id_collision` and cannot replace the retained entry | Property | P0 | FR-057-AC-5 | ✅ |
| TC-1209 | Completed, failed, inconclusive, and cause-not-established records remain queryable with byte-identical raw-evidence digests | Unit | P0 | FR-057-AC-6 | ✅ |
| TC-1210 | Experiment conclusions render as claims while measured effects and raw references render only as evidence | Unit | P0 | FR-057-AC-7 | ✅ |
| TC-1211 | Unknown/uncontrolled interactions and confounders render as counterevidence, and gaps remain separate beside owner and actions | Unit | P0 | FR-057-AC-8 | ✅ |
| TC-1212 | Human and JSON output contain no derived overall trust, confidence, or quality score and never infer causality from absent qualifiers | Unit | P0 | FR-057-AC-9, FR-057-CON-2 | ✅ |
| TC-1213 | Reordered store input renders byte-identically and human/JSON views expose the same claim-centered sections | Property | P0 | FR-057-AC-10 | ✅ |
| TC-1214 | Intake and reporting spawn no experiment or producer process | Static | P0 | FR-057-CON-1 | ✅ |
| TC-1215 | Existing measurement collections and pre-experiment evidence remain readable without migration | Integration | P0 | FR-057-CON-3 | ✅ |
| TC-1216 | Unsafe, missing, wrong-sized, and digest-mismatched raw evidence returns `raw_evidence_mismatch`, identifies each mismatch, and leaves the record store unchanged | Property | P0 | FR-057-AC-11 | ✅ |
| TC-1217 | Two real reports governed by a supported input-schema declaration and carrying the same scenario ids produce treatment-linked sample counts, pass rates, treatment-minus-baseline effects, and the treatment observation clock | Integration | P0 | FR-058-AC-1 | ✅ |
| TC-1218 | An absent or unsupported input-schema declaration and empty, malformed, structurally incompatible, duplicate, or scenario-mismatched reports are refused without an intervention record | Property | P0 | FR-058-AC-2 | ✅ |
| TC-1219 | Observation time and raw-evidence media type, byte size, and digest derive from exact report bytes and ignore caller-supplied substitutes | Property | P0 | FR-058-AC-3 | ✅ |
| TC-1220 | Inadequate repetition, uncontrolled/unknown qualifiers, and absent attribution method yield `cause_not_established`, none confidence, and explicit gaps | Property | P0 | FR-058-AC-4 | ✅ |
| TC-1221 | A real external two-run evaluation feeds retained reports to the producer, which spawns no process and persists through FR-057 | E2E | P0 | FR-058-AC-5 | ✅ |
| TC-1223 | The v1 envelope requires identities, observation timestamp, deployed scope, record shape, control kind, governance, raw evidence, and every unchanged producer-tuple field | Unit | P0 | FR-059-AC-1 | ✅ |
| TC-1224 | Every declared release, deployment, control, fallback, reporting, and version-pinning kind validates while an unknown kind is refused | Unit | P0 | FR-059-AC-2 | ✅ |
| TC-1225 | A standing capability requires surface, availability, roles, coverage, limitations, transitions, and an explicit clock-support choice; only supported clocks admit and require event/deadline fields | Unit | P0 | FR-059-AC-3 | ✅ |
| TC-1226 | An actual or drill exercise requires ordered timing, actor, trigger, outcome, before/after state, at least one observation, and clock applicability | Unit | P0 | FR-059-AC-4 | ✅ |
| TC-1227 | Exactly one of capability and exercise is present for every generated valid record, and adding or removing the discriminator-matched payload fails | Property | P0 | FR-059-AC-5 | ✅ |
| TC-1228 | Clocked exercises accept only timestamp-consistent open, met, or missed states; gaps cannot override a deterministic status; not-applicable clocks exclude clock timestamps | Property | P0 | FR-059-AC-6 | ✅ |
| TC-1229 | Every pin control requires a matching typed identity, revision, and digest; empty, duplicate, and wrong-kind pin lists fail | Property | P0 | FR-059-AC-7 | ✅ |
| TC-1230 | Succeeded, failed, partial, and aborted exercises all round-trip without collapsing their outcome | Property | P0 | FR-059-AC-8 | ✅ |
| TC-1231 | Gaps, owner, actions, and safe content-digested raw evidence with media type and byte size are required; invalid capability links and undeclared fields fail | Property | P0 | FR-059-AC-9 | ✅ |
| TC-1232 | Valid intake writes exactly one complete canonical operational record by one atomic same-directory no-replace publication | Integration | P0 | FR-060-AC-1 | ✅ |
| TC-1233 | Invalid schema, cross-record, or temporal input returns `invalid_record`; unsafe or byte-mismatched raw evidence returns `raw_evidence_mismatch`; neither writes a store entry | Property | P0 | FR-060-AC-2 | ✅ |
| TC-1234 | An absent or mismatched governing definition returns its stable reason code, names requested, expected, and observed versions, and leaves the store unchanged | Unit | P0 | FR-060-AC-3 | ✅ |
| TC-1235 | Repeating identical canonical bytes for one id is byte-idempotent for standalone records and records already retained in a pair | Property | P0 | FR-060-AC-4 | ✅ |
| TC-1236 | Same-id/different-bytes races at one destination and across standalone/pair containers cannot replace or duplicate the retained logical id; a stale global intake lock fails closed as `intake_busy` | Property | P0 | FR-060-AC-5 | ✅ |
| TC-1237 | Both standing capabilities and every exercise outcome remain queryable with byte-identical raw-evidence digests | Unit | P0 | FR-060-AC-6 | ✅ |
| TC-1238 | Only a kind, subject, scope, accepted-mode, and exact obligation-clock-condition match with succeeded outcome and timestamp-derived timely completion discharges; every other case names non-discharge | Property | P0 | FR-060-AC-7 | ✅ |
| TC-1239 | Only available capabilities and succeeded clock-satisfying exercises render as claims/evidence; unavailable, unknown, not-applicable, adverse, and incomplete states render as counterevidence/gaps | Unit | P0 | FR-060-AC-8 | ✅ |
| TC-1240 | Human and JSON output contain no derived overall trust, confidence, or quality score | Unit | P0 | FR-060-AC-9 | ✅ |
| TC-1241 | Reordered store input renders byte-identically and human/JSON views expose the same claim-centered sections | Property | P0 | FR-060-AC-10 | ✅ |
| TC-1242 | Intake and reporting invoke, drill, or alter no operational control | Static | P0 | FR-060-CON-1 | ✅ |
| TC-1243 | Existing measurement collections and intervention evidence remain readable and render alongside operational records without migration | Integration | P0 | FR-060-CON-2 | ✅ |
| TC-1244 | Quoin's retained release workflow and a real completed workflow-run/jobs export produce one linked capability/exercise pair with source-derived workflow, revision, actor, timing, outcome, and observations | Integration | P0 | FR-061-AC-1 | ✅ |
| TC-1245 | Malformed input, workflow/path/event/revision mismatch, and absent, duplicate, unstarted, or incomplete release jobs are refused without either operational record | Unit | P0 | FR-061-AC-2 | ✅ |
| TC-1246 | Workflow structure determines capability state and API exports determine exercise/clock state; caller-supplied substitutes cannot change them | Property | P0 | FR-061-AC-3 | ✅ |
| TC-1247 | Every non-success GitHub conclusion remains a named non-success exercise and cannot discharge a clocked release obligation | Property | P0 | FR-061-AC-4 | ✅ |
| TC-1248 | A real release run is captured outside Quoin, then retained artifacts are consumed with no network/process execution and the linked pair is persisted all-or-nothing through FR-060 | Integration | P0 | FR-061-AC-5 | ✅ |
| TC-1249 | Fan-out emits one row per suite with the sorted distinct live obligations and owners; duplicate symbols and duplicate suite-obligation records do not inflate the count | Property | P0 | FR-062-AC-1 | ✅ `graph-analysis.test.ts` |
| TC-1250 | A binding whose obligation is absent from the accepted export remains named under its suite and in gaps while contributing nothing to the live-obligation count | Unit | P0 | FR-062-AC-2 | ✅ `graph-analysis.test.ts` |
| TC-1251 | The exact default and replacement relation sets are recorded; reverse dependency closure reaches transitive and shared dependents, terminates cycles, and selects the lexicographically first shortest path independent of input order | Property | P0 | FR-062-AC-3 | ✅ `graph-analysis.test.ts` |
| TC-1252 | Every reached StR, US, FR, or NFR joins all live obligations and suites with depth and path; an unknown or non-requirement seed is a named gap with no partial closure for that seed | Unit | P0 | FR-062-AC-4 | ✅ `graph-analysis.test.ts` |
| TC-1253 | A reachable supported binding remains supported while change exposure is reported separately, and every other auditor verdict is copied byte-for-byte | Unit | P0 | FR-062-AC-5, FR-062-CON-3 | ✅ `graph-analysis.test.ts` |
| TC-1254 | One affirmation copied across several suite bindings deduplicates by obligation, actor, commit, and note while retaining every affected suite | Property | P0 | FR-062-AC-6 | ✅ `graph-analysis.test.ts` |
| TC-1255 | Churn retains zero-event live obligations, sorts count-descending then id, and reports affirmation history for an absent obligation only as a gap | Property | P0 | FR-062-AC-7 | ✅ `graph-analysis.test.ts` |
| TC-1256 | Every view carries the accepted full source revision and format/module premises byte-for-byte across repeated analysis of identical inputs | Property | P0 | FR-062-AC-8 | ✅ `graph-analysis.test.ts` |
| TC-1257 | Invalid inputs, absent/unreadable required `--export`/`--premises`/`--audit` paths, non-exact premises, mismatched audit identity, and syntactically valid but malformed bindings fail closed; unavailable retained inputs never appear as a healthy zero | Unit | P0 | FR-062-AC-9 | ✅ `graph-analysis.test.ts`, `graph-command.test.ts` |
| TC-1258 | Reordered equivalent premises, auditor verdict arrays, and graph inputs emit byte-identical JSON, and human and JSON render from the same sorted report object | Property | P0 | FR-062-AC-10 | ✅ `graph-analysis.test.ts` |
| TC-1259 | A non-JSON command-path and static boundaries reject the inherited update check plus producer, suite, Quire, Git, network, write, frontmatter-reader, and independently built graph dependencies from all three views | Static | P0 | FR-062-AC-11, FR-062-CON-1, FR-062-CON-4 | ✅ `graph-command.test.ts` |
| TC-1260 | Without graph invocation, evidence audit, assurance-case, and measurement goldens stay byte-identical; graph outputs contain no score or threshold classification | Integration | P0 | FR-062-AC-12, FR-062-CON-2 | ✅ `graph-analysis.test.ts` plus unchanged regression suites |
| TC-1261 | Change-record schema v1 requires every identity, revision, parent, digest, subject, source, impact, definition, and review binding and refuses undeclared fields | Unit | P1 | FR-063-AC-1 | ✅ |
| TC-1262 | Input permutations preserve required meaningful-empty collections, while duplicate source, requirement, constraint, proof, and unknown identities are refused | Property | P1 | FR-063-AC-2 | ✅ |
| TC-1263 | Every requirement retains its reviewed statement, and every proof retains owning evidence obligations, literal argv, repository-relative cwd, evidence kind, tool identity, and configuration digest | Unit | P1 | FR-063-AC-3 | ✅ |
| TC-1264 | Incomplete/truncated impact premises and every unknown disposition remain explicit and cannot become a completeness claim | Unit | P1 | FR-063-AC-4 | ✅ |
| TC-1265 | Official RFC 8785 vectors produce pinned canonical UTF-8 bytes and pinned BLAKE3 lowercase-hex record digests across supported runtimes | Unit | P1 | FR-063-AC-5 | ✅ |
| TC-1266 | Mutating each source, impact, definition, workflow, revision, and parent leaf independently invalidates the sealed record digest | Property | P1 | FR-063-AC-6 | ✅ |
| TC-1267 | Duplicate names, lone surrogates, non-I-JSON numbers, locale ordering, uppercase/prefixed hex, and BOM bytes fail closed | Property | P1 | FR-063-AC-7 | ✅ |
| TC-1268 | Genesis and successor lineage accepts only retained valid N-1 parents with the same record id and rejects missing, changed, skipped, and cross-record parents | Property | P1 | FR-063-AC-8 | ✅ |
| TC-1269 | Sealing a successor leaves the parent path/bytes unchanged and preserves rejected and revise-requested revisions by digest | Integration | P1 | FR-063-AC-9, FR-063-CON-2 | ✅ |
| TC-1270 | Only an intact matching ix-flow human review event for the exact run, kind, record, revision, and digest can approve a record | Integration | P1 | FR-063-AC-10 | ✅ |
| TC-1271 | Static and golden-text checks prove sealing runs nothing and makes no identity, authority, signature, authenticity, or non-repudiation claim | Static | P1 | FR-063-AC-11, FR-063-CON-1, FR-063-CON-3 | ✅ |
| TC-1272 | Proof-attestation schema v1 requires every record/candidate/proof/command/tool/config/environment/time/result/output binding and refuses undeclared fields | Unit | P1 | FR-064-AC-1 | ✅ |
| TC-1273 | Passed, failed, unavailable, and not-computed producer results round-trip distinctly and intake assigns no verification verdict | Unit | P1 | FR-064-AC-2 | ✅ |
| TC-1274 | Exact retained bytes reproduce BLAKE3 digest and size, while absent, changed, or extra bytes leave no newly stored artifact | Property | P1 | FR-064-AC-3 | ✅ |
| TC-1275 | Attestation RFC 8785/BLAKE3 vectors are pinned and each semantic-field mutation invalidates the attestation digest | Property | P1 | FR-064-AC-4 | ✅ |
| TC-1276 | Each absent record, candidate, proof, command, tool/configuration, environment, time, result, and output field is independently refused rather than inferred | Property | P1 | FR-064-AC-5, FR-064-CON-2 | ✅ |
| TC-1277 | Identical intake is byte-idempotent, failures before atomic directory rename expose neither artifact and recover cleanly, and a same-digest changed-content collision preserves the first pair | Integration | P1 | FR-064-AC-6 | ✅ |
| TC-1278 | Unavailable and not-computed attestations retain diagnostics, while wholly missing evidence creates no synthetic attestation | Unit | P1 | FR-064-AC-7 | ✅ |
| TC-1279 | Existing FR-030 run records and readers remain byte-compatible beside the distinct proof-attestation store family | Integration | P1 | FR-064-AC-8 | ✅ |
| TC-1280 | Static boundaries and golden terminology prove attestation intake runs nothing and assigns no audit, approval, identity, authorization, or non-repudiation conclusion | Static | P1 | FR-064-AC-9, FR-064-CON-1, FR-064-CON-3 | ✅ |
| TC-1281 | Receipt schema v1 retains every record/candidate, workflow, parent, check, proof, auditor finding, unknown, outcome, reason, and digest field and refuses undeclared fields | Unit | P1 | FR-065-AC-1 | ✅ |
| TC-1282 | Invalid dominates incomplete, incomplete dominates valid, and only a valid outcome permits an empty reason set under check permutations | Property | P1 | FR-065-AC-2 | ✅ |
| TC-1283 | Every reviewed proof emits one ordered receipt; missing, duplicate, and unknown-proof selections fail distinctly, while unselected stored attestations have no effect | Property | P1 | FR-065-AC-3 | ✅ |
| TC-1284 | Independent record, candidate, proof, argv/cwd, tool, and configuration mismatches invalidate the selected attestation and name the premise | Property | P1 | FR-065-AC-4 | ✅ |
| TC-1285 | Only exact-output passed evidence with healthy owning obligations can discharge; failed, stale, suspect, vacuous, unrelated, mismatched, and other defect evidence is invalid | Property | P1 | FR-065-AC-5 | ✅ |
| TC-1286 | Missing attestation/output, unavailable/not-computed result, and audit not-evaluated are incomplete and never passing or zero | Unit | P1 | FR-065-AC-6 | ✅ |
| TC-1287 | Exactly one intact approval permits review validity; missing history/event is incomplete; duplicate decisions, broken chain, mismatch, rejection, and revise request are invalid | Integration | P1 | FR-065-AC-7 | ✅ |
| TC-1288 | Incomplete/truncated impact and every non-resolved unknown stay named and force incomplete verification without blocking an advisory workflow | Unit | P1 | FR-065-AC-8 | ✅ |
| TC-1289 | Every FR-032 finding kind and obligation id is retained byte-for-byte beside its mapped reason without changing the source auditor verdict | Integration | P1 | FR-065-AC-9, FR-065-CON-2 | ✅ |
| TC-1290 | Input and selection permutations emit byte-identical canonical receipt JSON and the pinned RFC 8785/BLAKE3 digest | Property | P1 | FR-065-AC-10 | ✅ |
| TC-1291 | Existing audit/assurance goldens remain byte-identical without selection and static boundaries prove receipt generation runs and writes nothing | Integration | P1 | FR-065-AC-11, FR-065-CON-1, FR-065-CON-4 | ✅ |
| TC-1292 | Golden terminology describes actor labels and hashes only as recorded attribution and integrity, never identity, authority, authenticity, signature, or non-repudiation | Static | P1 | FR-065-AC-12, FR-065-CON-3 | ✅ |
| TC-1293 | The adapter registry exposes the two exact versioned graph adapters, and an unknown name fails with the available names before record parsing | Unit | P0 | FR-066-AC-1 | ✅ `tests/graph-adapters.test.ts` |
| TC-1294 | A valid Quire export is preserved field-for-field, while unknown format, module-version, and schema-digest premises each fail before any graph record is returned | Unit | P0 | FR-066-AC-2 | ✅ `tests/graph-adapters.test.ts` |
| TC-1295 | Standalone and portfolio graph views receive the same authoritative artifact, obligation, symbol, relation, observation, and availability records with no frontmatter read or relation translation | Integration | P0 | FR-066-AC-3, FR-066-CON-3 | ✅ `tests/graph-adapters.test.ts` |
| TC-1296 | Graph-quality v1 validation recomputes the canonical observation id and refuses changed, missing, or unknown fields before constructing a collection | Property | P0 | FR-066-AC-4 | ✅ `tests/graph-adapters.test.ts` |
| TC-1297 | The named scorer attachment is retained byte-for-byte with media type and digest; missing or changed bytes leave no collection | Property | P0 | FR-066-AC-5 | ✅ `tests/graph-adapters.test.ts` |
| TC-1298 | Subject, scope, timestamp, environment, every verification-stack field, and immutable producer identity are independently required and never synthesized | Unit | P0 | FR-066-AC-6, FR-066-CON-2 | ✅ `tests/graph-adapters.test.ts` |
| TC-1299 | An absent, inactive, wrong-id, or wrong-definition graph-quality plan is refused with expected and observed premises | Unit | P0 | FR-066-AC-7 | ✅ `tests/graph-adapters.test.ts` |
| TC-1300 | Population totals and every language, node-kind, relation-kind, and resolver-tier census cell map bijectively to sorted count observations | Property | P0 | FR-066-AC-8 | ✅ `tests/graph-adapters.test.ts` |
| TC-1301 | Every confusion component, unresolved count, ambiguous count, and recall ratio maps bijectively, while two facts mapping to one normalized key are refused | Property | P0 | FR-066-AC-9 | ✅ `tests/graph-adapters.test.ts` |
| TC-1302 | Empty, unreadable, and unsupported inputs retain valid census rows, emit one distinct not-computed state, and emit no result row | Unit | P0 | FR-066-AC-10 | ✅ `tests/graph-adapters.test.ts` |
| TC-1303 | Identical adapter intake is byte-idempotent, same-id changed content cannot replace a collection, and direct collection intake is unchanged | Integration | P0 | FR-066-AC-11, FR-066-CON-4 | ✅ `tests/graph-adapters.test.ts` |
| TC-1304 | Static boundaries reject Quire, extractor, scorer, suite, Git, network, and frontmatter-reader dependencies from the adapters | Static | P0 | FR-066-AC-12, FR-066-CON-1, FR-066-CON-3 | ✅ `tests/graph-adapters.test.ts` |
| TC-1305 | Each repository's current graph-quality row carries its active plan, definition, full producer tuple, source/corpus revisions, population identity, and raw record/scorer digests | Unit | P0 | FR-067-AC-1 | ✅ `tests/graph-portfolio.test.ts` |
| TC-1306 | History retains every structurally readable graph collection in timestamp/id order, including an explicitly incompatible record no longer governed by the active plan | Unit | P0 | FR-067-AC-2 | ✅ `tests/graph-portfolio.test.ts` |
| TC-1307 | Every measure/dimension/key partition remains separate under input and repository permutations; no partition or repository is summed or averaged | Property | P0 | FR-067-AC-3 | ✅ `tests/graph-portfolio.test.ts` |
| TC-1308 | Missing, unreadable, incompatible, unknown, and not-applicable availability remain distinct from measured/not-computed and from numeric zero | Property | P0 | FR-067-AC-4 | ✅ `tests/graph-portfolio.test.ts` |
| TC-1309 | Matching plans, definitions, configurations, tools, corpus revisions, and population identities permit a delta; changing each one independently blocks it and names the mismatch | Property | P0 | FR-067-AC-5 | ✅ `tests/graph-portfolio.test.ts` |
| TC-1310 | Current, history, and comparison rows resolve to the exact retained producer-record and scorer digests rather than copied summary fields | Integration | P0 | FR-067-AC-6 | ✅ `tests/graph-portfolio.test.ts` |
| TC-1311 | An accepted export/premises/audit triple embeds standalone FR-062 fan-out/churn and requested impact objects byte-for-byte; a partial triple is incompatible without reads, absent inputs are missing, and absent seeds are not-applicable | Integration | P0 | FR-067-AC-7, FR-067-CON-3 | ✅ `tests/graph-portfolio.test.ts`, `tests/graph-portfolio-command.test.ts` |
| TC-1312 | An unreadable repository, collection, attachment, or export is named locally without hiding readable repositories or structurally readable sibling collections in the same store | Unit | P0 | FR-067-AC-8 | ✅ `tests/graph-portfolio.test.ts` |
| TC-1313 | Equivalent repository arguments, changed-seed permutations, and store enumeration permutations emit byte-identical canonical JSON; conflicting export, premises, or audit mappings fail before reads; human output renders the same report object | Property | P0 | FR-067-AC-9 | ✅ `tests/graph-portfolio.test.ts` |
| TC-1314 | Historical measurement schemas and non-graph portfolio goldens remain readable without migration, and graph output contains no aggregate score or verdict | Integration | P0 | FR-067-AC-10, FR-067-CON-2, FR-067-CON-4 | ✅ `tests/graph-portfolio.test.ts` |
| TC-1315 | Static boundaries prove portfolio reporting executes and writes nothing and consumes FR-062 reports without recomputing graph semantics | Static | P0 | FR-067-AC-11, FR-067-CON-1, FR-067-CON-3 | ✅ `tests/graph-portfolio.test.ts` |
| TC-1316 | One retained producer fixture passes through the versioned adapter into a portfolio that preserves raw identity, partitions, availability, and compatibility premises while boundary sentinels observe no producer execution and no aggregate verdict | Integration | P0 | StR-007-VC-1 | ✅ `tests/graph-portfolio.test.ts` |
| TC-1317 | `change-assurance seal-record` seals an explicit body, retains it under its digest, and reports digest and path; a body carrying `digest` or an undeclared field is refused with exit 2 and retains nothing | Unit | P0 | FR-068-AC-1 | ✅ `tests/change-assurance-command.test.ts` |
| TC-1318 | `change-assurance seal-attestation` derives only retained-output media type, digest, and size from the named file and flag, and refuses a body supplying `retained_output` or `digest` | Unit | P0 | FR-068-AC-2, FR-068-CON-2 | ✅ `tests/change-assurance-command.test.ts` |
| TC-1319 | `change-assurance intake` retains exact attestation and output bytes as one pair, refuses a contradicting digest or size, is idempotent for identical bytes, and refuses a digest collision with differing bytes | Unit | P0 | FR-068-AC-3 | ✅ `tests/change-assurance-command.test.ts` |
| TC-1320 | `change-assurance receipt` assembles its verification input only from the named record, parents, explicit selections, decisions, and audits; an unselected stored attestation changes no receipt field | Unit | P0 | FR-068-AC-4 | ✅ `tests/change-assurance-command.test.ts` |
| TC-1321 | Unavailable, not-computed, and missing evidence survive the command as their own outcomes and reasons under permutation and are never rendered as pass or failure | Property | P0 | FR-068-AC-5 | ✅ `tests/change-assurance-command.test.ts` |
| TC-1322 | Exit status is 0 for a valid receipt, 1 for invalid and incomplete, and 2 for usage, parse, and integrity errors, with the receipt still emitted for the first two | Unit | P0 | FR-068-AC-6 | ✅ `tests/change-assurance-command.test.ts` |
| TC-1323 | `change-assurance verify-receipt` accepts a sealed receipt and refuses one whose semantic field or digest was altered | Unit | P0 | FR-068-AC-7 | ✅ `tests/change-assurance-command.test.ts` |
| TC-1324 | `change-assurance schema` lists the three normative asset names and emits each byte-identically to the packaged asset; an unknown name is refused with exit 2 | Unit | P0 | FR-068-AC-8 | ✅ `tests/change-assurance-command.test.ts` |
| TC-1325 | `change-assurance recover` removes only interrupted-intake staging directories, reports the count, and leaves retained records, attestations, and outputs untouched | Unit | P0 | FR-068-AC-9 | ✅ `tests/change-assurance-command.test.ts` |
| TC-1326 | Golden fixtures for a sealed record, a sealed attestation, and a valid receipt reproduce byte-identical canonical JSON through the commands | Integration | P0 | FR-068-AC-10, FR-068-CON-4 | ✅ `tests/change-assurance-command.test.ts` |
| TC-1327 | Static boundaries prove no change-assurance command spawns a process or performs Git or network work, and that no output or help text makes an identity, authorization, non-repudiation, or certification claim | Static | P0 | FR-068-AC-11, FR-068-CON-1, FR-068-CON-3 | ✅ `tests/change-assurance-command.test.ts` |
| TC-1328 | A real 99-row contract conformance run transcribes to one entry per replayed fixture, keyed by corpus, operation, and fixture | Unit | P0 | FR-069-AC-1 | ✅ `tests/campaign-adapters.test.ts` |
| TC-1329 | A real tl-mltl differential report transcribes to one entry per compared case | Unit | P0 | FR-069-AC-2 | ✅ `tests/campaign-adapters.test.ts` |
| TC-1330 | An unsupported case is named with its own state and reason rather than transcribed as skip, error, or pass | Unit | P0 | FR-069-AC-3 | ✅ `tests/campaign-adapters.test.ts` |
| TC-1331 | Unknown protocol, unknown schema version, unknown status, missing field, malformed JSON, and empty result set are each refused by line or case | Property | P0 | FR-069-AC-4 | ✅ `tests/campaign-adapters.test.ts` |
| TC-1332 | Both adapters are selectable by name and by declared tool and appear in the adapter listing | Unit | P0 | FR-069-AC-5 | ✅ `tests/campaign-adapters.test.ts` |
| TC-1333 | `quoin evidence record` prints every unrepresented result in human and JSON output | Integration | P0 | FR-069-AC-6, FR-069-CON-2 | ✅ `tests/campaign-adapters.test.ts` |
| TC-1334 | The inventory names a real producer and a verdict for every scope item, and a pinned sample for every added adapter | Static | P0 | FR-069-AC-7, FR-069-CON-4 | ✅ `tests/campaign-adapters.test.ts` |
| TC-1335 | Static boundaries prove neither adapter spawns a process, performs network work, or scrapes console text for a verdict | Static | P0 | FR-069-AC-8, FR-069-CON-1, FR-069-CON-3 | ✅ `tests/campaign-adapters.test.ts` |
| TC-1336 | Every default module manifest loads unchanged with no `semantic` block | Integration | P0 | FR-070-AC-1 | ✅ Complete |
| TC-1337 | A minimal `semantic` block loads and `quoin write` reports contract, semantic-core version, and package | Unit | P0 | FR-070-AC-2 | ✅ Complete |
| TC-1338 | An unknown key inside `semantic` is rejected naming the key | Unit | P0 | FR-070-AC-3 | ✅ Complete |
| TC-1339 | `exports` naming an undeclared object type is rejected naming it | Unit | P0 | FR-070-AC-4 | ✅ Complete |
| TC-1340 | `contract_version: 2.0.0` is rejected before any other semantic key is read | Unit | P0 | FR-070-AC-5 | ✅ Complete |
| TC-1341 | Two modules with one `semantic.package` fail to install together naming both; the later sorted module root is rejected | Unit | P0 | FR-070-AC-6 | ✅ Complete |
| TC-1342 | The manifest schema diff adds no required key to the root or any entry type | Static | P0 | FR-070-CON-1 | ✅ Complete |
| TC-1343 | The vendored module-manifest schema carries filament-core-service provenance (repository, revision, path, sha256) | Static | P0 | FR-070-CON-2 | ✅ Complete |
| TC-1344 | Quoin's FR-006 typed-table fixture has an expected `FieldDecl[]` whose elements validate against the vendored `FieldDecl.json` | Unit | P0 | FR-071-AC-1, US-020-EX-1 | ✅ Complete |
| TC-1345 | The `sysml` fence fixture's expected normalized `FieldDecl[]` is identical to the table fixture's | Unit | P0 | FR-071-AC-2, US-020-EX-2 | ✅ Complete |
| TC-1346 | Table plus fence in one artifact fails at the second form's locus | Unit | P0 | FR-071-AC-3 | ✅ Complete |
| TC-1347 | `UUID`, `Decimal(10,2)`, `Duration [ms]`, `ConfigOverlay` (title), `Status` (id), and an import resolve as specified; `Mystery` yields the advisory and placeholder target | Unit | P0 | FR-071-AC-4 | ✅ Complete |
| TC-1348 | Multiplicity cells `1`, `0..1`, `1..* ordered unique`, `2..5`, empty map as specified; `5..2` and `1 ordered` fail | Unit | P0 | FR-071-AC-5 | ✅ Complete |
| TC-1349 | Constraint cells incl. `pattern: /…/`, `enumValues: a` or `b`, `nonEmpty`, `format: ns:name` map as specified; `mnimum: 1` fails with locus | Unit | P0 | FR-071-AC-6 | ✅ Complete |
| TC-1350 | Fence lines `item x : Y;`, `part def X;`, `:> Y` fail with locus | Unit | P0 | FR-071-AC-7 | ✅ Complete |
| TC-1351 | Fence recognition is line-level with spans; brace content is opaque constraint text | Static | P0 | FR-071-CON-1 | ✅ Complete |
| TC-1352 | Golden fixtures under `tests/fixtures/semantic-module/` carry the semantic-core version and expected diagnostics | Static | P0 | FR-071-CON-2 | ✅ Complete |
| TC-1353 | One `ocl` fence under `### immutable` extracts to the expected `ClauseRef` and verbatim text | Unit | P0 | FR-072-AC-1 | ✅ Complete |
| TC-1354 | No language or `tla` fails at the fence; `sysml`, `fretish`, `acme:tla` yield `semantic.clause-language-unchecked` | Unit | P0 | FR-072-AC-2 | ✅ Complete |
| TC-1355 | Two `### immutable` clauses fail at the second; `### not-archived` fails at the heading | Unit | P0 | FR-072-AC-3 | ✅ Complete |
| TC-1356 | `### archive` with param table, `Returns: ConfigVersion[1]`, `Pre: notArchived`, `Post: archived` yields the expected `OperationDecl` | Unit | P0 | FR-072-AC-4 | ✅ Complete |
| TC-1357 | `Post: missing` fails at that line | Unit | P0 | FR-072-AC-5 | ✅ Complete |
| TC-1358 | A clause declared by a fence and by `Clause: ./clauses.md#immutable` fails at the second occurrence | Unit | P0 | FR-072-AC-6 | ✅ Complete |
| TC-1359 | No clause typechecking or evaluation code path exists in Quoin | Static | P0 | FR-072-CON-1 | ✅ Complete |
| TC-1360 | `data_schema: { schema, digest }` installs; the fixture's declaration set validates against `FieldDecl.json` and its record against `Entity.json` | Unit | P0 | FR-073-AC-1, US-020-EX-3 | ✅ Complete |
| TC-1361 | Digest mismatch, missing, non-JSON, and `$id`-less files each fail naming the path and reason | Unit | P0 | FR-073-AC-2 | ✅ Complete |
| TC-1362 | `$ref` to semantic-core `0.2.0` under `0.1.0`, an unshipped `$ref`, and a `$ref` cycle each fail naming the `$ref` | Unit | P0 | FR-073-AC-3 | ✅ Complete |
| TC-1363 | Inline `data_schema` under a `semantic` block yields `semantic.inline-data-schema`; without one it is silent | Unit | P0 | FR-073-AC-4 | ✅ Complete |
| TC-1364 | `..` and symlink escapes are rejected; `{ schema, digest, type }` is rejected as ambiguous | Unit | P0 | FR-073-AC-5 | ✅ Complete |
| TC-1365 | Schema reference resolution completes with the network disabled | Integration | P0 | FR-073-CON-1 | ✅ Complete |
| TC-1366 | Existing manifests without `semantic` keep the inline `data_schema` form valid with no warning | Integration | P0 | FR-073-CON-2 | ✅ Complete |
| TC-1367 | Quoin's pinned FR-006 copy yields one `semantic.legacy-properties-form` warning (`free-column-table`, locus) and byte-identical `properties` | Unit | P0 | FR-074-AC-1, US-020-EX-4 | ✅ Complete |
| TC-1368 | Bullet-list Properties warns with form `bullet-list`; a mixed section names the first block's form | Unit | P0 | FR-074-AC-2 | ✅ Complete |
| TC-1369 | `legacy_forms: error` with a valid `sweep_report` errors the artifact; without a report, or with a mismatched one, install rejects the manifest naming the report | Unit | P0 | FR-074-AC-3 | ✅ Complete |
| TC-1370 | The authoring pack shows the migration example once | Unit | P1 | FR-074-AC-4 | ✅ Complete |
| TC-1371 | Legacy detection leaves the `properties` extraction unchanged across the existing fixture suite | Integration | P0 | FR-074-CON-1 | ✅ Complete |
| TC-1372 | The derived `semantic/package-manifest.json` validates against `package-manifest.schema.json` with every required field derived as specified | Unit | P0 | FR-075-AC-1 | ✅ Complete |
| TC-1373 | The `registry.json` entry carries one digest per exported object type and changes with the emitted schema | Unit | P0 | FR-075-AC-2 | ✅ Complete |
| TC-1374 | An import no installed module provides fails `quoin module install` naming import and installed versions; an import cycle fails naming the cycle | Unit | P0 | FR-075-AC-3 | ✅ Complete |
| TC-1375 | Derived export `typeIdentity` values equal the identities the dynamic load exposes | Unit | P0 | FR-075-AC-4 | ✅ Complete |
| TC-1376 | `semantic.package` as `ix://agent-ix/x` or a URL is rejected | Unit | P0 | FR-075-AC-5, FR-075-CON-2 | ✅ Complete |
| TC-1377 | Quoin compiles, publishes, or fetches no generated package in this scope | Static | P0 | FR-075-CON-1 | ✅ Complete |
| TC-1378 | Package identities are `<org>/<repo>` and type identities `ix://<org>/<repo>/type/<Name>` in every derived document | Static | P0 | FR-075-CON-2 | ✅ Complete |
| TC-1379 | Every default module manifest loads with no new diagnostic | Integration | P0 | NFR-017-AC-1 | ✅ Complete |
| TC-1380 | The corpus fixture sweep reports only `warning`-severity semantic findings by default | Integration | P0 | NFR-017-AC-2 | ✅ Complete |
| TC-1381 | No corpus repository path appears in the change set | Static | P0 | NFR-017-AC-3 | ✅ Complete |
| TC-1382 | The manifest schema `required` arrays are unchanged | Static | P0 | NFR-017-AC-4 | ✅ Complete |
| TC-1383 | `targets: [go]` and `package: ix://agent-ix/x` are each rejected naming the value | Unit | P0 | FR-070-AC-7 | ✅ Complete |
| TC-1384 | A fixture row with bare `Decimal` yields the semantic-core reader diagnostic at the row locus | Unit | P0 | FR-071-AC-8 | ✅ Complete |
| TC-1385 | The vendored semantic-core bundle's provenance equals the filament-core-data `toolchain.json` digest at the recorded revision | Static | P0 | FR-073-AC-6 | ✅ Complete |
| TC-1386 | `quoin semantic sweep` over the fixture corpus yields a report that validates against the sweep-report schema and counts each legacy form | Unit | P0 | FR-074-AC-5 | ✅ Complete |
| TC-1077 | The supported Tier-1 update emits baseline JSON in the same format the repository gate enforces; updating a ratchet cannot make the next gate fail on style alone (#244) | Unit | P0 | FR-043-AC-17 | ✅ |
| TC-1078 | Multiple standing-adjudication entries for one advisory family are unioned by declaration; a later narrow ruling cannot silently erase an earlier ruling (#252) | Unit | P0 | FR-043-AC-16 | ✅ |
| TC-1079 | `untracked-id-has-minted-children` maps to a distinct located unminted-ID family rather than being folded into spelling near misses (#253) | Unit | P0 | FR-043-AC-22 | ✅ |
| TC-1080 | Quire, Quoin, and external-observation adapters preserve producer attribution and raw output without inventing missing evidence (#255) | Unit | P0 | FR-043-AC-23 | ✅ |
| TC-1081 | The finding-envelope reader refuses missing producers, malformed availability states, and unknown versions (#255) | Unit | P0 | FR-043-AC-23 | ✅ |
| TC-1082 | Actionability v2 scores positive, negative, unavailable, and not-applicable records with explicit counts, misses, and exclusions (#254) | Unit | P0 | FR-043-AC-24 | ✅ |
| TC-1083 | Historical actionability v1 remains reproducible while finding/locality grading consumes normalized envelopes (#254, #255) | Unit | P0 | FR-043-AC-24 | ✅ |
| TC-1084 | Span grounding distinguishes present, missing, unavailable, malformed, and non-specific/not-applicable records while retaining producer version and named misses (#219) | Unit | P0 | FR-043-AC-25 | ✅ |
| TC-1085 | A malformed properties payload is named rather than scored as zero, and `scoreScenario` consumes the same span-grounding scorer (#219) | Unit | P0 | FR-043-AC-25 | ✅ |
| TC-1086 | The computed span-grounding rate is a one-way ratchet that keeps the accepted floor on regression (#219) | Unit | P0 | FR-043-AC-25 | ✅ |
| TC-1092 | Exact controlled loci require matching text and coordinates, a present wrong span earns no correctness, and justified refusal requires both absent spans and the expected structured reason (#256) | Unit | P0 | FR-043-AC-26 | ✅ |
| TC-1093 | Exact-locus and safe-refusal scores ratchet overall and per failure-shape family, with each regressed family naming its misses (#256) | Unit | P0 | FR-043-AC-26 | ✅ |
| TC-1094 | The metric dictionary declares correctness and safe refusal independently of presence and maps each to its own active MeasurementPlan (#256) | Unit | P0 | FR-043-AC-26 | ✅ |
| TC-1095 | The retained controlled producer run covers positive, boundary-error, wrong-subject, and justified-refusal cases and exposes the wrong-span versus safe-refusal tradeoff (#256) | Integration | P0 | FR-043-AC-26 | ✅ |
| TC-1096 | Two families in one mode-language partition receive independent L1/L2/L3 rows and ratchets, so a gain in one cannot hide a regression in the other (#257) | Unit | P0 | FR-043-AC-27 | ✅ |
| TC-1097 | Every current L2/L3 miss is retained by case and family with producer, expected/observed locus, structural root cause, and explicit disposition; the report and measurement identity link to the same inventory (#257) | Integration | P0 | FR-043-AC-27 | ✅ |
| TC-1098 | An exact content-addressed ruling applies only to the compatible normalized finding; an unruled sibling remains outside the TP/FP denominator and named unadjudicated (#258) | Unit | P0 | FR-043-AC-28 | ✅ |
| TC-1099 | The retained 108-row population validates every finding digest and row-level disposition, reviewer, rationale, rubric version, disagreements, defect owner, and follow-up (#258) | Unit | P0 | FR-043-AC-28 | ✅ |
| TC-1100 | Changed finding bytes, incomplete review records, and incompatible metric versions are refused rather than consumed by a report (#258) | Unit | P0 | FR-043-AC-28 | ✅ |
| TC-1101 | The retained Tier-2 record content-addresses raw and normalized output for Quire coverage, Quoin validation, and Quoin evidence audit, including explicit unavailable output (#259) | Unit | P0 | FR-043-AC-29 | ✅ |
| TC-1102 | Repeating a candidate with the same inputs and expected unavailable premise produces no source change or regression, and comparison leaves the baseline object unchanged (#259) | Unit | P0 | FR-043-AC-29 | ✅ |
| TC-1103 | An evaluated source becoming unavailable loses its detected finding and is a named source regression without rewriting the retained baseline (#259) | Unit | P0 | FR-043-AC-29 | ✅ |
| TC-1104 | Tier-2 finding detection consumes the retained `finding-envelope-v2` view: changing only raw producer arrays cannot change a score, while removing the normalized finding does (#255, #259) | Unit | P0 | FR-043-AC-23, FR-043-AC-29 | ✅ |
| TC-1110 | Labeled span v2 independently accepts exact boundaries and an explicit no-domain safe refusal without inventing a missing precondition; all outcome classes remain separately counted (#261) | Unit | P0 | FR-043-AC-30 | ✅ |
| TC-1111 | A changed labeled-population multiplicity is malformed and cannot silently shrink the denominator (#261) | Unit | P0 | FR-043-AC-30 | ✅ |
| TC-1116 | Tier-2 detection requires the exact defect locus and absence on the pinned healthy-control cohort (#261) | Unit | P0 | FR-043-AC-29 | ✅ |
| TC-1117 | Tier-2 scoring keeps unavailable, invalid-answer-key, and healthy-control-failure states distinct from misses and detections (#261) | Unit | P0 | FR-043-AC-29 | ✅ |
| TC-1118 | Every valid Tier-2 answer-key finding binds a full-SHA cohort, a non-empty uniquely-routed declaration set, evidence-sidecar state, production source, reproduction command, locus state, and healthy-control state (#261, #263) | Unit | P0 | FR-043-AC-29, FR-043-AC-31 | ✅ |
| TC-1119 | The committed Tier-2 v2 baseline retains the exact cohort/source states, the five detected historical findings with no miss, and distinct unavailable/invalid states, and compares byte-identically against itself (#261, #263) | Unit | P0 | FR-043-AC-29 | ✅ |
| TC-1120 | `span_grounding_v2_rate` maps to its own active `property.span-grounding-v2` MeasurementPlan while historical span-grounding v1 keeps MP-204 (#261) | Unit | P0 | FR-043-AC-30 | ✅ |
| TC-1121 | Every declaration checkout route must identify the named repository and every pinned historical declaration commit must remain reachable from a remote-tracking ref; deleting that reachability fails validation (#263) | Unit | P0 | FR-043-AC-31 | ✅ |
| TC-1122 | The retained historical pass-2 cohort names both exact process and ISO declaration revisions, records clean current declaration checkouts, and canonicalizes coverage with `IX_FILAMENT_MODULES_PATH` and no `--module` override (#263) | Unit | P0 | FR-043-AC-31 | ✅ |
| TC-926 | Every metric in the dictionary declares `unit`, `population` and `method`; one missing any of the three is rejected at load rather than reported with a gap | Unit | P0 | FR-043-AC-1 | ✅ |
| TC-927 | `finding_precision`/`finding_recall` are declared per defect family, each keyed on a family the corpora label, so no score is reported over an unlabelled population | Unit | P0 | FR-043-AC-2 | ✅ |
| TC-928 | `span_grounding_rate` is declared with the pass-2 figure (0 of 65 specific-shape records) as its starting baseline | Unit | P0 | FR-043-AC-3 | ✅ |
| TC-929 | `actionability_rate` is declared with the pass-2 figure (15 of 496 findings carrying a row id) as its starting baseline | Unit | P0 | FR-043-AC-4 | ✅ |
| TC-930 | `cost_per_confirmed_insight` is declared in tokens AND tool calls per true positive, extending FR-042's eval metrics rather than opening a second accounting | Unit | P1 | FR-043-AC-5 | ✅ |
| TC-931 | The silent-zero sentinel is a gate, not a score: expected exactly 0, no tolerance, and a metric with `matched = 0` over a non-zero population and no diagnostic fails the run | Unit | P0 | FR-043-AC-6 | ✅ |
| TC-932 | A tier-1 entry's `labels.json` carries each seeded defect's family, location and whether it is expected to be found — so a scored miss is distinguishable from a defect nobody claimed was findable | Unit | P0 | FR-043-AC-7 | ✅ |
| TC-933 | A tier-2 entry declares a pinned SHA; a run against a different SHA is refused with a diagnostic naming both, and is never scored against the key | Unit | P0 | FR-043-AC-8 | ✅ |
| TC-934 | The score report carries, per metric, the enveloped value and the baseline compared against, and per corpus its tier and identity; two runs over identical inputs are byte-identical | Unit | P0 | FR-043-AC-9 | ✅ |
| TC-935 | Ratchet: better rewrites the baseline and passes, worse fails naming metric/values/corpus, equal passes and rewrites nothing; regeneration is deliberate and reviewable | Unit | P0 | FR-043-AC-10 | ✅ |
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
| TC-266 | An obligation whose `parameters` carry `target` or `threshold` mints `quantified-threshold` whatever the statement says — the ticket's NFR-022-M-12 record is advised `performance-benchmarking` — and parameters with neither key mint nothing (#166: the one structured signal quire emits was typed, parsed and read nowhere) | Unit | P0 | FR-031-AC-19 | ✅ |
| TC-267 | The compound-token regression corpus (#167): `unsafe-audit` mints no memory-safety and NFR-022-M-12 is not advised memory-safety methods end to end; bare `unsafe`, `memory-safe`, `memory safety`, `use-after-free`, `thread-safety` and `fail-safe` each still mint theirs; `thread-pool` is a fragment and mints nothing | Unit | P0 | FR-031-AC-20 | ✅ |
| TC-268 | Verbatim architectural statements from the battle corpus (filament-ide-rs FR-015/NFR-009) mint `layering` or `module-boundary` — zero-dependencies declaration, dependency-check rejection, acyclicity NFR, architecture-absence claim — making `architecture-conformance` reachable, while `top-level claim` (the assurance-case term) mints neither | Unit | P0 | FR-031-AC-21 | ✅ |
| TC-273 | Three states, not two (#168): an authored value the engine diagnosed uncatalogued is `uncatalogued: true, mismatch: false` even with recommendations present (where a mismatch WOULD have fired); a declared method the advisor disagrees with stays a mismatch; uncatalogued and inconclusive coexist rather than one consuming the other; an absent classification (engine predating CR-091) degrades to the two-state behaviour; an empty authored cell is never uncatalogued whatever the caller claims | Unit | P0 | FR-031-AC-22, FR-031-AC-23 | ✅ |
| TC-274 | The battle-test oracle (#168), end to end through the real command over a faked quire: `"Static Analysis"`, `"Audit Script"`, `"CI Measurement"`, `"Playwright rAF sampling"`, `"Deferred"` — authored on obligations whose statement DOES draw a recommendation — each render `⚠ uncatalogued` and never `mismatch`, in human output and JSON; the authored `Inspection` disagreement stays `⚠ mismatch`; and the CR-091 payload carrying diagnostic `value` and `vocabulary_coverage` validates against the vendored contract | Integration | P0 | FR-031-AC-22, FR-031-AC-24 | ✅ |
| TC-275 | Degraded mode (#168): `uncatalogued-verification-method` diagnostics without `value` yield an empty join marked degraded, the command warns "engine predates vocabulary classification" and reports the two-state behaviour exactly as before — the five battle strings as mismatches, `0 uncatalogued` — while a payload with no such diagnostics at all is NOT degraded, because under either engine vintage it means every authored value is catalogued | Integration | P0 | FR-031-AC-23 | ✅ |
| TC-276 | The filter sub-defect (#168): `--mismatch-only` returns only the genuine disagreement row; `--mismatch-only --inconclusive-only` combined selects the union (2 rows) rather than the guaranteed-empty intersection; and the footer tallies the full population (`2 of 7 obligation(s) shown. Of all 7: 1 mismatch, 5 uncatalogued, 1 inconclusive`) instead of understating the totals it filtered on | Integration | P0 | FR-031-AC-24 | ✅ |
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
| TC-258 | `--ratchet` with no baseline names the missing `baseline.json` and the command that writes it, and does NOT print `(new violations only)` — a day-one reader was told their entire backlog was new violations (#169) | Unit | P0 | FR-032-AC-13 | ✅ |
| TC-259 | `--ratchet` with a baseline present ratchets, says so, and prints no missing-baseline notice | Unit | P0 | FR-032-AC-13 | ✅ |
| TC-260 | The JSON `ratchet` field reflects whether ratcheting was APPLIED — `false` with no baseline, `true` with one, and the accepted finding is actually absent from the ratcheted output | Unit | P0 | FR-032-AC-13 | ✅ |
| TC-264 | `unknown-method` fires on an obligation with no bindings at all — it is a statement-vs-catalog comparison and is asked before the binding guard, so an unbound obligation with an uncatalogued method yields BOTH `undischarged` and `unknown-method`, a catalogued method yields only `undischarged`, and no catalog means the question is not asked (#165: 1,107 findings all undischarged, 90+ uncatalogued Verification values, zero unknown-method) | Unit | P0 | FR-032-AC-14 | ✅ |
| TC-265 | `quoin evidence audit` against a repository with no `spec/evidence/` store reports its uncatalogued methods, not only that nothing is bound — the one check that pays off on day one of adoption no longer requires having already adopted | Unit | P0 | FR-032-AC-14 | ✅ |
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
| US-013   | ✅ Covered | TC-1125..TC-1154 inspect the architecture record, decision ledger, and non-disruption scope; TC-1155 records named active maintainer `kreneskyp`'s review and admin merge of PR #311 as merge commit `4a82644ad3cf75770cc53ef3812e3b13e80b516d`. |
| US-014   | ✅ Covered | TC-1156..TC-1193 cover the complete read-only census, evidence-backed type-fit review, canonical outputs, reconciliation, and non-disruption; TC-1194 records the explicitly authorized admin merge of PR #316 while preserving every downstream major-interference gate. |
| US-015   | ✅ Covered | `tests/intervention.test.ts` — TC-1195..TC-1216 and TC-1217..TC-1221 cover record fidelity, negative outcomes, uncertainty, raw-evidence integrity, claim-centered reporting, the real producer, governance, and non-scoring. |
| US-016   | ✅ Covered | `tests/operational.test.ts` — TC-1223..TC-1243 and TC-1244..TC-1248 cover both operational shapes, the full control vocabulary, verified clocked discharge, raw-evidence integrity, adverse outcomes, claim-centered rendering, non-scoring, and the real GitHub Actions release producer. |
| US-018   | ✅ Covered | TC-1249..TC-1260 cover exact fan-out, dependency closure, unchanged auditor verdicts, reaffirmation churn, unavailable inputs, deterministic rendering, and read-only/non-scoring boundaries in `graph-analysis.test.ts` and `graph-command.test.ts`. |
| US-017   | ✅ Covered | TC-1261..TC-1292 cover revisioned definitions, exact integrity, workflow decisions, candidate-bound attestations, retained output, unchanged evidence-audit findings, valid/invalid/incomplete receipts, and non-identity boundaries in `tests/change-assurance.test.ts`. |
| US-019   | ✅ Covered | `tests/graph-adapters.test.ts`, `tests/graph-portfolio.test.ts`, and `tests/graph-portfolio-command.test.ts` TC-1293..TC-1315 cover lossless producer intake, plan/definition/population governance, raw evidence, graph history and partitions, availability, compatible comparisons, exact FR-062 portfolio views, and non-scoring. |
| US-020   | ✅ Covered | TC-1344, TC-1345, TC-1360, TC-1367 realise EX-1..EX-4 (typed table, `sysml` fence, schema by digest, legacy warning). |

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
