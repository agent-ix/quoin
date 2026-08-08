# Property Test Queue

<!-- Written by the spec-correctness skill (skills/spec-correctness/). Not a spec
     artifact; not validated by quire. -->

spec-correctness — `quoin` — 2026-08-08
quire-cli 0.12.0 · harness TypeScript/vitest · **fast-check not installed** · 130 criteria

## Run report

```
emitted unattended    0   fast-check is absent — see "Dependency remedies"
queued               48   proposals 17 · witnesses 31
refused              12   static-or-demonstration 12
second pass not run  70   62 example · 5 unclassified · 3 quantified/not-extractable
```

`0 + 48 + 12 + 70 = 130` — every record carrying a `row_id`.

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
  tests as `` `file.test.ts` :: "test name" `` prose, so nothing here counted as _already
  covered_ by tag match even where a hand-written test exists. Reconciling that prose form
  against tracking tags is a separate piece of work.
- **The second pass was not run.** 70 records is well past the ~30 threshold at which
  SKILL.md says to ask first.

## Dependency remedies

No test file is written when the generator library is absent — an import of a missing
library breaks vitest collection even for a skipped test. The proposals below are code
blocks until the dependency is added.

| harness    | remedy                                                                           |
| ---------- | -------------------------------------------------------------------------------- |
| fast-check | `make add-dev-packages p=fast-check`, then re-run spec-correctness to emit these |

## Proposals — 17 grounded properties

Each is `extraction: extractable`, grounded to a `file:line`, and would be emitted
unattended once fast-check is present.

<details><summary>FR-005-AC-1/2/3 · error-case · unknown commands and subcommands</summary>

Grounded: `spec/functional/FR-005-reject-unknown-commands.md` `## Behavior`; the runner
rejection path in `src/cli.ts`. Negative domain = any bareword outside the known set.

```ts
/**
 * Trace: FR-005-AC-1 — an unknown command raises an error that includes usage.
 * spec-correctness: row=FR-005-AC-1 property=error-case extraction=extractable origin=regex review=none
 */
describe("FR-005-AC-1 unknown commands are rejected", () => {
  it("rejects any bareword outside the known command set", async () => {
    await fc.assert(
      fc.asyncProperty(fc.stringMatching(/^[a-z]{1,12}$/), async (cmd) => {
        fc.pre(!KNOWN_COMMANDS.has(cmd));
        await expect(main([cmd])).rejects.toThrow(/Usage:/);
      }),
    );
  });
});
```

AC-2 and AC-3 are the same shape over `["catalog", sub]` and `["plugin", sub]`.

</details>

<details><summary>FR-008-AC-1/AC-2 · universal · module deduplication</summary>

Grounded: `src/catalog.ts:59` `loadCatalog`; `## Behavior` states roots dedupe by resolved
path and modules dedupe by declared name, first-wins.

```ts
/**
 * Trace: FR-008-AC-1 — a repeated module root is loaded only once.
 * spec-correctness: row=FR-008-AC-1 property=universal extraction=extractable origin=regex review=none
 */
it("FR-008-AC-1 loads each root once however often it repeats", () => {
  fc.assert(
    fc.property(
      fc.array(fc.constantFrom(...fixtureRoots), { maxLength: 16 }),
      (roots) => {
        const names = loadCatalog(roots).modules.map((m) => m.name);
        expect(names).toEqual([...new Set(names)]);
      },
    ),
  );
});
```

AC-2 asserts the same over roots whose manifests declare colliding names, and additionally
that the second module contributes no entries.

</details>

<details><summary>FR-009-AC-5 · universal · skeleton filename casing</summary>

Grounded: `src/catalog.ts` `skeletonPath` — matches against `readdirSync` entries rather
than probing with `existsSync`, precisely so a case-insensitive filesystem cannot resolve a
casing that is not on disk. Domain = arbitrary casings of a type name.

```ts
/**
 * Trace: FR-009-AC-5 — only the type's own casing and its lowercase form resolve.
 * spec-correctness: row=FR-009-AC-5 property=universal extraction=extractable origin=regex review=none
 */
it("FR-009-AC-5 resolves exactly the two accepted casings", () => {
  fc.assert(
    fc.property(casingsOf("FR"), (filename) => {
      const root = moduleRootWithSkeleton(filename);
      const resolved = loadCatalog([root]).entries.find(
        (e) => e.name === "FR",
      )?.skeletonPath;
      expect(Boolean(resolved)).toBe(
        filename === "FR.md" || filename === "fr.md",
      );
    }),
  );
});
```

