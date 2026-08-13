# Step 2: Clause Grounding

**Goal**: turn each criterion's prose into three executable pieces — a **generator domain**,
a **precondition**, and an **oracle** — each cited to a real location.

This is the step the engine cannot do. It reads one statement at a time; you can read the
whole FR and the code. Spans, when present, are a starting hint; ~96% of criteria have none.

## The grounding record

```
{
  row_id:            "FR-012-AC-1",
  symbol:            "detectDuplicates",
  module_path:       "src/catalog.ts",
  domain_expr:       "fc.array(entryArb, {maxLength: 32})",
  precondition_expr: null,
  oracle_expr:       "d.modules === [...d.modules].sort()",
  evidence: [
    "spec/functional/FR-012-detect-duplicate-types.md:31",   // Inputs
    "spec/functional/FR-012-detect-duplicate-types.md:44",   // Behavior SHALL
    "src/catalog.ts:88"                                       // signature
  ]
}
```

**Rule: every one of domain / precondition / oracle that the strategy needs must have at
least one `file:line` citation. No citation → no test.** A precondition of `null` is fine
when the criterion is unconditional; a fabricated one is not.

## Read in this order

1. **The criterion's own row.** Open `record.document` at `record.line`. Read the whole
   table row, including the `Verification` column — it often names the test file.

2. **The FR's structured sections.**
   - `## Inputs` → **domain**. The bullets name the parameters and their carrier types.
     "the assembled catalog and a `catalog validate` invocation with an optional `--json`
     flag" → a catalog arbitrary × an optional boolean.
   - `## Behavior` → **precondition** and **oracle**. Find the `SHALL` clause whose subject
     matches the criterion's subject. Its `when` / `where` / `if` tail is the precondition;
     the `SHALL` predicate is the oracle.
   - `## Outputs` → the **observable** the oracle must be written in (return value, exit
     status, stderr payload). Prefer the observable form. An oracle you cannot observe is
     not an oracle.
   - `## Constraints` / `## Options` → generator bounds. A stated min/max becomes a range
     and a boundary-biased generator; an option list becomes a permutation axis.

3. **Sibling ACs of the same FR.** They partition the domain — AC-1 covers the duplicate
   case, AC-2 the unique case. Use them to (a) avoid a generator that only ever hits one
   branch, and (b) find the paired negative domain when the criterion is an `error-case`.

4. **The test file named in `Verification`.** An existing example test hands you the real
   symbol, its import path, the constructed argument shapes, and the assertion vocabulary
   already in use. Reuse its fixtures rather than inventing parallel ones.

5. **The code.** Grep for the command, flag, type, or function named in the statement.
   - signature → the generator's concrete types, replacing the prose domain;
   - error enum / exception class → the `error-case` oracle. This is the **only** source
     for it: the engine emits `span:refused-weak-boundary` on these, meaning it declined to
     guess;
   - return type / struct fields → the `invariant` predicate;
   - state enum plus its transition `match` / `switch` → the `lifecycle` state machine.

6. **Upstream StR/US** via the FR's `relationships:` targets — only when the FR states its
   oracle in terms it does not itself define.

## Using non-null spans

When `domain`, `precondition`, or `oracle` is present, `statement[start..end] == text` and
the spans are in-bounds, non-overlapping and ascending. Use `domain.text` to seed the
generator's subject, `precondition.text` as the filter, `oracle.text` as the assertion —
but still do read 5. A span gives you the clause, not an expression.

## Refuse to ground

Record the reason and stop. Never fabricate to fill a hole.

| Reason | Trigger |
| --- | --- |
| `symbol-not-found` | Nothing in the source matches the statement's subject |
| `ambiguous-symbol` | Two or more plausible symbols, no `Verification` column to disambiguate |
| `oracle-is-adjectival` | The predicate is a quality word — "actionable", "clear", "reasonable", "appropriate" — with no observable |
| `unimplemented` | The spec is ahead of the code; the symbol does not exist yet |
| `no-state-machine` | A `lifecycle` criterion whose states and transitions are not enumerable |
| `domain-unbounded` | No type, no constraint, and no example to bound the generator |
| `singleton-domain` | The determiner quantifies over a domain with one element |
| `label-from-mention` | The metamorphic label came from the criterion *naming* a shape, not exhibiting one |
| `criterion-describes-its-test` | The criterion is written as a description of the test that verifies it |
| `static-or-demonstration` | The oracle is a fact about the source tree or an end-to-end narrative, not a function of an input |
| `no-row-id` | `row_id` is null, so no test could be reconciled |

### `singleton-domain` deserves its own note

`property: universal` most often comes from the `universal:determiner` signal — a closed
determiner set (`a`, `an`, `any`, `every`, `each`, `all`, `no`) read at three bounded
sentence positions. English uses the same determiner for a universal and for a single
witness: *"A repeated module root is loaded only once"* quantifies; *"A bareword after
`write` is parsed as a positional"* names one case.

The engine cannot tell these apart without giving up determinism, and it is not supposed
to — that is what this step is for. Ground the domain: if it has one element, the criterion
is a witness. Emit it as a `Unit` test (step 5's *witness* outcome) and record a finding, not as an
unattended property.

This is a **routing** decision, not a complaint about the criterion, and never a reason to
suggest rewording. Record it as a `singleton-domain` finding.

### `label-from-mention`

A criterion that *talks about* a property shape picks up that shape's signal. Real example:

> "A criterion carrying a round-trip, idempotence, ordering or invariant signal classifies
> as that metamorphic shape."

This is labelled `round-trip` because the words are in it. Its actual oracle is a
classification assertion — `classify(x).property == expected` — which is a `universal`, not
a metamorphic law. The tell is that the oracle contains no *pair* of operations to compose.

Re-derive the strategy from the grounded oracle, not from the label, and record
`label-from-mention` so the reclassification is visible on review. Do not report this to
quire-rs as a miss: the classifier is doing exactly what it says on the tin, and the
criterion is a fine criterion.

### `criterion-describes-its-test`

Common in mature specs written alongside their suite:

> "A proptest fuzzes patches across all archetypes in the test corpus and confirms
> `apply_patch` returns a valid document."
> "A static audit (`rg` for `Mutex`/`RwLock`) confirms the parallel parse uses no locks."

The criterion names its own verification method. Generating a second test from it duplicates
work that exists. **Go find the existing test first** — grep the suite for the named symbol
or the described technique. If it exists, record the criterion as *already covered* and, if
the existing test carries no tracking tag, note that adding one is the cheaper fix than
generating anything.

### `static-or-demonstration`

Two shapes turn up regularly and are neither properties nor witnesses:

- **Static** — an assertion about the source tree ("No hand-rolled argument dispatcher
  remains in `src/cli.ts`"). Verified by `Static` in the matrix vocabulary, not by a
  generated test.
- **Demonstration** — an end-to-end narrative, common in StR validation criteria ("An
  author installs a module, sees its types appear, requests them, and validates files
  authored against them"). Verified by an eval or a demo, not by a property.

Both are correct criteria verified by another method. Record the reason and move on.

A refusal is not a finding about the spec. `oracle-is-adjectival` in particular is often a
correct NFR that is verified by inspection rather than by test — say so, and do not suggest
rewording it.

## Output of this step

A grounding record per criterion, or a refusal reason. Grounded records go to step 3;
refusals go to step 5.
