# Property Test Queue

<!-- Written by the spec-correctness skill (skills/spec-correctness/). Not a spec
     artifact; not validated by quire. -->

spec-correctness — `quoin` — 2026-08-08
quire-cli 0.12.0 · harness TypeScript/vitest + fast-check 4.9.0 · 130 criteria

## Run report

```
emitted unattended   17   grounded, passing, in tests/props/
queued               31   witnesses (singleton-domain)
refused              12   static-or-demonstration 12
second pass not run  70   62 example · 5 unclassified · 3 quantified/not-extractable
```

`17 + 31 + 12 + 70 = 130` — every record carrying a `row_id`.

### Census

```
extraction    extractable 60 · candidate 0 · not-extractable 70
property      example 62 · universal 52 · error-case 9 · unclassified 5 · ordering 2
archetype     FR 123 · StR 6 · NFR 1
spans present domain 21 · precondition 8 · oracle 21
```

Counts only. No threshold, no verdict, no rewording suggestion — the classification carries
no severity and no promotion path by design (quire-rs FR-052-CON-1).

### Notes from this run

- **`universal` over-counts.** 52 criteria carry `property: universal`, but only 17 of them
  quantify over a domain with more than one element. English writes a universal and a single
  witness with the same determiner — _"A repeated module root is loaded only once"_ versus
  _"A bareword after `write` is parsed as a positional"_. Separating them is step 2's job,
  not the engine's; the 31 singletons are queued as `Unit` witnesses.
- **No existing test carries a `row_id` tag.** quoin's `spec/matrix.md` maps requirements to
  tests as `` `file.test.ts` :: "test name" `` prose, so nothing counted as _already
  covered_ by tag match even where a hand-written test exists. Reconciling that prose form
  against tracking tags is a separate piece of work.
- **The second pass was not run.** 70 records is well past the ~30 threshold at which
  SKILL.md says to ask first.

### Two grounding bugs the emit caught

Both were defects in the generated tests, not in `src/` — which is what step 4 says to
expect from a failing unattended test.

1. Four catalog properties failed on the first run because the fixtures wrote
   `artifacts:` / `objects:` where `loadCatalog` reads **`artifact_types:` / `object_types:`**.
   The oracle was right; the domain was not being built.
2. `FR-005-AC-1`'s second assertion used a `\b`-anchored regex on the generated command name.
   fast-check shrank it to `"a-"` — `\b` does not match after `-`. Replaced with containment.
   The behaviour was correct throughout; the assertion was not.

### Deviations from harness defaults

| test                             | change        | why                                                                                |
| -------------------------------- | ------------- | ---------------------------------------------------------------------------------- |
| `fr-005.prop.test.ts`            | `numRuns: 30` | each case dispatches through the real oclif runner; the negative domain is uniform |
| `fr-write.prop.test.ts` (FR-015) | `numRuns: 40` | each case creates a directory on disk                                              |

## Emitted — 17 criteria, unattended

All passing under `make test`; the repo's 100% coverage gate still passes.

| file                                  | criteria                                                                                  |
| ------------------------------------- | ----------------------------------------------------------------------------------------- |
| `tests/props/fr-002.prop.test.ts`     | FR-002-AC-4                                                                               |
| `tests/props/fr-005.prop.test.ts`     | FR-005-AC-1, AC-2, AC-3                                                                   |
| `tests/props/fr-catalog.prop.test.ts` | FR-006-AC-4, FR-008-AC-1, FR-008-AC-2, FR-009-AC-5, FR-010-AC-2, FR-012-AC-1, FR-012-AC-2 |
| `tests/props/fr-write.prop.test.ts`   | FR-013-AC-4, FR-015-AC-2, FR-015-AC-3                                                     |
| `tests/props/fr-018.prop.test.ts`     | FR-018-AC-5                                                                               |
| `tests/props/fr-025.prop.test.ts`     | FR-025-AC-9, FR-025-AC-10                                                                 |

Fixtures are built once per file and the generators range over which of them a case uses.
A generator that wrote a module tree per case would make the suite filesystem-bound for no
extra coverage.

## Witnesses — 31 `Unit` tests, not property coverage

Queued with reason `singleton-domain`: the determiner quantifies over one element. These are
correct criteria; they are examples, and an example gets an example-based test. Most already
have a hand-written test in `tests/`; what they lack is a `row_id` tracking tag.