</details>

<details><summary>FR-010-AC-2 · universal · case-insensitive type lookup</summary>

Grounded: `src/catalog.ts:118` `findCatalogEntry` → `normalizeTypeName` = `toLowerCase()`.
Domain = every casing permutation of a known type name. This is the cleanest property in the
repo: the oracle is total.

```ts
/**
 * Trace: FR-010-AC-2 — any casing of a known type name resolves the same entry.
 * spec-correctness: row=FR-010-AC-2 property=universal extraction=extractable origin=regex review=none
 */
it("FR-010-AC-2 resolves a known type under any casing", () => {
  fc.assert(
    fc.property(
      fc.constantFrom(...catalog.entries.map((e) => e.name)),
      anyCasing,
      (name, recase) => {
        expect(findCatalogEntry(catalog, recase(name))?.name).toBe(name);
      },
    ),
  );
});
```

</details>

<details><summary>FR-012-AC-1 · ordering · duplicate modules are sorted</summary>

Grounded: `src/catalog.ts:115` `findDuplicates(entries)`; `## Behavior` — "listing the
declaring modules in sorted order". Verb _sorted_ selects the `is_sorted_by` sub-assertion.

```ts
/**
 * Trace: FR-012-AC-1 — a type declared by two modules is reported with the modules sorted.
 * spec-correctness: row=FR-012-AC-1 property=ordering extraction=extractable origin=regex review=none
 */
it("FR-012-AC-1 lists declaring modules in sorted order", () => {
  fc.assert(
    fc.property(fc.array(entryArb, { maxLength: 32 }), (entries) => {
      for (const dup of findDuplicates(entries)) {
        expect(dup.modules).toEqual([...dup.modules].sort());
      }
    }),
  );
});
```

FR-012-AC-2 is the complement: an entry list with distinct names yields an empty duplicate
set. Together they partition the domain — the sibling-AC check from step 2.

</details>

<details><summary>FR-013-AC-4 · ordering · available types listed sorted on error</summary>

Grounded: `src/write.ts:50` `createAuthoringPack` throw path; `## Outputs` names the sorted
available-type list as the observable.

```ts
/**
 * Trace: FR-013-AC-4 — an unknown type raises an error listing available types in sorted order.
 * spec-correctness: row=FR-013-AC-4 property=ordering extraction=extractable origin=regex review=none
 */
it("FR-013-AC-4 sorts the available types it reports", () => {
  fc.assert(
    fc.property(fc.stringMatching(/^[a-z]{3,10}$/), (name) => {
      fc.pre(!findCatalogEntry(catalog, name));
      const listed = availableTypesFrom(() =>
        createAuthoringPack(catalog, repo, [name]),
      );
      expect(listed).toEqual([...listed].sort());
    }),
  );
});
```

</details>

<details><summary>FR-015-AC-2/AC-3 · universal · shell quoting of the repo path</summary>

Grounded: `src/write.ts:133` `shellQuote` — **not exported**, so the observable is
`createAuthoringPack(...).validation.command` (`src/write.ts:85`). Predicate from the
source: `/^[A-Za-z0-9_./:-]+$/` passes through unquoted; anything else is single-quoted with
`'` doubled as `'\''`.

```ts
/**
 * Trace: FR-015-AC-2 — a clean repo path is rendered without surrounding quotes.
 * Trace: FR-015-AC-3 — a repo path containing a space is single-quoted.
 * spec-correctness: row=FR-015-AC-2 property=universal extraction=extractable origin=regex review=none
 * spec-correctness: row=FR-015-AC-3 property=universal extraction=extractable origin=regex review=none
 */
it("FR-015-AC-2/AC-3 quotes exactly the paths that need it", () => {
  fc.assert(
    fc.property(repoPathArb, (dir) => {
      const { command } = createAuthoringPack(catalog, dir, ["FR"]);
      const rendered = command.split("--scope ")[1].split(' "spec')[0];
      expect(rendered.startsWith("'")).toBe(
        !/^[A-Za-z0-9_./:-]+$/.test(resolve(dir)),
      );
    }),
  );
});
```

Note the two `Trace:` lines: one test covers two criteria, so it carries both `row_id`s and
`gap-analysis` reconciles both.

</details>

