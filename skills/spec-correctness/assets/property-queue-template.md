# Property Test Queue

<!-- Written by the spec-correctness skill. Not a spec artifact; not validated by quire. -->

spec-correctness — `<repo>` — `<YYYY-MM-DD>`
quire-cli `<version>` · harness `<name>` · `<N>` criteria

## Run report

```
emitted unattended   <n>   (extractable, grounded)
queued               <n>   candidate <n> · second-pass <n> · downgraded <n> · dep-missing <n>
refused              <n>   symbol-not-found <n> · oracle-is-adjectival <n> · unimplemented <n>
already covered      <n>   hand-written tests already carry the row_id
witnesses            <n>   Unit tests, not property coverage
```

`emitted unattended + queued + refused + already covered` = records carrying a `row_id`.

## Queued — awaiting review

Each row is inert in the runner until accepted. Accepting is: delete the skip marker, move
the file out of `_review/`, change `review=required` to `review=accepted`, run the suite,
flip the matrix row from 🚧 to ✅. The tracking tag never changes.

| row_id | property | origin | confidence | file | why it is queued |
| --- | --- | --- | --- | --- | --- |
| FR-018-AC-3 | invariant | regex-candidate | — | `tests/props/_review/fr-018.prop.test.ts` | metamorphic label not corroborated by the structural pass |
| FR-005-AC-2 | example | llm-second-pass | medium | `tests/props/_review/fr-005.prop.test.ts` | reclassified as error-case over a negative domain |
| FR-050-AC-1 | concurrency | llm-second-pass | low | `tests/props/_review/test_fr_050.py` | hypothesis has no interleaving scheduler; interleavings not exhaustive |

## Witnesses — Unit tests, not property coverage

| row_id | file | the case it pins |
| --- | --- | --- |
| FR-002-AC-1 | `tests/props/_review/fr-002.prop.test.ts` | `version`, `--version`, `-v` each print the package version |

## Refused — no test written

Not findings. A refusal says this run could not ground the criterion; several of these are
criteria that are correctly verified by inspection rather than by test.

| row_id | reason | note |
| --- | --- | --- |
| NFR-003-AC-2 | oracle-is-adjectival | "actionable" has no observable in `## Outputs`; verified by inspection |
| FR-022-AC-4 | unimplemented | `selfUpdate` is not in the source yet |
| FR-011-AC-6 | ambiguous-symbol | `list` matches two exported functions; no `Verification` column to disambiguate |

## Rejected — do not re-propose

A re-run skips these `row_id`s unless the criterion's `statement` changes.

| row_id | rejected on | reason |
| --- | --- | --- |

## Dependency remedies

No test file is written when the generator library is absent — an import of a missing
library breaks test collection even for a skipped test. The proposals sit below as code
blocks until the dependency is added and the skill is re-run.

| harness | remedy |
| --- | --- |
| fast-check | `add fast-check to devDependencies, then re-run spec-correctness to emit these` |

### Proposals awaiting the dependency

<details><summary>FR-012-AC-1 · ordering · extractable</summary>

```ts
// tests/props/fr-012.prop.test.ts
/**
 * Trace: FR-012-AC-1 — declaring modules are listed in sorted order.
 * spec-correctness: row=FR-012-AC-1 property=ordering extraction=extractable origin=regex review=none
 */
```

</details>
