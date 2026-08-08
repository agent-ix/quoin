# Property Test Queue

<!-- Written by the spec-correctness skill (skills/spec-correctness/). Not a spec
     artifact; not validated by quire. -->

spec-correctness — `quoin` — 2026-08-08
quire-cli 0.12.0 · harness TypeScript/vitest + fast-check 4.9.0 · 130 criteria

## Run report

```
emitted unattended   17   new property tests, tests/props/
second pass          11   reclassified, reviewed, accepted, tests/props/second-pass.prop.test.ts
already covered      81   existing tests, now carrying row_id tracking tags
verified elsewhere   21   eval, inspection, or delegated — no code tag by design
```

`17 + 11 + 81 + 21 = 130` — every record carrying a `row_id`.

**109 of 130 criteria are reconcilable by tracking tag, from 0 when this started.**
The other 21 are each verified by a method that produces no test to tag, and are
listed under "Verified elsewhere".

Counts were measured with a right-boundary match. A naive substring check reports
109 as 110, because `FR-025-AC-1` is a prefix of `FR-025-AC-10`.

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
  not the engine's. All 31 singletons resolved to existing hand-written tests, which now
  carry tracking tags.
- **No existing test carried a `row_id` tag when this started.** quoin's `spec/matrix.md`
  maps requirements to tests as `` `file.test.ts` :: "test name" `` prose, so nothing was
  reconcilable by grep even where a hand-written test existed. Closed: 115 `// Trace:`
  comments now span 15 test files.
- **The second pass ran.** 70 records, of which 11 reclassified into real properties and the
  rest were genuine witnesses already covered.

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

## Second pass — the 11 reclassifications

`tests/props/second-pass.prop.test.ts`. Each was classified `not-extractable` —
the deterministic pass read it as a concrete example — and grounding found a real
domain behind it. Emitted inert under `_review/`, then **accepted** through the
documented procedure.

| row_id                   | reclassified as | grounded on                            |
| ------------------------ | --------------- | -------------------------------------- |
| FR-007-AC-2              | universal       | `src/catalog.ts:46` `.filter(Boolean)` |
| FR-013-AC-2              | error-case      | `src/write.ts:60` non-directory throw  |
| FR-013-AC-3              | universal       | `src/write.ts:42` `parseTypeList`      |
| FR-018-AC-1..AC-4        | universal       | `src/plugins.ts:34` `parseSourceArg`   |
| FR-025-AC-2, AC-3, AC-11 | universal       | `src/org.ts:190` `originOrg`           |
| FR-027-AC-6              | error-case      | `src/config-schema.ts:15` `.strict()`  |

**FR-028-AC-7 was proved mechanically during acceptance.** The `Trace:` and `row=`
tags were extracted before and after — lifting the skip, rewriting `review=required`
to `review=accepted`, and moving the file out of `_review/` — and diffed. Byte-identical.

## Already covered — 81 criteria

Every remaining witness resolved to an existing hand-written test rather than a
generated one. That is the cheaper and more honest outcome: the coverage was
already there, what it lacked was a tracking tag. 115 `// Trace:` comments were
added across 15 test files, taking reconcilable criteria from **0 to 109**.

`spec/matrix.md`'s prose mapping (`` `file.test.ts` :: "test name" ``) supplied 33
of these at AC granularity; the rest were mapped by hand against the suite.

**Checked, and not a finding:** 11 test names the matrix cites appeared absent on a
literal search. All 11 are `test.each` / `it.each` parameterized cases whose source
holds the template and whose matrix entry holds the expanded name. The matrix does
not over-claim.

## Verified elsewhere — 21 criteria, no code tag by design

| row_id                      | method                                                                                                                     |
| --------------------------- | -------------------------------------------------------------------------------------------------------------------------- |
| FR-003-AC-1, FR-003-AC-3    | Delegated — help rendering is `@oclif/core`'s, and `spec/matrix.md` records FR-003 as ⚠️ Delegated for exactly this reason |
| FR-028-AC-1…AC-12           | Eval (EV-050…EV-053) and inspection — this skill's own agent behavior                                                      |
| NFR-007-AC-1                | An accepted limitation, recorded rather than tested                                                                        |
| StR-001, 003, 005, 006-VC-1 | Demonstration — end-to-end narratives, the eval layer                                                                      |
| StR-002-VC-1, StR-004-VC-1  | Demonstration — end-to-end authoring and workflow narratives                                                               |

None of these is a gap, and none is a reason to reword a criterion.

## Rejected — do not re-propose

| row_id | rejected on | reason |
| ------ | ----------- | ------ |

## Next action

Nothing outstanding. Re-running the skill is idempotent: every emitted test carries
a provenance line, so an unchanged criterion leaves its file alone.