<details><summary>FR-018-AC-5 · universal · a bare argument maps to a path source</summary>

Grounded: `src/plugins.ts:34` `parseSourceArg` — the final `return { type: "path", ... }`.
Negative-space property: any argument carrying none of the known prefixes is a path source
whose `path` is the argument verbatim.

```ts
/**
 * Trace: FR-018-AC-5 — a bare argument maps to a path source.
 * spec-correctness: row=FR-018-AC-5 property=universal extraction=extractable origin=regex review=none
 */
it("FR-018-AC-5 treats any unprefixed argument as a verbatim path", () => {
  fc.assert(
    fc.property(fc.string({ minLength: 1, maxLength: 40 }), (arg) => {
      fc.pre(!/^(path:|github:|package:)/.test(arg));
      expect(parseSourceArg(arg)).toEqual({ type: "path", path: arg });
    }),
  );
});
```

</details>

<details><summary>FR-025-AC-9/AC-10 · universal · remote urls that name no org, and nested namespaces</summary>

Grounded: `src/org.ts:190` `originOrg` → `orgFromRemoteUrl`, whose doc comment enumerates
exactly the shapes that must yield `undefined` — local paths, `file://`, and host-based urls
with a single path segment. AC-10's oracle is "the segment immediately preceding the
repository", which the source implements as the second-to-last path segment.

```ts
/**
 * Trace: FR-025-AC-9 — a local-path remote and an owner-less host remote each yield no org.
 * spec-correctness: row=FR-025-AC-9 property=universal extraction=extractable origin=regex review=none
 */
it("FR-025-AC-9 never guesses an org from a remote that has none", () => {
  fc.assert(
    fc.property(orglessRemoteArb, (url) => {
      expect(originOrg(gitConfigWithOrigin(url))).toBeUndefined();
    }),
  );
});

/**
 * Trace: FR-025-AC-10 — a nested namespace qualifies by the segment before the repository.
 * spec-correctness: row=FR-025-AC-10 property=universal extraction=extractable origin=regex review=none
 */
it("FR-025-AC-10 takes the segment immediately preceding the repository", () => {
  fc.assert(
    fc.property(
      fc.array(segmentArb, { minLength: 2, maxLength: 5 }),
      (segments) => {
        const url = `https://git.example.com/${segments.join("/")}.git`;
        expect(originOrg(gitConfigWithOrigin(url))).toBe(segments.at(-2));
      },
    ),
  );
});
```

`orglessRemoteArb` is the union of the four families the doc comment names. This is the case
where the source, not the spec, supplied the oracle — exactly what step 2's read-5 is for.

</details>

<details><summary>FR-002-AC-4 · universal · baked version versus fallback</summary>

Grounded: `src/version.ts:23` `resolveVersion(baked)` — a two-branch total function, so the
property is total too.

```ts
/**
 * Trace: FR-002-AC-4 — a non-empty baked version is reported verbatim; an empty one falls back.
 * spec-correctness: row=FR-002-AC-4 property=universal extraction=extractable origin=regex review=none
 */
it("FR-002-AC-4 prefers any non-empty baked version verbatim", () => {
  fc.assert(
    fc.property(fc.string(), (baked) => {
      expect(resolveVersion(baked)).toBe(
        baked === "" ? packageJsonVersion : baked,
      );
    }),
  );
});
```

</details>

<details><summary>FR-006-AC-4 · universal · unusable candidates are skipped</summary>

Grounded: `src/catalog.ts` `locateModuleRoot` — three early returns (missing path, non-
directory, no manifest at either depth). Domain = the union of those three shapes.

```ts
/**
 * Trace: FR-006-AC-4 — a missing path, an empty directory, and a file candidate are skipped.
 * spec-correctness: row=FR-006-AC-4 property=universal extraction=extractable origin=regex review=none
 */
it("FR-006-AC-4 skips every unusable candidate without throwing", () => {
  fc.assert(
    fc.property(
      fc.array(unusableCandidateArb, { maxLength: 12 }),
      (candidates) => {
        expect(loadCatalog(candidates).modules).toEqual([]);
      },
    ),
  );
});
```

</details>

## Witnesses — 31 `Unit` tests, not property coverage

Queued with reason `singleton-domain`: the determiner quantifies over one element. These are
correct criteria; they are examples, and an example gets an example-based test.

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

`make add-dev-packages p=fast-check`, then re-run spec-correctness to emit the 17 proposals
above as real test files.