| row_id      | the single case it pins                                     |
| ----------- | ----------------------------------------------------------- |
| FR-001-AC-2 | a bareword after `write` is a positional                    |
| FR-001-AC-3 | a valueless long flag records boolean `true`                |
| FR-002-AC-3 | a missing/non-string version field raises                   |
| FR-003-AC-1 | no command prints root usage                                |
| FR-006-AC-1 | a directory containing `manifest.yaml` is a module root     |
| FR-006-AC-2 | a manifest one level deep is discovered                     |
| FR-006-AC-3 | a non-manifest child is skipped, its manifest sibling found |
| FR-009-AC-1 | a manifest without `name` falls back to the basename        |
| FR-009-AC-4 | the lowercase-filename skeleton fallback                    |
| FR-011-AC-2 | an omitted subcommand behaves as `list`                     |
| FR-013-AC-1 | an empty type list raises                                   |
| FR-014-AC-1 | a type renders name, kind, module, root                     |
| FR-014-AC-3 | a type with neither skeleton nor schema is "manifest only"  |
| FR-019-AC-2 | a missing or empty module name raises                       |
| FR-020-AC-2 | an unknown workflow name raises                             |
| FR-020-AC-3 | the error names the workflow and `IX_SPEC_WORKFLOWS_ROOT`   |
| FR-021-AC-1 | the spawned `ix-flow run` argv                              |
| FR-021-AC-2 | a non-zero child exit propagates                            |
| FR-021-AC-3 | a signal-terminated child yields exit 1                     |
| FR-021-AC-4 | a spawn failure surfaces an error                           |
| FR-024-AC-2 | install → list → loadCatalog round the same plugin          |
| FR-024-AC-3 | library and CLI target the same store                       |
| FR-025-AC-4 | three unresolved paths each resolve to unresolved           |
| FR-025-AC-5 | unresolved is reported with the `--org` remedy              |
| FR-025-AC-8 | a worktree `.git` file resolves as its main checkout does   |
| FR-026-AC-4 | unknown command and subcommand are rejected by the runner   |
| FR-026-AC-6 | a runner error propagates to the caller                     |
| FR-026-AC-7 | an `oclif.plugins` package dispatches with no install step  |
| FR-027-AC-1 | a stored org outranks the `origin` remote                   |
| FR-027-AC-5 | a malformed stored config does not fail the command         |
| FR-027-AC-9 | a project-local config overrides the user-level one         |

FR-024-AC-2 is worth a second look on review: _install → list → load_ is a round-trip in
shape, and over a generated plugin-name domain it would be a genuine `round-trip` property
rather than a witness.

## Refused — no test written

Not findings. Each is a correct criterion verified by a method other than a generated test.

| row_id       | reason                  | note                                                                                                        |
| ------------ | ----------------------- | ----------------------------------------------------------------------------------------------------------- |
| FR-026-AC-3  | static-or-demonstration | "No hand-rolled dispatcher remains in `src/cli.ts`" is a fact about the source tree — matrix `Type: Static` |
| StR-002-VC-1 | static-or-demonstration | an end-to-end authoring narrative — verified by eval, not by a property                                     |
| StR-004-VC-1 | static-or-demonstration | an end-to-end workflow narrative — verified by eval                                                         |
| FR-028-AC-1  | static-or-demonstration | describes this skill's own agent behavior — EV-050                                                          |
| FR-028-AC-2  | static-or-demonstration | EV-051                                                                                                      |
| FR-028-AC-3  | static-or-demonstration | EV-050                                                                                                      |
| FR-028-AC-4  | static-or-demonstration | EV-050                                                                                                      |
| FR-028-AC-5  | static-or-demonstration | EV-052                                                                                                      |
| FR-028-AC-6  | static-or-demonstration | EV-051                                                                                                      |
| FR-028-AC-8  | static-or-demonstration | EV-053                                                                                                      |
| FR-028-AC-10 | static-or-demonstration | inspection of `skills/spec-correctness/`                                                                    |
| FR-028-AC-11 | static-or-demonstration | EV-053                                                                                                      |

## Rejected — do not re-propose

| row_id | rejected on | reason |
| ------ | ----------- | ------ |

## Next action

Run the second pass over the 70 `example` / `unclassified` records, or hand the 31 witnesses
to `spec-matrix` as `Unit` rows so they gain tracking tags.
