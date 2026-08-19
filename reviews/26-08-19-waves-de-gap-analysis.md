---
id: SR-008
title: "Gap analysis — ADR-0011 Phase 2 Waves D and E (FR-036..FR-041)"
type: SpecReview
analysis: gap-analysis
scope: "spec/matrix.md, spec/evals.md, src/completeness/, src/assurance/, src/evidence/adapters/, src/auditor/, src/commands/"
review_set: subset
relationships:
  - target: "ix://agent-ix/quoin/FR-038"
    type: "reviews"
  - target: "ix://agent-ix/quoin/FR-041"
    type: "references"
---

## Summary

Verified Waves D and E — FR-036 through FR-041 — against the Test Matrix and the source tree, with
the optional semantic review skipped.

**Every vitest-backed row binds.** The gap is entirely in the eval-verified half: 9 of FR-038's
criteria are unbacked, because eval evidence is structurally invisible to the reconciler and nothing
records that the evals ran. They did — TC-EV-054..057, 4/4, live — and the repository has no way to
know that.

## Verdict

**FAIL** — nine matrix criteria have no backing tagged test. Structural and pre-existing (71 such rows
repo-wide, FR-028 among them), and FR-038 added nine to the pile rather than creating the problem.

## Findings

| ID      | Severity | Summary                                                                                     | Refs                                              |
| ------- | -------- | ------------------------------------------------------------------------------------------- | ------------------------------------------------- |
| FND-001 | medium   | Nine FR-038 criteria are unbacked: eval runs mint no symbol and are recorded as no evidence | spec/functional/FR-038-generate-fuzz-harnesses.md |
| FND-002 | medium   | `Eval` is not a declared verification method, so 19 rows have no dischargeable meaning      | spec/evals.md                                     |
| FND-003 | low      | The FR-036 adapter did not name FR-036, and the `FR-003` it names is another repo's         | src/evidence/adapters/audit-script.ts:34          |

## Detail

### FND-001 — the strongest verification leaves the least trace

`quire coverage` reconciles matrix rows against **test symbols in code**. Eval scenarios are data in
`evals/scenarios/index.mjs`; they mint no symbol and can never back a row.

| Document        | Unbacked rows targeting `TC-EV-*` |
| --------------- | --------------------------------- |
| `spec/evals.md` | 52                                |
| `FR-028`        | 10                                |
| `FR-038`        | 9                                 |
| **total**       | **71**                            |

FR-038 is consistent with the pattern FR-028 established, so this is not a regression. What makes it
worth a finding rather than a note is the asymmetry: **the evals for FR-038 were actually run** — a
real agent, through the real CLI, four scenarios, 4/4, and two earlier failures that were genuine and
fixed. That run produced `evals/reports/latest.json` and the repository dropped it.

So the matrix's ✅ on those nine rows rests on a fact recorded nowhere. That is the same shape as
1,014 trace tags binding to nothing, arriving through a verification method the evidence store cannot
hold.

### FND-002 — a method nobody can discharge

```
'Eval' is neither a declared verification_catalog method id nor a declared class,
so nothing can say what discharging it means (19 rows, first in FR-028)
```

An eval **is** a verification method, and arguably the most convincing one here. It has no catalog
entry, so `quoin advise` cannot recommend it, `method-conformance` cannot evaluate it, and
`unknown-method` has nothing to compare against. (`Review` shares the problem on one NFR-005 row.)

FND-001 and FND-002 are one gap seen from two ends, and are filed together as
`agent-ix/quoin#142` with a proposed shape: a catalog entry, plus an adapter over the eval report so
a scenario becomes a recorded run like any other suite.

### FND-003 — a traceability slip, fixed here

`src/evidence/adapters/audit-script.ts` implements FR-036's evidence intake and named FR-034 and
FR-003 but not FR-036. Worse, the `FR-003` it named is **quire-rs's** — it appears inside a quoted
example of a real audit script's output — and quoin has its own FR-003 (_Print usage and
command-scoped help_), so a reader or a grep-based tool resolves it to the wrong requirement.

Fixed: the module header now names FR-036, and both cross-repo references read `quire-rs FR-003-AC-4`.

## Coverage

**Plan completion** — `plan/PLAN-001-spec-correctness` is the only plan bundle and predates this work;
Waves D and E were executed from the epic's ticket list rather than a plan bundle, so step 1 has no
target and is recorded as not applicable rather than passed.

**Matrix verification** — `quire coverage --scope .`: 211 backed of 567, 284 criteria. Every row added
by Waves D and E binds except the nine in FND-001. **No status lies and no untracked symbols** among
them; the one untracked symbol repo-wide (`tests/catalog.test.ts :: aborts (strict) …`) predates this
work.

**Underspecified code** — every source file added by Waves D and E names its owning FR:
`src/completeness/` → FR-037, `src/assurance/` → FR-040, `sbom.ts` → FR-041, `audit-script.ts` →
FR-036 (after FND-003). No reverse gap; no stubs, no placeholder returns.

**Semantic review** — skipped, as requested. Intent↔test↔code agreement was not judged; the two
`high` findings in `SR-007` were found by probing behaviour, which is the closest this pass came to
that question.
