---
id: SR-007
title: "Code review — ADR-0011 Phase 2 Waves D and E (FR-036..FR-041)"
type: SpecReview
analysis: code-review
scope: "src/completeness/, src/assurance/, src/evidence/adapters/, src/auditor/audit.ts, src/commands/, src/advisor/advise.ts, skills/spec-fuzz/, evals/lib/, evals/scenarios/"
review_set: subset
relationships:
  - target: "ix://agent-ix/quoin/FR-040"
    type: "reviews"
---

## Summary

Reviewed Waves D and E as one diff — `984e970..51d0069`, 61 files, +5401 — covering FR-036
(architecture conformance), FR-037 (declared-vocabulary completeness), FR-038 (`spec-fuzz`), FR-039
(mutation threshold), FR-040 (assurance case) and FR-041 (SBOM inventories).

**Three findings, two of them high, and both highs are the same failure the code they live in was
written to prevent.** All three are fixed in this change with regression tests.

## Verdict

**FAIL** — two `high` findings. Both fixed; the verdict records what the review found, not what was
left behind.

## Findings

| ID      | Severity | Summary                                                                                        | Refs                       |
| ------- | -------- | ---------------------------------------------------------------------------------------------- | -------------------------- |
| FND-001 | high     | A requirement refining two claims appeared under one, and the other reported a false statement | src/assurance/graph.ts:160 |
| FND-002 | high     | A mutation floor with no catalog reported every obligation unmeasured, including scored ones   | src/auditor/audit.ts:487   |
| FND-003 | medium   | The completeness sweep re-read the whole bundle once per declaration, against NFR-011-M-2      | src/completeness/run.ts:60 |

## Detail

### FND-001 — the assurance case did the thing it forbids

`nodeFor` shared one `reached` set across every claim, using it both to prevent cycles and to mark
"already rendered". Those are different questions.

Probed rather than reasoned about, with `FR-001` tracing to both `StR-001` and `StR-002`:

```
StR-001 children: [ 'FR-001' ]
StR-002 children: []
unreachable: []
```

`StR-002` rendered as an **open goal with the reason "no sub-claim and no obligation traces to this
claim"** — a statement that is false, in an assurance case, about the very edge its author wrote. And
`unreachable` stayed empty, so nothing anywhere reported it.

FR-040's whole contract is that a case must not narrow silently. It narrowed, and then misreported
why.

**Fixed** by separating the two questions: a per-path `ancestors` set prevents cycles, `reached`
remains global for the unreachable calculation. `TC-237`/`TC-238` pin both halves — a shared child
appears under both parents, and a genuine `A refines B refines A` cycle still terminates.

### FND-002 — the documented behaviour was the opposite of the actual one

`mutationFinding`'s own doc comment, written hours earlier:

> With no such catalog entry, nothing is in scope and the check says nothing rather than something
> wrong.

It said something wrong. `scoresFor` returned `[]` when no catalog declared `mutation-testing`, and
an empty score list falls through to `unmeasured-mutation-score` — so **every obligation carrying a
declared floor was reported unmeasured, including one holding a real `0.95` from `cargo-mutants`**:

```
no-catalog findings: ["unmeasured-mutation-score"]
```

Worth recording how it survived: the tests added with FR-039 all supply a catalog, because the
tool-scoping fix made them, and the no-catalog path had no case. The comment describing the intended
behaviour was written and never executed.

**Fixed**: the check returns `null` when no catalog declares a mutation method, because the question
cannot be asked. `TC-239` pins it.

### FND-003 — one pass, stated and not kept

`assessBundle` called `readBundleClaims` per declaration, and each call walked the bundle and read
every file. N declarations meant N full passes.

`NFR-011-M-2` states the budget as **1 full pass per command invocation**, target and threshold both.
The repository declares one vocabulary today, so nothing was slow and no timing would have moved —
which is the only reason this needed reading rather than measuring.

**Fixed**: `readBundleFrontmatter` is called once and `claimsFor` projects the already-read documents
onto each declaration. `readBundleClaims` remains as the single-declaration convenience.

## Coverage

`make build` ✅ 0 type errors · `make test` ✅ **437 passed** (426 before the fixes, +3 regressions
and the pre-existing suite) · `make lint` ✅ · `quire validate` ✅ clean on every document in the
change · `quire coverage` ✅ every new matrix row binds, no status lie.

The Python-shaped sections of this skill — Test Standards, Mock Compliance, both Completeness
sections, Edge Case & Logic Review — were **not** run as written. This change is TypeScript and
markdown; transliterating them produces false findings that bury real ones. Their language-independent
equivalents were run: Integrity, Spec-Code Faithfulness, Code-Test Alignment and Gap Analysis.

## Method note

Both high findings were found by **writing a probe and running it**, not by reading. Each looked
correct on the page — the diamond case needs three documents to expose, and the no-catalog case
contradicts a comment sitting two lines above it. FND-003 is the reverse: invisible to any
measurement, because the condition that makes it cost anything does not exist in this repository yet.
